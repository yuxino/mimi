//! High-quality translation pipeline: Audio 3.0 recognition + Qwen-MT
//! translation with draft stabilization, preview preemption, translation
//! memory, and provisional-commit replacement. Ported 1:1 from
//! `Sources/MimiCore/HighQualityTranslationClient.swift`.

use crate::clients::audio3_client::Audio3ASRClient;
use crate::clients::qwen_mt_client::QwenMTClient;
use crate::core::committer::{final_covers_chunk, ASRDraftCommitter, FinishOutcome};
use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::protocols::qwen_mt::{QwenMTClientError, QwenMTMemoryPair, QwenMTModel};
use crate::core::subtitle_reducer::trim;
use crate::pipeline_log;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Clone)]
struct TranslationRequest {
    text: String,
    language: Option<String>,
    /// How the source text was confirmed: "server-final" (authoritative),
    /// "stable-draft" (provisional local commit), "maximum-wait" or
    /// "session-finish" (last-resort flushes). Provisional items may be
    /// coalesced or shed under backlog; authoritative ones never are.
    boundary: &'static str,
    enqueued_at: tokio::time::Instant,
}

struct Inner {
    committer: ASRDraftCommitter,
    latest_draft_language: Option<String>,
    final_queue: VecDeque<TranslationRequest>,
    translation_memory: Vec<QwenMTMemoryPair>,
    pending_revoke_count: usize,
    draft_stability_task: Option<JoinHandle<()>>,
    draft_maximum_wait_task: Option<JoinHandle<()>>,
    final_worker: Option<JoinHandle<()>>,
    asr_bridge: Option<JoinHandle<()>>,
    /// Throttles the per-draft diagnostic log (drafts stream several times
    /// per second while speech flows).
    last_draft_log_at: Option<tokio::time::Instant>,
}

#[derive(Clone)]
pub struct HighQualityTranslationClient {
    asr_client: Audio3ASRClient,
    mt: Arc<QwenMTClient>,
    source_language: SourceLanguage,
    translates_audio: bool,
    events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
    inner: Arc<Mutex<Inner>>,
    stable_draft_delay: Duration,
    maximum_wait_delay: Duration,
    streams_finals: bool,
}

