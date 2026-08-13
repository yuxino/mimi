//! Subtitle assembly state machine, ported 1:1 from
//! `Sources/MimiCore/SubtitleReducer.swift`.

use crate::core::models::{SubtitleEvent, SubtitleLine, SubtitlePair, SubtitleSnapshot};

pub struct SubtitleReducer {
    pub snapshot: SubtitleSnapshot,
    max_history_count: usize,
    pending_final_sources: Vec<String>,
}

impl SubtitleReducer {
    pub fn new(max_history_count: usize) -> Self {
        Self {
            snapshot: SubtitleSnapshot::empty(),
            max_history_count,
            pending_final_sources: Vec::new(),
        }
    }

    pub fn apply(&mut self, event: SubtitleEvent) {
        match event {
            SubtitleEvent::SourceDraft(text) => {
                self.snapshot.source = SubtitleLine::new(trim(&text), false);
            }
            SubtitleEvent::SourceFinal(text) => {
                let source = trim(&text);
                self.snapshot.source = SubtitleLine::new(source.clone(), true);
                if !source.is_empty() {
                    self.pending_final_sources.push(source);
                }
            }
            SubtitleEvent::TranslationDraft(text) => {
                let trimmed = trim(&text);
                // A blank draft must not overwrite an already-confirmed final.
                if trimmed.is_empty() && self.snapshot.translation.is_final {
                    return;
                }
                self.snapshot.translation = SubtitleLine::new(trimmed, false);
            }
            SubtitleEvent::TranslationFinal(text) => {
                let translation = trim(&text);
                self.snapshot.translation = SubtitleLine::new(translation.clone(), true);
                let source = if self.pending_final_sources.is_empty() {
                    self.snapshot.source.text.clone()
                } else {
                    self.pending_final_sources.remove(0)
                };
                self.append_history_if_possible(source, translation);
            }
            SubtitleEvent::RevokeLastConfirmed => {
                if !self.snapshot.history.is_empty() {
                    self.snapshot.history.pop();
                }
            }
            SubtitleEvent::Clear => {
                self.snapshot = SubtitleSnapshot::empty();
                self.pending_final_sources.clear();
            }
        }
    }

    fn append_history_if_possible(&mut self, source: String, translation: String) {
        if source.is_empty() || translation.is_empty() {
            return;
        }
        let pair = SubtitlePair::new(source, translation, now_epoch_ms());
        if self.snapshot.history.last() == Some(&pair) {
            return;
        }
        self.snapshot.history.push(pair);
        if self.snapshot.history.len() > self.max_history_count {
            let overflow = self.snapshot.history.len() - self.max_history_count;
            self.snapshot.history.drain(0..overflow);
        }
    }
}

impl Default for SubtitleReducer {
    fn default() -> Self {
        Self::new(20)
    }
}

/// Trims Unicode whitespace and newlines (same semantics as Swift's
/// `.trimmingCharacters(in: .whitespacesAndNewlines)`).
pub(crate) fn trim(text: &str) -> String {
    text.trim().to_string()
}

