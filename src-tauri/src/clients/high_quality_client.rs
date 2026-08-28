//! High-quality translation pipeline: Audio 3.0 recognition + Qwen-MT.
//!
//! Replaceable ASR drafts use a latest-only preview lane. Only authoritative
//! server finals (plus a bounded session-finish fallback) enter the durable,
//! serial final-translation queue.

use crate::clients::audio3_client::Audio3ASRClient;
use crate::clients::provider_events::{
    provider_event_channel, ProviderEventReceiver, ProviderEventSender,
};
use crate::clients::qwen_mt_client::QwenMTClient;
use crate::core::committer::ASRDraftCommitter;
use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::protocols::qwen_mt::{QwenMTClientError, QwenMTMemoryPair, QwenMTModel};
use crate::core::subtitle_reducer::trim;
use crate::pipeline_log;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

const MAX_FINAL_QUEUE_DEPTH: usize = 3;
const MAX_FINAL_REQUEST_AGE: Duration = Duration::from_secs(45);
const MAX_PREVIEW_REQUEST_AGE: Duration = Duration::from_secs(12);
const MAX_TRANSLATION_ATTEMPTS: usize = 3;
const ASR_BRIDGE_FINISH_TIMEOUT: Duration = Duration::from_secs(1);
const OVERLOAD_ERROR_CODE: &str = "translation_backlog_overflow";
const OVERLOAD_ERROR_MESSAGE: &str = "Translation fell behind live audio. mimi is reconnecting.";

type PartialHandler = Arc<dyn Fn(String) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FinalBoundary {
    ServerFinal,
    SessionFinish,
}

impl FinalBoundary {
    fn label(self) -> &'static str {
        match self {
            Self::ServerFinal => "server-final",
            Self::SessionFinish => "session-finish",
        }
    }
}

#[derive(Clone)]
struct TranslationRequest {
    text: String,
    language: Option<String>,
    boundary: FinalBoundary,
    utterance_revision: u64,
    enqueued_at: tokio::time::Instant,
}