impl HighQualityTranslationClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        api_key: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        final_model: QwenMTModel,
        stable_draft_delay: Duration,
        maximum_wait_delay: Duration,
        long_incomplete_commit_threshold: usize,
        events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
    ) -> Result<Self, QwenMTClientError> {
        let asr_client = Audio3ASRClient::new(workspace_id, api_key, source_language)
            .map_err(|_| QwenMTClientError::MissingAPIKey)?;
        let streams_finals = final_model != QwenMTModel::Plus;
        let domain_hint = Some(
            crate::core::protocols::qwen_mt::QwenMTDomainHint::spoken_dialogue(
                source_language,
                target_language,
            ),
        );
        let filler_terms = crate::core::protocols::qwen_mt::QwenMTDomainHint::filler_terms(
            source_language,
            target_language,
        );
        let mt = QwenMTClient::new(
            workspace_id,
            api_key,
            source_language,
            target_language,
            final_model,
            domain_hint,
            filler_terms,
            Duration::from_secs(8),
        )?;
        Ok(Self {
            asr_client,
            mt: Arc::new(mt),
            source_language,
            translates_audio: target_language.translates_audio(),
            events,
            inner: Arc::new(Mutex::new(Inner {
                committer: ASRDraftCommitter::new(long_incomplete_commit_threshold),
                latest_draft_language: None,
                final_queue: VecDeque::new(),
                translation_memory: Vec::new(),
                pending_revoke_count: 0,
                draft_stability_task: None,
                draft_maximum_wait_task: None,
                final_worker: None,
                asr_bridge: None,
                last_draft_log_at: None,
            })),
            stable_draft_delay,
            maximum_wait_delay,
            streams_finals,
        })
    }

    /// Connects the recognizer and resets all draft/final workers.
    pub async fn connect(&self) -> Result<(), QwenMTClientError> {
        self.reset_draft_finalization().await;
        self.cancel_final_translations().await;
        self.disconnect_asr_bridge().await;

        let task_id = Uuid::new_v4().simple().to_string();
        let (asr_tx, mut asr_rx) = mpsc::unbounded_channel();
        self.asr_client.set_event_sender(asr_tx).await;
        self.asr_client.connect(&task_id).await.map_err(|error| {
            QwenMTClientError::RequestFailed {
                status_code: 0,
                message: error.to_string(),
            }
        })?;

        // Bridge ASR events into this pipeline's handler.
        let self_arc = self.clone();
        let bridge = tokio::spawn(async move {
            while let Some(event) = asr_rx.recv().await {
                self_arc.handle_asr_event(event).await;
            }
        });
        self.inner.lock().await.asr_bridge = Some(bridge);
        Ok(())
    }

    pub async fn send_audio(&self, pcm_data: &[u8]) -> Result<(), QwenMTClientError> {
        self.asr_client.send_audio(pcm_data).await.map_err(|error| {
            QwenMTClientError::RequestFailed {
                status_code: 0,
                message: error.to_string(),
            }
        })
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), QwenMTClientError> {
        self.asr_client
            .ping(timeout)
            .await
            .map_err(|error| QwenMTClientError::RequestFailed {
                status_code: 0,
                message: error.to_string(),
            })
    }

    pub async fn finish(&self) {
        // Snappy stop: allow ~1s for the ASR teardown plus ~3s for the
        // in-flight translation; unfinished work is dropped afterwards
        // instead of blocking the stop for up to 35s.
        self.asr_client.finish(Duration::from_secs(1)).await;
        self.commit_pending_draft("session-finish").await;
        self.wait_for_final_translations(Duration::from_secs(3))
            .await;
        self.reset_draft_finalization().await;
        self.cancel_final_translations().await;
        self.disconnect_asr_bridge().await;
    }

    pub async fn disconnect(&self) {
        self.reset_draft_finalization().await;
        self.cancel_final_translations().await;
        self.asr_client.disconnect().await;
        self.disconnect_asr_bridge().await;
    }

    // MARK: ASR event handling

    async fn handle_asr_event(&self, event: LiveTranslateServerEvent) {
        match event {
            LiveTranslateServerEvent::SourceDraft { text, language } => {
                let text = trim(&text);
                if text.is_empty() {
                    return;
                }
                let (uncommitted_text, has_pending) = {
                    let mut inner = self.inner.lock().await;
                    let uncommitted = inner.committer.update_draft(&text);
                    inner.latest_draft_language = language.clone();
                    let has_pending = inner.committer.has_pending_text();
                    (uncommitted, has_pending)
                };
                // Throttled: drafts stream several times per second while
                // speech flows; one line per second is enough to see the
                // pipeline shape without log flood.
                let now = tokio::time::Instant::now();
                let log_due = self
                    .inner
                    .lock()
                    .await
                    .last_draft_log_at
                    .is_none_or(|at| now.duration_since(at).as_millis() >= 1000);
                if log_due {
                    self.inner.lock().await.last_draft_log_at = Some(now);
                    pipeline_log!(
                        "audio3 asr draft length={} pendingLength={} language={}",
                        text.chars().count(),
                        uncommitted_text.chars().count(),
                        language
                            .as_deref()
                            .unwrap_or(self.source_language.raw_value())
                    );
                }
                if !has_pending {
                    return;
                }
                self.schedule_draft_finalization().await;
                if !self.translates_audio {
                    self.emit(LiveTranslateServerEvent::SourceDraft {
                        text: uncommitted_text.clone(),
                        language: language.clone(),
                    });
                    self.emit(LiveTranslateServerEvent::TranslationDraft(uncommitted_text));
                } else {
                    // High-quality mode shows only confirmed finals unless no
                    // final translation is in flight.
                    let worker_idle = {
                        let inner = self.inner.lock().await;
                        inner.final_worker.is_none() && inner.final_queue.is_empty()
                    };
                    if worker_idle {
                        self.emit(LiveTranslateServerEvent::SourceDraft {
                            text: uncommitted_text,
                            language,
                        });
                    }
                }
            }
            LiveTranslateServerEvent::SourceFinal { text, language } => {
                let text = trim(&text);
                if text.is_empty() {
                    return;
                }
                self.cancel_draft_timers().await;
                self.inner.lock().await.latest_draft_language = None;

                let (uncommitted_text, was_replaced) = {
                    let mut inner = self.inner.lock().await;
                    match inner.committer.finish_sentence(&text) {
                        FinishOutcome::None => (None, false),
                        FinishOutcome::Appended(new_text) => (Some(new_text), false),
                        FinishOutcome::Replaced(new_text) => (Some(new_text), true),
                    }
                };
                pipeline_log!(
                    "audio3 asr final length={} pendingLength={} language={} queuedFinals={}",
                    text.chars().count(),
                    uncommitted_text
                        .as_ref()
                        .map(|t| t.chars().count())
                        .unwrap_or(0),
                    language
                        .as_deref()
                        .unwrap_or(self.source_language.raw_value()),
                    self.inner.lock().await.final_queue.len()
                );
                let Some(uncommitted_text) = uncommitted_text else {
                    pipeline_log!("audio3 asr final deduplicated");
                    return;
                };
                if was_replaced {
                    pipeline_log!("audio3 asr final superseded provisional");
                    if self.translates_audio {
                        // Translated mode commits through the serial final
                        // worker; revoke there, right before the authoritative
                        // replacement, so the provisional history entry has
                        // already landed.
                        self.inner.lock().await.pending_revoke_count += 1;
                    } else {
                        self.emit(LiveTranslateServerEvent::SubtitleRevoked);
                    }
                }
                self.enqueue_confirmed_source(
                    uncommitted_text,
                    language,
                    "server-final",
                    was_replaced,
                )
                .await;
            }
            other => self.emit(other),
        }
    }

    async fn schedule_draft_finalization(&self) {
        {
            let inner = self.inner.lock().await;
            if inner.draft_stability_task.is_some() && inner.draft_maximum_wait_task.is_some() {
                return;
            }
        }

        if self.inner.lock().await.draft_stability_task.is_none() {
            let self_arc = self.clone();
            let delay = self.stable_draft_delay;
            let task = tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                self_arc.commit_pending_draft("stable-draft").await;
            });
            self.inner.lock().await.draft_stability_task = Some(task);
        }

        if self.inner.lock().await.draft_maximum_wait_task.is_none() {
            let self_arc = self.clone();
            let delay = self.maximum_wait_delay;
            let task = tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                self_arc.commit_pending_draft("maximum-wait").await;
            });
            self.inner.lock().await.draft_maximum_wait_task = Some(task);
        }
    }

    async fn commit_pending_draft(&self, boundary: &'static str) {
        self.cancel_draft_timers().await;
        let text = {
            let mut inner = self.inner.lock().await;
            if boundary == "maximum-wait" || boundary == "session-finish" {
                inner.committer.commit_latest_draft(true)
            } else {
                inner.committer.commit_complete_sentences()
            }
        };
        let Some(text) = text else { return };
        let language = self.inner.lock().await.latest_draft_language.clone();
        pipeline_log!(
            "audio3 asr local final boundary={} length={} language={}",
            boundary,
            text.chars().count(),
            language
                .as_deref()
                .unwrap_or(self.source_language.raw_value())
        );
        self.enqueue_confirmed_source(text, language, boundary, false)
            .await;
    }

    async fn enqueue_confirmed_source(
        &self,
        text: String,
        language: Option<String>,
        boundary: &'static str,
        supersedes_provisional: bool,
    ) {
        if !self.translates_audio {
            self.emit(LiveTranslateServerEvent::SourceFinal {
                text: text.clone(),
                language: language.clone(),
            });
            self.emit(LiveTranslateServerEvent::TranslationFinal(text));
            return;
        }

        {
            let mut inner = self.inner.lock().await;
            // Coalesce: a server final that covers the queued local commit
            // replaces it in place. The provisional was never shown, so one
            // translation round-trip and one duplicate history row are both
            // avoided. (When the tail translation already started, the
            // provisional revoke path handles the replacement instead.)
            if boundary == "server-final" {
                if let Some(tail) = inner.final_queue.back_mut() {
                    if tail.boundary != "server-final" && final_covers_chunk(&text, &tail.text) {
                        tail.text = text;
                        tail.language = language;
                        tail.boundary = boundary;
                        if supersedes_provisional && inner.pending_revoke_count > 0 {
                            // The provisional never reached the screen; the
                            // revoke reserved for it must not fire.
                            inner.pending_revoke_count -= 1;
                        }
                        pipeline_log!("mt plus final coalesced depth={}", inner.final_queue.len());
                        return;
                    }
                }
            }
            // Shed the oldest still-queued provisional commit once the queue
            // is deep. Authoritative server finals and last-resort flushes
            // are never dropped; provisional stable-drafts are almost always
            // superseded by the server final that follows within a second,
            // and dropping them keeps latency bounded instead of letting a
            // speech burst build an ever-growing backlog.
            const MAX_QUEUE_DEPTH: usize = 3;
            if inner.final_queue.len() >= MAX_QUEUE_DEPTH {
                if let Some(position) = inner
                    .final_queue
                    .iter()
                    .position(|request| request.boundary == "stable-draft")
                {
                    let shed = inner
                        .final_queue
                        .remove(position)
                        .expect("position comes from the same queue");
                    pipeline_log!(
                        "mt plus final shed boundary={} depth={}",
                        shed.boundary,
                        inner.final_queue.len()
                    );
                }
            }
            inner.final_queue.push_back(TranslationRequest {
                text,
                language,
                boundary,
                enqueued_at: tokio::time::Instant::now(),
            });
            pipeline_log!(
                "mt plus final enqueued boundary={} depth={}",
                boundary,
                inner.final_queue.len()
            );
        }
        self.start_final_worker_if_needed().await;
    }

    async fn cancel_draft_timers(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(task) = inner.draft_stability_task.take() {
            task.abort();
        }
        if let Some(task) = inner.draft_maximum_wait_task.take() {
            task.abort();
        }
    }

    async fn reset_draft_finalization(&self) {
        self.cancel_draft_timers().await;
        let mut inner = self.inner.lock().await;
        inner.committer.reset();
        inner.latest_draft_language = None;
        inner.pending_revoke_count = 0;
    }

    async fn start_final_worker_if_needed(&self) {
        let mut inner = self.inner.lock().await;
        if inner.final_worker.is_some() {
            return;
        }
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            self_arc.run_final_worker().await;
        });
        inner.final_worker = Some(task);
    }

    async fn run_final_worker(&self) {
        loop {
            let request = {
                let mut inner = self.inner.lock().await;
                match inner.final_queue.pop_front() {
                    Some(request) => request,
                    None => {
                        inner.final_worker = None;
                        return;
                    }
                }
            };

            let started_at = tokio::time::Instant::now();
            pipeline_log!(
                "mt plus final started waitMs={} remaining={}",
                started_at
                    .saturating_duration_since(request.enqueued_at)
                    .as_millis(),
                self.inner.lock().await.final_queue.len()
            );
            self.emit(LiveTranslateServerEvent::TranslationStarted);
            self.emit(LiveTranslateServerEvent::SourceFinal {
                text: request.text.clone(),
                language: request.language.clone(),
            });

            match self
                .translate_with_retry(&request.text, request.language.as_deref())
                .await
            {
                Ok(translation) => {
                    {
                        let mut inner = self.inner.lock().await;
                        if inner.pending_revoke_count > 0 {
                            // The previous item was a provisional local commit
                            // that the server final superseded. Revoke it
                            // immediately before the authoritative replacement
                            // lands so history holds the sentence once.
                            inner.pending_revoke_count -= 1;
                            drop(inner);
                            self.emit(LiveTranslateServerEvent::SubtitleRevoked);
                        }
                    }
                    pipeline_log!(
                        "mt plus final completed requestMs={} remaining={}",
                        started_at.elapsed().as_millis(),
                        self.inner.lock().await.final_queue.len()
                    );
                    self.emit(LiveTranslateServerEvent::TranslationFinal(
                        translation.clone(),
                    ));
                    self.remember(&request.text, &translation).await;
                }
                Err(error) => {
                    pipeline_log!(
                        "mt plus final failed requestMs={} error={}",
                        started_at.elapsed().as_millis(),
                        error.diagnostic_label()
                    );
                    self.handle_translation_failure(&error).await;
                    return;
                }
            }
        }
    }

    async fn handle_translation_failure(&self, error: &QwenMTClientError) {
        let code = if error.is_authentication_failure() {
            "translation_authentication_failed"
        } else {
            "translation_failed"
        };
        self.emit(LiveTranslateServerEvent::Error {
            code: code.into(),
            message: error.to_string(),
        });
    }

    async fn translate_with_retry(
        &self,
        text: &str,
        language: Option<&str>,
    ) -> Result<String, QwenMTClientError> {
        let memory = {
            let inner = self.inner.lock().await;
            inner
                .translation_memory
                .iter()
                .rev()
                .take(6)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
        };
        let source_override = if self.source_language == SourceLanguage::Automatic {
            language.and_then(|l| SourceLanguage::from_detected(Some(l)))
        } else {
            None
        };
        let mut attempt = 1usize;

        loop {
            let result = if self.streams_finals {
                let mt = self.mt.clone();
                let events = self.events.clone();
                let text_owned = text.to_string();
                mt.translate_streaming(&text_owned, source_override, &memory, move |partial| {
                    let _ = events.send(LiveTranslateServerEvent::TranslationDraft(partial));
                })
                .await
            } else {
                self.mt.translate(text, source_override, &memory).await
            };

            match result {
                Ok(translation) => return Ok(translation),
                Err(error) => {
                    let Some(delay) =
                        crate::core::protocols::qwen_mt::QwenMTRetryPolicy::delay(&error, attempt)
                    else {
                        return Err(error);
                    };
                    pipeline_log!(
                        "mt plus final retrying attempt={} delayMs={} error={}",
                        attempt,
                        delay.as_millis(),
                        error.diagnostic_label()
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    async fn remember(&self, source: &str, translation: &str) {
        let mut inner = self.inner.lock().await;
        inner
            .translation_memory
            .retain(|pair| pair.source != source);
        inner.translation_memory.push(QwenMTMemoryPair {
            source: source.to_string(),
            target: translation.to_string(),
        });
        if inner.translation_memory.len() > 12 {
            let overflow = inner.translation_memory.len() - 12;
            inner.translation_memory.drain(0..overflow);
        }
    }

    async fn cancel_final_translations(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(task) = inner.final_worker.take() {
            task.abort();
        }
        inner.final_queue.clear();
        inner.translation_memory.clear();
        inner.pending_revoke_count = 0;
    }

    async fn wait_for_final_translations(&self, timeout: Duration) {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let has_worker = self.inner.lock().await.final_worker.is_some();
            if !has_worker || tokio::time::Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn disconnect_asr_bridge(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(task) = inner.asr_bridge.take() {
            task.abort();
        }
    }

    fn emit(&self, event: LiveTranslateServerEvent) {
        let _ = self.events.send(event);
    }
}
