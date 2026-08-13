//! Qwen-MT HTTP client (chat/completions, streaming and non-streaming),
//! ported from `Sources/MimiCore/QwenMTClient.swift`.

use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::qwen_mt::{
    QwenMTClientError, QwenMTDomainHint, QwenMTEndpoint, QwenMTMemoryPair, QwenMTModel,
    QwenMTProtocolError, QwenMTRequestEncoder, QwenMTResponseDecoder, QwenMTStreamDecoder,
    QwenMTTerm,
};
use futures_util::StreamExt;
use std::time::Duration;

pub struct QwenMTClient {
    endpoint: QwenMTEndpoint,
    api_key: String,
    source_language: SourceLanguage,
    target_language: TargetLanguage,
    model: QwenMTModel,
    domain_hint: Option<String>,
    terms: Vec<QwenMTTerm>,
    client: reqwest::Client,
    streaming_timeout: Duration,
}

impl QwenMTClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        api_key: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        model: QwenMTModel,
        domain_hint: Option<String>,
        terms: Vec<QwenMTTerm>,
        streaming_timeout: Duration,
    ) -> Result<Self, QwenMTClientError> {
        let trimmed_key = api_key.trim();
        if trimmed_key.is_empty() {
            return Err(QwenMTClientError::MissingAPIKey);
        }
        Ok(Self {
            endpoint: QwenMTEndpoint::new(workspace_id)
                .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?,
            api_key: trimmed_key.to_string(),
            source_language,
            target_language,
            model,
            domain_hint,
            terms,
            client: reqwest::Client::new(),
            streaming_timeout,
        })
    }

    pub async fn translate(
        &self,
        text: &str,
        source_language_override: Option<SourceLanguage>,
        translation_memory: &[QwenMTMemoryPair],
    ) -> Result<String, QwenMTClientError> {
        let body = self.make_body(text, source_language_override, false, translation_memory)?;
        let timeout = if self.model == QwenMTModel::Plus {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(10)
        };
        let response = self
            .client
            .post(self.endpoint.url.clone())
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .timeout(timeout)
            .body(body)
            .send()
            .await
            .map_err(|_| QwenMTClientError::RequestTimedOut)?;

        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
        if !status.is_success() {
            return Err(QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: error_message(&bytes),
            });
        }
        QwenMTResponseDecoder::decode(&String::from_utf8_lossy(&bytes)).map_err(|error| match error
        {
            QwenMTProtocolError::InvalidJSON => QwenMTClientError::InvalidHTTPResponse,
            QwenMTProtocolError::MissingTranslation => QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: "Qwen-MT returned no translated text.".into(),
            },
            other => QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: other.to_string(),
            },
        })
    }

    /// Streams the translation, invoking `on_partial` with the accumulated
    /// text after every chunk. The whole request must finish within
    /// `streaming_timeout`.
    pub async fn translate_streaming(
        &self,
        text: &str,
        source_language_override: Option<SourceLanguage>,
        translation_memory: &[QwenMTMemoryPair],
        on_partial: impl Fn(String) + Send + Sync,
    ) -> Result<String, QwenMTClientError> {
        let body = self.make_body(text, source_language_override, true, translation_memory)?;
        let timeout = self.streaming_timeout;
        let request = self
            .client
            .post(self.endpoint.url.clone())
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .body(body);

        let streamed =
            tokio::time::timeout(timeout, async { self.stream(request, &on_partial).await })
                .await
                .map_err(|_| QwenMTClientError::RequestTimedOut)??;
        Ok(streamed)
    }

    async fn stream(
        &self,
        request: reqwest::RequestBuilder,
        on_partial: &(impl Fn(String) + Send + Sync),
    ) -> Result<String, QwenMTClientError> {
        let response = request
            .send()
            .await
            .map_err(|_| QwenMTClientError::RequestTimedOut)?;
        let status = response.status();
        if !status.is_success() {
            let bytes = response.bytes().await.unwrap_or_default();
            return Err(QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: error_message(&bytes),
            });
        }

        let mut stream = response.bytes_stream();
        let mut translation = String::new();
        let mut buffer = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(newline) = buffer.find('\n') {
                let line: String = buffer.drain(..=newline).collect();
                let line = line.trim_end();
                let Some(payload) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                if payload.is_empty() {
                    continue;
                }
                if payload == "[DONE]" {
                    break;
                }
                let content = QwenMTStreamDecoder::decode_chunk(payload)
                    .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
                if let Some(content) = content {
                    if !content.is_empty() {
                        translation.push_str(&content);
                        on_partial(translation.clone());
                    }
                }
            }
        }

        let trimmed = translation.trim().to_string();
        if trimmed.is_empty() {
            return Err(QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: "Qwen-MT returned no translated text.".into(),
            });
        }
        Ok(trimmed)
    }

    fn make_body(
        &self,
        text: &str,
        source_language_override: Option<SourceLanguage>,
        stream: bool,
        translation_memory: &[QwenMTMemoryPair],
    ) -> Result<String, QwenMTClientError> {
        let trimmed_text = text.trim().to_string();
        if trimmed_text.is_empty() {
            return Err(QwenMTClientError::RequestFailed {
                status_code: 0,
                message: "Qwen-MT returned no translated text.".into(),
            });
        }
        let request = QwenMTRequestEncoder::request(
            &trimmed_text,
            source_language_override.unwrap_or(self.source_language),
            self.target_language,
            self.model,
            stream,
            self.domain_hint.as_deref(),
            &self.terms,
            translation_memory,
        )
        .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
        Ok(request.to_string())
    }
}

fn error_message(data: &[u8]) -> String {
    #[derive(serde::Deserialize)]
    struct ErrorBody {
        error: Option<ErrorInner>,
    }
    #[derive(serde::Deserialize)]
    struct ErrorInner {
        message: Option<String>,
    }
    serde_json::from_slice::<ErrorBody>(data)
        .ok()
        .and_then(|body| body.error)
        .and_then(|error| error.message)
        .unwrap_or_default()
}

/// Domain hint and filler terms for a (source, target) pair, matching the
/// Swift `HighQualityTranslationClient` construction.
pub fn spoken_dialogue_config(
    source_language: SourceLanguage,
    target_language: TargetLanguage,
) -> (Option<String>, Vec<QwenMTTerm>) {
    (
        Some(QwenMTDomainHint::spoken_dialogue(
            source_language,
            target_language,
        )),
        QwenMTDomainHint::filler_terms(source_language, target_language),
    )
}
