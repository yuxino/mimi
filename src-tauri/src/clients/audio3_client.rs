//! Audio 3.0 high-quality ASR WebSocket client.

use crate::clients::provider_events::ProviderEventSender;
use crate::core::models::SourceLanguage;
use crate::core::protocols::audio3::{
    Audio3ASREndpoint, Audio3ASRRequestEncoder, Audio3ASRServerEvent, Audio3ASRServerEventDecoder,
};
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TIMEOUT: Duration = Duration::from_millis(250);
const GENERIC_TRANSPORT_ERROR: &str = "The speech recognition connection closed.";
const GENERIC_PROTOCOL_ERROR: &str = "The speech recognition service returned invalid data.";
const GENERIC_TASK_ERROR: &str = "The speech recognition task failed.";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Audio3ASRClientError {
    #[error("Add an Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
    #[error("The speech recognition session is not connected.")]
    NotConnected,
    #[error("The speech recognition connection stopped responding.")]
    HealthCheckTimedOut,
    #[error("{0}")]
    Task(String),
    #[error("The speech recognition connection could not be established in time.")]
    ConnectionTimedOut,
    #[error("The speech recognition task did not start in time.")]
    TaskSetupTimedOut,
    #[error("The speech recognition transport failed.")]
    TransportFailure,
}

type Sink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

