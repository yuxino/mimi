import Foundation

public enum SourceLanguage: String, CaseIterable, Codable, Identifiable, Sendable {
    case automatic = "auto"
    case english = "en"
    case japanese = "ja"
    case korean = "ko"

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .automatic:
            "自动识别"
        case .english:
            "English"
        case .japanese:
            "日本語"
        case .korean:
            "한국어"
        }
    }
}

public enum TranslationMode: String, CaseIterable, Codable, Identifiable, Sendable {
    case lowLatency
    case highQuality

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .lowLatency:
            "低延迟（推荐）"
        case .highQuality:
            "高质量"
        }
    }
}

public enum SessionStatus: Equatable, Sendable {
    case idle
    case connecting
    case listening
    case stopping
    case error(String)

    public var isActive: Bool {
        switch self {
        case .connecting, .listening, .stopping:
            true
        case .idle, .error:
            false
        }
    }
}

public struct SubtitleLine: Equatable, Sendable {
    public var text: String
    public var isFinal: Bool

    public init(text: String = "", isFinal: Bool = false) {
        self.text = text
        self.isFinal = isFinal
    }
}

public struct SubtitlePair: Equatable, Sendable {
    public let source: String
    public let translation: String

    public init(source: String, translation: String) {
        self.source = source
        self.translation = translation
    }
}

public struct SubtitleSnapshot: Equatable, Sendable {
    public var source: SubtitleLine
    public var translation: SubtitleLine
    public var history: [SubtitlePair]

    public init(
        source: SubtitleLine = .init(),
        translation: SubtitleLine = .init(),
        history: [SubtitlePair] = []
    ) {
        self.source = source
        self.translation = translation
        self.history = history
    }

    public static let empty = SubtitleSnapshot()
}

public enum SubtitleEvent: Equatable, Sendable {
    case sourceDraft(String)
    case sourceFinal(String)
    case translationDraft(String)
    case translationFinal(String)
    case clear
}
