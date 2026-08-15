//! Session lifecycle manager, ported from `Sources/MimiApp/AppModel.swift`:
//! start/stop/pause/resume, language and mode switching, health checks,
//! automatic reconnection, and session-state event broadcasting.
//!
//! The manager is always shared behind `Arc<SessionManager>`; spawned tasks
//! hold clones of the same Arc so they observe one piece of session state.

use crate::audio::send_pipeline::AudioSendPipeline;
use crate::audio::SystemAudioCapture;
use crate::clients::translation_client::TranslationClient;
use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::models::{SessionStatus, SourceLanguage, TranslationMode};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::session::{TranslationSessionController, TranslationSessionState};
use crate::pipeline_log;
use crate::settings_store::SettingsStore;
use crate::windows::OverlayWindowManager;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
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
    audio_pipeline: Arc<Mutex<Option<Arc<AudioSendPipeline>>>>,
    active_settings: Arc<Mutex<Option<LiveTranslationConfiguration>>>,
    is_paused: Arc<AtomicBool>,
    is_recovering: Arc<AtomicBool>,
    is_overlay_collapsed: Arc<AtomicBool>,
    /// Set while a coalesced session-state broadcast is scheduled; chunks
    /// arriving in between just flip it instead of emitting per chunk.
    state_publish_pending: Arc<AtomicBool>,
    health_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    recovery_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    pump_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Fires when a translation stays pending too long (e.g. the server never
    /// returns the final for an incomplete sentence after the audio stops).
    /// Clears `is_translation_pending` so the UI does not sit on
    /// "正在翻译" forever; the shown draft/history is untouched.
    translation_timeout_task: Arc<Mutex<Option<JoinHandle<()>>>>,
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
            audio_pipeline: Arc::new(Mutex::new(None)),
            active_settings: Arc::new(Mutex::new(None)),
            is_paused: Arc::new(AtomicBool::new(false)),
            is_recovering: Arc::new(AtomicBool::new(false)),
            is_overlay_collapsed: Arc::new(AtomicBool::new(false)),
            state_publish_pending: Arc::new(AtomicBool::new(false)),
            health_task: Arc::new(Mutex::new(None)),
            recovery_task: Arc::new(Mutex::new(None)),
            pump_task: Arc::new(Mutex::new(None)),
            translation_timeout_task: Arc::new(Mutex::new(None)),
        })
    }

    pub fn is_active(&self) -> bool {
        self.controller.lock().unwrap().state.status.is_active()
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    pub fn is_overlay_collapsed(&self) -> bool {
        self.is_overlay_collapsed.load(Ordering::SeqCst)
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app
    }

    /// Starts (or restarts) a listening session with the saved settings.
    pub async fn start(self: &Arc<Self>, clear_subtitles: bool) -> Result<(), String> {
        if self.is_active() {
            return Ok(());
        }
        self.is_paused.store(false, Ordering::SeqCst);
        let configuration = match self.settings.configuration() {
            Ok(configuration) => configuration,
            Err(error) => {
                pipeline_log!("session settings failed error={}", error);
                self.controller.lock().unwrap().did_fail(error.clone());
                self.publish_state();
                return Err(error);
            }
        };
        pipeline_log!(
            "session start requested source={} target={} mode={:?}",
            configuration.source_language.raw_value(),
            configuration.target_language.raw_value(),
            configuration.effective_translation_mode()
        );

        self.settings.persist();
        *self.active_settings.lock().unwrap() = Some(configuration);
        self.establish_session(clear_subtitles).await
    }

    async fn establish_session(self: &Arc<Self>, clear_subtitles: bool) -> Result<(), String> {
        self.stop_health_checks().await;

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
                    *self.active_settings.lock().unwrap() = Some(configuration.clone());
                    configuration
                }
            };

            pipeline_log!("session connecting clear={}", u8::from(clear));
            if clear {
                self.controller.lock().unwrap().clear_subtitles();
            }
            self.controller.lock().unwrap().begin_connecting();
            self.publish_state();

            let result = Arc::clone(self)
                .connect_and_listen(configuration.clone())
                .await;
            if let Err(error) = result {
                pipeline_log!("session establish failed error={}", error);
                self.stop_pipeline().await;
                let capture = self.audio.lock().unwrap().clone();
                capture.stop().await;
                let taken = self.client.lock().unwrap().take();
                if let Some(client) = taken {
                    client.disconnect().await;
                }
                if !self.is_recovering.load(Ordering::SeqCst) {
                    *self.active_settings.lock().unwrap() = None;
                }
                self.controller.lock().unwrap().did_fail(error.clone());
                self.publish_state();
                return Err(error);
            }

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
                *self.active_settings.lock().unwrap() = Some(fresh);
            }
            pipeline_log!("session rebuild for settings changed mid-connect");
            self.stop_pipeline().await;
            let capture = self.audio.lock().unwrap().clone();
            capture.stop().await;
            let taken = self.client.lock().unwrap().take();
            if let Some(client) = taken {
                client.disconnect().await;
            }
            clear = false;
        }
    }

    fn connect_and_listen(
        self: Arc<Self>,
        configuration: LiveTranslationConfiguration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            // Create the client and consume its events through this manager.
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();
            let new_client = TranslationClient::new(&configuration, event_tx)
                .map_err(|error| error.to_string())?;
            *self.client.lock().unwrap() = Some(new_client);

            let client = self.client.lock().unwrap().clone().unwrap();
            client.connect().await.map_err(|error| error.to_string())?;
            pipeline_log!("asr websocket connected");

            // Spawn the event pump that drives the controller and reconnection.
            let self_arc = Arc::clone(&self);
            let pump = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    self_arc.handle_event(event).await;
                }
            });
            *self.pump_task.lock().unwrap() = Some(pump);

            // Start system-audio capture feeding the bounded send pipeline.
            let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            let audio_error_tx = self.capture_error_channel();
            let capture = self.audio.lock().unwrap().clone();
            capture
                .start(audio_tx, audio_error_tx)
                .await
                .map_err(|error| error.to_string())?;
            pipeline_log!("audio capture started");

            // The send pipeline serializes PCM buffers onto the socket.
            let client_shared = Arc::clone(&self.client);
            let on_error_self = Arc::clone(&self);
            let pipeline = Arc::new(AudioSendPipeline::spawn(
                move |data| {
                    let client_shared = Arc::clone(&client_shared);
                    Box::pin(async move {
                        let client = client_shared.lock().unwrap().clone();
                        match client {
                            Some(client) => {
                                client.send_audio(&data).await.map_err(|e| e.to_string())
                            }
                            None => Err("The live translation session is not connected.".into()),
                        }
                    })
                },
                move |message| {
                    let manager = Arc::clone(&on_error_self);
                    tokio::spawn(async move {
                        manager.handle_audio_transport_failure(message).await;
                    });
                },
            ));
            *self.audio_pipeline.lock().unwrap() = Some(Arc::clone(&pipeline));

            tokio::spawn(async move {
                while let Some(data) = audio_rx.recv().await {
                    pipeline.enqueue(data);
                }
            });

            self.controller.lock().unwrap().did_connect();
            self.publish_state();
            Arc::clone(&self).start_health_checks().await;
            pipeline_log!("session listening");
            Ok(())
        })
    }

    pub async fn stop(self: &Arc<Self>) {
        if !self.is_active() && self.client.lock().unwrap().is_none() {
            return;
        }
        pipeline_log!("session stop requested");

        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.cancel_translation_timeout();
        *self.active_settings.lock().unwrap() = None;
        self.is_recovering.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.controller.lock().unwrap().begin_stopping();
        self.publish_state();

        self.stop_pipeline().await;
        let capture = self.audio.lock().unwrap().clone();
        capture.stop().await;
        let taken = self.client.lock().unwrap().take();
        if let Some(client) = taken {
            client.finish().await;
        }
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
        let is_listening = self.controller.lock().unwrap().state.status == SessionStatus::Listening;
        if self.is_paused() || !is_listening {
            return;
        }
        pipeline_log!("session pause requested");
        self.is_paused.store(true, Ordering::SeqCst);
        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.cancel_translation_timeout();
        self.is_recovering.store(false, Ordering::SeqCst);
        self.controller.lock().unwrap().did_pause();
        self.publish_state();
        self.stop_pipeline().await;
        let capture = self.audio.lock().unwrap().clone();
        capture.stop().await;
        let taken = self.client.lock().unwrap().take();
        if let Some(client) = taken {
            client.disconnect().await;
        }
        self.controller.lock().unwrap().did_pause();
        self.publish_state();
        pipeline_log!("session paused");
    }

    pub async fn resume(self: &Arc<Self>) {
        if !self.is_paused() {
            return;
        }
        pipeline_log!("session resume requested");
        self.is_paused.store(false, Ordering::SeqCst);
        let resumed = self.establish_session(false).await;
        if resumed.is_ok() {
            pipeline_log!("session resumed");
            return;
        }
        self.is_paused.store(true, Ordering::SeqCst);
        self.controller.lock().unwrap().did_pause();
        self.publish_state();
        pipeline_log!("session resume failed; remaining paused");
    }

    /// Quick-switches the source language, reconnecting when needed.
    /// Selecting automatic detection also switches to the low-latency
    /// pipeline (the live-translate stream detects the language per
    /// utterance), mirroring the original app's auto→lowLatency behavior.
    pub async fn switch_source_language(self: &Arc<Self>, language: SourceLanguage) {
        let (target_language, needs_reconnect) = {
            let prefs = self.settings.preferences();
            let target = language
                .target_language_after_quick_switch(prefs.source_language, prefs.target_language);
            let needs_reconnect = self.controller.lock().unwrap().state.status
                == SessionStatus::Listening
                && (prefs.source_language != language || prefs.target_language != target);
            (target, needs_reconnect)
        };
        self.settings.update_preferences(|prefs| {
            prefs.source_language = language;
            prefs.target_language = target_language;
            if language == SourceLanguage::Automatic {
                prefs.translation_mode = TranslationMode::LowLatency;
            }
        });
        self.settings.persist();
        // Broadcast immediately: the reconnect below can take seconds, and
        // every window (including the language popover) must see the new
        // selection right away.
        self.publish_settings();
        if self.is_paused() {
            *self.active_settings.lock().unwrap() = self.settings.configuration().ok();
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
        self.reconnect().await;
    }

    /// Quick-switches the translation mode, reconnecting when needed.
    pub async fn switch_translation_mode(self: &Arc<Self>, mode: TranslationMode) {
        let current = self.settings.preferences().translation_mode;
        if current == mode {
            return;
        }
        self.settings
            .update_preferences(|prefs| prefs.translation_mode = mode);
        self.settings.persist();
        // Broadcast immediately: the reconnect below can take seconds.
        self.publish_settings();
        if self.is_paused() {
            *self.active_settings.lock().unwrap() = self.settings.configuration().ok();
            return;
        }
        let is_listening = self.controller.lock().unwrap().state.status == SessionStatus::Listening;
        if !is_listening {
            return;
        }

        pipeline_log!("session translation mode switch mode={:?}", mode);
        self.reconnect().await;
    }

    /// Tears down and re-establishes the session, keeping subtitles.
    async fn reconnect(self: &Arc<Self>) {
        self.stop_health_checks().await;
        self.cancel_recovery().await;
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        self.stop_pipeline().await;
        let capture = self.audio.lock().unwrap().clone();
        capture.stop().await;
        let taken = self.client.lock().unwrap().take();
        if let Some(client) = taken {
            client.disconnect().await;
        }
        let _ = self.establish_session(false).await;
    }

    /// Applies the listening-time language adjustments before a session
    /// starts (automatic → Japanese; Chinese → original subtitles).
    pub fn prepare_for_listening(&self) {
        self.settings.prepare_for_listening();
        *self.active_settings.lock().unwrap() = self.settings.configuration().ok();
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

    async fn handle_event(self: &Arc<Self>, event: LiveTranslateServerEvent) {
        if self.is_paused() {
            return;
        }

        if let LiveTranslateServerEvent::Error { code, message } = &event {
            if code == "transport_error" {
                pipeline_log!("session transport error code={}", code);
                self.cancel_translation_timeout();
                self.controller.lock().unwrap().begin_connecting();
                self.publish_state();
                self.queue_recovery(message.clone()).await;
                return;
            }
        }

        // A final translation resolves any pending timeout; a fresh
        // TranslationStarted arms a new one.
        if matches!(
            event,
            LiveTranslateServerEvent::TranslationFinal(_) | LiveTranslateServerEvent::Error { .. }
        ) {
            self.cancel_translation_timeout();
        }
        if matches!(event, LiveTranslateServerEvent::TranslationStarted) {
            self.arm_translation_timeout();
        }

        self.controller.lock().unwrap().handle(event.clone());
        self.publish_state();

        if matches!(event, LiveTranslateServerEvent::Error { .. }) {
            // Content-free: log only the machine-readable code, never the
            // server's free-text message.
            let code = match &event {
                LiveTranslateServerEvent::Error { code, .. } => code.as_str(),
                _ => "",
            };
            pipeline_log!("session terminal error code={code}");
            self.stop_health_checks().await;
            self.cancel_recovery().await;
            self.stop_pipeline().await;
            let capture = self.audio.lock().unwrap().clone();
            capture.stop().await;
            let taken = self.client.lock().unwrap().take();
            if let Some(client) = taken {
                client.disconnect().await;
            }
            *self.active_settings.lock().unwrap() = None;
        }
    }

    /// Arms a timer that clears a stuck "正在翻译" state. The low-latency
    /// stream protocol synthesizes TranslationStarted at the source final,
    /// then waits for the server's `response.text.done`; if the audio stops
    /// mid-sentence the server may never finalize, so without this the UI
    /// would stay pending forever. The shown subtitle is left untouched.
    fn arm_translation_timeout(self: &Arc<Self>) {
        self.cancel_translation_timeout();
        let this = Arc::clone(self);
        let task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            pipeline_log!("translation pending timed out; clearing");
            this.controller.lock().unwrap().clear_translation_pending();
            this.publish_state();
        });
        *self.translation_timeout_task.lock().unwrap() = Some(task);
    }

    fn cancel_translation_timeout(self: &Arc<Self>) {
        if let Some(task) = self.translation_timeout_task.lock().unwrap().take() {
            task.abort();
        }
    }

    async fn handle_capture_failure(self: &Arc<Self>, error: String) {
        if self.is_paused() {
            return;
        }
        pipeline_log!("audio capture failed error={}", error);
        self.stop_health_checks().await;
        self.stop_pipeline().await;
        let capture = self.audio.lock().unwrap().clone();
        capture.stop().await;
        let taken = self.client.lock().unwrap().take();
        if let Some(client) = taken {
            client.disconnect().await;
        }
        *self.active_settings.lock().unwrap() = None;
        self.controller.lock().unwrap().did_fail(error);
        self.publish_state();
    }

    async fn handle_audio_transport_failure(self: &Arc<Self>, message: String) {
        if self.is_paused()
            || self.is_recovering.load(Ordering::SeqCst)
            || self.active_settings.lock().unwrap().is_none()
        {
            return;
        }
        pipeline_log!("audio transport failed error={}", message);
        self.queue_recovery(message).await;
    }

    // MARK: health checks and recovery

    async fn start_health_checks(self: &Arc<Self>) {
        self.stop_health_checks().await;
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if !self_arc.check_connection_health().await {
                    return;
                }
            }
        });
        *self.health_task.lock().unwrap() = Some(task);
    }

    async fn stop_health_checks(&self) {
        if let Some(task) = self.health_task.lock().unwrap().take() {
            task.abort();
        }
    }

    async fn check_connection_health(self: &Arc<Self>) -> bool {
        if self.is_paused() || self.is_recovering.load(Ordering::SeqCst) {
            return false;
        }
        let Some(client) = self.client.lock().unwrap().clone() else {
            return false;
        };
        match client.ping(Duration::from_secs(4)).await {
            Ok(()) => true,
            Err(error) => {
                pipeline_log!("connection health failed error={}", error);
                *self.health_task.lock().unwrap() = None;
                self.queue_recovery(error.to_string()).await;
                false
            }
        }
    }

    async fn queue_recovery(self: &Arc<Self>, failure_message: String) {
        if self.is_paused()
            || self.recovery_task.lock().unwrap().is_some()
            || self.active_settings.lock().unwrap().is_none()
        {
            return;
        }
        let self_arc = self.clone();
        let task = tokio::spawn(async move {
            self_arc.recover_connection(failure_message).await;
            *self_arc.recovery_task.lock().unwrap() = None;
        });
        *self.recovery_task.lock().unwrap() = Some(task);
    }

    async fn recover_connection(self: &Arc<Self>, failure_message: String) {
        if self.is_paused() || self.is_recovering.load(Ordering::SeqCst) {
            return;
        }
        pipeline_log!("session recovery started");
        self.is_recovering.store(true, Ordering::SeqCst);
        self.stop_health_checks().await;
        self.controller.lock().unwrap().begin_connecting();
        self.publish_state();
        self.stop_pipeline().await;
        let capture = self.audio.lock().unwrap().clone();
        capture.stop().await;
        let taken = self.client.lock().unwrap().take();
        if let Some(client) = taken {
            client.disconnect().await;
        }

        let mut recovered = false;
        for (attempt, delay) in [0u64, 1].iter().enumerate() {
            pipeline_log!("session recovery attempt={}", attempt + 1);
            tokio::time::sleep(Duration::from_secs(*delay)).await;
            if self.active_settings.lock().unwrap().is_none() {
                self.is_recovering.store(false, Ordering::SeqCst);
                return;
            }
            recovered = self.establish_session(false).await.is_ok();
            if recovered {
                break;
            }
        }

        if !recovered {
            pipeline_log!("session recovery exhausted");
            *self.active_settings.lock().unwrap() = None;
            self.controller.lock().unwrap().did_fail(failure_message);
            self.publish_state();
        }
        self.is_recovering.store(false, Ordering::SeqCst);
    }

    async fn cancel_recovery(&self) {
        if let Some(task) = self.recovery_task.lock().unwrap().take() {
            task.abort();
        }
    }

    async fn stop_pipeline(&self) {
        if let Some(pipeline) = self.audio_pipeline.lock().unwrap().take() {
            pipeline.stop();
        }
    }

    /// Channel through which the audio capture reports fatal errors.
    fn capture_error_channel(self: &Arc<Self>) -> mpsc::UnboundedSender<String> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let self_arc = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(error) = rx.recv().await {
                self_arc.clone().handle_capture_failure(error).await;
            }
        });
        tx
    }

    // MARK: state publishing

    /// Builds the current session state snapshot without emitting it (used by
    /// windows that boot after the last broadcast, e.g. the language popover).
    pub fn current_state_event(&self) -> SessionStateEvent {
        let state = self.controller.lock().unwrap().state.clone();
        let mut event = SessionStateEvent::from(&state);
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
    /// pending flag. Status-only changes therefore reach the UI within 60ms
    /// and bursty chunks are folded into one emit.
    pub fn publish_state(self: &Arc<Self>) {
        if self.state_publish_pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            this.publish_state_now();
            this.state_publish_pending.store(false, Ordering::SeqCst);
        });
    }

    fn publish_state_now(self: &Arc<Self>) {
        let event = self.current_state_event();
        let is_active = event.is_active;
        let _ = self.app.emit("session-state", event);
        OverlayWindowManager::sync_visibility(&self.app, is_active);
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
}
