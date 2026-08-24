//! Live-translate WebSocket client (`qwen3.5-livetranslate-flash-realtime`).

use crate::clients::provider_events::ProviderEventSender;
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
use tokio::sync::{watch, Mutex, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const SETUP_TIMEOUT: Duration = Duration::from_secs(5);
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TIMEOUT: Duration = Duration::from_millis(250);
const GENERIC_TRANSPORT_ERROR: &str = "The live translation connection closed.";
const GENERIC_PROTOCOL_ERROR: &str = "The live translation service returned invalid data.";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LiveTranslateClientError {
    #[error("Add an Alibaba Cloud Model Studio API key in Settings.")]
    MissingAPIKey,
    #[error("The live translation session is not connected.")]
    NotConnected,
    #[error("The live translation connection stopped responding.")]
    HealthCheckTimedOut,
    #[error("The live translation connection could not be established in time.")]
    ConnectionTimedOut,
    #[error("The live translation session setup timed out.")]
    SessionSetupTimedOut,
    #[error("The live translation session setup was rejected.")]
    SessionSetupRejected,
    #[error("The live translation transport failed.")]
    TransportFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupState {
    Awaiting,
    Ready,
    Rejected,
}

type Sink = futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type Stream = futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

struct Inner {
    sink: Mutex<Option<Sink>>,
    received_session_finished: AtomicBool,
    pong_notify: Notify,
    receive_task: Mutex<Option<JoinHandle<()>>>,
}

/// An async client whose receive loop emits decoded server events onto the
/// session event channel. `disconnect` is idempotent and cancels the task.
#[derive(Clone)]
pub struct LiveTranslateClient {
    inner: Arc<Inner>,
    endpoint: LiveTranslateEndpoint,
    api_key: String,
    source_language: SourceLanguage,
    target_language: TargetLanguage,
    hotwords: BTreeMap<String, String>,
    events: ProviderEventSender,
}

impl LiveTranslateClient {
    pub fn new(
        api_key: &str,
        source_language: SourceLanguage,
        target_language: TargetLanguage,
        hotwords: BTreeMap<String, String>,
        events: ProviderEventSender,
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
            endpoint: LiveTranslateEndpoint::new()
                .map_err(|_| LiveTranslateClientError::MissingAPIKey)?,
            api_key: trimmed_key.to_string(),
            source_language,
            target_language,
            hotwords,
            events,
        })
    }

    /// Opens the socket, sends `session.update`, and waits for the matching
    /// server readiness acknowledgement.
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

        let (socket, _response) = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(request))
            .await
            .map_err(|_| LiveTranslateClientError::ConnectionTimedOut)?
            .map_err(|_| LiveTranslateClientError::TransportFailure)?;
        let (sink, stream) = socket.split();
        *self.inner.sink.lock().await = Some(sink);
        self.inner
            .received_session_finished
            .store(false, Ordering::SeqCst);

        let (setup_tx, setup_rx) = watch::channel(SetupState::Awaiting);
        let task = tokio::spawn(receive_loop(
            stream,
            Arc::clone(&self.inner),
            self.events.clone(),
            setup_tx,
        ));
        *self.inner.receive_task.lock().await = Some(task);

        let update = LiveTranslateRequestEncoder::session_update(
            self.source_language,
            self.target_language,
            &self.hotwords,
            None,
        )
        .expect("session.update encoding cannot fail");
        let setup = async {
            self.send_text(update.to_string()).await?;
            wait_for_setup(setup_rx).await
        };
        let result = match tokio::time::timeout(SETUP_TIMEOUT, setup).await {
            Ok(result) => result,
            Err(_) => Err(LiveTranslateClientError::SessionSetupTimedOut),
        };
        if result.is_err() {
            self.disconnect().await;
        }
        result
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
        let operation = async {
            let pong = self.inner.pong_notify.notified();
            tokio::pin!(pong);
            pong.as_mut().enable();
            {
                let mut sink = self.inner.sink.lock().await;
                let Some(sink) = sink.as_mut() else {
                    return Err(LiveTranslateClientError::NotConnected);
                };
                sink.send(Message::Ping(tokio_tungstenite::tungstenite::Bytes::new()))
                    .await
                    .map_err(|_| LiveTranslateClientError::TransportFailure)?;
            }
            pong.await;
            Ok(())
        };
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|_| LiveTranslateClientError::HealthCheckTimedOut)?
    }

    /// Sends `session.finish`, waits briefly for `session.finished`, then
    /// disconnects.
    pub async fn finish(&self, timeout: Duration) {
        if self.inner.sink.lock().await.is_none() {
            return;
        }
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
        let sink = self.inner.sink.lock().await.take();
        if let Some(mut sink) = sink {
            let _ = tokio::time::timeout(CLOSE_TIMEOUT, sink.close()).await;
        }
        self.inner
            .received_session_finished
            .store(false, Ordering::SeqCst);
    }

    async fn send_text(&self, text: String) -> Result<(), LiveTranslateClientError> {
        let operation = async {
            let mut sink = self.inner.sink.lock().await;
            let Some(sink) = sink.as_mut() else {
                return Err(LiveTranslateClientError::NotConnected);
            };
            sink.send(Message::Text(text.into()))
                .await
                .map_err(|_| LiveTranslateClientError::TransportFailure)
        };
        tokio::time::timeout(SEND_TIMEOUT, operation)
            .await
            .map_err(|_| LiveTranslateClientError::TransportFailure)?
    }
}