struct Inner {
    sink: Mutex<Option<Sink>>,
    task_started: AtomicBool,
    task_finished: AtomicBool,
    terminal_error: Mutex<Option<String>>,
    pong_notify: Notify,
    receive_task: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct Audio3ASRClient {
    inner: Arc<Inner>,
    endpoint: Audio3ASREndpoint,
    api_key: String,
    source_language: SourceLanguage,
    events: Arc<Mutex<Option<ProviderEventSender>>>,
    task_id: Arc<Mutex<Option<String>>>,
}

impl Audio3ASRClient {
    pub fn new(
        api_key: &str,
        source_language: SourceLanguage,
    ) -> Result<Self, Audio3ASRClientError> {
        let trimmed_key = api_key.trim();
        if trimmed_key.is_empty() {
            return Err(Audio3ASRClientError::MissingAPIKey);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                sink: Mutex::new(None),
                task_started: AtomicBool::new(false),
                task_finished: AtomicBool::new(false),
                terminal_error: Mutex::new(None),
                pong_notify: Notify::new(),
                receive_task: Mutex::new(None),
            }),
            endpoint: Audio3ASREndpoint::new().map_err(|_| Audio3ASRClientError::MissingAPIKey)?,
            api_key: trimmed_key.to_string(),
            source_language,
            events: Arc::new(Mutex::new(None)),
            task_id: Arc::new(Mutex::new(None)),
        })
    }

    /// Sets the channel the receive loop emits decoded events onto.
    pub async fn set_event_sender(&self, sender: ProviderEventSender) {
        *self.events.lock().await = Some(sender);
    }

    /// Opens the socket, sends `run-task`, and waits for `task-started`.
    pub async fn connect(&self, task_id: &str) -> Result<(), Audio3ASRClientError> {
        self.disconnect().await;
        let events = self
            .events
            .lock()
            .await
            .clone()
            .ok_or(Audio3ASRClientError::NotConnected)?;
        let run_task = Audio3ASRRequestEncoder::run_task(
            task_id,
            self.source_language,
            Some(
                crate::core::protocols::audio3::Audio3ASRContext::audiovisual_dialogue(
                    self.source_language,
                ),
            ),
        )
        .map_err(|_| Audio3ASRClientError::NotConnected)?;
        *self.task_id.lock().await = Some(task_id.to_string());

        let mut request = self
            .endpoint
            .url
            .clone()
            .into_client_request()
            .map_err(|_| Audio3ASRClientError::NotConnected)?;
        let auth = format!("Bearer {}", self.api_key);
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&auth).map_err(|_| Audio3ASRClientError::MissingAPIKey)?,
        );
        request
            .headers_mut()
            .insert("User-Agent", HeaderValue::from_static("mimi-tauri"));

        let (socket, _response) = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(request))
            .await
            .map_err(|_| Audio3ASRClientError::ConnectionTimedOut)?
            .map_err(|_| Audio3ASRClientError::TransportFailure)?;
        let (sink, mut stream) = socket.split();
        *self.inner.sink.lock().await = Some(sink);
        self.inner.task_started.store(false, Ordering::SeqCst);
        self.inner.task_finished.store(false, Ordering::SeqCst);
        *self.inner.terminal_error.lock().await = None;

        let inner = self.inner.clone();
        let source_language = self.source_language;
        let task = tokio::spawn(async move {
            loop {
                let message = stream.next().await;
                let Some(message) = message else {
                    if should_report_transport_end(inner.task_finished.load(Ordering::SeqCst)) {
                        fail_transport(&inner, &events).await;
                    }
                    return;
                };
                let text = match message {
                    Ok(Message::Text(text)) => text.to_string(),
                    Ok(Message::Binary(data)) => String::from_utf8_lossy(&data).to_string(),
                    Ok(Message::Pong(_)) => {
                        inner.pong_notify.notify_waiters();
                        continue;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Frame(_)) => {
                        continue;
                    }
                    Ok(Message::Close(_)) => {
                        if should_report_transport_end(inner.task_finished.load(Ordering::SeqCst)) {
                            fail_transport(&inner, &events).await;
                        }
                        return;
                    }
                    Err(_) => {
                        if should_report_transport_end(inner.task_finished.load(Ordering::SeqCst)) {
                            fail_transport(&inner, &events).await;
                        }
                        return;
                    }
                };
                {
                    let event = match Audio3ASRServerEventDecoder::decode(&text) {
                        Ok(event) => event,
                        Err(_) => {
                            *inner.terminal_error.lock().await =
                                Some(GENERIC_PROTOCOL_ERROR.into());
                            let _ = events.send(LiveTranslateServerEvent::Error {
                                code: "audio3_protocol_error".into(),
                                message: GENERIC_PROTOCOL_ERROR.into(),
                            });
                            return;
                        }
                    };
                    let is_task_finished = matches!(&event, Audio3ASRServerEvent::TaskFinished);
                    match &event {
                        Audio3ASRServerEvent::TaskStarted => {
                            inner.task_started.store(true, Ordering::SeqCst);
                        }
                        Audio3ASRServerEvent::TaskFinished => {}
                        Audio3ASRServerEvent::TaskFailed { .. } => {
                            *inner.terminal_error.lock().await = Some(GENERIC_TASK_ERROR.into());
                        }
                        _ => {}
                    }
                    let subtitle_event = match &event {
                        Audio3ASRServerEvent::TaskFailed { code, .. } => {
                            LiveTranslateServerEvent::Error {
                                code: sanitize_provider_code(code),
                                message: GENERIC_TASK_ERROR.into(),
                            }
                        }
                        _ => event.subtitle_event(source_language),
                    };
                    let is_task_failed =
                        matches!(subtitle_event, LiveTranslateServerEvent::Error { .. });
                    let send_result = events.send(subtitle_event);
                    // `finish` disconnects the socket as soon as this flag is
                    // visible. Publish SessionFinished first so that cleanup
                    // cannot abort the receive task between provider ack and
                    // the bridge event needed to drain an authoritative tail.
                    if is_task_finished {
                        inner.task_finished.store(true, Ordering::SeqCst);
                    }
                    if send_result.is_err() {
                        break;
                    }
                    if is_task_failed {
                        return;
                    }
                }
            }
        });
        *self.inner.receive_task.lock().await = Some(task);

        let result = async {
            self.send_text(run_task.to_string()).await?;
            self.wait_for_task_start(Duration::from_secs(10)).await
        }
        .await;
        if result.is_err() {
            self.disconnect().await;
        }
        result
    }

    pub async fn send_audio(&self, pcm_data: &[u8]) -> Result<(), Audio3ASRClientError> {
        if pcm_data.is_empty() {
            return Ok(());
        }
        if !self.inner.task_started.load(Ordering::SeqCst) {
            return Err(Audio3ASRClientError::NotConnected);
        }
        let operation = async {
            let mut sink = self.inner.sink.lock().await;
            let Some(sink) = sink.as_mut() else {
                return Err(Audio3ASRClientError::NotConnected);
            };
            sink.send(Message::Binary(pcm_data.to_vec().into()))
                .await
                .map_err(|_| Audio3ASRClientError::TransportFailure)
        };
        tokio::time::timeout(SEND_TIMEOUT, operation)
            .await
            .map_err(|_| Audio3ASRClientError::TransportFailure)?
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), Audio3ASRClientError> {
        let operation = async {
            let pong = self.inner.pong_notify.notified();
            tokio::pin!(pong);
            pong.as_mut().enable();
            {
                let mut sink = self.inner.sink.lock().await;
                let Some(sink) = sink.as_mut() else {
                    return Err(Audio3ASRClientError::NotConnected);
                };
                sink.send(Message::Ping(tokio_tungstenite::tungstenite::Bytes::new()))
                    .await
                    .map_err(|_| Audio3ASRClientError::TransportFailure)?;
            }
            pong.await;
            Ok(())
        };
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|_| Audio3ASRClientError::HealthCheckTimedOut)?
    }

    /// Sends `finish-task`, waits briefly for `task-finished`, then
    /// disconnects.
    pub async fn finish(&self, timeout: Duration) {
        if self.inner.sink.lock().await.is_none() {
            return;
        }
        let task_id = self.task_id.lock().await.clone().unwrap_or_default();
        if let Ok(command) = Audio3ASRRequestEncoder::finish_task(&task_id) {
            let _ = self.send_text(command.to_string()).await;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.inner.task_finished.load(Ordering::SeqCst) {
                break;
            }
            if self.inner.terminal_error.lock().await.is_some() {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        self.disconnect().await;
    }

    pub async fn disconnect(&self) {
        if let Some(task) = self.inner.receive_task.lock().await.take() {
            task.abort();
        }
        let sink = self.inner.sink.lock().await.take();
        if let Some(mut sink) = sink {
            let _ = tokio::time::timeout(CLOSE_TIMEOUT, sink.close()).await;
        }
        self.inner.task_started.store(false, Ordering::SeqCst);
        self.inner.task_finished.store(false, Ordering::SeqCst);
        *self.inner.terminal_error.lock().await = None;
    }

    async fn wait_for_task_start(&self, timeout: Duration) -> Result<(), Audio3ASRClientError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.inner.task_started.load(Ordering::SeqCst) {
                return Ok(());
            }
            if let Some(error) = self.inner.terminal_error.lock().await.clone() {
                return Err(Audio3ASRClientError::Task(error));
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(Audio3ASRClientError::TaskSetupTimedOut);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn send_text(&self, text: String) -> Result<(), Audio3ASRClientError> {
        let operation = async {
            let mut sink = self.inner.sink.lock().await;
            let Some(sink) = sink.as_mut() else {
                return Err(Audio3ASRClientError::NotConnected);
            };
            sink.send(Message::Text(text.into()))
                .await
                .map_err(|_| Audio3ASRClientError::TransportFailure)
        };
        tokio::time::timeout(SEND_TIMEOUT, operation)
            .await
            .map_err(|_| Audio3ASRClientError::TransportFailure)?
    }
}

async fn fail_transport(inner: &Inner, events: &ProviderEventSender) {
    *inner.terminal_error.lock().await = Some(GENERIC_TRANSPORT_ERROR.into());
    let _ = events.send(LiveTranslateServerEvent::Error {
        code: "transport_error".into(),
        message: GENERIC_TRANSPORT_ERROR.into(),
    });
}

fn should_report_transport_end(task_finished: bool) -> bool {
    !task_finished
}

fn sanitize_provider_code(code: &str) -> String {
    let sanitized: String = code
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .take(64)
        .collect();
    if sanitized.is_empty() {
        "audio3_task_failed".into()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_end_is_failure_only_before_task_finished() {
        assert!(should_report_transport_end(false));
        assert!(!should_report_transport_end(true));
    }

    #[test]
    fn provider_codes_are_bounded_and_sanitized() {
        assert_eq!(sanitize_provider_code("CLIENT_ERROR"), "CLIENT_ERROR");
        assert_eq!(sanitize_provider_code("bad code: secret"), "badcodesecret");
        assert!(sanitize_provider_code(&"x".repeat(100)).len() <= 64);
        assert_eq!(sanitize_provider_code("***"), "audio3_task_failed");
    }
}