/// Current wall-clock time as epoch milliseconds.
pub(crate) fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{SubtitleLine, SubtitlePair};

    #[test]
    fn subtitle_reducer_starts_empty() {
        let reducer = SubtitleReducer::default();
        assert_eq!(reducer.snapshot, SubtitleSnapshot::empty());
    }

    #[test]
    fn drafts_remain_visibly_unconfirmed() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceDraft("Hello wor".into()));
        reducer.apply(SubtitleEvent::TranslationDraft("你好，世".into()));

        assert_eq!(reducer.snapshot.source, SubtitleLine::new("Hello wor", false));
        assert_eq!(
            reducer.snapshot.translation,
            SubtitleLine::new("你好，世", false)
        );
    }

    #[test]
    fn final_translation_creates_a_history_pair() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("Hello world.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好，世界。".into()));

        assert_eq!(
            reducer.snapshot.source,
            SubtitleLine::new("Hello world.", true)
        );
        assert_eq!(
            reducer.snapshot.translation,
            SubtitleLine::new("你好，世界。", true)
        );
        assert_eq!(
            reducer.snapshot.history,
            vec![SubtitlePair::new("Hello world.".into(), "你好，世界。".into(), 0)]
        );
    }

    #[test]
    fn a_new_draft_keeps_confirmed_history_available() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("Hello.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好。".into()));
        reducer.apply(SubtitleEvent::SourceDraft("How are".into()));
        reducer.apply(SubtitleEvent::TranslationDraft("你最近".into()));

        assert_eq!(
            reducer.snapshot.history,
            vec![SubtitlePair::new("Hello.".into(), "你好。".into(), 0)]
        );
        assert_eq!(
            reducer.snapshot.translation,
            SubtitleLine::new("你最近", false)
        );
    }

    #[test]
    fn a_plus_final_replaces_its_preview_and_alone_enters_history() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceDraft("今日は晴れです".into()));
        reducer.apply(SubtitleEvent::TranslationDraft("今天晴天".into()));
        assert!(reducer.snapshot.history.is_empty());

        reducer.apply(SubtitleEvent::SourceFinal("今日は晴れです。".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("今天天气很好。".into()));

        assert_eq!(
            reducer.snapshot.history,
            vec![SubtitlePair::new("今日は晴れです。".into(), "今天天气很好。".into(), 0)]
        );
        assert_eq!(
            reducer.snapshot.translation,
            SubtitleLine::new("今天天气很好。", true)
        );
    }

    #[test]
    fn a_delayed_final_translation_stays_paired_with_its_original_source() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("First sentence.".into()));
        reducer.apply(SubtitleEvent::SourceDraft("Second sentence".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("第一句。".into()));

        assert_eq!(
            reducer.snapshot.history,
            vec![SubtitlePair::new("First sentence.".into(), "第一句。".into(), 0)]
        );
        assert_eq!(
            reducer.snapshot.source,
            SubtitleLine::new("Second sentence", false)
        );
    }

    #[test]
    fn duplicate_finals_do_not_duplicate_history() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("Hello.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好。".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好。".into()));
        assert_eq!(reducer.snapshot.history.len(), 1);
    }

    #[test]
    fn revoking_removes_only_the_last_confirmed_pair() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("First.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("第一句。".into()));
        reducer.apply(SubtitleEvent::SourceFinal("Second.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("第二句。".into()));

        reducer.apply(SubtitleEvent::RevokeLastConfirmed);

        assert_eq!(reducer.snapshot.history.len(), 1);
        assert_eq!(reducer.snapshot.history[0].translation, "第一句。");
    }

    #[test]
    fn revoking_an_empty_history_is_a_no_op() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::RevokeLastConfirmed);
        assert!(reducer.snapshot.history.is_empty());
    }

    #[test]
    fn identical_source_and_translation_remain_in_history() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("嗯啊".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("嗯啊".into()));

        assert_eq!(
            reducer.snapshot.history,
            vec![SubtitlePair::new("嗯啊".into(), "嗯啊".into(), 0)]
        );
    }

    #[test]
    fn history_is_bounded() {
        let mut reducer = SubtitleReducer::new(2);
        for index in 1..=3 {
            reducer.apply(SubtitleEvent::SourceFinal(format!("source {index}")));
            reducer.apply(SubtitleEvent::TranslationFinal(format!("translation {index}")));
        }

        assert_eq!(
            reducer.snapshot.history,
            vec![
                SubtitlePair::new("source 2".into(), "translation 2".into(), 0),
                SubtitlePair::new("source 3".into(), "translation 3".into(), 0),
            ]
        );
    }

    #[test]
    fn clear_resets_all_subtitle_state() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("Hello.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好。".into()));
        reducer.apply(SubtitleEvent::Clear);

        assert_eq!(reducer.snapshot, SubtitleSnapshot::empty());
    }

    #[test]
    fn blank_draft_does_not_overwrite_confirmed_final() {
        let mut reducer = SubtitleReducer::default();
        reducer.apply(SubtitleEvent::SourceFinal("Hello.".into()));
        reducer.apply(SubtitleEvent::TranslationFinal("你好。".into()));
        reducer.apply(SubtitleEvent::TranslationDraft("   ".into()));

        assert_eq!(
            reducer.snapshot.translation,
            SubtitleLine::new("你好。", true)
        );
    }
}
