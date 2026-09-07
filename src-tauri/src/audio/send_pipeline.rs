//! Bounded audio send pipeline with newest-drop buffering, finite graceful
//! drain, throttled level diagnostics, and a single fell-behind error signal.

use crate::core::diagnostics::milliseconds;
use crate::pipeline_log;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

const QUEUE_CAPACITY: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AudioPipelineFailure {
    #[error("The audio transport stopped unexpectedly.")]
    TransportStopped,
}

impl AudioPipelineFailure {
    pub fn diagnostic_label(self) -> &'static str {
        match self {
            Self::TransportStopped => "audio_pipeline.transport_stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AudioIngressError {
    #[error("The bounded audio queue is full.")]
    Backpressure,
    #[error("The audio pipeline is closed.")]
    Closed,
}

/// Cloneable, synchronous ingress for native audio callbacks. `try_send`
/// performs no await and holds no mutex; a full queue rejects the newest
/// buffer and permanently closes this generation's ingress.
#[derive(Clone)]
pub struct AudioIngress {
    tx: mpsc::Sender<Vec<u8>>,
    failed: Arc<AtomicBool>,
    accepting: Arc<AtomicBool>,
}

impl AudioIngress {
    pub fn try_send(&self, data: Vec<u8>) -> Result<(), AudioIngressError> {
        if !self.accepting.load(Ordering::SeqCst) {
            return Err(AudioIngressError::Closed);
        }
        match self.tx.try_send(data) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.accepting.store(false, Ordering::SeqCst);
                if !self.failed.swap(true, Ordering::SeqCst) {
                    Err(AudioIngressError::Backpressure)
                } else {
                    Err(AudioIngressError::Closed)
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.accepting.store(false, Ordering::SeqCst);
                self.failed.store(true, Ordering::SeqCst);
                Err(AudioIngressError::Closed)
            }
        }
    }
}

pub struct AudioSendPipeline {
    tx: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    failed: Arc<AtomicBool>,
    accepting: Arc<AtomicBool>,
    finish_tx: Mutex<Option<oneshot::Sender<()>>>,
    worker: Mutex<Option<AbortOnDropTask>>,
    abort_worker: tokio::task::AbortHandle,
}

// A cancelled finish future must not detach a worker that still owns queued
// audio and a provider client. Keep abort ownership while awaiting the task.
struct AbortOnDropTask(tokio::task::JoinHandle<bool>);

impl Drop for AbortOnDropTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl AudioSendPipeline {
    pub fn spawn<F, Fut, E>(
        send_audio: F,
        on_error: impl Fn(AudioPipelineFailure) + Send + Sync + 'static,
    ) -> Self
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(QUEUE_CAPACITY);
        let (finish_tx, mut finish_rx) = oneshot::channel();
        let failed = Arc::new(AtomicBool::new(false));
        let failed_worker = failed.clone();
        let accepting = Arc::new(AtomicBool::new(true));
        let accepting_worker = Arc::clone(&accepting);
        let on_error = Arc::new(on_error);
        let on_error_worker = on_error.clone();

        let worker = tokio::spawn(async move {
            let mut sent_buffer_count: u64 = 0;
            let mut sent_byte_count: u64 = 0;
            let mut peak_audio_sample: i32 = 0;
            let mut finishing = false;

            loop {
                let data = tokio::select! {
                    biased;
                    _ = &mut finish_rx, if !finishing => {
                        finishing = true;
                        // Native callbacks may retain ingress clones until
                        // their asynchronous teardown completes.
                        rx.close();
                        continue;
                    }
                    data = rx.recv() => match data {
                        Some(data) => data,
                        None => return true,
                    },
                };
                let started_at = Instant::now();
                let bytes = data.len();
                peak_audio_sample = peak_audio_sample.max(peak_pcm16_sample(&data));
                let result = send_audio(data).await;
                match result {
                    Ok(()) => {
                        sent_buffer_count += 1;
                        sent_byte_count += bytes as u64;
                        if sent_buffer_count == 1 || sent_buffer_count.is_multiple_of(100) {
                            pipeline_log!(
                                "audio sent buffers={} bytes={} peakDbFS={}",
                                sent_buffer_count,
                                sent_byte_count,
                                decibels_full_scale(peak_audio_sample)
                            );
                            peak_audio_sample = 0;
                        }
                        let send_ms = milliseconds(started_at, Instant::now());
                        if send_ms > 200 {
                            pipeline_log!("audio send blockedMs={} bytes={}", send_ms, bytes);
                        }
                    }
                    Err(_) => {
                        accepting_worker.store(false, Ordering::SeqCst);
                        pipeline_log!("audio send failed label=audio_pipeline.transport_stopped");
                        if !failed_worker.swap(true, Ordering::SeqCst) {
                            on_error_worker(AudioPipelineFailure::TransportStopped);
                        }
                        return false;
                    }
                }
            }
        });

