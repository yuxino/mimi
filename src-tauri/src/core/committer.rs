//! Splits a cumulative ASR draft into complete sentences so subtitles are never
//! committed mid-sentence, and so a later server final cannot duplicate text
//! that was already committed.

const SENTENCE_DELIMITERS: [char; 7] = ['。', '！', '？', '.', '!', '?', '\n'];

/// The outcome of handling a server-final sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishOutcome {
    /// The final is already covered by committed text; nothing new to show.
    None,
    /// New authoritative content was appended to the committed boundary.
    Appended(String),
    /// The final supersedes the last local (provisional) commit; the client
    /// should revoke that history entry and commit the full final once.
    Replaced(String),
}

/// Whether a server final structurally covers a locally committed chunk —
/// the same rule used to supersede provisional commits and to coalesce the
/// pending final-translation queue.
pub fn final_covers_chunk(final_text: &str, chunk: &str) -> bool {
    final_text != chunk
        && final_text.chars().count() >= 2
        && chunk.chars().count() >= 2
        && (final_text.starts_with(chunk) || final_text.contains(chunk))
}

pub struct ASRDraftCommitter {
    long_incomplete_commit_threshold: usize,
    latest_draft: String,
    committed_text: String,
    last_committed_chunk: String,
    last_commit_was_provisional: bool,
}

impl ASRDraftCommitter {
    pub fn new(long_incomplete_commit_threshold: usize) -> Self {
        Self {
            long_incomplete_commit_threshold: long_incomplete_commit_threshold.max(1),
            latest_draft: String::new(),
            committed_text: String::new(),
            last_committed_chunk: String::new(),
            last_commit_was_provisional: false,
        }
    }

    pub fn has_pending_text(&self) -> bool {
        Self::is_meaningful(&self.pending_text_in(&self.latest_draft))
    }

    /// Updates the latest draft and returns the pending (uncommitted) text.
    pub fn update_draft(&mut self, text: &str) -> String {
        self.latest_draft = text.trim().to_string();
        self.pending_text_in(&self.latest_draft)
    }

    /// Commits every complete sentence in the pending draft and returns the
    /// newly committed text. An incomplete trailing sentence stays pending.
    pub fn commit_complete_sentences(&mut self) -> Option<String> {
        let pending = self.pending_text_in(&self.latest_draft);
        if pending.is_empty() {
            return None;
        }
        let (complete, tail) = Self::split_sentences(&pending);
        if !Self::is_meaningful(&complete) {
            return None;
        }
        let tail_len = tail.chars().count();
        self.committed_text = self
            .latest_draft
            .chars()
            .take(self.latest_draft.chars().count().saturating_sub(tail_len))
            .collect();
        self.last_committed_chunk = complete.clone();
        self.last_commit_was_provisional = true;
        Some(complete)
    }

    /// Commits complete sentences; when `commit_long_incomplete` is true and no
    /// complete sentence exists, commits a long pending tail as a single chunk.
    pub fn commit_latest_draft(&mut self, commit_long_incomplete: bool) -> Option<String> {
        let complete = self.commit_complete_sentences();
        if complete.is_some() {
            return complete;
        }
        let pending = self.pending_text_in(&self.latest_draft);
        let long_enough = pending.chars().count() >= self.long_incomplete_commit_threshold;
        if !commit_long_incomplete || !long_enough || !Self::is_meaningful(&pending) {
            return None;
        }
        self.committed_text = self.latest_draft.clone();
        self.last_committed_chunk = pending.clone();
        self.last_commit_was_provisional = true;
        Some(pending)
    }

    /// Handles a server-final sentence. Returns `.none` when the final is
    /// already covered by committed text, `.appended` when the final adds new
    /// content, and `.replaced` when the final supersedes the last local
    /// commit so the provisional history entry can be revoked and committed
    /// exactly once.
    pub fn finish_sentence(&mut self, text: &str) -> FinishOutcome {
        let final_text = text.trim().to_string();
        if !Self::is_meaningful(&final_text) {
            return FinishOutcome::None;
        }

        if !self.committed_text.is_empty()
            && final_text.chars().count() >= 2
            && self.committed_text.contains(&final_text)
        {
            // The final matches text that is already committed (including a
            // locally committed sentence the server just confirmed verbatim).
            self.last_commit_was_provisional = false;
            self.last_committed_chunk.clear();
            return FinishOutcome::None;
        }

        if self.last_commit_was_provisional
            && !self.last_committed_chunk.is_empty()
            && final_text.chars().count() >= 2
        {
            let chunk = self.last_committed_chunk.clone();
            if final_covers_chunk(&final_text, &chunk) {
                if self.committed_text.ends_with(&chunk) {
                    let new_len = self.committed_text.chars().count() - chunk.chars().count();
                    self.committed_text = self.committed_text.chars().take(new_len).collect();
                } else {
                    self.committed_text.clear();
                }
                self.committed_text.push_str(&final_text);
                self.last_commit_was_provisional = false;
                self.last_committed_chunk.clear();
                return FinishOutcome::Replaced(final_text);
            }
        }

        let overlap = Self::suffix_overlap(&self.committed_text, &final_text);
        let new_text: String = final_text.chars().skip(overlap).collect();
        let new_text = new_text.trim();
        if !Self::is_meaningful(new_text) {
            self.last_commit_was_provisional = false;
            self.last_committed_chunk.clear();
            return FinishOutcome::None;
        }

        self.committed_text.push_str(new_text);
        self.last_commit_was_provisional = false;
        self.last_committed_chunk.clear();
        FinishOutcome::Appended(new_text.to_string())
    }

