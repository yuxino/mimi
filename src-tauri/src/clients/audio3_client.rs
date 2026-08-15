//! Audio 3.0 high-quality ASR WebSocket client, ported from
//! `Sources/MimiCore/Audio3ASRClient.swift`.

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
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Audio3ASRClientError {
    #[error("Add an Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
    #[error("The speech recognition session is not connected.")]
    NotConnected,
    #[error("The speech recognition connection stopped responding.")]
    HealthCheckTimedOut,
    #[error("The speech recognition service returned an unsupported WebSocket message.")]
    UnsupportedMessage,
    #[error("{0}")]
    Task(String),
    #[error("{0}")]
    Other(String),
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
    events: Arc<Mutex<Option<mpsc::UnboundedSender<LiveTranslateServerEvent>>>>,
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
    pub async fn set_event_sender(&self, sender: mpsc::UnboundedSender<LiveTranslateServerEvent>) {
        *self.events.lock().await = Some(sender);
    }

    /// Opens the socket, sends `run-task`, and waits for `task-started`.
    pub async fn connect(&self, task_id: &str) -> Result<(), Audio3ASRClientError> {
        self.disconnect().await;
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

        let (socket, _response) = connect_async(request)
            .await
            .map_err(|error| Audio3ASRClientError::Other(error.to_string()))?;
        let (sink, mut stream) = socket.split();
        *self.inner.sink.lock().await = Some(sink);
        self.inner.task_started.store(false, Ordering::SeqCst);
        self.inner.task_finished.store(false, Ordering::SeqCst);
        *self.inner.terminal_error.lock().await = None;

        let inner = self.inner.clone();
        let events = self
            .events
            .lock()
            .await
            .clone()
            .ok_or(Audio3ASRClientError::NotConnected)?;
        let source_language = self.source_language;
        let task = tokio::spawn(async move {
            loop {
                let message = stream.next().await;
                let Some(message) = message else { break };
                let text = match message {
                    Ok(Message::Text(text)) => text.to_string(),
                    Ok(Message::Binary(data)) => String::from_utf8_lossy(&data).to_string(),
                    Ok(Message::Pong(_)) => {
                        inner.pong_notify.notify_waiters();
                        continue;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Close(_)) | Ok(Message::Frame(_)) => {
                        continue;
                    }
                    Err(_) => {
                        *inner.terminal_error.lock().await =
                            Some("The speech recognition connection closed.".into());
                        if events
                            .send(LiveTranslateServerEvent::Error {
                                code: "transport_error".into(),
                                message: "The speech recognition connection closed.".into(),
                            })
                            .is_err()
                        {
                            break;
                        }
                        return;
                    }
                };
                {
                    let event = match Audio3ASRServerEventDecoder::decode(&text) {
                        Ok(event) => event,
                        Err(error) => {
                            *inner.terminal_error.lock().await = Some(error.to_string());
                            if events
                                .send(LiveTranslateServerEvent::Error {
                                    code: "transport_error".into(),
                                    message: error.to_string(),
                                })
                                .is_err()
                            {
                                break;
                            }
                            return;
                        }
                    };
                    match &event {
                        Audio3ASRServerEvent::TaskStarted => {
                            inner.task_started.store(true, Ordering::SeqCst);
                        }
                        Audio3ASRServerEvent::TaskFinished => {
                            inner.task_finished.store(true, Ordering::SeqCst);
                        }
                        Audio3ASRServerEvent::TaskFailed { code, message } => {
                            *inner.terminal_error.lock().await = Some(message.clone());
                            let _ = code;
                        }
                        _ => {}
                    }
                    let subtitle_event = event.subtitle_event(source_language);
                    let is_task_failed =
                        matches!(subtitle_event, LiveTranslateServerEvent::Error { .. });
                    if events.send(subtitle_event).is_err() {
                        break;
                    }
                    if is_task_failed {
                        return;
                    }
                }
            }
        });
        *self.inner.receive_task.lock().await = Some(task);

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
        self.send_text(run_task.to_string()).await?;

        self.wait_for_task_start(Duration::from_secs(10)).await
    }

    pub async fn send_audio(&self, pcm_data: &[u8]) -> Result<(), Audio3ASRClientError> {
        if pcm_data.is_empty() {
            return Ok(());
        }
        if !self.inner.task_started.load(Ordering::SeqCst) {
            return Err(Audio3ASRClientError::NotConnected);
        }
        let mut sink = self.inner.sink.lock().await;
        let Some(sink) = sink.as_mut() else {
            return Err(Audio3ASRClientError::NotConnected);
        };
        sink.send(Message::Binary(pcm_data.to_vec().into()))
            .await
            .map_err(|_| Audio3ASRClientError::NotConnected)
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), Audio3ASRClientError> {
        {
            let mut sink = self.inner.sink.lock().await;
            let Some(sink) = sink.as_mut() else {
                return Err(Audio3ASRClientError::NotConnected);
            };
            sink.send(Message::Ping(tokio_tungstenite::tungstenite::Bytes::new()))
                .await
                .map_err(|_| Audio3ASRClientError::NotConnected)?;
        }
        tokio::time::timeout(timeout, self.inner.pong_notify.notified())
            .await
            .map_err(|_| Audio3ASRClientError::HealthCheckTimedOut)?;
        Ok(())
    }

    /// Sends `finish-task`, waits briefly for `task-finished`, then
    /// disconnects.
    pub async fn finish(&self, timeout: Duration) {
        let Some(_sink) = self.inner.sink.lock().await.as_ref() else {
            return;
        };
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
        let mut sink = self.inner.sink.lock().await;
        if let Some(mut sink) = sink.take() {
            let _ = sink.close().await;
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
                return Err(Audio3ASRClientError::HealthCheckTimedOut);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn send_text(&self, text: String) -> Result<(), Audio3ASRClientError> {
        let mut sink = self.inner.sink.lock().await;
        let Some(sink) = sink.as_mut() else {
            return Err(Audio3ASRClientError::NotConnected);
        };
        sink.send(Message::Text(text.into()))
            .await
            .map_err(|_| Audio3ASRClientError::NotConnected)
    }
}
