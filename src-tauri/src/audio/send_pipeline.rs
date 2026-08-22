//! Bounded audio send pipeline with newest-drop buffering, finite graceful
//! drain, throttled level diagnostics, and a single fell-behind error signal.

use crate::core::diagnostics::milliseconds;
use crate::pipeline_log;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const QUEUE_CAPACITY: usize = 20;

pub struct AudioSendPipeline {
    tx: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    failed: Arc<AtomicBool>,
    accepting: AtomicBool,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
    worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl AudioSendPipeline {
    pub fn spawn<F, Fut>(send_audio: F, on_error: impl Fn(String) + Send + Sync + 'static) -> Self
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send,
    {
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(QUEUE_CAPACITY);
        let failed = Arc::new(AtomicBool::new(false));
        let failed_worker = failed.clone();
        let on_error = Arc::new(on_error);
        let on_error_worker = on_error.clone();

        let worker = tokio::spawn(async move {
            let mut sent_buffer_count: u64 = 0;
            let mut sent_byte_count: u64 = 0;
            let mut peak_audio_sample: i32 = 0;

            while let Some(data) = rx.recv().await {
                let started_at = Instant::now();
                let bytes = data.len();
                peak_audio_sample = peak_audio_sample.max(peak_pcm16_sample(&data));
                let result = send_audio(data).await;
                match result {
                    Ok(()) => {
                        sent_buffer_count += 1;
                        sent_byte_count += bytes as u64;
                        if sent_buffer_count == 1 || sent_buffer_count % 100 == 0 {
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
                    Err(error) => {
                        pipeline_log!("audio send failed error={}", error);
                        if !failed_worker.swap(true, Ordering::SeqCst) {
                            on_error_worker(
                                "Audio streaming fell behind. mimi is reconnecting.".into(),
                            );
                        }
                        return;
                    }
                }
            }
        });

        Self {
            tx: Mutex::new(Some(tx)),
            failed,
            accepting: AtomicBool::new(true),
            on_error,
            worker: Mutex::new(Some(worker)),
        }
    }

    /// Enqueues a PCM buffer; when the bounded queue is full, the newest
    /// buffer is dropped and the pipeline reports the fell-behind error once.
    pub fn enqueue(&self, data: Vec<u8>) {
        if !self.accepting.load(Ordering::SeqCst) {
            return;
        }
        let sender = self.tx.lock().unwrap();
        let Some(sender) = sender.as_ref() else {
            return;
        };
        match sender.try_send(data) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                pipeline_log!("audio queue dropped newest buffer");
                if !self.failed.swap(true, Ordering::SeqCst) {
                    (self.on_error)("Audio streaming fell behind. mimi is reconnecting.".into());
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }

    /// Stops accepting new buffers, closes the channel, and lets the worker
    /// send everything already queued. A stalled transport is aborted when
    /// the finite drain deadline expires.
    pub async fn finish(&self, timeout: Duration) -> bool {
        self.accepting.store(false, Ordering::SeqCst);
        self.tx.lock().unwrap().take();
        let worker = self.worker.lock().unwrap().take();
        let Some(mut worker) = worker else {
            return true;
        };
        match tokio::time::timeout(timeout, &mut worker).await {
            Ok(Ok(())) => true,
            Ok(Err(_)) => false,
            Err(_) => {
                worker.abort();
                false
            }
        }
    }

    pub fn stop(&self) {
        self.accepting.store(false, Ordering::SeqCst);
        self.failed.store(true, Ordering::SeqCst);
        self.tx.lock().unwrap().take();
        if let Some(worker) = self.worker.lock().unwrap().take() {
            worker.abort();
        }
    }
}

/// Absolute peak of a little-endian i16 PCM buffer.
fn peak_pcm16_sample(data: &[u8]) -> i32 {
    let mut peak = 0i32;
    for chunk in data.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
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
                    Ok(())
                }
            },
            |_| {},
        );
        pipeline.enqueue(vec![1, 0]);
        pipeline.enqueue(vec![2, 0]);

        assert!(pipeline.finish(Duration::from_millis(200)).await);
        assert_eq!(sent.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn graceful_finish_aborts_a_stalled_transport_at_the_deadline() {
        let pipeline = AudioSendPipeline::spawn(
            |_data| async move {
                std::future::pending::<()>().await;
                Ok(())
            },
            |_| {},
        );
        pipeline.enqueue(vec![1, 0]);

        let started = tokio::time::Instant::now();
        assert!(!pipeline.finish(Duration::from_millis(30)).await);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn graceful_finish_reports_a_cancelled_worker_as_failure() {
        let pipeline = AudioSendPipeline::spawn(|_data| async move { Ok(()) }, |_| {});
        pipeline.worker.lock().unwrap().as_ref().unwrap().abort();

        assert!(!pipeline.finish(Duration::from_millis(200)).await);
    }
}