impl TranslationRequest {
    fn key(&self) -> FinalRequestKey {
        FinalRequestKey {
            text: self.text.clone(),
            boundary: self.boundary,
            utterance_revision: self.utterance_revision,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct FinalRequestKey {
    text: String,
    boundary: FinalBoundary,
    utterance_revision: u64,
}

impl FinalRequestKey {
    fn matches(&self, request: &TranslationRequest) -> bool {
        self.text == request.text
            && (self.utterance_revision == request.utterance_revision
                || self.boundary == FinalBoundary::SessionFinish
                || request.boundary == FinalBoundary::SessionFinish)
    }
}

struct TaskSlot {
    id: u64,
    handle: JoinHandle<()>,
}

#[derive(Default)]
struct ASRBridgeState {
    task: Option<JoinHandle<()>>,
    finish_ack: Option<oneshot::Receiver<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DraftTimerKind {
    Stable,
    Maximum,
}

enum EnqueueOutcome {
    Queued,
    Overloaded,
    DeferredOverload,
}

struct Inner {
    committer: ASRDraftCommitter,
    latest_draft_language: Option<String>,
    draft_revision: u64,
    /// `(text, revision_after_final)`. An identical final with no intervening
    /// draft is a duplicate; the same spoken line after a new draft is not.
    last_server_final: Option<(String, u64)>,
    final_queue: VecDeque<TranslationRequest>,
    active_final: Option<FinalRequestKey>,
    translation_memory: Vec<QwenMTMemoryPair>,
    draft_stability_task: Option<TaskSlot>,
    draft_maximum_wait_task: Option<TaskSlot>,
    preview_task: Option<TaskSlot>,
    final_worker: Option<TaskSlot>,
    next_task_id: u64,
    pipeline_failed: bool,
    final_completion_in_progress: bool,
    deferred_overload: bool,
    /// Throttles the per-draft diagnostic log (drafts stream several times
    /// per second while speech flows).
    last_draft_log_at: Option<tokio::time::Instant>,
}

impl Inner {
    fn final_lane_busy(&self) -> bool {
        self.final_worker.is_some()
            || self.active_final.is_some()
            || !self.final_queue.is_empty()
            || self.final_completion_in_progress
    }
}

#[derive(Clone)]
pub struct HighQualityTranslationClient {
    asr_client: Audio3ASRClient,
    mt: Arc<QwenMTClient>,
    source_language: SourceLanguage,
    translates_audio: bool,
    events: ProviderEventSender,
    inner: Arc<Mutex<Inner>>,
    asr_bridge: Arc<Mutex<ASRBridgeState>>,
    preview_epoch: Arc<AtomicU64>,
    stable_draft_delay: Duration,
    maximum_wait_delay: Duration,
    streams_finals: bool,
}

impl HighQualityTranslationClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        final_model: QwenMTModel,
        stable_draft_delay: Duration,
        maximum_wait_delay: Duration,
        long_incomplete_commit_threshold: usize,
        events: ProviderEventSender,
    ) -> Result<Self, QwenMTClientError> {
        let asr_client = Audio3ASRClient::new(api_key, source_language)
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
                draft_revision: 0,
                last_server_final: None,
                final_queue: VecDeque::new(),
                active_final: None,
                translation_memory: Vec::new(),
                draft_stability_task: None,
                draft_maximum_wait_task: None,
                preview_task: None,
                final_worker: None,
                next_task_id: 0,
                pipeline_failed: false,
                final_completion_in_progress: false,
                deferred_overload: false,
                last_draft_log_at: None,
            })),
            asr_bridge: Arc::new(Mutex::new(ASRBridgeState::default())),
            preview_epoch: Arc::new(AtomicU64::new(0)),
            stable_draft_delay,
            maximum_wait_delay,
            streams_finals,
        })
    }

    /// Connects the recognizer and resets all draft/final workers.
    pub async fn connect(&self) -> Result<(), QwenMTClientError> {
        self.reset_draft_state().await;
        self.cancel_final_translations().await;
        self.disconnect_asr_bridge().await;

        let task_id = Uuid::new_v4().simple().to_string();
        let (asr_tx, asr_rx) = provider_event_channel();
        self.asr_client.set_event_sender(asr_tx).await;
        self.asr_client.connect(&task_id).await.map_err(|error| {
            QwenMTClientError::RequestFailed {
                status_code: 0,
                message: error.to_string(),
            }
        })?;

        self.install_asr_bridge(asr_rx).await;
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
        self.asr_client.finish(Duration::from_secs(1)).await;
        if !self
            .wait_for_asr_bridge_finish(ASR_BRIDGE_FINISH_TIMEOUT)
            .await
        {
            pipeline_log!("audio3 asr bridge finish timed out");
        }
        self.flush_pending_draft().await;
        self.wait_for_final_translations(Duration::from_secs(3))
            .await;
        self.reset_draft_state().await;
        self.cancel_final_translations().await;
        self.disconnect_asr_bridge().await;
    }

    pub async fn disconnect(&self) {
        self.reset_draft_state().await;
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
                let now = tokio::time::Instant::now();
                let (uncommitted_text, has_pending, revision, log_due) = {
                    let mut inner = self.inner.lock().await;
                    let uncommitted = inner.committer.update_draft(&text);
                    inner.latest_draft_language = language.clone();
                    next_nonzero(&mut inner.draft_revision);
                    let has_pending = inner.committer.has_pending_text();
                    let log_due = inner
                        .last_draft_log_at
                        .is_none_or(|at| now.duration_since(at).as_millis() >= 1000);
                    if log_due {
                        inner.last_draft_log_at = Some(now);
                    }
                    (uncommitted, has_pending, inner.draft_revision, log_due)
                };
                if log_due {
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

                if !self.translates_audio {
                    self.emit(LiveTranslateServerEvent::SourceDraft {
                        text: uncommitted_text.clone(),
                        language: language.clone(),
                    });
                    self.emit(LiveTranslateServerEvent::TranslationDraft(uncommitted_text));
                } else {
                    self.schedule_draft_finalization(revision).await;
                }
            }
            LiveTranslateServerEvent::SourceFinal { text, language } => {
                let text = trim(&text);
                if text.is_empty() {
                    return;
                }
                let Some(utterance_revision) = self.prepare_server_final(&text).await else {
                    pipeline_log!("audio3 asr final deduplicated");
                    return;
                };
                pipeline_log!(
                    "audio3 asr final length={} language={} queuedFinals={}",
                    text.chars().count(),
                    language
                        .as_deref()
                        .unwrap_or(self.source_language.raw_value()),
                    self.inner.lock().await.final_queue.len()
                );
                self.enqueue_final(
                    text,
                    language,
                    FinalBoundary::ServerFinal,
                    utterance_revision,
                )
                .await;
            }
            other => self.emit(other),
        }
    }

    async fn prepare_server_final(&self, text: &str) -> Option<u64> {
        let mut inner = self.inner.lock().await;
        abort_task(&mut inner.draft_stability_task);
        abort_task(&mut inner.draft_maximum_wait_task);
        abort_task(&mut inner.preview_task);
        self.advance_preview_epoch();

        let utterance_revision = inner.draft_revision;
        let duplicate = inner
            .last_server_final
            .as_ref()
            .is_some_and(|(last, revision)| last == text && *revision == utterance_revision);
        inner.committer.reset();
        inner.latest_draft_language = None;
        if duplicate {
            return None;
        }

        next_nonzero(&mut inner.draft_revision);
        inner.last_server_final = Some((text.to_string(), inner.draft_revision));
        Some(utterance_revision)
    }

    // MARK: Replaceable preview lane

    async fn schedule_draft_finalization(&self, revision: u64) {
        let mut inner = self.inner.lock().await;
        if inner.pipeline_failed
            || revision != inner.draft_revision
            || !inner.committer.has_pending_text()
            || inner.final_lane_busy()
        {
            return;
        }

        abort_task(&mut inner.draft_stability_task);
        let stable_id = next_nonzero(&mut inner.next_task_id);
        let stable_self = self.clone();
        let stable_delay = self.stable_draft_delay;
        let stable_task = tokio::spawn(async move {
            tokio::time::sleep(stable_delay).await;
            stable_self
                .handle_draft_timer(DraftTimerKind::Stable, stable_id, revision)
                .await;
        });
        inner.draft_stability_task = Some(TaskSlot {
            id: stable_id,
            handle: stable_task,
        });

        if inner.draft_maximum_wait_task.is_none() {
            let maximum_id = next_nonzero(&mut inner.next_task_id);
            let maximum_self = self.clone();
            let maximum_delay = self.maximum_wait_delay;
            let maximum_task = tokio::spawn(async move {
                tokio::time::sleep(maximum_delay).await;
                maximum_self
                    .handle_draft_timer(DraftTimerKind::Maximum, maximum_id, revision)
                    .await;
            });
            inner.draft_maximum_wait_task = Some(TaskSlot {
                id: maximum_id,
                handle: maximum_task,
            });
        }
    }

    async fn handle_draft_timer(
        &self,
        kind: DraftTimerKind,
        timer_id: u64,
        scheduled_revision: u64,
    ) {
        let preview = {
            let mut inner = self.inner.lock().await;
            let slot_matches = match kind {
                DraftTimerKind::Stable => inner
                    .draft_stability_task
                    .as_ref()
                    .is_some_and(|slot| slot.id == timer_id),
                DraftTimerKind::Maximum => inner
                    .draft_maximum_wait_task
                    .as_ref()
                    .is_some_and(|slot| slot.id == timer_id),
            };
            if !slot_matches {
                return;
            }

            match kind {
                DraftTimerKind::Stable => {
                    drop(inner.draft_stability_task.take());
                    if scheduled_revision != inner.draft_revision {
                        return;
                    }
                }
                DraftTimerKind::Maximum => {
                    drop(inner.draft_maximum_wait_task.take());
                    abort_task(&mut inner.draft_stability_task);
                }
            }

            if inner.pipeline_failed {
                return;
            }
            if inner.final_lane_busy() {
                return;
            }
            let text = match kind {
                DraftTimerKind::Stable => inner.committer.preview_complete_sentences(),
                DraftTimerKind::Maximum => inner.committer.preview_latest_draft(true),
            };
            text.map(|text| {
                (
                    text,
                    inner.latest_draft_language.clone(),
                    inner.draft_revision,
                    match kind {
                        DraftTimerKind::Stable => "stable-draft",
                        DraftTimerKind::Maximum => "maximum-wait",
                    },
                )
            })
        };

        if let Some((text, language, revision, boundary)) = preview {
            pipeline_log!(
                "mt preview scheduled boundary={} length={} language={}",
                boundary,
                text.chars().count(),
                language
                    .as_deref()
                    .unwrap_or(self.source_language.raw_value())
            );
            self.start_preview(text, language, revision).await;
        }
    }

    async fn start_preview(&self, text: String, language: Option<String>, revision: u64) {
        let mut inner = self.inner.lock().await;
        if inner.pipeline_failed
            || revision != inner.draft_revision
            || !inner.committer.has_pending_text()
            || inner.final_lane_busy()
        {
            return;
        }

        abort_task(&mut inner.preview_task);
        let preview_id = self.advance_preview_epoch();
        let preview_self = self.clone();
        let task = tokio::spawn(async move {
            preview_self.run_preview(preview_id, text, language).await;
        });
        inner.preview_task = Some(TaskSlot {
            id: preview_id,
            handle: task,
        });
    }

    async fn run_preview(&self, preview_id: u64, text: String, language: Option<String>) {
        if !self.preview_is_current(preview_id) {
            return;
        }
        self.emit(LiveTranslateServerEvent::SourceDraft {
            text: text.clone(),
            language: language.clone(),
        });

        let preview_epoch = Arc::clone(&self.preview_epoch);
        let events = self.events.clone();
        let partial_handler: PartialHandler = Arc::new(move |partial| {
            if preview_epoch.load(Ordering::SeqCst) == preview_id {
                let _ = events.send(LiveTranslateServerEvent::TranslationDraft(partial));
            }
        });
        let deadline = tokio::time::Instant::now() + MAX_PREVIEW_REQUEST_AGE;
        let result = self
            .translate_with_retry(&text, language.as_deref(), deadline, partial_handler)
            .await;

        if self.preview_is_current(preview_id) {
            match result {
                Ok(translation) => {
                    self.emit(LiveTranslateServerEvent::TranslationDraft(translation));
                    pipeline_log!("mt preview completed");
                }
                Err(error) => {
                    pipeline_log!("mt preview failed error={}", error.diagnostic_label());
                }
            }
        }
        self.clear_preview_task(preview_id).await;
    }

    fn preview_is_current(&self, preview_id: u64) -> bool {
        self.preview_epoch.load(Ordering::SeqCst) == preview_id
    }

    fn advance_preview_epoch(&self) -> u64 {
        let id = self
            .preview_epoch
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        if id == 0 {
            self.preview_epoch
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1)
        } else {
            id
        }
    }

    async fn clear_preview_task(&self, preview_id: u64) {
        let mut inner = self.inner.lock().await;
        if inner
            .preview_task
            .as_ref()
            .is_some_and(|task| task.id == preview_id)
        {
            drop(inner.preview_task.take());
        }
    }

    // MARK: Durable final lane

    async fn enqueue_final(
        &self,
        text: String,
        language: Option<String>,
        boundary: FinalBoundary,
        utterance_revision: u64,
    ) {
        if !self.translates_audio {
            self.emit(LiveTranslateServerEvent::SubtitleFinalPair {
                source: text.clone(),
                language,
                translation: text,
            });
            return;
        }

        let request = TranslationRequest {
            text,
            language,
            boundary,
            utterance_revision,
            enqueued_at: tokio::time::Instant::now(),
        };
        let outcome = {
            let mut inner = self.inner.lock().await;
            if inner.pipeline_failed {
                return;
            }

            if inner
                .active_final
                .as_ref()
                .is_some_and(|active| active.matches(&request))
            {
                return;
            }
            if let Some(queued) = inner
                .final_queue
                .iter_mut()
                .find(|queued| queued.key().matches(&request))
            {
                if boundary == FinalBoundary::ServerFinal {
                    queued.language = request.language;
                    queued.boundary = FinalBoundary::ServerFinal;
                    queued.utterance_revision = request.utterance_revision;
                }
                return;
            }

            if inner.final_queue.len() >= MAX_FINAL_QUEUE_DEPTH {
                if inner.final_completion_in_progress {
                    inner.deferred_overload = true;
                    EnqueueOutcome::DeferredOverload
                } else {
                    inner.pipeline_failed = true;
                    EnqueueOutcome::Overloaded
                }
            } else {
                pipeline_log!(
                    "mt final enqueued boundary={} depth={}",
                    boundary.label(),
                    inner.final_queue.len() + 1
                );
                inner.final_queue.push_back(request);
                EnqueueOutcome::Queued
            }
        };

        match outcome {
            EnqueueOutcome::Queued => self.start_final_worker_if_needed().await,
            EnqueueOutcome::Overloaded => {
                self.cancel_replaceable_work().await;
                pipeline_log!("mt final overload depth={MAX_FINAL_QUEUE_DEPTH}");
                self.emit_overload_error();
            }
            EnqueueOutcome::DeferredOverload => {}
        }
    }

    async fn start_final_worker_if_needed(&self) {
        let mut inner = self.inner.lock().await;
        if inner.pipeline_failed || inner.final_worker.is_some() || inner.final_queue.is_empty() {
            return;
        }
        let worker_id = next_nonzero(&mut inner.next_task_id);
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            self_arc.run_final_worker(worker_id).await;
        });
        inner.final_worker = Some(TaskSlot {
            id: worker_id,
            handle: task,
        });
    }

    async fn run_final_worker(&self, worker_id: u64) {
        loop {
            let request = {
                let mut inner = self.inner.lock().await;
                if inner.pipeline_failed {
                    clear_task_if_id(&mut inner.final_worker, worker_id);
                    inner.active_final = None;
                    return;
                }
                match inner.final_queue.pop_front() {
                    Some(request) => {
                        abort_task(&mut inner.draft_stability_task);
                        abort_task(&mut inner.draft_maximum_wait_task);
                        abort_task(&mut inner.preview_task);
                        self.advance_preview_epoch();
                        inner.active_final = Some(request.key());
                        Some(request)
                    }
                    None => {
                        clear_task_if_id(&mut inner.final_worker, worker_id);
                        inner.active_final = None;
                        None
                    }
                }
            };
            let Some(request) = request else {
                self.resume_pending_preview_if_final_lane_idle().await;
                return;
            };

            let started_at = tokio::time::Instant::now();
            let queue_age = started_at.saturating_duration_since(request.enqueued_at);
            if queue_age >= MAX_FINAL_REQUEST_AGE {
                self.fail_final_worker_overload(worker_id, queue_age).await;
                return;
            }
            pipeline_log!(
                "mt final started boundary={} waitMs={} remaining={}",
                request.boundary.label(),
                queue_age.as_millis(),
                self.inner.lock().await.final_queue.len()
            );
            self.emit(LiveTranslateServerEvent::TranslationStarted);
            self.emit(LiveTranslateServerEvent::SourceDraft {
                text: request.text.clone(),
                language: request.language.clone(),
            });

            let events = self.events.clone();
            let partial_handler: PartialHandler = Arc::new(move |partial| {
                let _ = events.send(LiveTranslateServerEvent::TranslationDraft(partial));
            });
            let deadline = request.enqueued_at + MAX_FINAL_REQUEST_AGE;
            match self
                .translate_with_retry(
                    &request.text,
                    request.language.as_deref(),
                    deadline,
                    partial_handler,
                )
                .await
            {
                Ok(translation) => {
                    let should_emit = {
                        let mut inner = self.inner.lock().await;
                        let owned = inner
                            .final_worker
                            .as_ref()
                            .is_some_and(|task| task.id == worker_id);
                        if inner.pipeline_failed || !owned {
                            clear_task_if_id(&mut inner.final_worker, worker_id);
                            inner.active_final = None;
                            false
                        } else {
                            inner.final_completion_in_progress = true;
                            true
                        }
                    };
                    if !should_emit {
                        return;
                    }

                    self.emit(LiveTranslateServerEvent::SubtitleFinalPair {
                        source: request.text.clone(),
                        language: request.language.clone(),
                        translation: translation.clone(),
                    });

                    let deferred_overload = {
                        let mut inner = self.inner.lock().await;
                        inner.final_completion_in_progress = false;
                        inner.active_final = None;
                        if inner.deferred_overload {
                            inner.deferred_overload = false;
                            inner.pipeline_failed = true;
                            clear_task_if_id(&mut inner.final_worker, worker_id);
                            true
                        } else {
                            false
                        }
                    };
                    if deferred_overload {
                        self.cancel_replaceable_work().await;
                        pipeline_log!("mt final overload depth={MAX_FINAL_QUEUE_DEPTH}");
                        self.emit_overload_error();
                        return;
                    }

                    pipeline_log!(
                        "mt final completed boundary={} requestMs={} remaining={}",
                        request.boundary.label(),
                        started_at.elapsed().as_millis(),
                        self.inner.lock().await.final_queue.len()
                    );
                    self.remember(&request.text, &translation).await;
                }
                Err(error) => {
                    let should_emit = {
                        let mut inner = self.inner.lock().await;
                        let owned = inner
                            .final_worker
                            .as_ref()
                            .is_some_and(|task| task.id == worker_id);
                        if owned {
                            inner.pipeline_failed = true;
                            inner.active_final = None;
                            clear_task_if_id(&mut inner.final_worker, worker_id);
                        }
                        owned
                    };
                    if should_emit {
                        pipeline_log!(
                            "mt final failed requestMs={} error={}",
                            started_at.elapsed().as_millis(),
                            error.diagnostic_label()
                        );
                        self.handle_translation_failure(&error);
                    }
                    return;
                }
            }
        }
    }

    async fn fail_final_worker_overload(&self, worker_id: u64, queue_age: Duration) {
        let should_emit = {
            let mut inner = self.inner.lock().await;
            let owned = inner
                .final_worker
                .as_ref()
                .is_some_and(|task| task.id == worker_id);
            if owned {
                inner.pipeline_failed = true;
                inner.active_final = None;
                clear_task_if_id(&mut inner.final_worker, worker_id);
            }
            owned
        };
        if should_emit {
            self.cancel_replaceable_work().await;
            pipeline_log!("mt final expired waitMs={}", queue_age.as_millis());
            self.emit_overload_error();
        }
    }

    async fn resume_pending_preview_if_final_lane_idle(&self) {
        let revision = {
            let inner = self.inner.lock().await;
            (!inner.pipeline_failed
                && !inner.final_lane_busy()
                && inner.committer.has_pending_text())
            .then_some(inner.draft_revision)
        };
        if let Some(revision) = revision {
            self.schedule_draft_finalization(revision).await;
        }
    }

    fn emit_overload_error(&self) {
        self.emit(LiveTranslateServerEvent::Error {
            code: OVERLOAD_ERROR_CODE.into(),
            message: OVERLOAD_ERROR_MESSAGE.into(),
        });
    }

    fn handle_translation_failure(&self, error: &QwenMTClientError) {
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
        deadline: tokio::time::Instant,
        on_partial: PartialHandler,
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
            language.and_then(|value| SourceLanguage::from_detected(Some(value)))
        } else {
            None
        };
        let mut attempt = 1usize;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(QwenMTClientError::RequestTimedOut);
            }

            let handler = Arc::clone(&on_partial);
            let result = tokio::time::timeout(remaining, async {
                if self.streams_finals {
                    self.mt
                        .translate_streaming(text, source_override, &memory, move |partial| {
                            (handler)(partial)
                        })
                        .await
                } else {
                    self.mt.translate(text, source_override, &memory).await
                }
            })
            .await
            .unwrap_or(Err(QwenMTClientError::RequestTimedOut));

            match result {
                Ok(translation) => return Ok(translation),
                Err(error) => {
                    if attempt >= MAX_TRANSLATION_ATTEMPTS {
                        return Err(error);
                    }
                    let Some(delay) =
                        crate::core::protocols::qwen_mt::QwenMTRetryPolicy::delay(&error, attempt)
                    else {
                        return Err(error);
                    };
                    if tokio::time::Instant::now() + delay >= deadline {
                        return Err(QwenMTClientError::RequestTimedOut);
                    }
                    pipeline_log!(
                        "mt retrying attempt={} delayMs={} error={}",
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

    // MARK: Lifecycle helpers

    async fn flush_pending_draft(&self) {
        let pending = {
            let mut inner = self.inner.lock().await;
            abort_task(&mut inner.draft_stability_task);
            abort_task(&mut inner.draft_maximum_wait_task);
            abort_task(&mut inner.preview_task);
            self.advance_preview_epoch();
            let text = inner.committer.preview_latest_draft(false);
            let language = inner.latest_draft_language.clone();
            let revision = inner.draft_revision;
            inner.committer.reset();
            inner.latest_draft_language = None;
            next_nonzero(&mut inner.draft_revision);
            text.map(|text| (text, language, revision))
        };
        if let Some((text, language, revision)) = pending {
            pipeline_log!("audio3 asr fallback final length={}", text.chars().count());
            self.enqueue_final(text, language, FinalBoundary::SessionFinish, revision)
                .await;
        }
    }

    async fn cancel_replaceable_work(&self) {
        let mut inner = self.inner.lock().await;
        abort_task(&mut inner.draft_stability_task);
        abort_task(&mut inner.draft_maximum_wait_task);
        abort_task(&mut inner.preview_task);
        self.advance_preview_epoch();
        next_nonzero(&mut inner.draft_revision);
    }

    async fn reset_draft_state(&self) {
        let mut inner = self.inner.lock().await;
        abort_task(&mut inner.draft_stability_task);
        abort_task(&mut inner.draft_maximum_wait_task);
        abort_task(&mut inner.preview_task);
        self.advance_preview_epoch();
        inner.committer.reset();
        inner.latest_draft_language = None;
        next_nonzero(&mut inner.draft_revision);
        inner.last_server_final = None;
        inner.last_draft_log_at = None;
    }

    async fn cancel_final_translations(&self) {
        let mut inner = self.inner.lock().await;
        abort_task(&mut inner.final_worker);
        inner.final_queue.clear();
        inner.active_final = None;
        inner.translation_memory.clear();
        inner.pipeline_failed = false;
        inner.final_completion_in_progress = false;
        inner.deferred_overload = false;
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

    async fn install_asr_bridge(&self, mut receiver: ProviderEventReceiver) {
        let (finish_tx, finish_rx) = oneshot::channel();
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            let mut finish_tx = Some(finish_tx);
            while let Some(event) = receiver.recv().await {
                let is_session_finished = event == LiveTranslateServerEvent::SessionFinished;
                self_arc.handle_asr_event(event).await;
                if is_session_finished {
                    if let Some(finish_tx) = finish_tx.take() {
                        let _ = finish_tx.send(());
                    }
                }
            }
        });
        let mut bridge = self.asr_bridge.lock().await;
        if let Some(previous_task) = bridge.task.replace(task) {
            previous_task.abort();
        }
        bridge.finish_ack = Some(finish_rx);
    }

    async fn wait_for_asr_bridge_finish(&self, timeout: Duration) -> bool {
        let finish_ack = self.take_asr_bridge_finish_ack().await;
        let Some(finish_ack) = finish_ack else {
            return false;
        };
        matches!(tokio::time::timeout(timeout, finish_ack).await, Ok(Ok(())))
    }

    async fn take_asr_bridge_finish_ack(&self) -> Option<oneshot::Receiver<()>> {
        self.asr_bridge.lock().await.finish_ack.take()
    }

    async fn disconnect_asr_bridge(&self) {
        let mut bridge = self.asr_bridge.lock().await;
        bridge.finish_ack = None;
        if let Some(task) = bridge.task.take() {
            task.abort();
        }
    }

    fn emit(&self, event: LiveTranslateServerEvent) {
        let _ = self.events.send(event);
    }
}

fn next_nonzero(counter: &mut u64) -> u64 {
    *counter = counter.wrapping_add(1);
    if *counter == 0 {
        *counter = 1;
    }
    *counter
}

fn abort_task(slot: &mut Option<TaskSlot>) {
    if let Some(task) = slot.take() {
        task.handle.abort();
    }
}

fn clear_task_if_id(slot: &mut Option<TaskSlot>, task_id: u64) {
    if slot.as_ref().is_some_and(|task| task.id == task_id) {
        drop(slot.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::provider_events::ProviderEventReceiver;

    fn test_client(
        target_language: TargetLanguage,
        threshold: usize,
    ) -> (HighQualityTranslationClient, ProviderEventReceiver) {
        let (events, receiver) = provider_event_channel();
        let client = HighQualityTranslationClient::new(
            "test-key",
            SourceLanguage::Japanese,
            target_language,
            QwenMTModel::Plus,
            Duration::from_secs(60),
            Duration::from_secs(60),
            threshold,
            events,
        )
        .unwrap();
        (client, receiver)
    }

    #[tokio::test]
    async fn stable_timer_resets_while_maximum_timer_keeps_its_first_token() {
        let (client, _events) = test_client(TargetLanguage::SimplifiedChinese, 100);
        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "まだ話しています".into(),
                language: Some("ja".into()),
            })
            .await;
        let (first_stable, first_maximum) = {
            let inner = client.inner.lock().await;
            (
                inner.draft_stability_task.as_ref().unwrap().id,
                inner.draft_maximum_wait_task.as_ref().unwrap().id,
            )
        };

        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "まだ話し続けています".into(),
                language: Some("ja".into()),
            })
            .await;
        let (second_stable, second_maximum) = {
            let inner = client.inner.lock().await;
            (
                inner.draft_stability_task.as_ref().unwrap().id,
                inner.draft_maximum_wait_task.as_ref().unwrap().id,
            )
        };

        assert_ne!(first_stable, second_stable);
        assert_eq!(first_maximum, second_maximum);
        client.reset_draft_state().await;
    }

    #[tokio::test]
    async fn stable_callback_clears_only_itself_and_preserves_maximum_wait() {
        let (client, _events) = test_client(TargetLanguage::SimplifiedChinese, 100);
        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "短い未完の文".into(),
                language: Some("ja".into()),
            })
            .await;
        let (stable_id, maximum_id, revision) = {
            let inner = client.inner.lock().await;
            (
                inner.draft_stability_task.as_ref().unwrap().id,
                inner.draft_maximum_wait_task.as_ref().unwrap().id,
                inner.draft_revision,
            )
        };

        client
            .handle_draft_timer(DraftTimerKind::Stable, stable_id, revision)
            .await;
        let inner = client.inner.lock().await;
        assert!(inner.draft_stability_task.is_none());
        assert_eq!(
            inner.draft_maximum_wait_task.as_ref().map(|task| task.id),
            Some(maximum_id)
        );
        drop(inner);
        client.reset_draft_state().await;
    }

    #[tokio::test]
    async fn original_mode_commits_server_final_as_one_atomic_pair() {
        let (client, mut events) = test_client(TargetLanguage::Original, 20);
        client
            .handle_asr_event(LiveTranslateServerEvent::SourceFinal {
                text: "今日は晴れです。".into(),
                language: Some("ja".into()),
            })
            .await;

        assert_eq!(
            events.recv().await,
            Some(LiveTranslateServerEvent::SubtitleFinalPair {
                source: "今日は晴れです。".into(),
                language: Some("ja".into()),
                translation: "今日は晴れです。".into(),
            })
        );
    }

    #[tokio::test]
    async fn server_final_cancels_timers_and_invalidates_an_inflight_preview() {
        let (client, _events) = test_client(TargetLanguage::SimplifiedChinese, 20);
        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "今日は晴れです。".into(),
                language: Some("ja".into()),
            })
            .await;
        let preview_id = client.advance_preview_epoch();
        {
            let mut inner = client.inner.lock().await;
            inner.preview_task = Some(TaskSlot {
                id: preview_id,
                handle: tokio::spawn(std::future::pending()),
            });
        }

        assert!(client
            .prepare_server_final("今日は晴れです。")
            .await
            .is_some());

        let inner = client.inner.lock().await;
        assert!(inner.draft_stability_task.is_none());
        assert!(inner.draft_maximum_wait_task.is_none());
        assert!(inner.preview_task.is_none());
        assert!(!inner.committer.has_pending_text());
        assert_ne!(client.preview_epoch.load(Ordering::SeqCst), preview_id);
    }

    #[tokio::test]
    async fn active_final_defers_preview_until_the_final_lane_is_idle() {
        let (client, mut events) = test_client(TargetLanguage::SimplifiedChinese, 20);
        {
            let mut inner = client.inner.lock().await;
            inner.final_worker = Some(TaskSlot {
                id: 999,
                handle: tokio::spawn(std::future::pending()),
            });
            inner.active_final = Some(FinalRequestKey {
                text: "earlier final".into(),
                boundary: FinalBoundary::ServerFinal,
                utterance_revision: 1,
            });
        }

        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "まだ話し続けています".into(),
                language: Some("ja".into()),
            })
            .await;

        let pending_revision = {
            let inner = client.inner.lock().await;
            assert!(inner.committer.has_pending_text());
            assert!(inner.draft_stability_task.is_none());
            assert!(inner.draft_maximum_wait_task.is_none());
            assert!(inner.preview_task.is_none());
            inner.draft_revision
        };
        assert!(events.try_recv().is_err());

        {
            let mut inner = client.inner.lock().await;
            abort_task(&mut inner.final_worker);
            inner.active_final = None;
        }
        client.resume_pending_preview_if_final_lane_idle().await;

        let inner = client.inner.lock().await;
        assert_eq!(inner.draft_revision, pending_revision);
        assert!(inner.draft_stability_task.is_some());
        assert!(inner.draft_maximum_wait_task.is_some());
        drop(inner);
        client.reset_draft_state().await;
    }

    #[tokio::test]
    async fn identical_server_final_is_only_deduplicated_without_a_new_draft() {
        let (client, mut events) = test_client(TargetLanguage::Original, 20);
        let final_event = LiveTranslateServerEvent::SourceFinal {
            text: "はい。".into(),
            language: Some("ja".into()),
        };
        client.handle_asr_event(final_event.clone()).await;
        client.handle_asr_event(final_event.clone()).await;
        assert!(matches!(
            events.recv().await,
            Some(LiveTranslateServerEvent::SubtitleFinalPair { .. })
        ));
        assert!(events.try_recv().is_err());

        client
            .handle_asr_event(LiveTranslateServerEvent::SourceDraft {
                text: "はい".into(),
                language: Some("ja".into()),
            })
            .await;
        let _ = events.recv().await;
        let _ = events.recv().await;
        client.handle_asr_event(final_event).await;
        assert!(matches!(
            events.recv().await,
            Some(LiveTranslateServerEvent::SubtitleFinalPair { .. })
        ));
    }

    #[tokio::test]
    async fn final_queue_has_a_hard_limit_and_reports_overload_once() {
        let (client, mut events) = test_client(TargetLanguage::SimplifiedChinese, 20);
        {
            let mut inner = client.inner.lock().await;
            inner.final_worker = Some(TaskSlot {
                id: 999,
                handle: tokio::spawn(std::future::pending()),
            });
        }

        for revision in 1..=MAX_FINAL_QUEUE_DEPTH as u64 {
            client
                .enqueue_final(
                    format!("server final {revision}"),
                    Some("ja".into()),
                    FinalBoundary::ServerFinal,
                    revision,
                )
                .await;
        }
        client
            .enqueue_final(
                "overflow".into(),
                Some("ja".into()),
                FinalBoundary::ServerFinal,
                99,
            )
            .await;
        client
            .enqueue_final(
                "second overflow".into(),
                Some("ja".into()),
                FinalBoundary::ServerFinal,
                100,
            )
            .await;

        let inner = client.inner.lock().await;
        assert_eq!(inner.final_queue.len(), MAX_FINAL_QUEUE_DEPTH);
        assert!(inner.pipeline_failed);
        drop(inner);
        assert_eq!(
            events.recv().await,
            Some(LiveTranslateServerEvent::Error {
                code: OVERLOAD_ERROR_CODE.into(),
                message: OVERLOAD_ERROR_MESSAGE.into(),
            })
        );
        assert!(events.try_recv().is_err());
        client.cancel_final_translations().await;
    }

    #[tokio::test]
    async fn finish_ack_waits_for_the_bridge_to_consume_the_queued_final() {
        let (client, mut events) = test_client(TargetLanguage::Original, 20);
        let (asr_events, asr_receiver) = provider_event_channel();
        client.install_asr_bridge(asr_receiver).await;
        let mut finish_ack = client.take_asr_bridge_finish_ack().await.unwrap();

        // Hold the draft/final state so the bridge cannot finish consuming the
        // authoritative final. The per-connection acknowledgement uses separate
        // bridge state, so observing it never shares this lock.
        let inner = client.inner.lock().await;
        asr_events
            .send(LiveTranslateServerEvent::SourceFinal {
                text: "最後の字幕。".into(),
                language: Some("ja".into()),
            })
            .unwrap();
        asr_events
            .send(LiveTranslateServerEvent::SessionFinished)
            .unwrap();
        tokio::task::yield_now().await;
        assert!(matches!(
            finish_ack.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));

        drop(inner);
        assert!(tokio::time::timeout(Duration::from_secs(1), finish_ack)
            .await
            .unwrap()
            .is_ok());
        assert_eq!(
            events.recv().await,
            Some(LiveTranslateServerEvent::SubtitleFinalPair {
                source: "最後の字幕。".into(),
                language: Some("ja".into()),
                translation: "最後の字幕。".into(),
            })
        );
        assert_eq!(
            events.recv().await,
            Some(LiveTranslateServerEvent::SessionFinished)
        );
        client.disconnect_asr_bridge().await;
    }

    #[tokio::test]
    async fn finish_ack_is_scoped_to_one_bridge_connection() {
        let (client, _events) = test_client(TargetLanguage::Original, 20);

        let (first_events, first_receiver) = provider_event_channel();
        client.install_asr_bridge(first_receiver).await;
        first_events
            .send(LiveTranslateServerEvent::SessionFinished)
            .unwrap();
        assert!(
            client
                .wait_for_asr_bridge_finish(Duration::from_secs(1))
                .await
        );

        let (_second_events, second_receiver) = provider_event_channel();
        client.install_asr_bridge(second_receiver).await;
        assert!(
            !client
                .wait_for_asr_bridge_finish(Duration::from_millis(25))
                .await
        );
        client.disconnect_asr_bridge().await;
    }
}