async fn wait_for_setup(
    mut setup: watch::Receiver<SetupState>,
) -> Result<(), LiveTranslateClientError> {
    loop {
        match *setup.borrow() {
            SetupState::Ready => return Ok(()),
            SetupState::Rejected => return Err(LiveTranslateClientError::SessionSetupRejected),
            SetupState::Awaiting => {}
        }
        if setup.changed().await.is_err() {
            return Err(LiveTranslateClientError::SessionSetupRejected);
        }
    }
}

async fn receive_loop(
    mut stream: Stream,
    inner: Arc<Inner>,
    events: ProviderEventSender,
    setup: watch::Sender<SetupState>,
) {
    while let Some(message) = stream.next().await {
        let decoded = match message {
            Ok(Message::Text(text)) => LiveTranslateServerEvent::decode(&text),
            Ok(Message::Binary(data)) => {
                LiveTranslateServerEvent::decode(&String::from_utf8_lossy(&data))
            }
            Ok(Message::Pong(_)) => {
                inner.pong_notify.notify_waiters();
                continue;
            }
            Ok(Message::Ping(_)) | Ok(Message::Frame(_)) => continue,
            Ok(Message::Close(_)) | Err(_) => {
                if should_report_transport_end(
                    inner.received_session_finished.load(Ordering::SeqCst),
                ) {
                    fail_receive_loop(&events, &setup, "transport_error", GENERIC_TRANSPORT_ERROR);
                }
                return;
            }
        };

        let event = match decoded {
            Ok(event) => event,
            Err(_) => {
                fail_receive_loop(
                    &events,
                    &setup,
                    "live_translate_protocol_error",
                    GENERIC_PROTOCOL_ERROR,
                );
                return;
            }
        };
        if !emit_server_event(&inner, &events, &setup, event) {
            return;
        }
    }

    if should_report_transport_end(inner.received_session_finished.load(Ordering::SeqCst)) {
        fail_receive_loop(&events, &setup, "transport_error", GENERIC_TRANSPORT_ERROR);
    }
}

