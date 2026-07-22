import Foundation

public struct SubtitleReducer: Sendable {
    public private(set) var snapshot: SubtitleSnapshot

    private let maxHistoryCount: Int

    public init(maxHistoryCount: Int = 20) {
        self.snapshot = .empty
        self.maxHistoryCount = max(0, maxHistoryCount)
    }

    public mutating func apply(_ event: SubtitleEvent) {
        switch event {
        case let .sourceDraft(text):
            snapshot.source = SubtitleLine(text: text.trimmed, isFinal: false)

        case let .sourceFinal(text):
            snapshot.source = SubtitleLine(text: text.trimmed, isFinal: true)

        case let .translationDraft(text):
            snapshot.translation = SubtitleLine(text: text.trimmed, isFinal: false)

        case let .translationFinal(text):
            let translation = text.trimmed
            snapshot.translation = SubtitleLine(text: translation, isFinal: true)
            appendHistoryIfPossible(translation: translation)

        case .clear:
            snapshot = .empty
        }
    }

    private mutating func appendHistoryIfPossible(translation: String) {
        let source = snapshot.source.text
        guard !source.isEmpty, !translation.isEmpty else { return }

        let pair = SubtitlePair(source: source, translation: translation)
        guard snapshot.history.last != pair else { return }

        snapshot.history.append(pair)
        if snapshot.history.count > maxHistoryCount {
            snapshot.history.removeFirst(snapshot.history.count - maxHistoryCount)
        }
    }
}

private extension String {
    var trimmed: String {
        trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
