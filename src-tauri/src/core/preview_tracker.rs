//! Identifies the newest ASR draft so asynchronous preview callbacks can
//! reject results that belong to older text or a cancelled session. Ported 1:1
//! from `Sources/MimiCore/DraftPreviewTracker.swift`.

#[derive(Debug, Default)]
pub struct DraftPreviewTracker {
    current_text: String,
    generation: u64,
}

impl DraftPreviewTracker {
    /// Registers a new draft and returns its generation, or `None` when the
    /// text is empty or unchanged.
    pub fn update(&mut self, text: &str) -> Option<u64> {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == self.current_text {
            return None;
        }
        self.current_text = trimmed.to_string();
        self.generation += 1;
        Some(self.generation)
    }

    /// Whether a result for `text` at `generation` is still the newest draft.
    pub fn accepts(&self, text: &str, generation: u64) -> bool {
        self.generation == generation && self.current_text == text
    }

    pub fn reset(&mut self) {
        self.current_text.clear();
        self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_returns_generations_for_new_text() {
        let mut tracker = DraftPreviewTracker::default();
        assert_eq!(tracker.update("こんにちは"), Some(1));
        assert_eq!(tracker.update("こんにちは、今日は"), Some(2));
    }

    #[test]
    fn update_rejects_empty_or_unchanged_text() {
        let mut tracker = DraftPreviewTracker::default();
        assert_eq!(tracker.update("   "), None);
        assert_eq!(tracker.update("こんにちは"), Some(1));
        assert_eq!(tracker.update("こんにちは"), None);
        // Trimming normalizes the comparison like the Swift version.
        assert_eq!(tracker.update("  こんにちは  "), None);
    }

    #[test]
    fn accepts_only_the_current_generation_and_text() {
        let mut tracker = DraftPreviewTracker::default();
        let generation = tracker.update("こんにちは").unwrap();

        assert!(tracker.accepts("こんにちは", generation));
        // Older generation is rejected after a newer draft arrives.
        tracker.update("こんにちは、今日");
        assert!(!tracker.accepts("こんにちは", generation));
        // Same generation but different text is rejected.
        assert!(!tracker.accepts("別の文章", generation + 1));
    }

    #[test]
    fn reset_invalidates_pending_generations() {
        let mut tracker = DraftPreviewTracker::default();
        let generation = tracker.update("こんにちは").unwrap();

        tracker.reset();

        assert!(!tracker.accepts("こんにちは", generation));
        // New updates start from a fresh generation and are accepted.
        let new_generation = tracker.update("こんにちは").unwrap();
        assert!(tracker.accepts("こんにちは", new_generation));
    }
}
