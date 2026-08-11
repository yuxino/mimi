import Foundation

/// Splits a cumulative ASR draft into complete sentences so subtitles are never
/// committed mid-sentence, and so a later server final cannot duplicate text
/// that was already committed.
///
/// Local commits (stable-draft and maximum-wait) are treated as replaceable
/// previews: when the server later finalizes a sentence that structurally
/// covers the last local commit, the committer reports a replacement so the
/// client can revoke the provisional history entry and commit the authoritative
/// final exactly once.
public struct ASRDraftCommitter: Sendable {
    private static let sentenceDelimiters: Set<Character> = ["。", "！", "？", ".", "!", "?", "\n"]
    private static let longIncompleteCommitThreshold = 20

    private var latestDraft = ""
    private var committedText = ""
    private var lastCommittedChunk = ""
    private var lastCommitWasProvisional = false

    public init() {}

    public var hasPendingText: Bool {
        Self.isMeaningful(pendingText(in: latestDraft))
    }

    @discardableResult
    public mutating func updateDraft(_ text: String) -> String {
        latestDraft = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return pendingText(in: latestDraft)
    }

    /// Commits every complete sentence in the pending draft and returns the newly
    /// committed text. An incomplete trailing sentence stays pending so the UI
    /// keeps showing it as the live draft.
    public mutating func commitCompleteSentences() -> String? {
        let pending = pendingText(in: latestDraft)
        guard !pending.isEmpty else { return nil }
        let (complete, tail) = Self.splitSentences(pending)
        guard Self.isMeaningful(complete) else { return nil }
        committedText = String(latestDraft.dropLast(tail.count))
        lastCommittedChunk = complete
        lastCommitWasProvisional = true
        return complete
    }

    /// Commits complete sentences; when `commitLongIncomplete` is true and no
    /// complete sentence exists, commits a long pending tail as a single chunk so
    /// subtitles keep flowing during very long uninterrupted speech.
    public mutating func commitLatestDraft(commitLongIncomplete: Bool = false) -> String? {
        if let complete = commitCompleteSentences() {
            return complete
        }
        let pending = pendingText(in: latestDraft)
        guard
            commitLongIncomplete,
            pending.count >= Self.longIncompleteCommitThreshold,
            Self.isMeaningful(pending)
        else {
            return nil
        }
        committedText = latestDraft
        lastCommittedChunk = pending
        lastCommitWasProvisional = true
        return pending
    }

    /// The outcome of handling a server-final sentence.
    public enum FinishOutcome: Equatable, Sendable {
        /// The final is already covered by committed text; nothing new to show.
        case none
        /// New authoritative content was appended to the committed boundary.
        case appended(String)
        /// The final supersedes the last local (provisional) commit; the client
        /// should revoke that history entry and commit the full final once.
        case replaced(String)
    }

    /// Handles a server-final sentence. Returns `.none` when the final is already
    /// covered by committed text, `.appended` when the final adds new content,
    /// and `.replaced` when the final supersedes the last local commit so the
    /// provisional history entry can be revoked and committed exactly once.
    public mutating func finishSentence(_ text: String) -> FinishOutcome {
        let finalText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isMeaningful(finalText) else { return .none }

        if !committedText.isEmpty, finalText.count >= 2, committedText.contains(finalText) {
            // The final matches text that is already committed (including a
            // locally committed sentence the server just confirmed verbatim).
            lastCommitWasProvisional = false
            lastCommittedChunk = ""
            return .none
        }

        if lastCommitWasProvisional, !lastCommittedChunk.isEmpty, finalText.count >= 2 {
            let chunk = lastCommittedChunk
            let supersedes = finalText != chunk
                && (finalText.hasPrefix(chunk) || finalText.contains(chunk))
            if supersedes {
                committedText = committedText.hasSuffix(chunk)
                    ? String(committedText.dropLast(chunk.count))
                    : ""
                committedText += finalText
                lastCommitWasProvisional = false
                lastCommittedChunk = ""
                return .replaced(finalText)
            }
        }

        let overlap = Self.suffixOverlap(of: committedText, prefix: finalText)
        let newText = String(finalText.dropFirst(overlap))
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isMeaningful(newText) else {
            lastCommitWasProvisional = false
            lastCommittedChunk = ""
            return .none
        }

        committedText += newText
        lastCommitWasProvisional = false
        lastCommittedChunk = ""
        return .appended(newText)
    }

    public mutating func reset() {
        latestDraft = ""
        committedText = ""
        lastCommittedChunk = ""
        lastCommitWasProvisional = false
    }

    private func pendingText(in text: String) -> String {
        guard !committedText.isEmpty else { return text }
        guard text.hasPrefix(committedText) else {
            // Server revised earlier text; show the whole corrected draft and let
            // server finals (not local commits) advance the committed boundary.
            return text
        }
        return String(text.dropFirst(committedText.count))
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func splitSentences(_ text: String) -> (complete: String, tail: String) {
        var complete = ""
        var current = ""
        for character in text {
            current.append(character)
            if sentenceDelimiters.contains(character) {
                complete += current
                current = ""
            }
        }
        return (complete, current)
    }

    private static func suffixOverlap(of text: String, prefix: String) -> Int {
        let textChars = Array(text)
        let prefixChars = Array(prefix)
        guard !textChars.isEmpty, !prefixChars.isEmpty else { return 0 }
        let maximum = min(textChars.count, prefixChars.count)
        for length in stride(from: maximum, through: 1, by: -1)
        where prefixChars[..<length].elementsEqual(textChars[(textChars.count - length)...]) {
            return length
        }
        return 0
    }

    private static func isMeaningful(_ text: String) -> Bool {
        text.unicodeScalars.contains { scalar in
            !CharacterSet.whitespacesAndNewlines.contains(scalar)
                && !CharacterSet.punctuationCharacters.contains(scalar)
        }
    }
}
