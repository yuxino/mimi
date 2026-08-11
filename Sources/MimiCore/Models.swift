import Foundation

public enum SourceLanguage: String, CaseIterable, Codable, Identifiable, Sendable {
    case automatic = "auto"
    case chinese = "zh"
    case english = "en"
    case japanese = "ja"
    case korean = "ko"

    public static let manualCases: [SourceLanguage] = [
        .japanese,
        .english,
        .korean,
        .chinese
    ]

    public var id: String { rawValue }

    public init?(detectedLanguage: String?) {
        guard let normalized = detectedLanguage?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        else {
            return nil
        }

        if normalized == "zh"
            || normalized.hasPrefix("zh-")
            || normalized == "chinese"
            || normalized == "mandarin" {
            self = .chinese
        } else if normalized == "ja" || normalized.hasPrefix("ja-") || normalized == "japanese" {
            self = .japanese
        } else if normalized == "en" || normalized.hasPrefix("en-") || normalized == "english" {
            self = .english
        } else if normalized == "ko" || normalized.hasPrefix("ko-") || normalized == "korean" {
            self = .korean
        } else {
            return nil
        }
    }

    public var displayName: String {
        switch self {
        case .automatic:
            "自动识别"
        case .chinese:
            "中文"
        case .english:
            "English"
        case .japanese:
            "日本語"
        case .korean:
            "한국어"
        }
    }

    public func statusDisplayName(
        detectedLanguage: DetectedLanguage?,
        targetLanguage: TargetLanguage
    ) -> String {
        guard self == .automatic else { return displayName }
        guard let detectedLanguage else { return "自动识别中" }
        if targetLanguage == .simplifiedChinese, detectedLanguage.code == "zh" {
            return "自动识别中"
        }
        return "自动识别（\(detectedLanguage.displayName)）"
    }

    public func targetLanguageAfterQuickSwitch(
        from previousSource: SourceLanguage,
        currentTarget: TargetLanguage
    ) -> TargetLanguage {
        if self == .chinese {
            return .original
        }
        if previousSource == .chinese, currentTarget == .original {
            return .simplifiedChinese
        }
        return currentTarget
    }
}

public struct DetectedLanguage: Equatable, Sendable {
    public let code: String

    public init?(reportedLanguage: String?) {
        guard
            let normalized = reportedLanguage?
                .trimmingCharacters(in: .whitespacesAndNewlines)
                .lowercased(),
            !normalized.isEmpty
        else {
            return nil
        }
        self.code = normalized.split(separator: "-").first.map(String.init) ?? normalized
    }

    public var displayName: String {
        switch code {
        case "zh", "chinese", "mandarin":
            "中文"
        case "yue", "cantonese":
            "粤语"
        case "en", "english":
            "English"
        case "ja", "japanese":
            "日本語"
        case "ko", "korean":
            "한국어"
        case "de":
            "Deutsch"
        case "fr":
            "Français"
        case "es":
            "Español"
        case "pt":
            "Português"
        case "it":
            "Italiano"
        case "ru":
            "Русский"
        case "ar":
            "العربية"
        case "hi":
            "हिन्दी"
        case "id":
            "Bahasa Indonesia"
        case "th":
            "ไทย"
        case "tr":
            "Türkçe"
        case "vi":
            "Tiếng Việt"
        case "uk":
            "Українська"
        case "cs":
            "Čeština"
        case "da":
            "Dansk"
        case "tl", "fil":
            "Filipino"
        case "fi":
            "Suomi"
        case "is":
            "Íslenska"
        case "ms":
            "Bahasa Melayu"
        case "no", "nb":
            "Norsk"
        case "pl":
            "Polski"
        case "sv":
            "Svenska"
        default:
            code.uppercased()
        }
    }
}

public enum TargetLanguage: String, CaseIterable, Codable, Identifiable, Sendable {
    case original = "original"
    case simplifiedChinese = "zh"
    case english = "en"
    case japanese = "ja"

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .original:
            "原文（不翻译）"
        case .simplifiedChinese:
            "简体中文"
        case .english:
            "English"
        case .japanese:
            "日本語"
        }
    }

    public var qwenMTName: String {
        switch self {
        case .original:
            ""
        case .simplifiedChinese:
            "Chinese"
        case .english:
            "English"
        case .japanese:
            "Japanese"
        }
    }

    public var translatesAudio: Bool {
        self != .original
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
    public let createdAt: Date

    public init(source: String, translation: String, createdAt: Date = .now) {
        self.source = source
        self.translation = translation
        self.createdAt = createdAt
    }

    public static func == (lhs: SubtitlePair, rhs: SubtitlePair) -> Bool {
        lhs.source == rhs.source && lhs.translation == rhs.translation
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
    /// Removes the last confirmed history pair so a provisional local commit can
    /// be replaced by the authoritative server final for the same sentence.
    case revokeLastConfirmed
    case clear
}
