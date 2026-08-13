//! Audio 3.0 high-quality ASR protocol (`qwen-audio-3.0-asr-flash-streaming`
//! over `/api-ws/v1/inference`), ported 1:1 from
//! `Sources/MimiCore/Audio3ASRProtocol.swift`.

use crate::core::configuration::is_valid_workspace_id;
use crate::core::models::SourceLanguage;
use crate::core::protocols::live_translate::{
    LiveTranslateProtocolError, LiveTranslateServerEvent, WORKSPACE_HOST_SUFFIX,
};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Audio3ASREndpoint {
    pub url: url::Url,
}

impl Audio3ASREndpoint {
    pub const MODEL: &'static str = "qwen-audio-3.0-asr-flash-streaming";

    pub fn new(workspace_id: &str) -> Result<Self, LiveTranslateProtocolError> {
        if !is_valid_workspace_id(workspace_id) {
            return Err(LiveTranslateProtocolError::InvalidWorkspaceID);
        }
        let raw = format!("wss://{workspace_id}{WORKSPACE_HOST_SUFFIX}/api-ws/v1/inference");
        Ok(Self {
            url: url::Url::parse(&raw).map_err(|_| LiveTranslateProtocolError::InvalidEndpoint)?,
        })
    }
}

pub enum Audio3ASRRequestEncoder {}

impl Audio3ASRRequestEncoder {
    pub fn run_task(
        task_id: &str,
        source_language: SourceLanguage,
        context: Option<&str>,
    ) -> Result<Value, LiveTranslateProtocolError> {
        let trimmed_context = context.map(str::trim).filter(|t| !t.is_empty());

        let mut parameters = json!({
            "format": "pcm",
            "sample_rate": 16_000,
            "semantic_punctuation_enabled": true,
            "heartbeat": true
        });
        if source_language != SourceLanguage::Automatic {
            parameters["language_hints"] = json!([source_language.raw_value()]);
        }

        let mut input = json!({});
        if let Some(text) = trimmed_context {
            input["context"] = json!([{
                "role": "user",
                "content": [{ "type": "input_text", "text": text }]
            }]);
        }

        Ok(json!({
            "header": {
                "action": "run-task",
                "task_id": task_id,
                "streaming": "duplex"
            },
            "payload": {
                "task_group": "audio",
                "task": "asr",
                "function": "recognition",
                "model": Audio3ASREndpoint::MODEL,
                "parameters": parameters,
                "input": input
            }
        }))
    }

