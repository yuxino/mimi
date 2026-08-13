//! Live-translate WebSocket client (`qwen3.5-livetranslate-flash-realtime`),
//! ported from `Sources/MimiCore/LiveTranslateClient.swift`.

use crate::core::models::{SourceLanguage, TargetLanguage};
use crate::core::protocols::live_translate::{
    LiveTranslateEndpoint, LiveTranslateRequestEncoder, LiveTranslateServerEvent,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::BTreeMap;
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
pub enum LiveTranslateClientError {
    #[error("Add an Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
    #[error("The live translation session is not connected.")]
    NotConnected,
    #[error("The live translation connection stopped responding.")]
    HealthCheckTimedOut,
    #[error("The live translation service returned an unsupported WebSocket message.")]
    UnsupportedMessage,
}

type Sink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

struct Inner {
    sink: Mutex<Option<Sink>>,
    received_session_finished: AtomicBool,
    pong_notify: Notify,
    receive_task: Mutex<Option<JoinHandle<()>>>,
}

/// An async client whose receive loop emits decoded server events onto a
/// bounded channel. `disconnect` is idempotent and cancels the receive task.
#[derive(Clone)]
pub struct LiveTranslateClient {
    inner: Arc<Inner>,
    endpoint: LiveTranslateEndpoint,
    api_key: String,
    source_language: SourceLanguage,
    target_language: TargetLanguage,
    hotwords: BTreeMap<String, String>,
    events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
}

impl LiveTranslateClient {
    pub fn new(
        workspace_id: &str,
        api_key: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        hotwords: BTreeMap<String, String>,
        events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
    ) -> Result<Self, LiveTranslateClientError> {
        let trimmed_key = api_key.trim();
        if trimmed_key.is_empty() {
            return Err(LiveTranslateClientError::MissingAPIKey);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                sink: Mutex::new(None),
                received_session_finished: AtomicBool::new(false),
                pong_notify: Notify::new(),
                receive_task: Mutex::new(None),
            }),
            endpoint: LiveTranslateEndpoint::new(workspace_id)
                .map_err(|_| LiveTranslateClientError::MissingAPIKey)?,
            api_key: trimmed_key.to_string(),
            source_language,
            target_language,
            hotwords,
            events,
        })
    }

    /// Opens the socket, sends `session.update`, and starts the receive loop.
    pub async fn connect(&self) -> Result<(), LiveTranslateClientError> {
        self.disconnect().await;

        let mut request = self
            .endpoint
            .url
            .clone()
            .into_client_request()
            .map_err(|_| LiveTranslateClientError::NotConnected)?;
        let auth = format!("Bearer {}", self.api_key);
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&auth).map_err(|_| LiveTranslateClientError::MissingAPIKey)?,
        );

        let (socket, _response) = connect_async(request)
            .await
            .map_err(|_| LiveTranslateClientError::NotConnected)?;
        let (sink, mut stream) = socket.split();
        *self.inner.sink.lock().await = Some(sink);
        self.inner
            .received_session_finished
            .store(false, Ordering::SeqCst);

        let update = LiveTranslateRequestEncoder::session_update(
            self.source_language,
            self.target_language,
            &self.hotwords,
            None,
        )
        .expect("session.update encoding cannot fail");
        self.send_text(update.to_string()).await?;

        let inner = self.inner.clone();
        let events = self.events.clone();
        let task = tokio::spawn(async move {
            loop {
                let message = stream.next().await;
                let Some(message) = message else { break };
                match message {
                    Ok(Message::Text(text)) => match LiveTranslateServerEvent::decode(&text) {
                        Ok(event) => {
                            if event == LiveTranslateServerEvent::SessionFinished {
                                inner
                                    .received_session_finished
                                    .store(true, Ordering::SeqCst);
                            }
                            if events.send(event).is_err() {
                                break;
                            }
                        }
                        Err(error) => {
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
                    },
                    Ok(Message::Binary(data)) => {
                        match LiveTranslateServerEvent::decode(&String::from_utf8_lossy(&data)) {
                            Ok(event) => {
                                if events.send(event).is_err() {
                                    break;
                                }
                            }
                            Err(error) => {
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
                        }
                    }
                    Ok(Message::Pong(_)) => inner.pong_notify.notify_waiters(),
                    Ok(Message::Ping(payload)) => {
                        // tokio-tungstenite auto-responds to pings by default.
                        let _ = payload;
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Frame(_)) => {}
                    Err(_) => {
                        if events
                            .send(LiveTranslateServerEvent::Error {
                                code: "transport_error".into(),
                                message: "The live translation connection closed.".into(),
                            })
                            .is_err()
                        {
                            break;
                        }
                        return;
                    }
                }
            }
        });
        *self.inner.receive_task.lock().await = Some(task);
        Ok(())
    }

    pub async fn send_audio(&self, pcm_data: &[u8]) -> Result<(), LiveTranslateClientError> {
        if pcm_data.is_empty() {
            return Ok(());
        }
        let message = LiveTranslateRequestEncoder::audio_append(pcm_data, None)
            .expect("audio append encoding cannot fail");
        self.send_text(message.to_string()).await
    }

    /// Sends a WebSocket ping and waits for the matching pong (4s timeout).
    pub async fn ping(&self, timeout: Duration) -> Result<(), LiveTranslateClientError> {
        {
            let mut sink = self.inner.sink.lock().await;
            let Some(sink) = sink.as_mut() else {
                return Err(LiveTranslateClientError::NotConnected);
            };
            sink.send(Message::Ping(tokio_tungstenite::tungstenite::Bytes::new()))
                .await
                .map_err(|_| LiveTranslateClientError::NotConnected)?;
        }
        tokio::time::timeout(timeout, self.inner.pong_notify.notified())
            .await
            .map_err(|_| LiveTranslateClientError::HealthCheckTimedOut)?;
        Ok(())
    }

    /// Sends `session.finish`, waits briefly for `session.finished`, then
    /// disconnects.
    pub async fn finish(&self, timeout: Duration) {
        let Some(_sink) = self.inner.sink.lock().await.as_ref() else {
            return;
        };
        if let Ok(message) = LiveTranslateRequestEncoder::finish(None) {
            let _ = self.send_text(message.to_string()).await;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        while !self.inner.received_session_finished.load(Ordering::SeqCst)
            && tokio::time::Instant::now() < deadline
        {
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
        self.inner
            .received_session_finished
            .store(false, Ordering::SeqCst);
    }

    async fn send_text(&self, text: String) -> Result<(), LiveTranslateClientError> {
        let mut sink = self.inner.sink.lock().await;
        let Some(sink) = sink.as_mut() else {
            return Err(LiveTranslateClientError::NotConnected);
        };
        sink.send(Message::Text(text.into()))
            .await
            .map_err(|_| LiveTranslateClientError::NotConnected)
    }
}