        let abort_worker = worker.abort_handle();
        Self {
            tx: Mutex::new(Some(tx)),
            failed,
            accepting,
            finish_tx: Mutex::new(Some(finish_tx)),
            worker: Mutex::new(Some(AbortOnDropTask(worker))),
            abort_worker,
        }
    }

    pub fn ingress(&self) -> Option<AudioIngress> {
        self.tx.lock().unwrap().as_ref().map(|tx| AudioIngress {
            tx: tx.clone(),
            failed: Arc::clone(&self.failed),
            accepting: Arc::clone(&self.accepting),
        })
    }

    /// Stops accepting new buffers, closes the channel, and lets the worker
    /// send everything already queued. A stalled transport is aborted when
    /// the finite drain deadline expires.
    pub async fn finish(&self, timeout: Duration) -> bool {
        self.accepting.store(false, Ordering::SeqCst);
        self.tx.lock().unwrap().take();
        if let Some(finish_tx) = self.finish_tx.lock().unwrap().take() {
            let _ = finish_tx.send(());
        }
        let worker = self.worker.lock().unwrap().take();
        let Some(mut worker) = worker else {
            return true;
        };
        match tokio::time::timeout(timeout, &mut worker.0).await {
            Ok(Ok(drained)) => drained,
            Ok(Err(_)) => false,
            Err(_) => {
                worker.0.abort();
                false
            }
        }
    }

    pub fn stop(&self) {
        self.accepting.store(false, Ordering::SeqCst);
        self.failed.store(true, Ordering::SeqCst);
        self.tx.lock().unwrap().take();
        self.abort_worker.abort();
        self.worker.lock().unwrap().take();
    }
}

impl Drop for AudioSendPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Absolute peak of a little-endian i16 PCM buffer.
fn peak_pcm16_sample(data: &[u8]) -> i32 {
    let mut peak = 0i32;
    for chunk in data.as_chunks::<2>().0 {
        let sample = i16::from_le_bytes(*chunk) as i32;
        peak = peak.max(sample.abs());
    }
    peak
}

