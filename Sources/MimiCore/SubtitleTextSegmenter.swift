import Foundation

public enum SubtitleTextSegmenter {
    private static let sentenceEndings: Set<Character> = [
        "。", "！", "？", "!", "?", "；", ";", "\n"
    ]
    private static let preferredBreaks: Set<Character> = [
        "，", "、", ",", "：", ":", "—", "–", "-", " "
    ]

    public static func segments(
        in text: String,
        maximumCharacters: Int
    ) -> [String] {
        let maximumCharacters = max(4, maximumCharacters)
        var remaining = Array(
            text.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        var result: [String] = []

        while !remaining.isEmpty {
            trimLeadingWhitespace(from: &remaining)
            guard !remaining.isEmpty else { break }

            let searchCount = min(maximumCharacters, remaining.count)
            if let sentenceEnd = remaining[..<searchCount]
                .firstIndex(where: sentenceEndings.contains) {
                appendSegment(
                    from: &remaining,
                    through: sentenceEnd + 1,
                    to: &result
                )
                continue
            }

            guard remaining.count > maximumCharacters else {
                appendSegment(
                    from: &remaining,
                    through: remaining.count,
                    to: &result
                )
                continue
            }

            let minimumPreferredBreak = max(1, maximumCharacters / 2)
            let preferredBreak = stride(
                from: maximumCharacters - 1,
                through: minimumPreferredBreak,
                by: -1
            ).first { preferredBreaks.contains(remaining[$0]) }

            let end: Int
            if let preferredBreak {
                end = remaining[preferredBreak].isWhitespace
                    ? preferredBreak
                    : preferredBreak + 1
            } else {
                end = maximumCharacters
            }
            appendSegment(from: &remaining, through: max(1, end), to: &result)
        }

        return result
    }

    public static func visibleDraftSegments(
        in text: String,
        maximumCharacters: Int,
        maximumSegments: Int = 2
    ) -> [String] {
        let maximumSegments = max(0, maximumSegments)
        guard maximumSegments > 0 else { return [] }
        return Array(
            segments(in: text, maximumCharacters: maximumCharacters)
                .suffix(maximumSegments)
        )
    }

    private static func appendSegment(
        from remaining: inout [Character],
        through end: Int,
        to result: inout [String]
    ) {
        let safeEnd = min(max(1, end), remaining.count)
        let segment = String(remaining[..<safeEnd])
            .trimmingCharacters(in: .whitespacesAndNewlines)
        remaining.removeFirst(safeEnd)
        if !segment.isEmpty {
            result.append(segment)
        }
    }

    private static func trimLeadingWhitespace(from characters: inout [Character]) {
        while characters.first?.isWhitespace == true {
            characters.removeFirst()
        }
    }
}
