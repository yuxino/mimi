import Foundation

public struct SubtitleReducer: Sendable {
    public private(set) var snapshot: SubtitleSnapshot

    private let maxHistoryCount: Int
    private var pendingFinalSources: [String]

    public init(maxHistoryCount: Int = 20) {
        self.snapshot = .empty
        self.maxHistoryCount = max(0, maxHistoryCount)
        self.pendingFinalSources = []
    }

    public mutating func apply(_ event: SubtitleEvent) {
        switch event {
        case let .sourceDraft(text):
            snapshot.source = SubtitleLine(text: text.trimmed, isFinal: false)

        case let .sourceFinal(text):
            let source = text.trimmed
            snapshot.source = SubtitleLine(text: source, isFinal: true)
            if !source.isEmpty {
                pendingFinalSources.append(source)
            }

        case let .translationDraft(text):
            let trimmed = text.trimmed
            // 不让空白 draft 覆盖已确认的 final 译文
            if trimmed.isEmpty, snapshot.translation.isFinal {
                return
            }
            snapshot.translation = SubtitleLine(text: trimmed, isFinal: false)

        case let .translationFinal(text):
            let translation = text.trimmed
            snapshot.translation = SubtitleLine(text: translation, isFinal: true)
            appendHistoryIfPossible(
                source: pendingFinalSources.isEmpty
                    ? snapshot.source.text
                    : pendingFinalSources.removeFirst(),
                translation: translation
            )

        case .revokeLastConfirmed:
            if !snapshot.history.isEmpty {
                snapshot.history.removeLast()
            }

        case .clear:
            snapshot = .empty
            pendingFinalSources = []
        }
    }

    private mutating func appendHistoryIfPossible(source: String, translation: String) {
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