    pub fn reset(&mut self) {
        self.latest_draft.clear();
        self.committed_text.clear();
        self.last_committed_chunk.clear();
        self.last_commit_was_provisional = false;
    }

    fn pending_text_in(&self, text: &str) -> String {
        if self.committed_text.is_empty() {
            return text.to_string();
        }
        if !text.starts_with(&self.committed_text) {
            // Server revised earlier text; show the whole corrected draft and
            // let server finals (not local commits) advance the boundary.
            return text.to_string();
        }
        let suffix: String = text
            .chars()
            .skip(self.committed_text.chars().count())
            .collect();
        suffix.trim().to_string()
    }

    fn split_sentences(text: &str) -> (String, String) {
        let mut complete = String::new();
        let mut current = String::new();
        for character in text.chars() {
            current.push(character);
            if SENTENCE_DELIMITERS.contains(&character) {
                complete.push_str(&current);
                current.clear();
            }
        }
        (complete, current)
    }

    /// Length of the longest suffix of `text` that is also a prefix of
    /// `prefix`.
    fn suffix_overlap(text: &str, prefix: &str) -> usize {
        let text_chars: Vec<char> = text.chars().collect();
        let prefix_chars: Vec<char> = prefix.chars().collect();
        if text_chars.is_empty() || prefix_chars.is_empty() {
            return 0;
        }
        let maximum = text_chars.len().min(prefix_chars.len());
        for length in (1..=maximum).rev() {
            let text_suffix = &text_chars[text_chars.len() - length..];
            let prefix_head = &prefix_chars[..length];
            if text_suffix == prefix_head {
                return length;
            }
        }
        0
    }

    /// Text is meaningful when it contains a non-whitespace, non-punctuation
    /// character.
    fn is_meaningful(text: &str) -> bool {
        text.chars()
            .any(|c| !c.is_whitespace() && !is_punctuation(c))
    }
}

fn is_punctuation(c: char) -> bool {
    // ASCII classification alone misses CJK punctuation, so include the
    // relevant Unicode punctuation and symbol categories.
    !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control()
        || c.is_ascii_punctuation()
        || matches!(
            c,
            '。' | '、'
                | '！'
                | '？'
                | '「'
                | '」'
                | '『'
                | '』'
                | '（'
                | '）'
                | '・'
                | '…'
                | '—'
                | '–'
        )
}

impl Default for ASRDraftCommitter {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_an_incomplete_tail_pending() {
        let mut committer = ASRDraftCommitter::default();

        assert_eq!(committer.update_draft("今日は"), "今日は");
        assert_eq!(committer.update_draft("今日は晴れ"), "今日は晴れ");
        // No sentence-ending punctuation yet: nothing is committed.
        assert_eq!(committer.commit_complete_sentences(), None);
        assert_eq!(committer.update_draft("今日は晴れです"), "今日は晴れです");
        assert_eq!(committer.commit_complete_sentences(), None);
        assert!(
            committer.has_pending_text(),
            "the incomplete tail should stay pending"
        );
    }

