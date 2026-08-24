//! Provider-neutral session state controller.

use crate::core::models::{DetectedLanguage, SessionStatus, SubtitleSnapshot};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::subtitle_reducer::SubtitleReducer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationSessionState {
    pub status: SessionStatus,
    pub subtitles: SubtitleSnapshot,
    pub detected_language: Option<DetectedLanguage>,
    pub is_translation_pending: bool,
}

impl Default for TranslationSessionState {
    fn default() -> Self {
        Self {
            status: SessionStatus::Idle,
            subtitles: SubtitleSnapshot::empty(),
            detected_language: None,
            is_translation_pending: false,
        }
    }
}

#[derive(Default)]
pub struct TranslationSessionController {
    pub state: TranslationSessionState,
    subtitle_reducer: SubtitleReducer,
}

impl TranslationSessionController {
    pub fn begin_connecting(&mut self) {
        self.subtitle_reducer.reset_transient();
        self.state.subtitles = self.subtitle_reducer.snapshot.clone();
        self.state.status = SessionStatus::Connecting;
        self.state.detected_language = None;
        self.state.is_translation_pending = false;
    }

    pub fn did_connect(&mut self) {
        self.state.status = SessionStatus::Listening;
    }

    pub fn did_pause(&mut self) {
        self.state.status = SessionStatus::Listening;
        self.state.is_translation_pending = false;
    }

    /// Clears only the waiting-for-final flag (the "正在翻译" indicator)
    /// without touching status or subtitles. Used by the translation-timeout
    /// guard when the server never returns a final (e.g. audio stopped
    /// mid-sentence); the draft/history already shown stays on screen.
    pub fn clear_translation_pending(&mut self) {
        self.state.is_translation_pending = false;
    }

    pub fn begin_stopping(&mut self) {
        self.state.status = SessionStatus::Stopping;
        self.state.is_translation_pending = false;
    }

    pub fn did_stop(&mut self) {
        self.state.status = SessionStatus::Idle;
        self.state.is_translation_pending = false;
    }

    pub fn did_fail(&mut self, message: impl Into<String>) {
        self.state.status = SessionStatus::Error(message.into());
        self.state.is_translation_pending = false;
    }

    pub fn clear_subtitles(&mut self) {
        self.subtitle_reducer
            .apply(crate::core::models::SubtitleEvent::Clear);
        self.state.subtitles = self.subtitle_reducer.snapshot.clone();
    }

    pub fn handle(&mut self, event: LiveTranslateServerEvent) {
        // Alibaba teardown can emit synthetic source/translation cleanup
        // finals, which are intentionally ignored. OpenAI has no separate
        // final events: a real `session.closed` may flush one client-aligned
        // atomic pair, and that verified tail is safe to keep.
        if self.state.status == SessionStatus::Stopping
            && !matches!(event, LiveTranslateServerEvent::SubtitleFinalPair { .. })
        {
            return;
        }

        match event {
            LiveTranslateServerEvent::SessionCreated => {}
            LiveTranslateServerEvent::SessionUpdated => self.did_connect(),
            LiveTranslateServerEvent::SourceDraft { text, language } => {
                self.update_detected_language(language.as_deref());
                self.subtitle_reducer
                    .apply(crate::core::models::SubtitleEvent::SourceDraft(text));
            }
            LiveTranslateServerEvent::SourceFinal { text, language } => {
                self.update_detected_language(language.as_deref());
                self.subtitle_reducer
                    .apply(crate::core::models::SubtitleEvent::SourceFinal(text));
            }
            LiveTranslateServerEvent::TranslationStarted => {
                self.state.is_translation_pending = true;
            }
            LiveTranslateServerEvent::TranslationDraft(text) => {
                self.subtitle_reducer
                    .apply(crate::core::models::SubtitleEvent::TranslationDraft(text));
            }
            LiveTranslateServerEvent::TranslationFinal(text) => {
                self.state.is_translation_pending = false;
                self.subtitle_reducer
                    .apply(crate::core::models::SubtitleEvent::TranslationFinal(text));
            }
            LiveTranslateServerEvent::SubtitleFinalPair {
                source,
                language,
                translation,
            } => {
                self.update_detected_language(language.as_deref());
                self.state.is_translation_pending = false;
                self.subtitle_reducer
                    .apply(crate::core::models::SubtitleEvent::FinalPair {
                        source,
                        translation,
                    });
            }
            LiveTranslateServerEvent::SessionFinished => self.did_stop(),
            LiveTranslateServerEvent::Error { message, .. } => self.did_fail(message),
            LiveTranslateServerEvent::Ignored { .. } => return,
        }

        self.state.subtitles = self.subtitle_reducer.snapshot.clone();
    }

