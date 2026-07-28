import Foundation

public struct TranslationSessionState: Equatable, Sendable {
    public var status: SessionStatus
    public var subtitles: SubtitleSnapshot
    public var detectedLanguage: DetectedLanguage?

    public init(
        status: SessionStatus = .idle,
        subtitles: SubtitleSnapshot = .empty,
        detectedLanguage: DetectedLanguage? = nil
    ) {
        self.status = status
        self.subtitles = subtitles
        self.detectedLanguage = detectedLanguage
    }
}

public struct TranslationSessionController: Sendable {
    public private(set) var state: TranslationSessionState

    private var subtitleReducer: SubtitleReducer

    public init() {
        self.subtitleReducer = SubtitleReducer()
        self.state = TranslationSessionState()
    }

    public mutating func beginConnecting() {
        state.status = .connecting
        state.detectedLanguage = nil
    }

    public mutating func didConnect() {
        state.status = .listening
    }

    public mutating func beginStopping() {
        state.status = .stopping
    }

    public mutating func didStop() {
        state.status = .idle
    }

    public mutating func didFail(_ message: String) {
        state.status = .error(message)
    }

    public mutating func clearSubtitles() {
        subtitleReducer.apply(.clear)
        state.subtitles = subtitleReducer.snapshot
    }

    public mutating func handle(_ event: LiveTranslateServerEvent) {
        // Finishing a realtime session can flush synthetic tail events after
        // audio capture has already stopped. Keep the last real subtitle on
        // screen instead of presenting that service-generated cleanup text.
        guard state.status != .stopping else { return }

        switch event {
        case .sessionCreated:
            break
        case .sessionUpdated:
            didConnect()
        case let .sourceDraft(text, language):
            updateDetectedLanguage(language)
            subtitleReducer.apply(.sourceDraft(text))
        case let .sourceFinal(text, language):
            updateDetectedLanguage(language)
            subtitleReducer.apply(.sourceFinal(text))
        case let .translationDraft(text):
            subtitleReducer.apply(.translationDraft(text))
        case let .translationFinal(text):
            subtitleReducer.apply(.translationFinal(text))
        case .sessionFinished:
            didStop()
        case let .error(_, message):
            didFail(message)
        case .ignored:
            return
        }

        state.subtitles = subtitleReducer.snapshot
    }

    private mutating func updateDetectedLanguage(_ reportedLanguage: String?) {
        if let language = DetectedLanguage(reportedLanguage: reportedLanguage) {
            state.detectedLanguage = language
        }
    }
}
