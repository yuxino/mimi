//! Session lifecycle: start/stop/pause/resume, capability-aware switching,
//! health checks, automatic reconnection, and state-event broadcasting.
//!
//! The manager is always shared behind `Arc<SessionManager>`; spawned tasks
//! hold clones of the same Arc so they observe one piece of session state.

use crate::audio::send_pipeline::{AudioPipelineFailure, AudioSendPipeline};
use crate::audio::{
    AudioCaptureFormat, CaptureFailureSender, SystemAudioCapture, SystemAudioCaptureFailure,
};
use crate::clients::provider_events::provider_event_channel;
use crate::clients::translation_client::TranslationClient;
use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::models::{SessionStatus, SourceLanguage, TranslationMode};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::provider::ProviderKind;
use crate::core::session::{TranslationSessionController, TranslationSessionState};
use crate::pipeline_log;
use crate::settings_store::SettingsStore;
use crate::windows::OverlayWindowManager;
use serde::Serialize;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex as TokioMutex, Notify, OwnedMutexGuard};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StatusPayload {
    Idle,
    Connecting,
    Listening,
    Stopping,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateEvent {
    pub status: StatusPayload,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "isPaused")]
    pub is_paused: bool,
    #[serde(rename = "isOverlayCollapsed")]
    pub is_overlay_collapsed: bool,
    pub subtitles: crate::core::models::SubtitleSnapshot,
    #[serde(rename = "detectedLanguage")]
    pub detected_language: Option<String>,
    #[serde(rename = "isTranslationPending")]
    pub is_translation_pending: bool,
}

const NO_GENERATION: u64 = 0;
const SESSION_START_CANCELLED: &str = "The session start was superseded by a newer request.";
const RECOVERY_ATTEMPTS: usize = 4;

struct LifecycleOperationGuard {
    count: Arc<AtomicUsize>,
}

impl Drop for LifecycleOperationGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::SeqCst);
    }
}

struct StartRequestGuard {
    in_progress: Arc<AtomicBool>,
}

struct TeardownOperationGuard {
    count: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

impl Drop for TeardownOperationGuard {
    fn drop(&mut self) {
        if self.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.notify.notify_waiters();
        }
    }
}

fn translation_mode_after_source_switch(
    provider: ProviderKind,
    source_language: SourceLanguage,
    current_mode: TranslationMode,
) -> TranslationMode {
    match (provider, source_language) {
        (ProviderKind::AlibabaCloud, SourceLanguage::Automatic) => {
            if current_mode == TranslationMode::Turbo {
                TranslationMode::Turbo
            } else {
                TranslationMode::LowLatency
            }
        }
        (ProviderKind::OpenAIRealtime, _) => TranslationMode::Turbo,
        _ => current_mode,
    }
}

fn pipeline_settings_mutation_is_allowed(
    status: &SessionStatus,
    lifecycle_operations: usize,
) -> bool {
    lifecycle_operations == 0
        && !matches!(status, SessionStatus::Connecting | SessionStatus::Stopping)
}

fn lifecycle_activity_is_active(has_active_session: bool, lifecycle_operations: usize) -> bool {
    has_active_session || lifecycle_operations > 0
}

fn start_request_can_proceed(
    has_active_session: bool,
    is_recovering: bool,
    active_generation: u64,
) -> bool {
    !has_active_session || (is_recovering && active_generation == NO_GENERATION)
}

fn apply_establish_failure_state(
    controller: &mut TranslationSessionController,
    error: String,
    is_recovering: bool,
) {
    if is_recovering {
        controller.begin_connecting();
    } else {
        controller.did_fail(error);
    }
}

fn source_switch_requires_reconnect(
    is_listening: bool,
    current_source: SourceLanguage,
    current_target: crate::core::models::TargetLanguage,
    current_mode: TranslationMode,
    next_source: SourceLanguage,
    next_target: crate::core::models::TargetLanguage,
    next_mode: TranslationMode,
) -> bool {
    is_listening
        && (current_source != next_source
            || current_target != next_target
            || current_mode != next_mode)
}

fn pause_transition_is_valid(
    status: &SessionStatus,
    is_paused: bool,
    active_generation: u64,
) -> bool {
    !is_paused && *status == SessionStatus::Listening && active_generation != NO_GENERATION
}

fn resume_transition_is_valid(
    status: &SessionStatus,
    is_paused: bool,
    active_generation: u64,
    has_active_settings: bool,
) -> bool {
    is_paused
        && *status == SessionStatus::Listening
        && active_generation == NO_GENERATION
        && has_active_settings
}