    fn update_detected_language(&mut self, reported_language: Option<&str>) {
        if let Some(language) = DetectedLanguage::from_reported(reported_language) {
            self.state.detected_language = Some(language);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_follows_the_happy_path_lifecycle() {
        let mut controller = TranslationSessionController::default();

        controller.begin_connecting();
        assert_eq!(controller.state.status, SessionStatus::Connecting);

        controller.did_connect();
        assert_eq!(controller.state.status, SessionStatus::Listening);

        controller.begin_stopping();
        assert_eq!(controller.state.status, SessionStatus::Stopping);

        controller.did_stop();
        assert_eq!(controller.state.status, SessionStatus::Idle);
    }

    #[test]
    fn server_events_update_subtitle_state() {
        let mut controller = TranslationSessionController::default();
        controller.handle(LiveTranslateServerEvent::SourceDraft {
            text: "Hello wor".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationDraft(
            "你好，世".into(),
        ));

        assert_eq!(controller.state.subtitles.source.text, "Hello wor");
        assert!(!controller.state.subtitles.source.is_final);
        assert_eq!(controller.state.subtitles.translation.text, "你好，世");

        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Hello world.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationFinal(
            "你好，世界。".into(),
        ));

        assert_eq!(controller.state.subtitles.history.len(), 1);
        assert!(controller.state.subtitles.translation.is_final);
        assert_eq!(
            controller.state.detected_language.as_ref().unwrap().code,
            "en"
        );
    }

    #[test]
    fn a_new_connection_clears_the_previously_detected_language() {
        let mut controller = TranslationSessionController::default();
        controller.handle(LiveTranslateServerEvent::SourceDraft {
            text: "こんにちは".into(),
            language: Some("ja".into()),
        });
        assert_eq!(
            controller.state.detected_language.as_ref().unwrap().code,
            "ja"
        );

        controller.begin_connecting();
        assert_eq!(controller.state.detected_language, None);
    }

    #[test]
    fn service_errors_move_the_session_to_error() {
        let mut controller = TranslationSessionController::default();
        controller.begin_connecting();
        controller.handle(LiveTranslateServerEvent::Error {
            code: "invalid_value".into(),
            message: "Bad language".into(),
        });

        assert_eq!(
            controller.state.status,
            SessionStatus::Error("Bad language".into())
        );
    }

    #[test]
    fn translation_activity_follows_the_real_plus_request_lifecycle() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();

        controller.handle(LiveTranslateServerEvent::TranslationStarted);
        assert!(controller.state.is_translation_pending);

        controller.handle(LiveTranslateServerEvent::TranslationFinal(
            "翻译完成。".into(),
        ));
        assert!(!controller.state.is_translation_pending);

        controller.handle(LiveTranslateServerEvent::TranslationStarted);
        controller.did_fail("Request failed");
        assert!(!controller.state.is_translation_pending);
    }

    #[test]
    fn pausing_clears_translation_activity_without_discarding_subtitles() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Please wait.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationFinal(
            "请稍等。".into(),
        ));
        controller.handle(LiveTranslateServerEvent::TranslationStarted);
        let subtitles_before_pause = controller.state.subtitles.clone();

        controller.did_pause();

        assert_eq!(controller.state.status, SessionStatus::Listening);
        assert!(!controller.state.is_translation_pending);
        assert_eq!(controller.state.subtitles, subtitles_before_pause);
    }

    #[test]
    fn clearing_translation_pending_keeps_status_and_subtitles() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Hello.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationStarted);
        assert!(controller.state.is_translation_pending);
        let subtitles_before = controller.state.subtitles.clone();

        // The timeout guard clears only the waiting-for-final flag.
        controller.clear_translation_pending();

        assert_eq!(controller.state.status, SessionStatus::Listening);
        assert!(!controller.state.is_translation_pending);
        assert_eq!(controller.state.subtitles, subtitles_before);
    }

    #[test]
    fn clearing_subtitles_does_not_change_session_status() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Hello.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationFinal("你好。".into()));

        controller.clear_subtitles();

        assert_eq!(controller.state.status, SessionStatus::Listening);
        assert_eq!(controller.state.subtitles, SubtitleSnapshot::empty());
    }

    #[test]
    fn stopping_ignores_flushed_tail_subtitles() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Last real line.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationFinal(
            "最后一句正常字幕。".into(),
        ));
        let subtitles_before_stopping = controller.state.subtitles.clone();

        controller.begin_stopping();
        controller.handle(LiveTranslateServerEvent::SourceFinal {
            text: "Translation mode ended.".into(),
            language: Some("en".into()),
        });
        controller.handle(LiveTranslateServerEvent::TranslationFinal(
            "翻译模式已结束。".into(),
        ));
        controller.handle(LiveTranslateServerEvent::SessionFinished);

        assert_eq!(controller.state.status, SessionStatus::Stopping);
        assert_eq!(controller.state.subtitles, subtitles_before_stopping);
    }

    #[test]
    fn stopping_accepts_a_provider_confirmed_atomic_tail_pair() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.begin_stopping();

        controller.handle(LiveTranslateServerEvent::SubtitleFinalPair {
            source: "Confirmed tail".into(),
            language: Some("en".into()),
            translation: "已确认的尾句".into(),
        });

        assert_eq!(controller.state.status, SessionStatus::Stopping);
        assert_eq!(controller.state.subtitles.history.len(), 1);
        assert_eq!(
            controller.state.subtitles.history[0].source,
            "Confirmed tail"
        );
        assert_eq!(
            controller.state.subtitles.history[0].translation,
            "已确认的尾句"
        );
    }

    #[test]
    fn unknown_server_events_leave_state_unchanged() {
        let mut controller = TranslationSessionController::default();
        let before = controller.state.clone();
        controller.handle(LiveTranslateServerEvent::Ignored {
            kind: "response.created".into(),
        });
        assert_eq!(controller.state, before);
    }

    #[test]
    fn atomic_pair_updates_history_and_detected_language_together() {
        let mut controller = TranslationSessionController::default();
        controller.did_connect();
        controller.handle(LiveTranslateServerEvent::SubtitleFinalPair {
            source: "Hello.".into(),
            language: Some("en".into()),
            translation: "你好。".into(),
        });

        assert_eq!(controller.state.subtitles.history.len(), 1);
        assert_eq!(controller.state.subtitles.history[0].source, "Hello.");
        assert_eq!(controller.state.subtitles.history[0].translation, "你好。");
        assert_eq!(
            controller
                .state
                .detected_language
                .as_ref()
                .map(|value| value.code.as_str()),
            Some("en")
        );
    }
}
