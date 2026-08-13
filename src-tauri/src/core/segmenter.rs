//! Subtitle text segmentation for display, ported 1:1 from
//! `Sources/MimiCore/SubtitleTextSegmenter.swift`.

const SENTENCE_ENDINGS: &[char] = &['。', '！', '？', '!', '?', '；', ';', '\n'];
const PREFERRED_BREAKS: &[char] = &['，', '、', ',', '：', ':', '—', '–', '-', ' '];

pub enum SubtitleTextSegmenter {}

impl SubtitleTextSegmenter {
    /// Splits `text` into display segments, preferring sentence punctuation,
    /// then commas/other preferred breaks, and finally a hard character cap.
    pub fn segments(text: &str, maximum_characters: usize) -> Vec<String> {
        let maximum_characters = maximum_characters.max(4);
        let mut remaining: Vec<char> = text.trim().chars().collect();
        let mut result: Vec<String> = Vec::new();

        while !remaining.is_empty() {
            trim_leading_whitespace(&mut remaining);
            if remaining.is_empty() {
                break;
            }

            let search_count = maximum_characters.min(remaining.len());
            if let Some(sentence_end) = remaining[..search_count]
                .iter()
                .position(|c| SENTENCE_ENDINGS.contains(c))
            {
                append_segment(&mut remaining, sentence_end + 1, &mut result);
                continue;
            }

            if remaining.len() <= maximum_characters {
                let remaining_len = remaining.len();
                append_segment(&mut remaining, remaining_len, &mut result);
                continue;
            }

            let minimum_preferred_break = (maximum_characters / 2).max(1);
            let preferred_break = (minimum_preferred_break..maximum_characters)
                .rev()
                .find(|&i| PREFERRED_BREAKS.contains(&remaining[i]));

            let end = match preferred_break {
                Some(break_index) if remaining[break_index].is_whitespace() => break_index,
                Some(break_index) => break_index + 1,
                None => maximum_characters,
            };
            append_segment(&mut remaining, end.max(1), &mut result);
        }

        result
    }

    /// The visible tail of a long draft: only the newest `maximum_segments`
    /// segments are rendered.
    pub fn visible_draft_segments(
        text: &str,
        maximum_characters: usize,
        maximum_segments: usize,
    ) -> Vec<String> {
        if maximum_segments == 0 {
            return Vec::new();
        }
        let all = Self::segments(text, maximum_characters);
        let skip = all.len().saturating_sub(maximum_segments);
        all.into_iter().skip(skip).collect()
    }
}

fn append_segment(remaining: &mut Vec<char>, end: usize, result: &mut Vec<String>) {
    let safe_end = end.max(1).min(remaining.len());
    let segment: String = remaining[..safe_end].iter().collect();
    remaining.drain(0..safe_end);
    let segment = segment.trim();
    if !segment.is_empty() {
        result.push(segment.to_string());
    }
}

fn trim_leading_whitespace(characters: &mut Vec<char>) {
    while characters.first().is_some_and(|c| c.is_whitespace()) {
        characters.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_subtitle_remains_a_single_segment() {
        assert_eq!(
            SubtitleTextSegmenter::segments("今天的天气很好。", 28),
            vec!["今天的天气很好。"]
        );
    }

    #[test]
    fn sentence_punctuation_creates_stable_subtitle_segments() {
        assert_eq!(
            SubtitleTextSegmenter::segments(
                "第一句话已经说完。第二句话也说完了！第三句还在继续",
                28
            ),
            vec!["第一句话已经说完。", "第二句话也说完了！", "第三句还在继续"]
        );
    }

    #[test]
    fn continuous_cjk_speech_is_bounded_without_losing_text() {
        let text =
            "这是一段完全没有句号而且会持续不断增长的字幕内容用来模拟视频里人物一直讲话的情况";
        let segments = SubtitleTextSegmenter::segments(text, 14);

        assert!(segments.len() > 1, "long continuous speech should be split");
        assert!(
            segments.iter().all(|s| s.chars().count() <= 14),
            "every CJK segment should respect the requested limit"
        );
        assert_eq!(segments.concat(), text);
    }

    #[test]
    fn english_subtitles_prefer_word_boundaries() {
        let text = "This is a continuous English subtitle that should never split a normal word.";
        let segments = SubtitleTextSegmenter::segments(text, 24);

        assert!(segments.len() > 1, "long English speech should be split");
        assert_eq!(segments.join(" "), text);
        assert!(
            segments[..segments.len() - 1]
                .iter()
                .all(|s| !s.ends_with(' ')),
            "segments should not retain boundary whitespace"
        );
    }

    #[test]
    fn extending_a_long_draft_preserves_completed_segment_prefixes() {
        let first =
            SubtitleTextSegmenter::segments("持续讲话时字幕会不断增长直到超过一行然后继续", 12);
        let extended = SubtitleTextSegmenter::segments(
            "持续讲话时字幕会不断增长直到超过一行然后继续显示后面的新增内容",
            12,
        );

        let first_len = first.len();
        assert_eq!(&extended[..first_len - 1], &first[..first_len - 1]);
    }

    #[test]
    fn long_draft_preview_keeps_only_its_newest_two_segments() {
        let text = "第一段已经说完。第二段也已经说完。第三段正在继续。第四段是最新内容";
        let full_segments = SubtitleTextSegmenter::segments(text, 12);

        assert_eq!(
            SubtitleTextSegmenter::visible_draft_segments(text, 12, 2),
            full_segments[full_segments.len() - 2..].to_vec()
        );
    }

    #[test]
    fn short_draft_preview_remains_intact() {
        assert_eq!(
            SubtitleTextSegmenter::visible_draft_segments("短字幕", 12, 2),
            vec!["短字幕"]
        );
    }

    #[test]
    fn zero_segment_preview_is_empty() {
        assert!(SubtitleTextSegmenter::visible_draft_segments("短字幕", 12, 0).is_empty());
    }
}