fn try_begin_start(in_progress: &AtomicBool) -> bool {
    in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

fn invalidate_generation_atoms(
    active_generation: &AtomicU64,
    lifecycle_sequence: &AtomicU64,
    generation: u64,
) -> Option<u64> {
    if lifecycle_sequence.load(Ordering::SeqCst) != generation {
        return None;
    }
    if active_generation
        .compare_exchange(
            generation,
            NO_GENERATION,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    {
        let mut owned_epoch = generation.wrapping_add(1);
        if owned_epoch == NO_GENERATION {
            owned_epoch = 1;
        }
        lifecycle_sequence
            .compare_exchange(generation, owned_epoch, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| owned_epoch)
    } else {
        None
    }
}

fn advance_lifecycle_sequence_if_current(
    lifecycle_sequence: &AtomicU64,
    expected: u64,
) -> Option<u64> {
    let mut next = expected.wrapping_add(1);
    if next == NO_GENERATION {
        next = 1;
    }
    lifecycle_sequence
        .compare_exchange(expected, next, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| next)
}

fn lifecycle_sequence_matches(lifecycle_sequence: &AtomicU64, expected: u64) -> bool {
    lifecycle_sequence.load(Ordering::SeqCst) == expected
}

fn resume_failure_is_still_owned(
    error: &str,
    failure_epoch: u64,
    current_epoch: u64,
    status: &SessionStatus,
    active_generation: u64,
) -> bool {
    error != SESSION_START_CANCELLED
        && failure_epoch == current_epoch
        && matches!(status, SessionStatus::Error(message) if message == error)
        && active_generation == NO_GENERATION
}

fn cancelled_recovery_attempt_is_retryable(
    retry_generation: u64,
    attempt_generation: u64,
    current_epoch: u64,
    has_active_settings: bool,
) -> bool {
    retry_generation == attempt_generation
        && current_epoch == attempt_generation.wrapping_add(1)
        && has_active_settings
}

fn recovery_exhaustion_is_still_owned(
    recovery_epoch: u64,
    current_epoch: u64,
    active_generation: u64,
) -> bool {
    recovery_epoch == current_epoch && active_generation == NO_GENERATION
}

fn clear_recovery_atoms(is_recovering: &AtomicBool, retry_generation: &AtomicU64) {
    retry_generation.store(NO_GENERATION, Ordering::SeqCst);
    is_recovering.store(false, Ordering::SeqCst);
}

/// Bounded exponential recovery delay with deterministic per-generation
/// jitter. Determinism keeps lifecycle tests reliable while preventing two
/// mimi instances from reconnecting in lockstep after a shared outage.
fn recovery_delay(attempt: usize, generation: u64) -> Duration {
    if attempt == 0 {
        return Duration::ZERO;
    }
    let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(u32::MAX);
    let base_ms = 500_u64.saturating_mul(2_u64.saturating_pow(exponent.min(3)));
    let mixed = generation
        .wrapping_add((attempt as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let jitter_ms = mixed % (base_ms / 4 + 1);
    Duration::from_millis(base_ms + jitter_ms)
}

fn generation_accepts_event(
    active_generation: u64,
    stopping_tail_generation: u64,
    generation: u64,
    event: &LiveTranslateServerEvent,
) -> bool {
    generation != NO_GENERATION
        && (active_generation == generation
            || (stopping_tail_generation == generation
                && matches!(event, LiveTranslateServerEvent::SubtitleFinalPair { .. })))
}

fn provider_error_is_retryable(code: &str) -> bool {
    matches!(
        code,
        "transport_error" | "provider_event_backlog_overflow" | "translation_backlog_overflow"
    )
}

fn clear_task_slot_if_id(
    slot: &Mutex<Option<JoinHandle<()>>>,
    current_id: &AtomicU64,
    task_id: u64,
) -> bool {
    let mut slot = slot.lock().unwrap();
    if current_id.load(Ordering::SeqCst) != task_id {
        return false;
    }
    current_id.store(NO_GENERATION, Ordering::SeqCst);
    drop(slot.take());
    true
}

fn clear_owned_value_if_generation<T>(
    slot: &Mutex<Option<T>>,
    owner_generation: &AtomicU64,
    generation: u64,
) -> bool {
    let mut slot = slot.lock().unwrap();
    if owner_generation.load(Ordering::SeqCst) != generation {
        return false;
    }
    *slot = None;
    owner_generation.store(NO_GENERATION, Ordering::SeqCst);
    true
}

fn update_owned_value<T>(
    slot: &Mutex<Option<T>>,
    owner_generation: &AtomicU64,
    update: impl FnOnce(&mut T),
) -> bool {
    let mut slot = slot.lock().unwrap();
    if owner_generation.load(Ordering::SeqCst) == NO_GENERATION {
        return false;
    }
    let Some(value) = slot.as_mut() else {
        return false;
    };
    update(value);
    true
}

async fn lock_after_operations(
    lock: Arc<TokioMutex<()>>,
    operation_count: Arc<AtomicUsize>,
    notify: Arc<Notify>,
) -> OwnedMutexGuard<()> {
    loop {
        let notified = notify.notified();
        tokio::pin!(notified);
        // Register before inspecting the counter. `notify_waiters` does not
        // retain a permit, so awaiting an unregistered future after the last
        // teardown completes could otherwise sleep forever.
        notified.as_mut().enable();
        let guard = Arc::clone(&lock).lock_owned().await;
        if operation_count.load(Ordering::SeqCst) == 0 {
            return guard;
        }
        drop(guard);
        notified.await;
    }
}

async fn wait_until_generation_changes(
    active_generation: Arc<AtomicU64>,
    lifecycle_sequence: Arc<AtomicU64>,
    lifecycle_notify: Arc<Notify>,
    generation: u64,
) {
    loop {
        let notified = lifecycle_notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if lifecycle_sequence.load(Ordering::SeqCst) != generation
            || active_generation.load(Ordering::SeqCst) != generation
        {
            return;
        }
        notified.await;
    }
}

async fn run_generation_bound_operation<T>(
    active_generation: Arc<AtomicU64>,
    lifecycle_sequence: Arc<AtomicU64>,
    lifecycle_notify: Arc<Notify>,
    generation: u64,
    operation: impl Future<Output = T>,
) -> Result<T, String> {
    tokio::select! {
        result = operation => Ok(result),
        () = wait_until_generation_changes(
            active_generation,
            lifecycle_sequence,
            lifecycle_notify,
            generation,
        ) => Err(SESSION_START_CANCELLED.into()),
    }
}

impl Drop for StartRequestGuard {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::SeqCst);
    }
}

impl From<&TranslationSessionState> for SessionStateEvent {
    fn from(state: &TranslationSessionState) -> Self {
        let status = match &state.status {
            SessionStatus::Idle => StatusPayload::Idle,
            SessionStatus::Connecting => StatusPayload::Connecting,
            SessionStatus::Listening => StatusPayload::Listening,
            SessionStatus::Stopping => StatusPayload::Stopping,
            SessionStatus::Error(message) => StatusPayload::Error {
                message: message.clone(),
            },
        };
        Self {
            is_active: state.status.is_active(),
            status,
            is_paused: false,
            is_overlay_collapsed: false,
            subtitles: state.subtitles.clone(),
            detected_language: state
                .detected_language
                .as_ref()
                .map(|language| language.code.clone()),
            is_translation_pending: state.is_translation_pending,
        }
    }
}

#[derive(Clone)]
pub struct SessionManager {
    app: AppHandle,
    settings: Arc<SettingsStore>,
    controller: Arc<Mutex<TranslationSessionController>>,
    audio: Arc<Mutex<SystemAudioCapture>>,
    client: Arc<Mutex<Option<TranslationClient>>>,
    client_generation: Arc<AtomicU64>,
    audio_pipeline: Arc<Mutex<Option<Arc<AudioSendPipeline>>>>,
    audio_pipeline_generation: Arc<AtomicU64>,
    capture_generation: Arc<AtomicU64>,
    active_settings: Arc<Mutex<Option<LiveTranslationConfiguration>>>,
    active_settings_generation: Arc<AtomicU64>,
    is_paused: Arc<AtomicBool>,
    is_recovering: Arc<AtomicBool>,
    is_overlay_collapsed: Arc<AtomicBool>,
    /// Set whenever a session-state broadcast is requested; the single
    /// scheduled publisher clears it after each publish and keeps looping
    /// until no newer request has arrived.
    publish_dirty: Arc<AtomicBool>,
    /// Serializes publisher scheduling: exactly one task owns this lock.
    publish_lock: Arc<TokioMutex<()>>,
    health_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    health_task_id: Arc<AtomicU64>,
    recovery_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    recovery_task_id: Arc<AtomicU64>,
    /// Marks a provider transport failure that cancelled the current recovery
    /// attempt. The existing recovery owner consumes it and performs the next
    /// retry instead of trying to enqueue a second recovery task.
    recovery_retry_generation: Arc<AtomicU64>,
    background_task_sequence: Arc<AtomicU64>,
    pump_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    pump_generation: Arc<AtomicU64>,
    /// Fires when a translation stays pending too long (e.g. the server never
    /// returns the final for an incomplete sentence after the audio stops).
    /// Clears `is_translation_pending` so the UI does not sit on
    /// "正在翻译" forever; the shown draft/history is untouched.
    translation_timeout_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    translation_timeout_task_id: Arc<AtomicU64>,
    /// Explicit lifecycle operations and settings mutations share this lock,
    /// eliminating check-then-mutate races around credential/profile reads.
    lifecycle_lock: Arc<TokioMutex<()>>,
    lifecycle_operations: Arc<AtomicUsize>,
    lifecycle_sequence: Arc<AtomicU64>,
    lifecycle_notify: Arc<Notify>,
    active_generation: Arc<AtomicU64>,
    stopping_tail_generation: Arc<AtomicU64>,
    start_in_progress: Arc<AtomicBool>,
    generation_transition: Arc<Mutex<()>>,
    teardown_operations: Arc<AtomicUsize>,
    teardown_notify: Arc<Notify>,
}

impl SessionManager {
    pub fn new(app: AppHandle, settings: Arc<SettingsStore>) -> Arc<Self> {
        let audio_capture = SystemAudioCapture::for_app(&app);
        Arc::new(Self {
            app,
            settings,
            controller: Arc::new(Mutex::new(TranslationSessionController::default())),
            audio: Arc::new(Mutex::new(audio_capture)),
            client: Arc::new(Mutex::new(None)),
            client_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            audio_pipeline: Arc::new(Mutex::new(None)),
            audio_pipeline_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            capture_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            active_settings: Arc::new(Mutex::new(None)),
            active_settings_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            is_paused: Arc::new(AtomicBool::new(false)),
            is_recovering: Arc::new(AtomicBool::new(false)),
            is_overlay_collapsed: Arc::new(AtomicBool::new(false)),
            publish_dirty: Arc::new(AtomicBool::new(false)),
            publish_lock: Arc::new(TokioMutex::new(())),
            health_task: Arc::new(Mutex::new(None)),
            health_task_id: Arc::new(AtomicU64::new(NO_GENERATION)),
            recovery_task: Arc::new(Mutex::new(None)),
            recovery_task_id: Arc::new(AtomicU64::new(NO_GENERATION)),
            recovery_retry_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            background_task_sequence: Arc::new(AtomicU64::new(0)),
            pump_task: Arc::new(Mutex::new(None)),
            pump_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            translation_timeout_task: Arc::new(Mutex::new(None)),
            translation_timeout_task_id: Arc::new(AtomicU64::new(NO_GENERATION)),
            lifecycle_lock: Arc::new(TokioMutex::new(())),
            lifecycle_operations: Arc::new(AtomicUsize::new(0)),
            lifecycle_sequence: Arc::new(AtomicU64::new(0)),
            lifecycle_notify: Arc::new(Notify::new()),
            active_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            stopping_tail_generation: Arc::new(AtomicU64::new(NO_GENERATION)),
            start_in_progress: Arc::new(AtomicBool::new(false)),
            generation_transition: Arc::new(Mutex::new(())),
            teardown_operations: Arc::new(AtomicUsize::new(0)),
            teardown_notify: Arc::new(Notify::new()),
        })
    }

    pub fn is_active(&self) -> bool {
        lifecycle_activity_is_active(
            self.has_active_session(),
            self.lifecycle_operations.load(Ordering::SeqCst),
        )
    }

    /// True only for an established/in-flight session, excluding a settings
    /// mutation that merely owns the shared lifecycle lock.
    pub fn has_active_session(&self) -> bool {
        self.controller.lock().unwrap().state.status.is_active()
            || self.active_generation.load(Ordering::SeqCst) != NO_GENERATION
    }

    /// Serializes a profile or listening-settings mutation with start/stop.
    /// The caller must retain the returned guard until its synchronous store
    /// mutation and persistence have both completed.
    pub async fn settings_mutation_guard(
        self: &Arc<Self>,
        require_inactive: bool,
    ) -> Result<OwnedMutexGuard<()>, String> {
        let guard = self.lock_after_teardown().await;
        if require_inactive && self.is_active() {
            Err("Listening settings cannot be changed while a session is active.".into())
        } else {
            Ok(guard)
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    fn can_pause_current_session(&self) -> bool {
        let status = self.controller.lock().unwrap().state.status.clone();
        pause_transition_is_valid(
            &status,
            self.is_paused(),
            self.active_generation.load(Ordering::SeqCst),
        )
    }

    fn can_resume_current_session(&self) -> bool {
        let status = self.controller.lock().unwrap().state.status.clone();
        let has_active_settings = self.active_settings.lock().unwrap().is_some();
        resume_transition_is_valid(
            &status,
            self.is_paused(),
            self.active_generation.load(Ordering::SeqCst),
            has_active_settings,
        )
    }

    pub fn is_overlay_collapsed(&self) -> bool {
        self.is_overlay_collapsed.load(Ordering::SeqCst)
    }

    /// Starts (or restarts) a listening session with the saved settings.
    pub async fn start(self: &Arc<Self>, clear_subtitles: bool) -> Result<(), String> {
        if !try_begin_start(&self.start_in_progress) {
            return Ok(());
        }
        let _start_request = StartRequestGuard {
            in_progress: Arc::clone(&self.start_in_progress),
        };
        if !start_request_can_proceed(
            self.has_active_session(),
            self.is_recovering.load(Ordering::SeqCst),
            self.active_generation.load(Ordering::SeqCst),
        ) {
            return Ok(());
        }
        let _operation = self.begin_lifecycle_operation();
        let request_generation = self.next_lifecycle_request();
        let lifecycle = self.lock_after_teardown().await;
        if !self.is_lifecycle_request_current(request_generation)
            || !start_request_can_proceed(
                self.has_active_session(),
                self.is_recovering.load(Ordering::SeqCst),
                self.active_generation.load(Ordering::SeqCst),
            )
        {
            return Ok(());
        }
        // A manual start during recovery backoff is the newer user intent.
        // Cancel the old owner before installing this generation so its
        // global recovery flag cannot affect the new session's error path.
        self.cancel_recovery().await;
        self.active_generation
            .store(request_generation, Ordering::SeqCst);
        self.stopping_tail_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        if clear_subtitles {
            self.controller.lock().unwrap().clear_subtitles();
        }
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        // UI fixtures must never read credentials, open a socket, or touch
        // ScreenCaptureKit. This branch intentionally runs before resolving
        // settings because that resolution reads the OS credential store.
        if self.is_ui_test() {
            if self.is_generation_current(request_generation) {
                self.establish_ui_test_session(false);
            }
            return Ok(());
        }
        if let Err(error) = self.settings.prepare_for_listening() {
            if self.invalidate_generation(request_generation) {
                self.controller.lock().unwrap().did_fail(error.clone());
                self.publish_state();
                return Err(error);
            }
            return Err(SESSION_START_CANCELLED.into());
        }
        let configuration = match self.settings.configuration() {
            Ok(configuration) => configuration,
            Err(error) => {
                pipeline_log!("session settings failed label=settings_configuration");
                if self.invalidate_generation(request_generation) {
                    self.controller.lock().unwrap().did_fail(error.clone());
                    self.publish_state();
                    return Err(error);
                }
                return Err(SESSION_START_CANCELLED.into());
            }
        };
        pipeline_log!(
            "session start requested provider={} source={} target={} mode={:?}",
            configuration.provider.wire_value(),
            configuration.source_language.raw_value(),
            configuration.target_language.raw_value(),
            configuration.effective_translation_mode()
        );

        self.set_active_settings(request_generation, Some(configuration));
        // All shared state is now generation-tagged. Do not hold the
        // lifecycle gate across socket setup or capture authorization: stop
        // must be able to invalidate this generation immediately.
        drop(lifecycle);
        self.establish_session(false, request_generation).await
    }

    async fn establish_session(
        self: &Arc<Self>,
        clear_subtitles: bool,
        generation: u64,
    ) -> Result<(), String> {
        self.stop_health_checks().await;

        self.ensure_generation_current(generation)?;

        if self.is_ui_test() {
            self.establish_ui_test_session(clear_subtitles);
            return Ok(());
        }

        let mut clear = clear_subtitles;
        // A language/mode switch may land while connecting (the picker stays
        // usable during connecting); at most one rebuild picks up the change
        // so a rapid switch storm cannot loop forever.
        let mut attempts = 0usize;
        loop {
            attempts += 1;
            let configuration = match self.active_settings.lock().unwrap().clone() {
                Some(configuration) => configuration,
                None => {
                    let configuration = self.settings.configuration()?;
                    self.set_active_settings(generation, Some(configuration.clone()));
                    configuration
                }
            };

            pipeline_log!("session connecting clear={}", u8::from(clear));
            if clear {
                self.controller.lock().unwrap().clear_subtitles();
            }
            self.ensure_generation_current(generation)?;
            self.controller.lock().unwrap().begin_connecting();
            self.publish_state();

            let result = Arc::clone(self)
                .connect_and_listen(configuration.clone(), generation)
                .await;
            if let Err(error) = result {
                pipeline_log!("session establish failed label=provider_or_capture_setup");
                if !self.is_generation_current(generation) {
                    self.cleanup_generation_without_pump(generation).await;
                    return Err(SESSION_START_CANCELLED.into());
                }
                self.cleanup_generation(generation).await;
                let Some(failure_epoch) = self.invalidate_generation_with_epoch(generation) else {
                    return Err(SESSION_START_CANCELLED.into());
                };
                let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
                if self.lifecycle_sequence.load(Ordering::SeqCst) != failure_epoch {
                    return Err(SESSION_START_CANCELLED.into());
                }
                let is_recovering = self.is_recovering.load(Ordering::SeqCst);
                if !is_recovering {
                    self.clear_active_settings_for_generation(generation);
                }
                apply_establish_failure_state(
                    &mut self.controller.lock().unwrap(),
                    error.clone(),
                    is_recovering,
                );
                self.publish_state();
                return Err(error);
            }

            self.ensure_generation_current(generation)?;

            // The picker allows switching while connecting; if the settings
            // changed under the in-flight connect, rebuild the session with
            // the fresh configuration instead of going live with the stale
            // one (the user sees the capsule state and the session diverge
            // otherwise).
            let fresh = self.settings.configuration().ok();
            let stale = fresh.as_ref().is_none_or(|fresh| fresh != &configuration);
            if !stale || attempts >= 2 {
                return Ok(());
            }
            if let Some(fresh) = fresh {
                self.set_active_settings(generation, Some(fresh));
            }
            pipeline_log!("session rebuild for settings changed mid-connect");
            self.cleanup_generation(generation).await;
            self.ensure_generation_current(generation)?;
            clear = false;
        }
    }

    fn connect_and_listen(
        self: Arc<Self>,
        configuration: LiveTranslationConfiguration,
        generation: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            // Create the client and consume its events through this manager.
            let (event_tx, mut event_rx) = provider_event_channel();
            let new_client = TranslationClient::new(&configuration, event_tx).map_err(|error| {
                pipeline_log!(
                    "provider client creation failed label={}",
                    error.diagnostic_label()
                );
                error.to_string()
            })?;
            self.ensure_generation_current(generation)?;
            self.install_client(generation, new_client)?;

            // Start consuming before awaiting setup: a provider may acknowledge
            // setup and immediately send a terminal error/close in the same
            // socket read. The terminal event invalidates this generation, and
            // every later startup step checks it before installing resources.
            let self_arc = Arc::clone(&self);
            let pump = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    self_arc.handle_event(generation, event).await;
                }
            });
            self.install_pump(generation, pump);

            let client = self
                .client_for_generation(generation)
                .ok_or_else(|| SESSION_START_CANCELLED.to_string())?;
            let connect_result = self
                .run_while_generation_current(generation, client.connect())
                .await;
            let connect_result = match connect_result {
                Ok(result) => result,
                Err(error) => {
                    let _ = tokio::time::timeout(Duration::from_secs(1), client.disconnect()).await;
                    return Err(error);
                }
            };
            connect_result.map_err(|error| error.to_string())?;
            tokio::task::yield_now().await;
            if let Err(error) = self.ensure_generation_current(generation) {
                let _ = tokio::time::timeout(Duration::from_secs(1), client.disconnect()).await;
                return Err(error);
            }
            pipeline_log!("asr websocket connected");

            // Create the sole bounded audio queue before capture starts. The
            // native callback writes directly to this synchronous ingress;
            // there is no unbounded bridge ahead of the network sender.
            let audio_format = AudioCaptureFormat::pcm16_mono(
                configuration.provider.capabilities().input_sample_rate_hz,
            )
            .map_err(|error| error.to_string())?;
            let send_manager = Arc::clone(&self);
            let on_error_self = Arc::clone(&self);
            let pipeline = Arc::new(AudioSendPipeline::spawn(
                move |data| {
                    let manager = Arc::clone(&send_manager);
                    Box::pin(async move {
                        let client = manager.client_for_generation(generation);
                        match client {
                            Some(client) => client.send_audio(&data).await.map_err(|_| ()),
                            None => Err(()),
                        }
                    })
                },
                move |failure| {
                    let manager = Arc::clone(&on_error_self);
                    tokio::spawn(async move {
                        manager
                            .handle_audio_transport_failure(generation, failure)
                            .await;
                    });
                },
            ));
            let audio_ingress = pipeline
                .ingress()
                .ok_or_else(|| "The bounded audio pipeline is unavailable.".to_string())?;
            self.ensure_generation_current(generation)?;
            self.install_pipeline(generation, Arc::clone(&pipeline))?;

            let audio_failure_tx = self.capture_failure_channel(generation);
            self.ensure_generation_current(generation)?;
            self.capture_generation
                .compare_exchange(
                    NO_GENERATION,
                    generation,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .map_err(|_| {
                    "System audio capture is already assigned to a session.".to_string()
                })?;
            let capture = self.audio.lock().unwrap().clone();
            match self
                .run_while_generation_current(
                    generation,
                    capture.start(audio_ingress, audio_failure_tx, audio_format),
                )
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    // A platform capture may fail after it has installed a
                    // native stream but before its start acknowledgement is
                    // delivered. Always request generation-scoped teardown;
                    // the platform implementation also cleans up its own
                    // start token before returning this error.
                    self.stop_capture_for_generation(generation).await;
                    return Err(error.to_string());
                }
                Err(error) => {
                    self.stop_capture_for_generation(generation).await;
                    return Err(error);
                }
            }
            if let Err(error) = self.ensure_generation_current(generation) {
                self.stop_capture_for_generation(generation).await;
                return Err(error);
            }
            pipeline_log!("audio capture started");

            self.commit_listening(generation)?;
            self.publish_state();
            Arc::clone(&self).start_health_checks(generation).await;
            self.ensure_generation_current(generation)?;
            pipeline_log!("session listening");
            Ok(())
        })
    }

    pub async fn stop(self: &Arc<Self>) {
        let _operation = self.begin_lifecycle_operation();
        let _stop_request = self.next_lifecycle_request();
        let stopping_generation = self.active_generation.swap(NO_GENERATION, Ordering::SeqCst);
        if stopping_generation != NO_GENERATION {
            self.stopping_tail_generation
                .store(stopping_generation, Ordering::SeqCst);
        }
        let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.controller.lock().unwrap().state.status.is_active()
            && self.client.lock().unwrap().is_none()
            && self.capture_generation.load(Ordering::SeqCst) == NO_GENERATION
        {
            self.stop_health_checks().await;
            self.cancel_recovery().await;
            self.cancel_translation_timeout();
            self.clear_active_settings();
            self.is_recovering.store(false, Ordering::SeqCst);
            self.is_paused.store(false, Ordering::SeqCst);
            self.stopping_tail_generation
                .store(NO_GENERATION, Ordering::SeqCst);
            self.controller.lock().unwrap().did_stop();
            self.publish_state();
            return;
        }
        pipeline_log!("session stop requested");

        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.cancel_translation_timeout();
        self.clear_active_settings();
        self.is_recovering.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.controller.lock().unwrap().begin_stopping();
        self.publish_state();

        // Stop capture first so the queue has a fixed upper bound, then give
        // buffers already accepted by the network pipeline a finite drain
        // window before asking the provider to close its session.
        if stopping_generation != NO_GENERATION {
            self.stop_capture_for_generation(stopping_generation).await;
            self.finish_pipeline_for_generation(stopping_generation, Duration::from_secs(1))
                .await;
        } else {
            self.stop_any_capture().await;
            self.finish_pipeline(Duration::from_secs(1)).await;
        }
        let taken = if stopping_generation == NO_GENERATION {
            self.take_any_client()
        } else {
            self.take_client_for_generation(stopping_generation)
        };
        if let Some(client) = taken {
            if tokio::time::timeout(Duration::from_secs(6), client.finish())
                .await
                .is_err()
            {
                pipeline_log!("provider finish timed out");
                let _ = tokio::time::timeout(Duration::from_secs(1), client.disconnect()).await;
            }
        }
        if stopping_generation != NO_GENERATION {
            self.finish_pump_for_generation(stopping_generation, Duration::from_millis(500))
                .await;
        } else {
            self.stop_any_pump();
        }
        self.stopping_tail_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        self.controller.lock().unwrap().did_stop();
        self.publish_state();
        pipeline_log!("session stopped");
    }

    pub async fn toggle_paused(self: &Arc<Self>) {
        if self.is_ui_test() {
            let paused = self.is_paused.load(Ordering::SeqCst);
            self.is_paused.store(!paused, Ordering::SeqCst);
            if paused {
                self.controller.lock().unwrap().did_connect();
            } else {
                self.controller.lock().unwrap().did_pause();
            }
            self.publish_state();
            return;
        }
        if self.is_paused() {
            self.resume().await;
        } else {
            self.pause().await;
        }
    }

    pub async fn pause(self: &Arc<Self>) {
        if !self.can_pause_current_session() {
            return;
        }
        let _operation = self.begin_lifecycle_operation();
        let pause_request = self.next_lifecycle_request();
        let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.is_lifecycle_request_current(pause_request) || !self.can_pause_current_session() {
            return;
        }
        let paused_generation = self.active_generation.swap(NO_GENERATION, Ordering::SeqCst);
        pipeline_log!("session pause requested");
        self.is_paused.store(true, Ordering::SeqCst);
        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.cancel_translation_timeout();
        self.is_recovering.store(false, Ordering::SeqCst);
        self.controller.lock().unwrap().did_pause();
        self.publish_state();
        if paused_generation != NO_GENERATION {
            self.cleanup_generation(paused_generation).await;
        }
        pipeline_log!("session paused");
    }

    pub async fn resume(self: &Arc<Self>) {
        if !self.can_resume_current_session() {
            return;
        }
        let _operation = self.begin_lifecycle_operation();
        let resume_generation = self.next_lifecycle_request();
        let lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.is_lifecycle_request_current(resume_generation)
            || !self.can_resume_current_session()
        {
            return;
        }
        pipeline_log!("session resume requested");
        let paused_configuration = self.active_settings.lock().unwrap().clone();
        self.is_paused.store(false, Ordering::SeqCst);
        self.active_generation
            .store(resume_generation, Ordering::SeqCst);
        self.retag_active_settings(resume_generation);
        drop(lifecycle);
        let resumed = self.establish_session(false, resume_generation).await;
        match resumed {
            Ok(()) => {
                pipeline_log!("session resumed");
                return;
            }
            Err(error) if error == SESSION_START_CANCELLED => return,
            Err(error) => {
                let failure_epoch = resume_generation.wrapping_add(1);
                let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
                let status = self.controller.lock().unwrap().state.status.clone();
                if !resume_failure_is_still_owned(
                    &error,
                    failure_epoch,
                    self.lifecycle_sequence.load(Ordering::SeqCst),
                    &status,
                    self.active_generation.load(Ordering::SeqCst),
                ) {
                    return;
                }
            }
        }
        self.set_active_settings(NO_GENERATION, paused_configuration);
        self.is_paused.store(true, Ordering::SeqCst);
        self.controller.lock().unwrap().did_pause();
        self.publish_state();
        pipeline_log!("session resume failed; remaining paused");
    }

    /// Quick-switches the source language, reconnecting when needed.
    /// Alibaba automatic detection preserves an explicit Turbo choice and
    /// otherwise normalizes to Low Latency. OpenAI always remains on Turbo.
    pub async fn switch_source_language(self: &Arc<Self>, language: SourceLanguage) {
        let switch_epoch = self.lifecycle_sequence.load(Ordering::SeqCst);
        let lifecycle = match self.settings_mutation_guard(false).await {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if !self.is_lifecycle_request_current(switch_epoch) {
            return;
        }
        let status = self.controller.lock().unwrap().state.status.clone();
        if !pipeline_settings_mutation_is_allowed(
            &status,
            self.lifecycle_operations.load(Ordering::SeqCst),
        ) {
            return;
        }
        let provider = self
            .settings
            .active_profile()
            .map(|profile| profile.provider)
            .unwrap_or(ProviderKind::AlibabaCloud);
        if !provider.capabilities().source_languages.contains(&language) {
            return;
        }
        let (target_language, next_mode, needs_reconnect) = {
            let prefs = self.settings.preferences();
            let target = language
                .target_language_after_quick_switch(prefs.source_language, prefs.target_language);
            let mode =
                translation_mode_after_source_switch(provider, language, prefs.translation_mode);
            let needs_reconnect = source_switch_requires_reconnect(
                self.controller.lock().unwrap().state.status == SessionStatus::Listening,
                prefs.source_language,
                prefs.target_language,
                prefs.translation_mode,
                language,
                target,
                mode,
            );
            (target, mode, needs_reconnect)
        };
        if self
            .settings
            .save_preferences_for_active_profile(|prefs| {
                prefs.source_language = language;
                prefs.target_language = target_language;
                prefs.translation_mode = next_mode;
            })
            .is_err()
        {
            pipeline_log!("preferences unavailable label=source_switch_write_failed");
            return;
        }
        // A pause/stop can claim a newer lifecycle epoch while waiting for
        // this guard. Keep the current session's immutable settings snapshot
        // in sync before releasing the guard so a later pause/resume cannot
        // revive the pre-switch configuration even if reconnect loses that
        // newer-intent race.
        update_owned_value(
            &self.active_settings,
            &self.active_settings_generation,
            |configuration| {
                configuration.source_language = language;
                configuration.target_language = target_language;
                configuration.translation_mode = next_mode;
            },
        );
        // Broadcast immediately: the reconnect below can take seconds, and
        // every window (including the overlay control) must see the new
        // selection right away.
        self.publish_settings();
        if self.is_paused() {
            return;
        }
        if !needs_reconnect {
            return;
        }

        pipeline_log!(
            "session language switch source={} target={}",
            language.raw_value(),
            target_language.raw_value()
        );
        drop(lifecycle);
        self.reconnect_if_current(switch_epoch).await;
    }

    /// Quick-switches the translation mode, reconnecting when needed.
    pub async fn switch_translation_mode(self: &Arc<Self>, mode: TranslationMode) {
        let switch_epoch = self.lifecycle_sequence.load(Ordering::SeqCst);
        let lifecycle = match self.settings_mutation_guard(false).await {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if !self.is_lifecycle_request_current(switch_epoch) {
            return;
        }
        let status = self.controller.lock().unwrap().state.status.clone();
        if !pipeline_settings_mutation_is_allowed(
            &status,
            self.lifecycle_operations.load(Ordering::SeqCst),
        ) {
            return;
        }
        let provider = self
            .settings
            .active_profile()
            .map(|profile| profile.provider)
            .unwrap_or(ProviderKind::AlibabaCloud);
        if !provider.capabilities().translation_modes.contains(&mode) {
            return;
        }
        let current = self.settings.preferences().translation_mode;
        if current == mode {
            return;
        }
        if self
            .settings
            .save_preferences_for_active_profile(|prefs| prefs.translation_mode = mode)
            .is_err()
        {
            pipeline_log!("preferences unavailable label=mode_switch_write_failed");
            return;
        }
        update_owned_value(
            &self.active_settings,
            &self.active_settings_generation,
            |configuration| configuration.translation_mode = mode,
        );
        // Broadcast immediately: the reconnect below can take seconds.
        self.publish_settings();
        if self.is_paused() {
            return;
        }
        let is_listening = self.controller.lock().unwrap().state.status == SessionStatus::Listening;
        if !is_listening {
            return;
        }

        pipeline_log!("session translation mode switch mode={:?}", mode);
        drop(lifecycle);
        self.reconnect_if_current(switch_epoch).await;
    }

    /// Tears down and re-establishes the session, keeping subtitles.
    async fn reconnect_if_current(self: &Arc<Self>, expected_epoch: u64) {
        let _operation = self.begin_lifecycle_operation();
        let Some(reconnect_generation) = self.advance_lifecycle_request_if_current(expected_epoch)
        else {
            return;
        };
        let lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.is_lifecycle_request_current(reconnect_generation) {
            return;
        }
        let old_generation = self.active_generation.swap(NO_GENERATION, Ordering::SeqCst);
        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        drop(lifecycle);
        if old_generation != NO_GENERATION {
            self.cleanup_generation(old_generation).await;
        }
        let lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.is_lifecycle_request_current(reconnect_generation) {
            return;
        }
        self.active_generation
            .store(reconnect_generation, Ordering::SeqCst);
        self.retag_active_settings(reconnect_generation);
        drop(lifecycle);
        let _ = self.establish_session(false, reconnect_generation).await;
    }

    /// Machine-readable status kind for the global shortcut gate.
    pub fn status_kind(&self) -> String {
        match self.controller.lock().unwrap().state.status {
            SessionStatus::Idle => "idle".into(),
            SessionStatus::Connecting => "connecting".into(),
            SessionStatus::Listening => "listening".into(),
            SessionStatus::Stopping => "stopping".into(),
            SessionStatus::Error(_) => "error".into(),
        }
    }

    pub fn clear_subtitles(self: &Arc<Self>) {
        self.controller.lock().unwrap().clear_subtitles();
        self.publish_state();
    }

    pub fn set_overlay_collapsed(&self, collapsed: bool) {
        self.is_overlay_collapsed.store(collapsed, Ordering::SeqCst);
    }

    // MARK: event handling

    async fn handle_event(self: &Arc<Self>, generation: u64, event: LiveTranslateServerEvent) {
        if !self.accepts_event(generation, &event) || self.is_paused() {
            return;
        }

        // Setup acknowledgements are consumed by `connect`; only
        // `connect_and_listen` may transition to Listening after audio and the
        // send pipeline for the same generation are installed.
        if matches!(
            event,
            LiveTranslateServerEvent::SessionCreated | LiveTranslateServerEvent::SessionUpdated
        ) {
            return;
        }

        if let LiveTranslateServerEvent::Error { code, message } = &event {
            if provider_error_is_retryable(code) {
                let _teardown = self.begin_teardown_operation();
                let _operation = self.begin_lifecycle_operation();
                let recovery_owns_attempt = self.is_recovering.load(Ordering::SeqCst);
                if recovery_owns_attempt {
                    // Register the handoff before invalidation wakes the
                    // generation-bound connect future. The current recovery
                    // task consumes this marker and owns the next retry.
                    self.recovery_retry_generation
                        .store(generation, Ordering::SeqCst);
                }
                if !self.invalidate_generation(generation) {
                    if recovery_owns_attempt {
                        let _ = self.recovery_retry_generation.compare_exchange(
                            generation,
                            NO_GENERATION,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                    return;
                }
                pipeline_log!("session transport error code={}", code);
                self.cancel_translation_timeout();
                self.controller.lock().unwrap().begin_connecting();
                self.publish_state();
                let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
                self.stop_health_checks().await;
                self.cleanup_generation_without_pump(generation).await;
                if !recovery_owns_attempt {
                    self.queue_recovery(generation, message.clone()).await;
                }
                return;
            }
        }

        // A final translation resolves any pending timeout; a fresh
        // TranslationStarted arms a new one.
        if matches!(
            event,
            LiveTranslateServerEvent::TranslationFinal(_)
                | LiveTranslateServerEvent::SubtitleFinalPair { .. }
                | LiveTranslateServerEvent::Error { .. }
        ) {
            self.cancel_translation_timeout();
        }
        if matches!(event, LiveTranslateServerEvent::TranslationStarted) {
            self.arm_translation_timeout(generation);
        }

        let is_terminal = matches!(
            event,
            LiveTranslateServerEvent::Error { .. } | LiveTranslateServerEvent::SessionFinished
        );
        let terminal_teardown = is_terminal.then(|| self.begin_teardown_operation());
        let terminal_operation = is_terminal.then(|| self.begin_lifecycle_operation());
        if is_terminal && !self.invalidate_generation(generation) {
            return;
        }
        self.controller.lock().unwrap().handle(event.clone());
        self.publish_state();

        if is_terminal {
            // Provider error codes are not universally trustworthy (some
            // protocols carry arbitrary server strings). Keep diagnostics on
            // a fixed local label just like free-text messages.
            let label = match &event {
                LiveTranslateServerEvent::Error { .. } => "provider_terminal_error",
                LiveTranslateServerEvent::SessionFinished => "session_finished",
                _ => "terminal_event",
            };
            pipeline_log!("session terminal event label={label}");
            let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
            self.stop_health_checks().await;
            self.cancel_recovery().await;
            self.cleanup_generation_without_pump(generation).await;
            self.clear_active_settings_for_generation(generation);
            drop(terminal_operation);
            drop(terminal_teardown);
        }
    }

    /// Arms a timer that clears a stuck "正在翻译" state. The low-latency
    /// stream protocol synthesizes TranslationStarted at the source final,
    /// then waits for the server's `response.text.done`; if the audio stops
    /// mid-sentence the server may never finalize, so without this the UI
    /// would stay pending forever. The shown subtitle is left untouched.
    fn arm_translation_timeout(self: &Arc<Self>, generation: u64) {
        // Install/cancel under one slot lock. Otherwise a concurrent cancel
        // can run after `spawn` but before the handle is stored, leaving a
        // detached stale handle in the slot even though its ownership id was
        // already invalidated.
        let mut slot = self.translation_timeout_task.lock().unwrap();
        if let Some(task) = slot.take() {
            task.abort();
        }
        let task_id = self.next_background_task_id();
        self.translation_timeout_task_id
            .store(task_id, Ordering::SeqCst);
        let this = Arc::clone(self);
        let task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            if !this.is_generation_current(generation)
                || !this.clear_translation_timeout_task_if_id(task_id)
            {
                return;
            }
            pipeline_log!("translation pending timed out; clearing");
            this.controller.lock().unwrap().clear_translation_pending();
            this.publish_state();
        });
        *slot = Some(task);
    }

    fn cancel_translation_timeout(self: &Arc<Self>) {
        let mut slot = self.translation_timeout_task.lock().unwrap();
        self.translation_timeout_task_id
            .store(NO_GENERATION, Ordering::SeqCst);
        if let Some(task) = slot.take() {
            task.abort();
        }
    }

    async fn handle_capture_failure(
        self: &Arc<Self>,
        generation: u64,
        failure: SystemAudioCaptureFailure,
    ) {
        self.handle_recoverable_runtime_failure(
            generation,
            failure.to_string(),
            failure.diagnostic_label(),
        )
        .await;
    }

    async fn handle_audio_transport_failure(
        self: &Arc<Self>,
        generation: u64,
        failure: AudioPipelineFailure,
    ) {
        self.handle_recoverable_runtime_failure(
            generation,
            failure.to_string(),
            failure.diagnostic_label(),
        )
        .await;
    }

    async fn handle_recoverable_runtime_failure(
        self: &Arc<Self>,
        generation: u64,
        message: String,
        diagnostic_label: &'static str,
    ) {
        let _teardown = self.begin_teardown_operation();
        let _operation = self.begin_lifecycle_operation();
        if self.is_paused() || self.active_settings.lock().unwrap().is_none() {
            return;
        }
        let recovery_owns_attempt = self.is_recovering.load(Ordering::SeqCst);
        if recovery_owns_attempt {
            self.recovery_retry_generation
                .store(generation, Ordering::SeqCst);
        }
        if !self.invalidate_generation(generation) {
            if recovery_owns_attempt {
                let _ = self.recovery_retry_generation.compare_exchange(
                    generation,
                    NO_GENERATION,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }
            return;
        }
        pipeline_log!("runtime stream failed label={}", diagnostic_label);
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        self.stop_health_checks().await;
        self.cleanup_generation(generation).await;
        if !recovery_owns_attempt {
            self.queue_recovery(generation, message).await;
        }
    }

    // MARK: health checks and recovery

    async fn start_health_checks(self: &Arc<Self>, generation: u64) {
        self.stop_health_checks().await;
        let task_id = self.next_background_task_id();
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if !self_arc.check_connection_health(generation, task_id).await {
                    return;
                }
            }
        });
        let mut slot = self.health_task.lock().unwrap();
        self.health_task_id.store(task_id, Ordering::SeqCst);
        *slot = Some(task);
    }

    async fn stop_health_checks(&self) {
        let mut slot = self.health_task.lock().unwrap();
        self.health_task_id.store(NO_GENERATION, Ordering::SeqCst);
        if let Some(task) = slot.take() {
            task.abort();
        }
    }

    async fn check_connection_health(self: &Arc<Self>, generation: u64, task_id: u64) -> bool {
        if self.is_paused()
            || self.is_recovering.load(Ordering::SeqCst)
            || !self.is_generation_current(generation)
            || self.health_task_id.load(Ordering::SeqCst) != task_id
        {
            return false;
        }
        let Some(client) = self.client_for_generation(generation) else {
            return false;
        };
        match client.ping(Duration::from_secs(4)).await {
            Ok(()) => {
                self.is_generation_current(generation)
                    && self.health_task_id.load(Ordering::SeqCst) == task_id
            }
            Err(error) => {
                if !self.is_generation_current(generation)
                    || self.health_task_id.load(Ordering::SeqCst) != task_id
                {
                    return false;
                }
                pipeline_log!(
                    "connection health failed label={}",
                    error.diagnostic_label()
                );
                self.clear_health_task_if_id(task_id);
                if self.invalidate_generation(generation) {
                    self.queue_recovery(generation, error.to_string()).await;
                }
                false
            }
        }
    }

    async fn queue_recovery(self: &Arc<Self>, failed_generation: u64, failure_message: String) {
        if self.is_paused() || self.active_settings.lock().unwrap().is_none() {
            return;
        }
        let mut slot = self.recovery_task.lock().unwrap();
        if slot.is_some() {
            return;
        }
        let task_id = self.next_background_task_id();
        self.recovery_task_id.store(task_id, Ordering::SeqCst);
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            self_arc
                .recover_connection(failed_generation, failure_message)
                .await;
            self_arc.clear_recovery_task_if_id(task_id);
        });
        *slot = Some(task);
    }

    async fn recover_connection(self: &Arc<Self>, failed_generation: u64, failure_message: String) {
        if self.is_paused() || self.is_recovering.load(Ordering::SeqCst) {
            return;
        }
        let _operation = self.begin_lifecycle_operation();
        let mut recovery_generation = self.next_lifecycle_request();
        let lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
        if !self.is_lifecycle_request_current(recovery_generation) {
            return;
        }
        pipeline_log!("session recovery started");
        self.is_recovering.store(true, Ordering::SeqCst);
        self.recovery_retry_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        self.stop_health_checks().await;
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        drop(lifecycle);
        self.cleanup_generation(failed_generation).await;

        let mut recovered = false;
        let mut recovery_epoch = recovery_generation;
        for attempt in 0..RECOVERY_ATTEMPTS {
            let delay = recovery_delay(attempt, failed_generation);
            pipeline_log!(
                "session recovery attempt={} delayMs={}",
                attempt + 1,
                delay.as_millis()
            );
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            if self.active_settings.lock().unwrap().is_none() {
                clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                return;
            }
            if attempt > 0 {
                let Some(next_generation) =
                    self.advance_lifecycle_request_if_current(recovery_epoch)
                else {
                    clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                    return;
                };
                recovery_generation = next_generation;
            }
            let lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
            if !self.is_lifecycle_request_current(recovery_generation) {
                clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                return;
            }
            self.active_generation
                .store(recovery_generation, Ordering::SeqCst);
            self.retag_active_settings(recovery_generation);
            drop(lifecycle);
            match self.establish_session(false, recovery_generation).await {
                Ok(()) => {
                    recovered = true;
                    break;
                }
                Err(error) if error == SESSION_START_CANCELLED => {
                    let retry_generation = self
                        .recovery_retry_generation
                        .swap(NO_GENERATION, Ordering::SeqCst);
                    let current_epoch = self.lifecycle_sequence.load(Ordering::SeqCst);
                    if self.is_recovering.load(Ordering::SeqCst)
                        && cancelled_recovery_attempt_is_retryable(
                            retry_generation,
                            recovery_generation,
                            current_epoch,
                            self.active_settings.lock().unwrap().is_some(),
                        )
                    {
                        recovery_epoch = current_epoch;
                        continue;
                    }
                    clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                    return;
                }
                Err(_) => {
                    let failure_epoch = recovery_generation.wrapping_add(1);
                    if self.lifecycle_sequence.load(Ordering::SeqCst) != failure_epoch {
                        clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                        return;
                    }
                    recovery_epoch = failure_epoch;
                }
            }
        }

        if !recovered {
            let _lifecycle = Arc::clone(&self.lifecycle_lock).lock_owned().await;
            if !recovery_exhaustion_is_still_owned(
                recovery_epoch,
                self.lifecycle_sequence.load(Ordering::SeqCst),
                self.active_generation.load(Ordering::SeqCst),
            ) {
                clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
                return;
            }
            pipeline_log!("session recovery exhausted");
            self.clear_active_settings_for_generation(recovery_generation);
            clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
            self.controller.lock().unwrap().did_fail(failure_message);
            self.publish_state();
            return;
        }
        clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
    }

    async fn cancel_recovery(&self) {
        let mut slot = self.recovery_task.lock().unwrap();
        self.recovery_task_id.store(NO_GENERATION, Ordering::SeqCst);
        clear_recovery_atoms(&self.is_recovering, &self.recovery_retry_generation);
        if let Some(task) = slot.take() {
            task.abort();
        }
    }

    async fn finish_pipeline(&self, timeout: Duration) {
        let pipeline = self.audio_pipeline.lock().unwrap().take();
        if let Some(pipeline) = pipeline {
            self.audio_pipeline_generation
                .store(NO_GENERATION, Ordering::SeqCst);
            if !pipeline.finish(timeout).await {
                pipeline_log!("audio pipeline drain timed out");
            }
        }
    }

    /// Capacity-one channel through which a native callback reports its first
    /// fatal failure without blocking the real-time audio thread.
    fn capture_failure_channel(self: &Arc<Self>, generation: u64) -> CaptureFailureSender {
        let (tx, mut rx) = CaptureFailureSender::channel();
        let self_arc = Arc::clone(self);
        tokio::spawn(async move {
            if let Some(failure) = rx.recv().await {
                self_arc
                    .clone()
                    .handle_capture_failure(generation, failure)
                    .await;
            }
        });
        tx
    }

    fn begin_lifecycle_operation(&self) -> LifecycleOperationGuard {
        self.lifecycle_operations.fetch_add(1, Ordering::SeqCst);
        LifecycleOperationGuard {
            count: Arc::clone(&self.lifecycle_operations),
        }
    }

    fn begin_teardown_operation(&self) -> TeardownOperationGuard {
        self.teardown_operations.fetch_add(1, Ordering::SeqCst);
        TeardownOperationGuard {
            count: Arc::clone(&self.teardown_operations),
            notify: Arc::clone(&self.teardown_notify),
        }
    }

    async fn lock_after_teardown(&self) -> OwnedMutexGuard<()> {
        lock_after_operations(
            Arc::clone(&self.lifecycle_lock),
            Arc::clone(&self.teardown_operations),
            Arc::clone(&self.teardown_notify),
        )
        .await
    }

    fn next_lifecycle_request(&self) -> u64 {
        let _transition = self.generation_transition.lock().unwrap();
        let generation = self
            .lifecycle_sequence
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        let generation = if generation == NO_GENERATION {
            self.lifecycle_sequence.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            generation
        };
        self.lifecycle_notify.notify_waiters();
        generation
    }

    fn advance_lifecycle_request_if_current(&self, expected: u64) -> Option<u64> {
        let _transition = self.generation_transition.lock().unwrap();
        let generation = advance_lifecycle_sequence_if_current(&self.lifecycle_sequence, expected)?;
        self.lifecycle_notify.notify_waiters();
        Some(generation)
    }

    fn next_background_task_id(&self) -> u64 {
        let task_id = self
            .background_task_sequence
            .fetch_add(1, Ordering::SeqCst)
            .wrapping_add(1);
        if task_id == NO_GENERATION {
            self.background_task_sequence.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            task_id
        }
    }

    fn clear_health_task_if_id(&self, task_id: u64) -> bool {
        clear_task_slot_if_id(&self.health_task, &self.health_task_id, task_id)
    }

    fn clear_recovery_task_if_id(&self, task_id: u64) -> bool {
        clear_task_slot_if_id(&self.recovery_task, &self.recovery_task_id, task_id)
    }

    fn clear_translation_timeout_task_if_id(&self, task_id: u64) -> bool {
        clear_task_slot_if_id(
            &self.translation_timeout_task,
            &self.translation_timeout_task_id,
            task_id,
        )
    }

    fn is_lifecycle_request_current(&self, generation: u64) -> bool {
        lifecycle_sequence_matches(&self.lifecycle_sequence, generation)
    }

    fn is_generation_current(&self, generation: u64) -> bool {
        generation != NO_GENERATION && self.active_generation.load(Ordering::SeqCst) == generation
    }

    fn ensure_generation_current(&self, generation: u64) -> Result<(), String> {
        if self.is_generation_current(generation) {
            Ok(())
        } else {
            Err(SESSION_START_CANCELLED.into())
        }
    }

    async fn run_while_generation_current<T>(
        &self,
        generation: u64,
        operation: impl Future<Output = T>,
    ) -> Result<T, String> {
        run_generation_bound_operation(
            Arc::clone(&self.active_generation),
            Arc::clone(&self.lifecycle_sequence),
            Arc::clone(&self.lifecycle_notify),
            generation,
            operation,
        )
        .await
    }

    /// Invalidates only the specified generation. The sequence bump also
    /// cancels a startup that has not yet reached its next generation check.
    fn invalidate_generation(&self, generation: u64) -> bool {
        self.invalidate_generation_with_epoch(generation).is_some()
    }

    fn invalidate_generation_with_epoch(&self, generation: u64) -> Option<u64> {
        let _transition = self.generation_transition.lock().unwrap();
        let owned_epoch = invalidate_generation_atoms(
            &self.active_generation,
            &self.lifecycle_sequence,
            generation,
        );
        if owned_epoch.is_some() {
            self.lifecycle_notify.notify_waiters();
        }
        owned_epoch
    }

    fn commit_listening(&self, generation: u64) -> Result<(), String> {
        let _transition = self.generation_transition.lock().unwrap();
        self.ensure_generation_current(generation)?;
        self.controller.lock().unwrap().did_connect();
        Ok(())
    }

    fn accepts_event(&self, generation: u64, event: &LiveTranslateServerEvent) -> bool {
        generation_accepts_event(
            self.active_generation.load(Ordering::SeqCst),
            self.stopping_tail_generation.load(Ordering::SeqCst),
            generation,
            event,
        )
    }

    fn install_client(&self, generation: u64, client: TranslationClient) -> Result<(), String> {
        let mut slot = self.client.lock().unwrap();
        if slot.is_some() {
            return Err("A live translation client is already installed.".into());
        }
        *slot = Some(client);
        self.client_generation.store(generation, Ordering::SeqCst);
        Ok(())
    }

    fn set_active_settings(
        &self,
        generation: u64,
        configuration: Option<LiveTranslationConfiguration>,
    ) {
        let mut slot = self.active_settings.lock().unwrap();
        *slot = configuration;
        self.active_settings_generation.store(
            if slot.is_some() {
                generation
            } else {
                NO_GENERATION
            },
            Ordering::SeqCst,
        );
    }

    fn retag_active_settings(&self, generation: u64) {
        let slot = self.active_settings.lock().unwrap();
        if slot.is_some() {
            self.active_settings_generation
                .store(generation, Ordering::SeqCst);
        }
    }

    fn clear_active_settings_for_generation(&self, generation: u64) -> bool {
        clear_owned_value_if_generation(
            &self.active_settings,
            &self.active_settings_generation,
            generation,
        )
    }

    fn clear_active_settings(&self) {
        *self.active_settings.lock().unwrap() = None;
        self.active_settings_generation
            .store(NO_GENERATION, Ordering::SeqCst);
    }

    fn client_for_generation(&self, generation: u64) -> Option<TranslationClient> {
        let slot = self.client.lock().unwrap();
        if self.client_generation.load(Ordering::SeqCst) == generation {
            slot.clone()
        } else {
            None
        }
    }

    fn take_client_for_generation(&self, generation: u64) -> Option<TranslationClient> {
        let mut slot = self.client.lock().unwrap();
        if self.client_generation.load(Ordering::SeqCst) != generation {
            return None;
        }
        self.client_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        slot.take()
    }

    fn take_any_client(&self) -> Option<TranslationClient> {
        let mut slot = self.client.lock().unwrap();
        self.client_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        slot.take()
    }

    fn install_pipeline(
        &self,
        generation: u64,
        pipeline: Arc<AudioSendPipeline>,
    ) -> Result<(), String> {
        let mut slot = self.audio_pipeline.lock().unwrap();
        if slot.is_some() {
            return Err("An audio send pipeline is already installed.".into());
        }
        *slot = Some(pipeline);
        self.audio_pipeline_generation
            .store(generation, Ordering::SeqCst);
        Ok(())
    }

    fn take_pipeline_for_generation(&self, generation: u64) -> Option<Arc<AudioSendPipeline>> {
        let mut slot = self.audio_pipeline.lock().unwrap();
        if self.audio_pipeline_generation.load(Ordering::SeqCst) != generation {
            return None;
        }
        self.audio_pipeline_generation
            .store(NO_GENERATION, Ordering::SeqCst);
        slot.take()
    }

    fn install_pump(&self, generation: u64, pump: JoinHandle<()>) {
        let mut slot = self.pump_task.lock().unwrap();
        if let Some(old) = slot.replace(pump) {
            old.abort();
        }
        self.pump_generation.store(generation, Ordering::SeqCst);
    }

    fn stop_pump_for_generation(&self, generation: u64) {
        if let Some(task) = self.take_pump_for_generation(generation) {
            task.abort();
        }
    }

    fn take_pump_for_generation(&self, generation: u64) -> Option<JoinHandle<()>> {
        let mut slot = self.pump_task.lock().unwrap();
        if self.pump_generation.load(Ordering::SeqCst) != generation {
            return None;
        }
        self.pump_generation.store(NO_GENERATION, Ordering::SeqCst);
        slot.take()
    }

    async fn finish_pump_for_generation(&self, generation: u64, timeout: Duration) {
        let Some(mut task) = self.take_pump_for_generation(generation) else {
            return;
        };
        if tokio::time::timeout(timeout, &mut task).await.is_err() {
            task.abort();
        }
    }

    fn stop_any_pump(&self) {
        self.pump_generation.store(NO_GENERATION, Ordering::SeqCst);
        if let Some(task) = self.pump_task.lock().unwrap().take() {
            task.abort();
        }
    }

    async fn stop_capture_for_generation(&self, generation: u64) {
        if self
            .capture_generation
            .compare_exchange(
                generation,
                NO_GENERATION,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return;
        }
        let capture = self.audio.lock().unwrap().clone();
        if tokio::time::timeout(Duration::from_secs(2), capture.stop())
            .await
            .is_err()
        {
            pipeline_log!("audio capture stop timed out");
        }
    }

    async fn stop_any_capture(&self) {
        if self
            .capture_generation
            .swap(NO_GENERATION, Ordering::SeqCst)
            == NO_GENERATION
        {
            return;
        }
        let capture = self.audio.lock().unwrap().clone();
        if tokio::time::timeout(Duration::from_secs(2), capture.stop())
            .await
            .is_err()
        {
            pipeline_log!("audio capture stop timed out");
        }
    }

    async fn finish_pipeline_for_generation(&self, generation: u64, timeout: Duration) {
        if let Some(pipeline) = self.take_pipeline_for_generation(generation) {
            if !pipeline.finish(timeout).await {
                pipeline_log!("audio pipeline drain timed out");
            }
        }
    }

    async fn cleanup_generation(&self, generation: u64) {
        self.cleanup_generation_resources(generation, true).await;
    }

    async fn cleanup_generation_without_pump(&self, generation: u64) {
        self.cleanup_generation_resources(generation, false).await;
    }

    async fn cleanup_generation_resources(&self, generation: u64, stop_pump: bool) {
        if let Some(pipeline) = self.take_pipeline_for_generation(generation) {
            pipeline.stop();
        }
        self.stop_capture_for_generation(generation).await;
        if let Some(client) = self.take_client_for_generation(generation) {
            let _ = tokio::time::timeout(Duration::from_secs(2), client.disconnect()).await;
        }
        if stop_pump {
            self.stop_pump_for_generation(generation);
        }
    }

    // MARK: state publishing

    /// Builds the current session state snapshot without emitting it (used by
    /// windows that boot after the last broadcast, e.g. the overlay control).
    pub fn current_state_event(&self) -> SessionStateEvent {
        let state = self.controller.lock().unwrap().state.clone();
        let mut event = SessionStateEvent::from(&state);
        event.is_active |= self.is_recovering.load(Ordering::SeqCst);
        event.is_paused = self.is_paused();
        event.is_overlay_collapsed = self.is_overlay_collapsed();
        event
    }

    /// Broadcasts the current session state to the frontend, coalescing
    /// high-frequency calls: subtitle chunks stream in at tens of events per
    /// second and every snapshot carries the full subtitle history, so
    /// emitting per chunk is the main UI-lag cost during live listening. The
    /// first call schedules a single trailing broadcast ~60ms later that
    /// always carries the latest snapshot; intermediate calls only set the
    /// dirty flag. Status-only changes therefore reach the UI within 60ms
    /// and bursty chunks are folded into one emit.
    pub fn publish_state(self: &Arc<Self>) {
        self.publish_dirty.store(true, Ordering::SeqCst);
        let Ok(guard) = Arc::clone(&self.publish_lock).try_lock_owned() else {
            return;
        };
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let guard = guard;
            // The first caller's dirty bit is represented by this task itself;
            // clear it before waiting so the first publish below is not
            // followed by a duplicate one.
            this.publish_dirty.store(false, Ordering::SeqCst);
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                this.publish_state_now();
                if !this.publish_dirty.swap(false, Ordering::SeqCst) {
                    // Release the scheduling lock first, then check for a
                    // caller that set the dirty bit while the lock was still
                    // held (that caller could not schedule its own task).
                    drop(guard);
                    if this.publish_dirty.load(Ordering::SeqCst) {
                        this.publish_state_now();
                    }
                    return;
                }
            }
        });
    }

    fn publish_state_now(self: &Arc<Self>) {
        let event = self.current_state_event();
        let is_active = event.is_active;
        let is_collapsed = event.is_overlay_collapsed;
        let preferences = self.settings.preferences();
        let click_through =
            preferences.overlay_locked || preferences.subtitle_blends_with_background;
        let _ = self.app.emit("session-state", event);
        OverlayWindowManager::sync_presentation(&self.app, is_active, is_collapsed, click_through);
    }

    /// Broadcasts the current settings snapshot to every window (used after
    /// preference writes so no window keeps a stale selection, and when a
    /// window re-shows in case its webview missed events while hidden).
    pub fn publish_settings(&self) {
        let _ = self.app.emit(
            "settings-changed",
            crate::commands::SettingsSnapshotPayload::from_store(&self.settings),
        );
    }

    fn is_ui_test(&self) -> bool {
        self.settings.is_ui_test()
    }

    fn establish_ui_test_session(self: &Arc<Self>, clear_subtitles: bool) {
        self.cancel_translation_timeout();
        self.clear_active_settings();
        if clear_subtitles {
            self.controller.lock().unwrap().clear_subtitles();
        }
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        self.controller.lock().unwrap().did_connect();
        self.publish_state();
        pipeline_log!("ui-test synthetic session listening");
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn recovery_backoff_is_bounded_exponential_and_deterministic() {
        let generation = 42;
        assert_eq!(recovery_delay(0, generation), Duration::ZERO);

        for (attempt, base_ms) in [(1, 500_u128), (2, 1_000), (3, 2_000)] {
            let first = recovery_delay(attempt, generation);
            let second = recovery_delay(attempt, generation);
            assert_eq!(first, second);
            assert!(first.as_millis() >= base_ms);
            assert!(first.as_millis() <= base_ms + base_ms / 4);
        }

        assert!(recovery_delay(2, generation) > recovery_delay(1, generation));
        assert!(recovery_delay(3, generation) > recovery_delay(2, generation));
    }

    #[test]
    fn only_transient_transport_and_bounded_queue_errors_recover() {
        for code in [
            "transport_error",
            "provider_event_backlog_overflow",
            "translation_backlog_overflow",
        ] {
            assert!(provider_error_is_retryable(code));
        }
        assert!(!provider_error_is_retryable("invalid_api_key"));
        assert!(!provider_error_is_retryable("invalid_configuration"));
    }

    #[test]
    fn terminal_after_setup_ack_invalidates_startup_before_listening_commit() {
        let active = AtomicU64::new(41);
        let sequence = AtomicU64::new(41);

        assert_eq!(
            invalidate_generation_atoms(&active, &sequence, 41),
            Some(42)
        );
        assert_eq!(active.load(Ordering::SeqCst), NO_GENERATION);
        assert_ne!(sequence.load(Ordering::SeqCst), 41);
        assert!(!generation_accepts_event(
            active.load(Ordering::SeqCst),
            NO_GENERATION,
            41,
            &LiveTranslateServerEvent::TranslationStarted,
        ));
    }

    #[test]
    fn stopped_start_does_not_publish_a_late_configuration_failure() {
        let generation = 51;
        let active = AtomicU64::new(generation);
        let sequence = AtomicU64::new(generation);
        let mut controller = TranslationSessionController::default();
        controller.begin_connecting();

        // Stop claims the generation before the in-flight settings read
        // returns its error.
        sequence.store(generation + 1, Ordering::SeqCst);
        assert_eq!(active.swap(NO_GENERATION, Ordering::SeqCst), generation);
        if invalidate_generation_atoms(&active, &sequence, generation).is_some() {
            controller.did_fail("late settings failure");
        }

        controller.did_stop();
        assert_eq!(controller.state.status, SessionStatus::Idle);
    }

    #[tokio::test]
    async fn stop_after_error_invalidation_cannot_be_adopted_by_the_old_failure() {
        let generation = 61;
        let active = Arc::new(AtomicU64::new(generation));
        let sequence = Arc::new(AtomicU64::new(generation));
        let controller = Arc::new(Mutex::new(TranslationSessionController::default()));
        controller.lock().unwrap().begin_connecting();
        let (invalidated_tx, invalidated_rx) = tokio::sync::oneshot::channel();
        let (continue_tx, continue_rx) = tokio::sync::oneshot::channel();

        let old_failure = {
            let active = Arc::clone(&active);
            let sequence = Arc::clone(&sequence);
            let controller = Arc::clone(&controller);
            tokio::spawn(async move {
                let owned_epoch =
                    invalidate_generation_atoms(&active, &sequence, generation).unwrap();
                invalidated_tx.send(owned_epoch).unwrap();
                continue_rx.await.unwrap();
                if sequence.load(Ordering::SeqCst) == owned_epoch {
                    controller.lock().unwrap().did_fail("stale connect failure");
                }
            })
        };

        let failure_epoch = invalidated_rx.await.unwrap();
        assert_eq!(failure_epoch, generation + 1);
        // Stop is the newer owner and reaches Idle before the old connect
        // failure resumes between invalidation and publication.
        sequence.fetch_add(1, Ordering::SeqCst);
        active.store(NO_GENERATION, Ordering::SeqCst);
        controller.lock().unwrap().did_stop();
        continue_tx.send(()).unwrap();
        old_failure.await.unwrap();

        assert_eq!(controller.lock().unwrap().state.status, SessionStatus::Idle);
        assert_eq!(sequence.load(Ordering::SeqCst), generation + 2);
    }

    #[test]
    fn generation_invalidation_cannot_claim_an_epoch_already_owned_by_stop() {
        let generation = 71;
        let active = AtomicU64::new(generation);
        let sequence = AtomicU64::new(generation + 1);

        assert_eq!(
            invalidate_generation_atoms(&active, &sequence, generation),
            None
        );
        assert_eq!(active.load(Ordering::SeqCst), generation);
        assert_eq!(sequence.load(Ordering::SeqCst), generation + 1);
    }

    #[test]
    fn concurrent_double_start_has_one_owner() {
        let in_progress = Arc::new(AtomicBool::new(false));
        assert!(try_begin_start(&in_progress));
        assert!(!try_begin_start(&in_progress));

        let owner = StartRequestGuard {
            in_progress: Arc::clone(&in_progress),
        };
        drop(owner);
        assert!(try_begin_start(&in_progress));
        in_progress.store(false, Ordering::SeqCst);
    }

    #[test]
    fn stop_rejects_old_events_but_keeps_atomic_confirmed_tail() {
        let generation = 9;
        let final_pair = LiveTranslateServerEvent::SubtitleFinalPair {
            source: "tail".into(),
            language: Some("en".into()),
            translation: "尾句".into(),
        };
        assert!(generation_accepts_event(
            NO_GENERATION,
            generation,
            generation,
            &final_pair,
        ));
        assert!(!generation_accepts_event(
            NO_GENERATION,
            generation,
            generation,
            &LiveTranslateServerEvent::TranslationDraft("stale".into()),
        ));
    }

    #[tokio::test]
    async fn settings_mutation_and_lifecycle_operation_are_serialized() {
        let lock = Arc::new(TokioMutex::new(()));
        let settings_guard = Arc::clone(&lock).lock_owned().await;
        let lock_for_stop = Arc::clone(&lock);
        let mut stop_waiter = tokio::spawn(async move {
            let _stop_guard = lock_for_stop.lock_owned().await;
        });

        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut stop_waiter)
                .await
                .is_err()
        );
        drop(settings_guard);
        assert!(
            tokio::time::timeout(Duration::from_millis(200), stop_waiter)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn terminal_teardown_blocks_new_start_or_profile_mutation() {
        let lock = Arc::new(TokioMutex::new(()));
        let teardown_count = Arc::new(AtomicUsize::new(1));
        let notify = Arc::new(Notify::new());
        let mut waiter = tokio::spawn(lock_after_operations(
            Arc::clone(&lock),
            Arc::clone(&teardown_count),
            Arc::clone(&notify),
        ));

        assert!(tokio::time::timeout(Duration::from_millis(20), &mut waiter)
            .await
            .is_err());
        teardown_count.store(0, Ordering::SeqCst);
        notify.notify_waiters();
        assert!(tokio::time::timeout(Duration::from_millis(200), waiter)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn teardown_notification_registered_before_await_is_not_lost() {
        let notify = Notify::new();
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        notify.notify_waiters();

        assert!(tokio::time::timeout(Duration::from_millis(50), notified)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn stop_generation_change_cancels_a_slow_connect_immediately() {
        let generation = 41;
        let active = Arc::new(AtomicU64::new(generation));
        let sequence = Arc::new(AtomicU64::new(generation));
        let notify = Arc::new(Notify::new());
        let mut operation = tokio::spawn(run_generation_bound_operation(
            Arc::clone(&active),
            Arc::clone(&sequence),
            Arc::clone(&notify),
            generation,
            std::future::pending::<()>(),
        ));

        tokio::task::yield_now().await;
        sequence.store(generation + 1, Ordering::SeqCst);
        active.store(NO_GENERATION, Ordering::SeqCst);
        notify.notify_waiters();

        assert_eq!(
            tokio::time::timeout(Duration::from_millis(100), &mut operation)
                .await
                .unwrap()
                .unwrap(),
            Err(SESSION_START_CANCELLED.into())
        );
    }

    #[tokio::test]
    async fn stopped_slow_resume_cannot_restore_a_paused_listening_state() {
        let generation = 61;
        let active = Arc::new(AtomicU64::new(generation));
        let sequence = Arc::new(AtomicU64::new(generation));
        let notify = Arc::new(Notify::new());
        let operation = tokio::spawn(run_generation_bound_operation(
            Arc::clone(&active),
            Arc::clone(&sequence),
            Arc::clone(&notify),
            generation,
            std::future::pending::<()>(),
        ));
        let mut controller = TranslationSessionController::default();
        controller.begin_connecting();

        tokio::task::yield_now().await;
        sequence.store(generation + 1, Ordering::SeqCst);
        active.store(NO_GENERATION, Ordering::SeqCst);
        controller.did_stop();
        notify.notify_waiters();

        let error = operation.await.unwrap().unwrap_err();
        assert!(!resume_failure_is_still_owned(
            &error,
            generation.wrapping_add(1),
            sequence.load(Ordering::SeqCst),
            &controller.state.status,
            active.load(Ordering::SeqCst),
        ));
        assert_eq!(controller.state.status, SessionStatus::Idle);
    }

    #[test]
    fn manual_start_during_recovery_backoff_wins_the_epoch() {
        let recovery_failure_epoch = 72;
        let sequence = AtomicU64::new(recovery_failure_epoch);

        assert!(start_request_can_proceed(true, true, NO_GENERATION));
        assert!(!start_request_can_proceed(true, true, 71));
        assert!(!start_request_can_proceed(true, false, NO_GENERATION));

        let manual_start_generation = sequence.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(manual_start_generation, 73);
        assert_eq!(
            advance_lifecycle_sequence_if_current(&sequence, recovery_failure_epoch),
            None
        );
        assert_eq!(sequence.load(Ordering::SeqCst), manual_start_generation);
    }

    #[test]
    fn transport_error_during_recovery_is_retried_only_by_the_existing_owner() {
        let generation = 81;
        assert!(cancelled_recovery_attempt_is_retryable(
            generation,
            generation,
            generation + 1,
            true,
        ));
        assert!(!cancelled_recovery_attempt_is_retryable(
            NO_GENERATION,
            generation,
            generation + 1,
            true,
        ));
        assert!(!cancelled_recovery_attempt_is_retryable(
            generation,
            generation,
            generation + 2,
            true,
        ));
    }

    #[test]
    fn terminal_cancellation_clears_global_recovery_state() {
        let recovering = AtomicBool::new(true);
        let retry_generation = AtomicU64::new(82);

        clear_recovery_atoms(&recovering, &retry_generation);

        assert!(!recovering.load(Ordering::SeqCst));
        assert_eq!(retry_generation.load(Ordering::SeqCst), NO_GENERATION);
    }

    #[tokio::test]
    async fn manual_start_before_recovery_exhaustion_prevents_stale_error_publish() {
        let lifecycle = Arc::new(TokioMutex::new(()));
        let sequence = Arc::new(AtomicU64::new(91));
        let active = Arc::new(AtomicU64::new(NO_GENERATION));
        let controller = Arc::new(Mutex::new(TranslationSessionController::default()));
        controller.lock().unwrap().did_fail("old recovery failure");

        let manual_start = Arc::clone(&lifecycle).lock_owned().await;
        sequence.store(92, Ordering::SeqCst);
        active.store(92, Ordering::SeqCst);
        controller.lock().unwrap().begin_connecting();

        let recovery_publish = {
            let lifecycle = Arc::clone(&lifecycle);
            let sequence = Arc::clone(&sequence);
            let active = Arc::clone(&active);
            let controller = Arc::clone(&controller);
            tokio::spawn(async move {
                let _guard = lifecycle.lock_owned().await;
                if recovery_exhaustion_is_still_owned(
                    91,
                    sequence.load(Ordering::SeqCst),
                    active.load(Ordering::SeqCst),
                ) {
                    controller
                        .lock()
                        .unwrap()
                        .did_fail("stale recovery failure");
                    true
                } else {
                    false
                }
            })
        };
        drop(manual_start);

        assert!(!recovery_publish.await.unwrap());
        assert_eq!(
            controller.lock().unwrap().state.status,
            SessionStatus::Connecting
        );
    }

    #[test]
    fn stop_after_settings_switch_prevents_late_reconnect_claim() {
        let sequence = AtomicU64::new(101);
        let switch_epoch = sequence.load(Ordering::SeqCst);

        let stop_epoch = sequence.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(stop_epoch, 102);
        assert_eq!(
            advance_lifecycle_sequence_if_current(&sequence, switch_epoch),
            None
        );
        assert_eq!(sequence.load(Ordering::SeqCst), stop_epoch);
    }

    #[tokio::test]
    async fn switch_waiting_on_the_gate_cannot_mutate_after_a_newer_session_connects() {
        let lifecycle = Arc::new(TokioMutex::new(()));
        let sequence = Arc::new(AtomicU64::new(111));
        let preference_changed = Arc::new(AtomicBool::new(false));
        let controller = Arc::new(Mutex::new(TranslationSessionController::default()));
        let switch_epoch = sequence.load(Ordering::SeqCst);
        let newer_lifecycle = Arc::clone(&lifecycle).lock_owned().await;

        let stale_switch = {
            let lifecycle = Arc::clone(&lifecycle);
            let sequence = Arc::clone(&sequence);
            let preference_changed = Arc::clone(&preference_changed);
            tokio::spawn(async move {
                let _guard = lifecycle.lock_owned().await;
                if lifecycle_sequence_matches(&sequence, switch_epoch) {
                    preference_changed.store(true, Ordering::SeqCst);
                }
            })
        };
        tokio::task::yield_now().await;

        sequence.store(switch_epoch + 1, Ordering::SeqCst);
        controller.lock().unwrap().begin_connecting();
        controller.lock().unwrap().did_connect();
        drop(newer_lifecycle);
        stale_switch.await.unwrap();

        assert!(!preference_changed.load(Ordering::SeqCst));
        assert_eq!(
            controller.lock().unwrap().state.status,
            SessionStatus::Listening
        );
    }

    #[tokio::test]
    async fn pause_claim_after_switch_validation_inherits_the_saved_configuration() {
        let lifecycle = Arc::new(TokioMutex::new(()));
        let sequence = Arc::new(AtomicU64::new(121));
        let operations = Arc::new(AtomicUsize::new(0));
        let owner = Arc::new(AtomicU64::new(121));
        let active_settings = Arc::new(Mutex::new(Some(
            LiveTranslationConfiguration::for_provider(
                crate::core::provider::ProviderKind::AlibabaCloud,
                "test-key",
                SourceLanguage::English,
                crate::core::models::TargetLanguage::SimplifiedChinese,
                TranslationMode::LowLatency,
            ),
        )));

        // The switch has acquired the lifecycle gate and passed both stale
        // intent checks. Pause then claims the next epoch but must wait for
        // the durable preference write to finish.
        let switch_guard = Arc::clone(&lifecycle).lock_owned().await;
        assert!(lifecycle_sequence_matches(&sequence, 121));
        assert!(pipeline_settings_mutation_is_allowed(
            &SessionStatus::Listening,
            operations.load(Ordering::SeqCst),
        ));
        let (pause_claimed_tx, pause_claimed_rx) = tokio::sync::oneshot::channel();
        let pause = {
            let lifecycle = Arc::clone(&lifecycle);
            let sequence = Arc::clone(&sequence);
            let operations = Arc::clone(&operations);
            let active_settings = Arc::clone(&active_settings);
            tokio::spawn(async move {
                operations.fetch_add(1, Ordering::SeqCst);
                sequence.fetch_add(1, Ordering::SeqCst);
                pause_claimed_tx.send(()).unwrap();
                let _pause_guard = lifecycle.lock_owned().await;
                operations.fetch_sub(1, Ordering::SeqCst);
                active_settings.lock().unwrap().clone().unwrap()
            })
        };
        pause_claimed_rx.await.unwrap();
        assert_eq!(sequence.load(Ordering::SeqCst), 122);

        assert!(update_owned_value(
            &active_settings,
            &owner,
            |configuration| {
                configuration.source_language = SourceLanguage::Japanese;
                configuration.target_language = crate::core::models::TargetLanguage::English;
                configuration.translation_mode = TranslationMode::Turbo;
            }
        ));
        drop(switch_guard);

        let resumed_configuration = pause.await.unwrap();
        assert_eq!(
            resumed_configuration.source_language,
            SourceLanguage::Japanese
        );
        assert_eq!(
            resumed_configuration.target_language,
            crate::core::models::TargetLanguage::English
        );
        assert_eq!(
            resumed_configuration.translation_mode,
            TranslationMode::Turbo
        );
    }

    #[test]
    fn recovery_failure_stays_connecting_and_blocks_pipeline_mutations_during_backoff() {
        let mut controller = TranslationSessionController::default();
        controller.begin_connecting();

        apply_establish_failure_state(&mut controller, "retryable failure".into(), true);

        assert_eq!(controller.state.status, SessionStatus::Connecting);
        assert!(SessionStateEvent::from(&controller.state).is_active);
        assert!(lifecycle_activity_is_active(
            controller.state.status.is_active(),
            1,
        ));
        assert!(!pipeline_settings_mutation_is_allowed(
            &controller.state.status,
            1,
        ));
    }

    #[tokio::test]
    async fn queued_profile_mutation_is_rejected_during_resume_failure_handoff() {
        let lifecycle = Arc::new(TokioMutex::new(()));
        let lifecycle_operations = Arc::new(AtomicUsize::new(1));
        let status = Arc::new(Mutex::new(SessionStatus::Error("resume failure".into())));
        let resume_guard = Arc::clone(&lifecycle).lock_owned().await;

        let mutation = {
            let lifecycle = Arc::clone(&lifecycle);
            let lifecycle_operations = Arc::clone(&lifecycle_operations);
            let status = Arc::clone(&status);
            tokio::spawn(async move {
                let _guard = lifecycle.lock_owned().await;
                !lifecycle_activity_is_active(
                    status.lock().unwrap().is_active(),
                    lifecycle_operations.load(Ordering::SeqCst),
                )
            })
        };
        tokio::task::yield_now().await;
        drop(resume_guard);

        assert!(
            !mutation.await.unwrap(),
            "profile mutation must be rejected"
        );
        lifecycle_operations.store(0, Ordering::SeqCst);
        assert!(!lifecycle_activity_is_active(false, 0));
    }

    #[tokio::test]
    async fn pause_rechecks_state_after_a_concurrent_stop_owns_the_gate() {
        let lock = Arc::new(TokioMutex::new(()));
        let status = Arc::new(Mutex::new(SessionStatus::Listening));
        let paused = Arc::new(AtomicBool::new(false));
        let active = Arc::new(AtomicU64::new(7));
        let stop_guard = Arc::clone(&lock).lock_owned().await;

        let pause_waiter = {
            let lock = Arc::clone(&lock);
            let status = Arc::clone(&status);
            let paused = Arc::clone(&paused);
            let active = Arc::clone(&active);
            tokio::spawn(async move {
                let _pause_guard = lock.lock_owned().await;
                pause_transition_is_valid(
                    &status.lock().unwrap(),
                    paused.load(Ordering::SeqCst),
                    active.load(Ordering::SeqCst),
                )
            })
        };

        *status.lock().unwrap() = SessionStatus::Idle;
        active.store(NO_GENERATION, Ordering::SeqCst);
        drop(stop_guard);

        assert!(!pause_waiter.await.unwrap());
        assert!(!resume_transition_is_valid(
            &SessionStatus::Idle,
            true,
            NO_GENERATION,
            true,
        ));
    }

    #[tokio::test]
    async fn delayed_health_or_recovery_cleanup_cannot_erase_replacement_task() {
        for stale_id in [11, 21] {
            let current_id = AtomicU64::new(stale_id + 1);
            let slot = Mutex::new(Some(tokio::spawn(async {
                std::future::pending::<()>().await;
            })));

            assert!(!clear_task_slot_if_id(&slot, &current_id, stale_id));
            assert_eq!(current_id.load(Ordering::SeqCst), stale_id + 1);
            let replacement = slot.lock().unwrap().take().unwrap();
            replacement.abort();
        }
    }

    #[tokio::test]
    async fn concurrent_pause_or_reconnect_has_exactly_one_cleanup_owner() {
        async fn claim_after_gate(
            lock: Arc<TokioMutex<()>>,
            sequence: Arc<AtomicU64>,
            active: Arc<AtomicU64>,
            ready: Arc<tokio::sync::Barrier>,
        ) -> u64 {
            let request = sequence.fetch_add(1, Ordering::SeqCst) + 1;
            ready.wait().await;
            let _guard = lock.lock_owned().await;
            if sequence.load(Ordering::SeqCst) != request {
                return NO_GENERATION;
            }
            active.swap(NO_GENERATION, Ordering::SeqCst)
        }

        for _operation in ["pause", "reconnect"] {
            let lock = Arc::new(TokioMutex::new(()));
            let sequence = Arc::new(AtomicU64::new(100));
            let active = Arc::new(AtomicU64::new(77));
            let ready = Arc::new(tokio::sync::Barrier::new(2));
            let first = tokio::spawn(claim_after_gate(
                Arc::clone(&lock),
                Arc::clone(&sequence),
                Arc::clone(&active),
                Arc::clone(&ready),
            ));
            let second = tokio::spawn(claim_after_gate(
                Arc::clone(&lock),
                Arc::clone(&sequence),
                Arc::clone(&active),
                Arc::clone(&ready),
            ));

            let claims = [first.await.unwrap(), second.await.unwrap()];
            assert_eq!(claims.iter().filter(|claim| **claim == 77).count(), 1);
            assert_eq!(active.load(Ordering::SeqCst), NO_GENERATION);
        }
    }

    #[test]
    fn stale_terminal_cleanup_cannot_clear_new_generation_settings() {
        let settings = Mutex::new(Some("new-generation".to_string()));
        let owner = AtomicU64::new(8);

        assert!(!clear_owned_value_if_generation(&settings, &owner, 7));
        assert_eq!(settings.lock().unwrap().as_deref(), Some("new-generation"));
        assert_eq!(owner.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn automatic_source_preserves_supported_turbo_modes() {
        assert_eq!(
            translation_mode_after_source_switch(
                ProviderKind::OpenAIRealtime,
                SourceLanguage::Automatic,
                TranslationMode::HighQuality,
            ),
            TranslationMode::Turbo
        );
        assert_eq!(
            translation_mode_after_source_switch(
                ProviderKind::AlibabaCloud,
                SourceLanguage::Automatic,
                TranslationMode::HighQuality,
            ),
            TranslationMode::LowLatency
        );
        assert_eq!(
            translation_mode_after_source_switch(
                ProviderKind::AlibabaCloud,
                SourceLanguage::Automatic,
                TranslationMode::Turbo,
            ),
            TranslationMode::Turbo
        );

        assert!(source_switch_requires_reconnect(
            true,
            SourceLanguage::Automatic,
            crate::core::models::TargetLanguage::SimplifiedChinese,
            TranslationMode::Turbo,
            SourceLanguage::Automatic,
            crate::core::models::TargetLanguage::SimplifiedChinese,
            TranslationMode::LowLatency,
        ));
    }
}
