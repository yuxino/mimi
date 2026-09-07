//! Qwen-MT HTTP client for streaming and non-streaming chat completions.

use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::qwen_mt::{
    QwenMTClientError, QwenMTEndpoint, QwenMTMemoryPair, QwenMTModel, QwenMTProtocolError,
    QwenMTRequestEncoder, QwenMTResponseDecoder, QwenMTStreamDecoder, QwenMTTerm,
};
use futures_util::StreamExt;
use std::time::Duration;

// A sentence translation should stay far below these bounds. Enforce byte
// limits even when Content-Length is absent or the server never sends '\n'.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_TRANSLATION_BYTES: usize = 64 * 1024;

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
            endpoint: QwenMTEndpoint::new().map_err(|_| QwenMTClientError::InvalidHTTPResponse)?,
            api_key: trimmed_key.to_string(),
            source_language,
            target_language,
            model,
            domain_hint,
            terms,
            client: http_client_builder()
                .build()
                .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?,
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
        let bytes = read_bounded_response(response).await;
        if !status.is_success() {
            return Err(QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: error_message(&bytes.unwrap_or_default()),
            });
        }
        let bytes = bytes?;
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
            let bytes = read_bounded_response(response).await.unwrap_or_default();
            return Err(QwenMTClientError::RequestFailed {
                status_code: status.as_u16(),
                message: error_message(&bytes),
            });
        }

        let mut stream = response.bytes_stream();
        let mut decoder = BoundedSseResponse::default();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
            if decoder.push(&chunk, on_partial)? {
                break;
            }
        }

        let trimmed = decoder.finish(on_partial)?;
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

fn http_client_builder() -> reqwest::ClientBuilder {
    // reqwest 0.13's no-provider mode requires explicit initialization. The
    // updater does the same when checking for updates, but translation may be
    // used first. A previously installed provider is intentionally preserved.
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
}