fn emit_server_event(
    inner: &Inner,
    events: &ProviderEventSender,
    setup: &watch::Sender<SetupState>,
    event: LiveTranslateServerEvent,
) -> bool {
    if event == LiveTranslateServerEvent::SessionFinished {
        inner
            .received_session_finished
            .store(true, Ordering::SeqCst);
    }
    if *setup.borrow() == SetupState::Awaiting {
        if matches!(event, LiveTranslateServerEvent::Error { .. }) {
            let _ = setup.send(SetupState::Rejected);
            return false;
        }
        if event == LiveTranslateServerEvent::SessionUpdated {
            // Publishing the acknowledgement also proves the bounded session
            // receiver is still alive before setup is marked ready.
            if events.send(event).is_err() {
                let _ = setup.send(SetupState::Rejected);
                return false;
            }
            return setup.send(SetupState::Ready).is_ok();
        }
    }

    // The stream protocol has no "translation started" message: the source
    // final is the reliable boundary at which translation begins.
    if matches!(event, LiveTranslateServerEvent::SourceFinal { .. })
        && events
            .send(LiveTranslateServerEvent::TranslationStarted)
            .is_err()
    {
        return false;
    }
    if events.send(event).is_err() {
        if *setup.borrow() == SetupState::Awaiting {
            let _ = setup.send(SetupState::Rejected);
        }
        return false;
    }
    true
}

fn fail_receive_loop(
    events: &ProviderEventSender,
    setup: &watch::Sender<SetupState>,
    code: &str,
    message: &str,
) {
    if *setup.borrow() == SetupState::Awaiting {
        let _ = setup.send(SetupState::Rejected);
    } else {
        let _ = events.send(LiveTranslateServerEvent::Error {
            code: code.into(),
            message: message.into(),
        });
    }
}

fn should_report_transport_end(received_session_finished: bool) -> bool {
    !received_session_finished
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::provider_events::provider_event_channel;

    fn test_inner() -> Inner {
        Inner {
            sink: Mutex::new(None),
            received_session_finished: AtomicBool::new(false),
            pong_notify: Notify::new(),
            receive_task: Mutex::new(None),
        }
    }

    #[tokio::test]
    async fn setup_waits_for_server_updated_acknowledgement() {
        let inner = test_inner();
        let (events, mut event_rx) = provider_event_channel();
        let (tx, rx) = watch::channel(SetupState::Awaiting);
        let mut wait = Box::pin(wait_for_setup(rx));
        assert!(tokio::time::timeout(Duration::from_millis(5), &mut wait)
            .await
            .is_err());

        assert!(emit_server_event(
            &inner,
            &events,
            &tx,
            LiveTranslateServerEvent::SessionUpdated,
        ));
        assert_eq!(wait.await, Ok(()));
        assert_eq!(
            event_rx.try_recv(),
            Ok(LiveTranslateServerEvent::SessionUpdated)
        );
    }

    #[tokio::test]
    async fn rejected_setup_fails_without_waiting_for_health_check() {
        let inner = test_inner();
        let (events, mut event_rx) = provider_event_channel();
        let (tx, rx) = watch::channel(SetupState::Awaiting);
        assert!(!emit_server_event(
            &inner,
            &events,
            &tx,
            LiveTranslateServerEvent::Error {
                code: "provider-private-code".into(),
                message: "provider-private-message".into(),
            },
        ));

        assert_eq!(
            wait_for_setup(rx).await,
            Err(LiveTranslateClientError::SessionSetupRejected)
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn setup_is_rejected_when_the_bounded_session_receiver_is_gone() {
        let inner = test_inner();
        let (events, event_rx) = provider_event_channel();
        drop(event_rx);
        let (tx, rx) = watch::channel(SetupState::Awaiting);

        assert!(!emit_server_event(
            &inner,
            &events,
            &tx,
            LiveTranslateServerEvent::SessionUpdated,
        ));
        assert_eq!(
            wait_for_setup(rx).await,
            Err(LiveTranslateClientError::SessionSetupRejected)
        );
    }

    #[test]
    fn only_a_confirmed_session_finish_makes_socket_end_expected() {
        assert!(should_report_transport_end(false));
        assert!(!should_report_transport_end(true));
    }
}
