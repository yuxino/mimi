//! Bounded audio send pipeline, ported from `AppModel.AudioSendPipeline`:
//! newest-drop buffering with a 20-slot queue, peak-level logging every
//! 1/100 buffers, and a single fell-behind error signal.

use crate::core::diagnostics::milliseconds;
use crate::pipeline_log;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

const QUEUE_CAPACITY: usize = 20;

pub struct AudioSendPipeline {
    tx: mpsc::Sender<Vec<u8>>,
    failed: Arc<AtomicBool>,
    on_error: Arc<dyn Fn(String) + Send + Sync>,
    worker: tokio::task::JoinHandle<()>,
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
            tx,
            failed,
            on_error,
            worker,
        }
    }

    /// Enqueues a PCM buffer; when the bounded queue is full, the newest
    /// buffer is dropped and the pipeline reports the fell-behind error once.
    pub fn enqueue(&self, data: Vec<u8>) {
        match self.tx.try_send(data) {
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

    pub fn stop(&self) {
        self.failed.store(true, Ordering::SeqCst);
        self.worker.abort();
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
