import Foundation

/// Identifies the newest ASR draft so asynchronous preview callbacks can
/// reject results that belong to older text or a cancelled session.
public struct DraftPreviewTracker: Sendable {
    public private(set) var currentText = ""
    public private(set) var generation = 0

    public init() {}

    public mutating func update(_ text: String) -> Int? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed != currentText else { return nil }
        currentText = trimmed
        generation += 1
        return generation
    }

    public func accepts(text: String, generation: Int) -> Bool {
        self.generation == generation && currentText == text
    }

    public mutating func reset() {
        currentText = ""
        generation += 1
    }
}
