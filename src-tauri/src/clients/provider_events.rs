//! Bounded provider-to-session event transport.
//!
//! Drafts are latest-value snapshots; lifecycle and confirmed subtitle events
//! use a reliable bounded lane. If the reliable lane fills, a separate
//! one-shot control signal forces generation recovery instead of silently
//! dropping a final or growing an unbounded queue.

use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

const DEFAULT_RELIABLE_CAPACITY: usize = 64;
const OVERFLOW_CODE: &str = "provider_event_backlog_overflow";
const OVERFLOW_MESSAGE: &str = "Translation event processing fell behind. mimi is reconnecting.";

#[derive(Debug, Clone)]
struct SequencedEvent {
    sequence: u64,
    event: LiveTranslateServerEvent,
}

struct SenderInner {
    reliable: mpsc::Sender<SequencedEvent>,
    source_draft: watch::Sender<Option<SequencedEvent>>,
    translation_draft: watch::Sender<Option<SequencedEvent>>,
    overflow: watch::Sender<bool>,
    dispatch: Mutex<()>,
    sequence: AtomicU64,
    failed: AtomicBool,
}

#[derive(Clone)]
pub struct ProviderEventSender {
    inner: Arc<SenderInner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProviderEventSendError {
    #[error("The session event receiver is closed.")]
    Closed,
    #[error("The reliable session event queue is full.")]
    Backpressure,
}

pub struct ProviderEventReceiver {
    reliable: mpsc::Receiver<SequencedEvent>,
    source_draft: watch::Receiver<Option<SequencedEvent>>,
    translation_draft: watch::Receiver<Option<SequencedEvent>>,
    overflow: watch::Receiver<bool>,
    reliable_open: bool,
    source_open: bool,
    translation_open: bool,
    overflow_open: bool,
    overflow_delivered: bool,
    source_pending: Option<SequencedEvent>,
    translation_pending: Option<SequencedEvent>,
    last_delivered_sequence: u64,
}

pub fn provider_event_channel() -> (ProviderEventSender, ProviderEventReceiver) {
    provider_event_channel_with_capacity(DEFAULT_RELIABLE_CAPACITY)
}

fn provider_event_channel_with_capacity(
    reliable_capacity: usize,
) -> (ProviderEventSender, ProviderEventReceiver) {
    assert!(reliable_capacity > 0);
    let (reliable_tx, reliable_rx) = mpsc::channel(reliable_capacity);
    let (source_tx, source_rx) = watch::channel(None);
    let (translation_tx, translation_rx) = watch::channel(None);
    let (overflow_tx, overflow_rx) = watch::channel(false);
    (
        ProviderEventSender {
            inner: Arc::new(SenderInner {
                reliable: reliable_tx,
                source_draft: source_tx,
                translation_draft: translation_tx,
                overflow: overflow_tx,
                dispatch: Mutex::new(()),
                sequence: AtomicU64::new(0),
                failed: AtomicBool::new(false),
            }),
        },
        ProviderEventReceiver {
            reliable: reliable_rx,
            source_draft: source_rx,
            translation_draft: translation_rx,
            overflow: overflow_rx,
            reliable_open: true,
            source_open: true,
            translation_open: true,
            overflow_open: true,
            overflow_delivered: false,
            source_pending: None,
            translation_pending: None,
            last_delivered_sequence: 0,
        },
    )
}

impl ProviderEventSender {
    pub fn send(&self, event: LiveTranslateServerEvent) -> Result<(), ProviderEventSendError> {
        // Sequence allocation and lane publication are one tiny synchronous
        // critical section. Provider timers and socket tasks can publish from
        // different Tokio workers; without serialization, sequence 2 could
        // become visible before sequence 1 and let an old draft regress the
        // UI after a newer event.
        let _dispatch = self.inner.dispatch.lock().unwrap();
        if self.inner.failed.load(Ordering::SeqCst) {
            return Err(ProviderEventSendError::Backpressure);
        }
        if matches!(event, LiveTranslateServerEvent::Ignored { .. }) {
            return Ok(());
        }
        let event = SequencedEvent {
            sequence: self
                .inner
                .sequence
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1),
            event,
        };
        match &event.event {
            LiveTranslateServerEvent::SourceDraft { .. } => {
                if self.inner.source_draft.receiver_count() == 0 {
                    return Err(ProviderEventSendError::Closed);
                }
                self.inner.source_draft.send_replace(Some(event));
                Ok(())
            }
            LiveTranslateServerEvent::TranslationDraft(_) => {
                if self.inner.translation_draft.receiver_count() == 0 {
                    return Err(ProviderEventSendError::Closed);
                }
                self.inner.translation_draft.send_replace(Some(event));
                Ok(())
            }
            _ => match self.inner.reliable.try_send(event) {
                Ok(()) => Ok(()),
                Err(mpsc::error::TrySendError::Closed(_)) => Err(ProviderEventSendError::Closed),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    if !self.inner.failed.swap(true, Ordering::SeqCst) {
                        self.inner.overflow.send_replace(true);
                    }
                    Err(ProviderEventSendError::Backpressure)
                }
            },
        }
    }
}