async fn read_bounded_response(response: reqwest::Response) -> Result<Vec<u8>, QwenMTClientError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(QwenMTClientError::ResponseTooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
        if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() {
            return Err(QwenMTClientError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Default)]
struct BoundedSseResponse {
    buffer: Vec<u8>,
    scanned: usize,
    received: usize,
    translation: String,
    done: bool,
}

impl BoundedSseResponse {
    fn push(
        &mut self,
        chunk: &[u8],
        on_partial: &(impl Fn(String) + Send + Sync),
    ) -> Result<bool, QwenMTClientError> {
        if self.done {
            return Ok(true);
        }
        if chunk.len() > MAX_RESPONSE_BYTES - self.received {
            return Err(QwenMTClientError::ResponseTooLarge);
        }
        self.received += chunk.len();
        self.buffer.extend_from_slice(chunk);
        let mut consumed = 0;
        // Scan only newly received bytes and compact once per network chunk.
        // UTF-8 is decoded only after a complete line has arrived.
        while let Some(offset) = self.buffer[self.scanned..].iter().position(|&b| b == b'\n') {
            let end = self.scanned + offset + 1;
            let line = std::str::from_utf8(&self.buffer[consumed..end])
                .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
            self.done = handle_sse_line(line.trim_end(), &mut self.translation, on_partial)?;
            consumed = end;
            self.scanned = end;
            if self.done {
                break;
            }
        }
        self.buffer.drain(..consumed);
        self.scanned = self.buffer.len();
        Ok(self.done)
    }

    fn finish(
        mut self,
        on_partial: &(impl Fn(String) + Send + Sync),
    ) -> Result<String, QwenMTClientError> {
        // Preserve support for servers that omit the final newline.
        if !self.done && !self.buffer.is_empty() {
            let line = std::str::from_utf8(&self.buffer)
                .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
            handle_sse_line(line.trim_end(), &mut self.translation, on_partial)?;
        }
        Ok(self.translation.trim().to_string())
    }
}

/// Handles one SSE `data:` line, appending decoded content to `translation`.
/// Returns `true` when the stream has reached `[DONE]`.
fn handle_sse_line(
    line: &str,
    translation: &mut String,
    on_partial: &(impl Fn(String) + Send + Sync),
) -> Result<bool, QwenMTClientError> {
    let Some(payload) = line.strip_prefix("data:") else {
        return Ok(false);
    };
    let payload = payload.trim();
    if payload.is_empty() {
        return Ok(false);
    }
    if payload == "[DONE]" {
        return Ok(true);
    }
    let content = QwenMTStreamDecoder::decode_chunk(payload)
        .map_err(|_| QwenMTClientError::InvalidHTTPResponse)?;
    if let Some(content) = content {
        if !content.is_empty() {
            if content.len() > MAX_TRANSLATION_BYTES - translation.len() {
                return Err(QwenMTClientError::ResponseTooLarge);
            }
            translation.push_str(&content);
            on_partial(translation.clone());
        }
    }
    Ok(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn sse_accepts_every_utf8_chunk_boundary_and_ignores_data_after_done() {
        let data =
            "data: {\"choices\":[{\"delta\":{\"content\":\"今天\"}}]}\r\n\ndata: [DONE]\ninvalid";
        for split in 0..=data.len() {
            let mut decoder = BoundedSseResponse::default();
            decoder.push(&data.as_bytes()[..split], &|_| {}).unwrap();
            decoder.push(&data.as_bytes()[split..], &|_| {}).unwrap();
            assert_eq!(decoder.finish(&|_| {}).unwrap(), "今天");
        }
    }

    #[test]
    fn sse_accepts_a_final_line_without_newline() {
        let mut decoder = BoundedSseResponse::default();
        decoder
            .push(
                b"data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}",
                &|_| {},
            )
            .unwrap();
        assert_eq!(decoder.finish(&|_| {}).unwrap(), "hello");
    }

    #[test]
    fn sse_rejects_oversized_unterminated_and_repeated_lines_before_growth() {
        let mut decoder = BoundedSseResponse::default();
        decoder
            .push(&vec![b' '; MAX_RESPONSE_BYTES], &|_| {})
            .unwrap();
        assert_eq!(
            decoder.push(b" ", &|_| {}),
            Err(QwenMTClientError::ResponseTooLarge)
        );
        assert_eq!(decoder.buffer.len(), MAX_RESPONSE_BYTES);

        let mut decoder = BoundedSseResponse::default();
        for _ in 0..16 {
            decoder
                .push(&vec![b'\n'; MAX_RESPONSE_BYTES / 16], &|_| {})
                .unwrap();
        }
        assert!(decoder.buffer.is_empty());
        assert_eq!(
            decoder.push(b"\n", &|_| {}),
            Err(QwenMTClientError::ResponseTooLarge)
        );
    }

    #[test]
    fn translation_limit_rejects_before_publishing_an_oversized_preview() {
        let mut translation = "x".repeat(MAX_TRANSLATION_BYTES);
        let error = handle_sse_line(
            r#"data: {"choices":[{"delta":{"content":"x"}}]}"#,
            &mut translation,
            &|_| panic!("oversized partial must not be published"),
        );
        assert_eq!(error, Err(QwenMTClientError::ResponseTooLarge));
        assert_eq!(translation.len(), MAX_TRANSLATION_BYTES);
    }

    async fn fixture_client(response: String) -> (QwenMTClient, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let read = socket.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            // Oversized responses may intentionally close before reading the body.
            let _ = socket.write_all(response.as_bytes()).await;
            String::from_utf8(request).unwrap()
        });
        let mut client = QwenMTClient::new(
            "fixture-only",
            SourceLanguage::English,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Flash,
            None,
            vec![],
            Duration::from_millis(300),
        )
        .unwrap();
        client.endpoint.url = format!("http://{address}/translate").parse().unwrap();
        client.client = http_client_builder().no_proxy().build().unwrap();
        (client, server)
    }

    #[tokio::test]
    async fn http_translation_preserves_json_request_and_response() {
        let body = r#"{"choices":[{"message":{"content":"你好"}}]}"#;
        let (client, server) = fixture_client(format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        ))
        .await;
        assert_eq!(client.translate("hello", None, &[]).await.unwrap(), "你好");
        let request = server.await.unwrap();
        assert!(request.starts_with("POST /translate HTTP/1.1\r\n"));
        assert!(request.contains("authorization: Bearer fixture-only"));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["model"], "qwen-mt-flash");
        assert_eq!(body["stream"], false);
    }

    #[tokio::test]
    async fn http_streaming_preserves_partials_and_done() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\ndata: [DONE]\n";
        let (client, server) = fixture_client(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n", body.len(),
        )).await;
        let partials = std::sync::Mutex::new(Vec::new());
        assert_eq!(
            client
                .translate_streaming("hello", None, &[], |text| {
                    partials.lock().unwrap().push(text);
                })
                .await
                .unwrap(),
            "你好"
        );
        assert_eq!(*partials.lock().unwrap(), vec!["你", "你好"]);
        let request = server.await.unwrap();
        assert!(request.contains("accept: text/event-stream"));
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["stream"], true);
    }

    #[tokio::test]
    async fn streaming_deadline_cancels_a_stalled_http_body() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let read = socket.read(&mut request).await.unwrap();
            assert!(read > 0);
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                .await
                .unwrap();
            std::future::pending::<()>().await;
        });
        let mut client = QwenMTClient::new(
            "fixture-only",
            SourceLanguage::English,
            TargetLanguage::SimplifiedChinese,
            QwenMTModel::Flash,
            None,
            vec![],
            Duration::from_millis(100),
        )
        .unwrap();
        client.endpoint.url = format!("http://{address}/translate").parse().unwrap();
        client.client = http_client_builder().no_proxy().build().unwrap();
        let result = client.translate_streaming("hello", None, &[], |_| {}).await;
        server.abort();
        assert_eq!(result, Err(QwenMTClientError::RequestTimedOut));
    }

    #[tokio::test]
    #[ignore = "manual public HTTPS certificate/proxy smoke; sends no credentials or subtitle data"]
    async fn public_https_uses_system_certificate_validation() {
        let client = http_client_builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap();
        for endpoint in ["https://dashscope.aliyuncs.com", "https://github.com"] {
            client
                .head(endpoint)
                .send()
                .await
                .expect("public HTTPS handshake must succeed");
        }
    }

    #[tokio::test]
    async fn chunked_http_body_is_bounded_without_content_length() {
        let chunk = " ".repeat(MAX_RESPONSE_BYTES + 1);
        let (client, server) = fixture_client(format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{chunk}\r\n0\r\n\r\n", chunk.len(),
        )).await;
        assert_eq!(
            client.translate("hello", None, &[]).await,
            Err(QwenMTClientError::ResponseTooLarge)
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn oversized_http_error_preserves_authentication_status() {
        let (client, server) = fixture_client(format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_RESPONSE_BYTES + 1,
        ))
        .await;
        let error = client.translate("hello", None, &[]).await.unwrap_err();
        assert!(error.is_authentication_failure());
        assert_eq!(
            error,
            QwenMTClientError::RequestFailed {
                status_code: 401,
                message: String::new()
            }
        );
        server.await.unwrap();
    }

    #[test]
    fn sse_line_appends_content_and_reports_done() {
        let mut translation = String::new();
        let partials = std::sync::Mutex::new(Vec::new());

        let handled = handle_sse_line(
            r#"data:{"choices":[{"delta":{"content":"今天"}}]}"#,
            &mut translation,
            &|text| partials.lock().unwrap().push(text),
        )
        .unwrap();
        assert!(!handled);
        assert_eq!(translation, "今天");
        assert_eq!(*partials.lock().unwrap(), vec!["今天".to_string()]);

        let done = handle_sse_line("data: [DONE]", &mut translation, &|_| {}).unwrap();
        assert!(done);
        assert_eq!(translation, "今天");
    }

    #[test]
    fn non_data_lines_are_ignored() {
        let mut translation = String::new();
        let handled = handle_sse_line("event: message", &mut translation, &|_| {}).unwrap();
        assert!(!handled);
        assert!(translation.is_empty());
    }
}