    pub fn finish_task(task_id: &str) -> Result<Value, LiveTranslateProtocolError> {
        Ok(json!({
            "header": {
                "action": "finish-task",
                "task_id": task_id,
                "streaming": "duplex"
            },
            "payload": {
                "input": {}
            }
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audio3ASRServerEvent {
    TaskStarted,
    Transcription { text: String, is_final: bool },
    Heartbeat,
    TaskFinished,
    TaskFailed { code: String, message: String },
    Ignored { kind: String },
}

impl Audio3ASRServerEvent {
    /// Maps this recognizer event to the shared subtitle event stream.
    pub fn subtitle_event(&self, source_language: SourceLanguage) -> LiveTranslateServerEvent {
        let reported_language = (source_language != SourceLanguage::Automatic)
            .then(|| source_language.raw_value().to_string());
        match self {
            Self::TaskStarted => LiveTranslateServerEvent::SessionCreated,
            Self::Transcription { text, is_final } => {
                if *is_final {
                    LiveTranslateServerEvent::SourceFinal {
                        text: text.clone(),
                        language: reported_language,
                    }
                } else {
                    LiveTranslateServerEvent::SourceDraft {
                        text: text.clone(),
                        language: reported_language,
                    }
                }
            }
            Self::Heartbeat => LiveTranslateServerEvent::Ignored {
                kind: "heartbeat".into(),
            },
            Self::TaskFinished => LiveTranslateServerEvent::SessionFinished,
            Self::TaskFailed { code, message } => LiveTranslateServerEvent::Error {
                code: code.clone(),
                message: message.clone(),
            },
            Self::Ignored { kind } => LiveTranslateServerEvent::Ignored { kind: kind.clone() },
        }
    }
}

pub enum Audio3ASRServerEventDecoder {}

impl Audio3ASRServerEventDecoder {
    pub fn decode(text: &str) -> Result<Audio3ASRServerEvent, LiveTranslateProtocolError> {
        let json: Value =
            serde_json::from_str(text).map_err(|_| LiveTranslateProtocolError::InvalidJSON)?;
        let header = json
            .get("header")
            .ok_or(LiveTranslateProtocolError::InvalidJSON)?;
        let event = header
            .get("event")
            .and_then(Value::as_str)
            .ok_or(LiveTranslateProtocolError::MissingEventType)?;

        match event {
            "task-started" => Ok(Audio3ASRServerEvent::TaskStarted),
            "task-finished" => Ok(Audio3ASRServerEvent::TaskFinished),
            "task-failed" => Ok(Audio3ASRServerEvent::TaskFailed {
                code: header
                    .get("error_code")
                    .and_then(Value::as_str)
                    .unwrap_or("asr_task_failed")
                    .to_string(),
                message: header
                    .get("error_message")
                    .and_then(Value::as_str)
                    .unwrap_or("Alibaba Cloud speech recognition failed.")
                    .to_string(),
            }),
            "result-generated" => {
                let sentence = json
                    .pointer("/payload/output/sentence")
                    .ok_or(LiveTranslateProtocolError::InvalidJSON)?;
                if sentence.get("heartbeat").and_then(Value::as_bool) == Some(true) {
                    return Ok(Audio3ASRServerEvent::Heartbeat);
                }
                let text = sentence
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if text.is_empty() {
                    return Ok(Audio3ASRServerEvent::Ignored {
                        kind: "empty-result".into(),
                    });
                }
                Ok(Audio3ASRServerEvent::Transcription {
                    text,
                    is_final: sentence.get("sentence_end").and_then(Value::as_bool) == Some(true),
                })
            }
            other => Ok(Audio3ASRServerEvent::Ignored {
                kind: other.to_string(),
            }),
        }
    }
}

/// Recognition context hints, verbatim from the Swift `Audio3ASRContext`.
pub enum Audio3ASRContext {}

impl Audio3ASRContext {
    pub fn audiovisual_dialogue(language: SourceLanguage) -> &'static str {
        match language {
            SourceLanguage::Automatic => {
                "Natural audiovisual dialogue, including interjections, breaths, gasps, moans, cries, laughter, and other vocalizations."
            }
            SourceLanguage::Chinese => {
                "中文影视口语对白，包括语气词、停顿、喘息、呻吟、哭声、笑声和其他发声。"
            }
            SourceLanguage::English => {
                "Natural English audiovisual dialogue, including interjections, hesitations, breaths, gasps, moans, cries, laughter, and other vocalizations."
            }
            SourceLanguage::Japanese => {
                "日本語の映像作品の自然な口語会話。感動詞、間投詞、息遣い、喘ぎ声、うめき声、泣き声、笑い声などの発声を含む。"
            }
            SourceLanguage::Korean => {
                "한국어 영상 작품의 자연스러운 구어 대화. 감탄사, 머뭇거림, 숨소리, 신음, 울음, 웃음 등 발성을 포함함."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio3_endpoint_uses_the_binary_inference_websocket() {
        let endpoint = Audio3ASREndpoint::new("ws-abc123").unwrap();
        assert_eq!(
            endpoint.url.as_str(),
            "wss://ws-abc123.cn-beijing.maas.aliyuncs.com/api-ws/v1/inference"
        );
    }

    #[test]
    fn run_task_favors_accurate_sentence_boundaries() {
        let data = Audio3ASRRequestEncoder::run_task(
            "task-123",
            SourceLanguage::Japanese,
            Some("日本語の自然な会話"),
        )
        .unwrap();
        let header = &data["header"];
        let payload = &data["payload"];
        let parameters = &payload["parameters"];
        let input = &payload["input"];

        assert_eq!(header["action"], "run-task");
        assert_eq!(header["task_id"], "task-123");
        assert_eq!(payload["model"], "qwen-audio-3.0-asr-flash-streaming");
        assert_eq!(parameters["format"], "pcm");
        assert_eq!(parameters["sample_rate"], 16_000);
        assert_eq!(parameters["semantic_punctuation_enabled"], true);
        assert_eq!(parameters["heartbeat"], true);
        assert_eq!(parameters["language_hints"], json!(["ja"]));
        assert!(
            input.get("context").is_some(),
            "dialogue context should improve recognition"
        );
        assert!(
            parameters.get("special_word_filter").is_none(),
            "sensitive filtering must stay disabled"
        );
    }

    #[test]
    fn automatic_recognition_omits_language_hints() {
        let data = Audio3ASRRequestEncoder::run_task("task-auto", SourceLanguage::Automatic, None)
            .unwrap();
        let parameters = &data["payload"]["parameters"];
        assert!(parameters.get("language_hints").is_none());
    }

    #[test]
    fn finish_task_preserves_the_task_identifier() {
        let data = Audio3ASRRequestEncoder::finish_task("task-finish").unwrap();
        let header = &data["header"];
        let payload = &data["payload"];

        assert_eq!(header["action"], "finish-task");
        assert_eq!(header["task_id"], "task-finish");
        assert!(
            payload["input"].is_object(),
            "finish-task requires an empty input object"
        );
    }

    #[test]
    fn interim_and_final_results_map_to_subtitle_events() {
        let draft = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"今日は","heartbeat":false,"sentence_end":false}}}}"#,
        )
        .unwrap();
        let final_event = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"今日は晴れです。","heartbeat":false,"sentence_end":true}}}}"#,
        )
        .unwrap();

        assert_eq!(
            draft.subtitle_event(SourceLanguage::Japanese),
            LiveTranslateServerEvent::SourceDraft {
                text: "今日は".into(),
                language: Some("ja".into())
            }
        );
        assert_eq!(
            final_event.subtitle_event(SourceLanguage::Japanese),
            LiveTranslateServerEvent::SourceFinal {
                text: "今日は晴れです。".into(),
                language: Some("ja".into())
            }
        );
    }

    #[test]
    fn lifecycle_and_failures_decode() {
        let started = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"task-started"},"payload":{}}"#,
        )
        .unwrap();
        let failed = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"task-failed","error_code":"CLIENT_ERROR","error_message":"Bad request"},"payload":{}}"#,
        )
        .unwrap();

        assert_eq!(started, Audio3ASRServerEvent::TaskStarted);
        assert_eq!(
            failed,
            Audio3ASRServerEvent::TaskFailed {
                code: "CLIENT_ERROR".into(),
                message: "Bad request".into()
            }
        );
    }

    #[test]
    fn heartbeat_and_empty_results_are_ignored() {
        let heartbeat = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"","heartbeat":true,"sentence_end":false}}}}"#,
        )
        .unwrap();
        assert_eq!(heartbeat, Audio3ASRServerEvent::Heartbeat);

        let empty = Audio3ASRServerEventDecoder::decode(
            r#"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"  ","heartbeat":false,"sentence_end":false}}}}"#,
        )
        .unwrap();
        assert_eq!(
            empty,
            Audio3ASRServerEvent::Ignored {
                kind: "empty-result".into()
            }
        );
    }

    #[test]
    fn context_hints_are_verbatim() {
        assert_eq!(
            Audio3ASRContext::audiovisual_dialogue(SourceLanguage::Japanese),
            "日本語の映像作品の自然な口語会話。感動詞、間投詞、息遣い、喘ぎ声、うめき声、泣き声、笑い声などの発声を含む。"
        );
        assert!(
            Audio3ASRContext::audiovisual_dialogue(SourceLanguage::Chinese)
                .contains("中文影视口语对白")
        );
    }
}