    #[test]
    fn commits_only_complete_sentences() {
        let mut committer = ASRDraftCommitter::default();

        assert_eq!(
            committer.update_draft("こんにちは。今日は天気が"),
            "こんにちは。今日は天気が"
        );
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("こんにちは。")
        );
        // The incomplete tail remains pending and is not committed.
        assert_eq!(committer.commit_complete_sentences(), None);
        assert_eq!(
            committer.update_draft("こんにちは。今日は天気がいいですね。"),
            "今日は天気がいいですね。"
        );
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("今日は天気がいいですね。")
        );
    }

    #[test]
    fn splits_multiple_sentences_per_draft() {
        let mut committer = ASRDraftCommitter::default();

        assert_eq!(
            committer.update_draft("あ！え？うん。まだ"),
            "あ！え？うん。まだ"
        );
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("あ！え？うん。")
        );
        assert_eq!(
            committer.update_draft("あ！え？うん。まだ続きます"),
            "まだ続きます"
        );
    }

    #[test]
    fn supports_english_and_chinese_delimiters() {
        let mut committer = ASRDraftCommitter::default();
        let _ = committer.update_draft("Hello there. How are you?");
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("Hello there. How are you?")
        );

        let mut chinese = ASRDraftCommitter::default();
        let _ = chinese.update_draft("你好！今天天气不错。明天");
        assert_eq!(
            chinese.commit_complete_sentences().as_deref(),
            Some("你好！今天天气不错。")
        );
    }

    #[test]
    fn suppresses_an_already_committed_server_final() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("こんにちは。今日は");
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("こんにちは。")
        );
        // The same sentence arriving later as a server final is already
        // committed verbatim, so it must be dropped.
        assert_eq!(
            committer.finish_sentence("こんにちは。"),
            FinishOutcome::None
        );
        // A genuinely new final is committed.
        assert_eq!(
            committer.finish_sentence("今日は天気がいいですね。"),
            FinishOutcome::Appended("今日は天気がいいですね。".into())
        );
    }

    #[test]
    fn commits_a_clean_server_final_after_drafts() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("今日は晴れ");
        let _ = committer.commit_complete_sentences();
        assert_eq!(
            committer.finish_sentence("今日は晴れですが、寒いです"),
            FinishOutcome::Appended("今日は晴れですが、寒いです".into())
        );
        assert_eq!(committer.update_draft("次の文です"), "次の文です");
    }

    #[test]
    fn strips_overlapping_suffix_from_late_final() {
        let mut committer = ASRDraftCommitter::default();

        // Long-incomplete fallback committed a mid-sentence chunk.
        let long_draft = "あいうえお".repeat(10);
        let _ = committer.update_draft(&long_draft);
        assert_eq!(
            committer.commit_latest_draft(true).as_deref(),
            Some(long_draft.as_str())
        );
        // The server final extends the locally committed chunk, so the whole
        // authoritative final replaces it and history holds the sentence once.
        let extended = format!("{long_draft}かきくけこ");
        assert_eq!(
            committer.finish_sentence(&extended),
            FinishOutcome::Replaced(extended.clone())
        );
    }

    #[test]
    fn ignores_punctuation_only_finals() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("今日は晴れです");
        let _ = committer.commit_complete_sentences();
        assert_eq!(committer.finish_sentence("。"), FinishOutcome::None);
    }

    #[test]
    fn server_final_replaces_a_locally_committed_sentence_it_extends() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("こんにちは。今日は");
        let _ = committer.commit_complete_sentences();
        // The server finalizes the locally committed sentence together with its
        // continuation; the full final replaces the provisional entry instead
        // of splitting into a fragment plus a tail.
        assert_eq!(
            committer.finish_sentence("こんにちは。今日は天気がいいですね。"),
            FinishOutcome::Replaced("こんにちは。今日は天気がいいですね。".into())
        );
        // The authoritative final becomes the new committed boundary.
        assert_eq!(
            committer.update_draft("こんにちは。今日は天気がいいですね。次の話です"),
            "次の話です"
        );
    }

    #[test]
    fn server_final_with_leading_words_replaces_the_provisional_commit() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("行きます。まだ");
        let _ = committer.commit_complete_sentences();
        assert_eq!(
            committer.finish_sentence("私は東京に行きます。"),
            FinishOutcome::Replaced("私は東京に行きます。".into())
        );
    }

    #[test]
    fn revised_server_final_is_appended_when_it_no_longer_covers_the_local_commit() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("私は東京に行きます。");
        let _ = committer.commit_complete_sentences();
        // The wording differs, so structural supersede detection does not
        // fire; the revised final is still committed so it is never dropped.
        assert_eq!(
            committer.finish_sentence("私は東京へ行きます。"),
            FinishOutcome::Appended("私は東京へ行きます。".into())
        );
    }

    #[test]
    fn keeps_subtitles_flowing_with_long_incomplete_speech() {
        let mut committer = ASRDraftCommitter::default();

        let short_incomplete = "まだ話してる途中";
        let _ = committer.update_draft(short_incomplete);
        assert_eq!(
            committer.commit_latest_draft(true),
            None,
            "short incomplete text should stay pending even on max-wait"
        );

        let long_incomplete = "話し手が長いあいだ途切れずに話し続けても字幕は読みやすい長さで";
        let _ = committer.update_draft(long_incomplete);
        assert_eq!(
            committer.commit_latest_draft(true).as_deref(),
            Some(long_incomplete)
        );
    }

    #[test]
    fn final_covers_chunk_detects_structural_supersede() {
        // Exact repeat of the committed chunk is not a cover (dedup).
        assert!(!final_covers_chunk("こんにちは。", "こんにちは。"));
        // Extension and leading-word additions are covers.
        assert!(final_covers_chunk(
            "こんにちは。今日は天気がいいです。",
            "こんにちは。"
        ));
        assert!(final_covers_chunk("はい、こんにちは。", "こんにちは。"));
        // Unrelated text is not a cover.
        assert!(!final_covers_chunk("明日は雨です。", "こんにちは。"));
        // Degenerate single-character inputs never cover.
        assert!(!final_covers_chunk("あ", "あ"));
    }

    #[test]
    fn resets_all_sentence_state() {
        let mut committer = ASRDraftCommitter::default();

        let _ = committer.update_draft("こんにちは。途中まで");
        let _ = committer.commit_complete_sentences();
        committer.reset();

        assert_eq!(
            committer.update_draft("こんにちは。途中まで別の文"),
            "こんにちは。途中まで別の文"
        );
        assert_eq!(
            committer.commit_complete_sentences().as_deref(),
            Some("こんにちは。")
        );
        assert!(
            committer.has_pending_text(),
            "a new draft should be pending after reset"
        );
    }
}