fn decibels_full_scale(peak: i32) -> i32 {
    if peak <= 0 {
        return -96;
    }
    (20.0 * (peak as f64 / 32_768.0).log10()).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn graceful_finish_drains_buffers_already_in_the_queue() {
        let sent = Arc::new(AtomicUsize::new(0));
        let sent_by_worker = Arc::clone(&sent);
        let pipeline = AudioSendPipeline::spawn(
            move |_data| {
                let sent = Arc::clone(&sent_by_worker);
                async move {
                    sent.fetch_add(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                }
            },
            |_| {},
        );
        let ingress = pipeline.ingress().unwrap();
        ingress.try_send(vec![1, 0]).unwrap();
        ingress.try_send(vec![2, 0]).unwrap();
        // Native teardown can retain the callback and its ingress even after
        // capture has stopped; that must not keep the receiver open.
        assert!(pipeline.finish(Duration::from_millis(200)).await);
        assert_eq!(sent.load(Ordering::SeqCst), 2);
        assert_eq!(ingress.try_send(vec![3, 0]), Err(AudioIngressError::Closed));
    }

    #[tokio::test]
    async fn graceful_finish_reports_transport_failure() {
        let pipeline = AudioSendPipeline::spawn(|_data| async { Err::<(), ()>(()) }, |_| {});
        pipeline.ingress().unwrap().try_send(vec![0, 0]).unwrap();

        assert!(!pipeline.finish(Duration::from_millis(200)).await);
    }

    struct ReleaseSignal(Option<oneshot::Sender<()>>);

    impl Drop for ReleaseSignal {
        fn drop(&mut self) {
            let _ = self.0.take().unwrap().send(());
        }
    }

    fn stalled_pipeline() -> (AudioSendPipeline, oneshot::Receiver<()>) {
        let (released_tx, released_rx) = oneshot::channel();
        let guard = Arc::new(ReleaseSignal(Some(released_tx)));
        let pipeline = AudioSendPipeline::spawn(
            move |_data| {
                let guard = Arc::clone(&guard);
                async move {
                    let _guard = guard;
                    std::future::pending::<Result<(), ()>>().await
                }
            },
            |_| {},
        );
        pipeline.ingress().unwrap().try_send(vec![0, 0]).unwrap();
        (pipeline, released_rx)
    }

    #[tokio::test]
    async fn dropping_pipeline_releases_a_stalled_worker() {
        let (pipeline, released) = stalled_pipeline();
        let ingress = pipeline.ingress().unwrap();
        drop(pipeline);

        tokio::time::timeout(Duration::from_secs(1), released)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ingress.try_send(vec![0, 0]), Err(AudioIngressError::Closed));
    }

    #[tokio::test]
    async fn cancelling_finish_releases_a_stalled_worker() {
        let (pipeline, released) = stalled_pipeline();
        // Timing out the caller drops finish after it has taken ownership of
        // the worker handle, before the pipeline's own drain deadline.
        assert!(tokio::time::timeout(
            Duration::from_millis(20),
            pipeline.finish(Duration::from_secs(60)),
        )
        .await
        .is_err());

        tokio::time::timeout(Duration::from_secs(1), released)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn stop_can_abort_a_worker_already_owned_by_finish() {
        let (pipeline, released) = stalled_pipeline();
        let finish = pipeline.finish(Duration::from_secs(60));
        tokio::pin!(finish);
        assert!(tokio::time::timeout(Duration::from_millis(20), &mut finish)
            .await
            .is_err());
        pipeline.stop();

        assert!(!finish.await);
        released.await.unwrap();
    }

    #[tokio::test]
    async fn graceful_finish_aborts_a_stalled_transport_at_the_deadline() {
        let pipeline = AudioSendPipeline::spawn(
            |_data| async move {
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            },
            |_| {},
        );
        let ingress = pipeline.ingress().unwrap();
        ingress.try_send(vec![1, 0]).unwrap();
        drop(ingress);

        let started = tokio::time::Instant::now();
        assert!(!pipeline.finish(Duration::from_millis(30)).await);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn graceful_finish_reports_a_cancelled_worker_as_failure() {
        let pipeline = AudioSendPipeline::spawn(|_data| async move { Ok::<(), ()>(()) }, |_| {});
        pipeline.abort_worker.abort();

        assert!(!pipeline.finish(Duration::from_millis(200)).await);
    }

    #[tokio::test]
    async fn native_ingress_is_non_blocking_and_fails_closed_when_full() {
        let pipeline = AudioSendPipeline::spawn(
            |_data| async move {
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            },
            |_| {},
        );
        let ingress = pipeline.ingress().unwrap();
        assert_eq!(ingress.try_send(vec![0, 0]), Ok(()));

        let mut accepted = 1;
        while ingress.try_send(vec![0, 0]) == Ok(()) {
            accepted += 1;
            assert!(accepted <= QUEUE_CAPACITY + 1);
        }

        assert!(accepted <= QUEUE_CAPACITY + 1);
        assert_eq!(ingress.try_send(vec![0, 0]), Err(AudioIngressError::Closed));
        pipeline.stop();
    }

    #[tokio::test]
    async fn queue_backpressure_is_reported_exactly_once_by_ingress() {
        let pipeline = AudioSendPipeline::spawn(
            |_data| async move {
                std::future::pending::<()>().await;
                Ok::<(), ()>(())
            },
            |_| {},
        );
        let ingress = pipeline.ingress().unwrap();
        let mut backpressure_count = 0;
        for _ in 0..QUEUE_CAPACITY + 4 {
            if ingress.try_send(vec![0, 0]) == Err(AudioIngressError::Backpressure) {
                backpressure_count += 1;
            }
        }

        assert_eq!(backpressure_count, 1);
        pipeline.stop();
    }
}
