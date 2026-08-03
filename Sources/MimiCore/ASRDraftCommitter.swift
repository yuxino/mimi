import Foundation

/// Splits a cumulative ASR draft into chunks without translating the same
/// confirmed prefix twice when the server's final result arrives later.
public struct ASRDraftCommitter: Sendable {
    private var latestDraft = ""
    private var committedPrefix = ""

    public init() {}

    public var hasPendingText: Bool {
        Self.isMeaningful(uncommittedText(in: latestDraft))
    }

    @discardableResult
    public mutating func updateDraft(_ text: String) -> String {
        latestDraft = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return uncommittedText(in: latestDraft)
    }

    public mutating func commitLatestDraft() -> String? {
        let remainder = uncommittedText(in: latestDraft)
        committedPrefix = latestDraft
        return Self.isMeaningful(remainder) ? remainder : nil
    }

    public mutating func finishSentence(_ text: String) -> String? {
        let finalText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let remainder = uncommittedText(in: finalText)
        reset()
        return Self.isMeaningful(remainder) ? remainder : nil
    }

    public mutating func reset() {
        latestDraft = ""
        committedPrefix = ""
    }

    private func uncommittedText(in text: String) -> String {
        guard !committedPrefix.isEmpty, text.hasPrefix(committedPrefix) else {
            return text
        }
        return String(text.dropFirst(committedPrefix.count))
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func isMeaningful(_ text: String) -> Bool {
        text.unicodeScalars.contains { scalar in
            !CharacterSet.whitespacesAndNewlines.contains(scalar)
                && !CharacterSet.punctuationCharacters.contains(scalar)
        }
    }
}