impl ProviderEventReceiver {
    #[cfg(test)]
    pub fn try_recv(&mut self) -> Result<LiveTranslateServerEvent, mpsc::error::TryRecvError> {
        if !self.overflow_delivered && *self.overflow.borrow() {
            self.overflow_delivered = true;
            return Ok(overflow_event());
        }
        if self.reliable_open {
            match self.reliable.try_recv() {
                Ok(event) => {
                    self.last_delivered_sequence = self.last_delivered_sequence.max(event.sequence);
                    return Ok(event.event);
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.reliable_open = false;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
            }
        }
        self.refresh_draft_pending();
        if let Some(event) = self.take_next_draft() {
            return Ok(event);
        }
        if self.overflow_open && self.overflow.has_changed().is_err() {
            self.overflow_open = false;
        }
        if !self.reliable_open && !self.source_open && !self.translation_open && !self.overflow_open
        {
            Err(mpsc::error::TryRecvError::Disconnected)
        } else {
            Err(mpsc::error::TryRecvError::Empty)
        }
    }

    pub async fn recv(&mut self) -> Option<LiveTranslateServerEvent> {
        loop {
            if !self.overflow_delivered && *self.overflow.borrow() {
                self.overflow_delivered = true;
                return Some(overflow_event());
            }
            if self.reliable_open {
                match self.reliable.try_recv() {
                    Ok(event) => {
                        self.last_delivered_sequence =
                            self.last_delivered_sequence.max(event.sequence);
                        return Some(event.event);
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.reliable_open = false;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {}
                }
            }
            self.refresh_draft_pending();
            if let Some(event) = self.take_next_draft() {
                return Some(event);
            }
            if !self.reliable_open
                && !self.source_open
                && !self.translation_open
                && (!self.overflow_open || self.overflow_delivered)
            {
                return None;
            }

            tokio::select! {
                biased;

                result = self.overflow.changed(), if self.overflow_open && !self.overflow_delivered => {
                    match result {
                        Ok(()) if *self.overflow.borrow_and_update() => {
                            self.overflow_delivered = true;
                            return Some(overflow_event());
                        }
                        Ok(()) => {}
                        Err(_) => self.overflow_open = false,
                    }
                }
                event = self.reliable.recv(), if self.reliable_open => {
                    match event {
                        Some(event) => {
                            self.last_delivered_sequence =
                                self.last_delivered_sequence.max(event.sequence);
                            return Some(event.event);
                        }
                        None => self.reliable_open = false,
                    }
                }
                result = self.source_draft.changed(), if self.source_open => {
                    match result {
                        Ok(()) => {
                            self.source_pending = self.source_draft.borrow().clone();
                        }
                        Err(_) => self.source_open = false,
                    }
                }
                result = self.translation_draft.changed(), if self.translation_open => {
                    match result {
                        Ok(()) => {
                            self.translation_pending = self.translation_draft.borrow().clone();
                        }
                        Err(_) => self.translation_open = false,
                    }
                }
            }
        }
    }

    fn refresh_draft_pending(&mut self) {
        refresh_pending(
            &mut self.source_draft,
            &mut self.source_open,
            &mut self.source_pending,
        );
        refresh_pending(
            &mut self.translation_draft,
            &mut self.translation_open,
            &mut self.translation_pending,
        );
    }

    fn take_next_draft(&mut self) -> Option<LiveTranslateServerEvent> {
        self.source_pending
            .take_if(|event| event.sequence <= self.last_delivered_sequence);
        self.translation_pending
            .take_if(|event| event.sequence <= self.last_delivered_sequence);

        let take_source = match (&self.source_pending, &self.translation_pending) {
            (Some(source), Some(translation)) => source.sequence < translation.sequence,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return None,
        };
        let event = if take_source {
            self.source_pending.take().unwrap()
        } else {
            self.translation_pending.take().unwrap()
        };
        self.last_delivered_sequence = event.sequence;
        Some(event.event)
    }
}

fn refresh_pending(
    receiver: &mut watch::Receiver<Option<SequencedEvent>>,
    is_open: &mut bool,
    pending: &mut Option<SequencedEvent>,
) {
    if !*is_open {
        return;
    }
    match receiver.has_changed() {
        Ok(true) => *pending = receiver.borrow_and_update().clone(),
        Ok(false) => {}
        Err(_) => {
            *is_open = false;
        }
    }
}

fn overflow_event() -> LiveTranslateServerEvent {
    LiveTranslateServerEvent::Error {
        code: OVERFLOW_CODE.into(),
        message: OVERFLOW_MESSAGE.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn drafts_are_latest_only() {
        let (sender, mut receiver) = provider_event_channel();
        sender
            .send(LiveTranslateServerEvent::SourceDraft {
                text: "first".into(),
                language: Some("en".into()),
            })
            .unwrap();
        sender
            .send(LiveTranslateServerEvent::SourceDraft {
                text: "latest".into(),
                language: Some("en".into()),
            })
            .unwrap();

        assert_eq!(
            receiver.recv().await,
            Some(LiveTranslateServerEvent::SourceDraft {
                text: "latest".into(),
                language: Some("en".into()),
            })
        );
    }

    #[tokio::test]
    async fn interleaved_draft_lanes_are_delivered_in_publish_order() {
        let (sender, mut receiver) = provider_event_channel();
        let translation = LiveTranslateServerEvent::TranslationDraft("older preview".into());
        let source = LiveTranslateServerEvent::SourceDraft {
            text: "new source".into(),
            language: Some("en".into()),
        };
        sender.send(translation.clone()).unwrap();
        sender.send(source.clone()).unwrap();

        assert_eq!(receiver.recv().await, Some(translation));
        assert_eq!(receiver.recv().await, Some(source));
    }

    #[tokio::test]
    async fn reliable_events_take_priority_and_suppress_older_drafts() {
        let (sender, mut receiver) = provider_event_channel();
        sender
            .send(LiveTranslateServerEvent::SourceDraft {
                text: "stale".into(),
                language: Some("en".into()),
            })
            .unwrap();
        let final_pair = LiveTranslateServerEvent::SubtitleFinalPair {
            source: "confirmed".into(),
            language: Some("en".into()),
            translation: "已确认".into(),
        };
        sender.send(final_pair.clone()).unwrap();

        assert_eq!(receiver.recv().await, Some(final_pair));
        let no_stale =
            tokio::time::timeout(std::time::Duration::from_millis(20), receiver.recv()).await;
        assert!(no_stale.is_err());
    }

    #[tokio::test]
    async fn reliable_overflow_emits_one_recoverable_control_event() {
        let (sender, mut receiver) = provider_event_channel_with_capacity(1);
        sender
            .send(LiveTranslateServerEvent::TranslationStarted)
            .unwrap();
        assert_eq!(
            sender.send(LiveTranslateServerEvent::SessionFinished),
            Err(ProviderEventSendError::Backpressure)
        );

        assert_eq!(receiver.recv().await, Some(overflow_event()));
        assert_eq!(
            sender.send(LiveTranslateServerEvent::SessionFinished),
            Err(ProviderEventSendError::Backpressure)
        );
        assert_eq!(
            receiver.recv().await,
            Some(LiveTranslateServerEvent::TranslationStarted)
        );
    }

    #[test]
    fn overflow_diagnostic_is_content_free() {
        let LiveTranslateServerEvent::Error { code, message } = overflow_event() else {
            unreachable!();
        };
        assert_eq!(code, OVERFLOW_CODE);
        assert!(!message.contains("subtitle"));
        assert!(!message.contains("translation text"));
    }
}
