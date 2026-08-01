import Foundation

public enum QwenMTProtocolError: Error, LocalizedError, Equatable, Sendable {
    case invalidWorkspaceID
    case invalidEndpoint
    case invalidJSON
    case missingTranslation

    public var errorDescription: String? {
        switch self {
        case .invalidWorkspaceID:
            "The Workspace ID is not valid."
        case .invalidEndpoint:
            "The Qwen-MT endpoint could not be created."
        case .invalidJSON:
            "Qwen-MT returned an invalid response."
        case .missingTranslation:
            "Qwen-MT returned no translated text."
        }
    }
}

public struct QwenMTEndpoint: Equatable, Sendable {
    public let url: URL

    public init(workspaceID: String) throws {
        let pattern = "^[A-Za-z0-9][A-Za-z0-9-]{1,126}[A-Za-z0-9]$"
        guard workspaceID.range(of: pattern, options: .regularExpression) != nil else {
            throw QwenMTProtocolError.invalidWorkspaceID
        }

        var components = URLComponents()
        components.scheme = "https"
        components.host = "\(workspaceID).cn-beijing.maas.aliyuncs.com"
        components.path = "/compatible-mode/v1/chat/completions"

        guard let url = components.url else {
            throw QwenMTProtocolError.invalidEndpoint
        }
        self.url = url
    }
}

public enum QwenMTModel: String, Sendable {
    case lite = "qwen-mt-lite"
    case flash = "qwen-mt-flash"
}

public enum QwenMTDomainHint {
    public static func spokenDialogue(for targetLanguage: TargetLanguage) -> String {
        let languageGuidance = switch targetLanguage {
        case .original:
            ""
        case .simplifiedChinese:
            "Use concise, idiomatic Simplified Chinese and preserve natural particles such as 嗯、啊、呢、吧、嘛."
        case .english:
            "Use concise, idiomatic conversational English with natural contractions and interjections."
        case .japanese:
            "Use natural conversational Japanese with appropriate register, particles, and sentence endings."
        }
        return """
        Natural spoken dialogue. \(languageGuidance) Preserve the speaker's tone and \
        implied subjects from context. Avoid literal, explanatory, or translation-like \
        wording. Faithfully preserve every vocalization represented in the source, \
        including interjections, hesitation, breaths, gasps, moans, and cries. Render \
        non-lexical sounds naturally in the target language. Do not sanitize, euphemize, \
        censor, or omit explicit dialogue or vocal sounds. Collapse only clear accidental \
        ASR repetition.
        """
    }
}

public struct QwenMTMemoryPair: Equatable, Sendable, Encodable {
    public let source: String
    public let target: String

    public init(source: String, target: String) {
        self.source = source
        self.target = target
    }
}

public enum QwenMTRequestEncoder {
    public static func request(
        text: String,
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage = .simplifiedChinese,
        model: QwenMTModel = .lite,
        stream: Bool = false,
        domainHint: String? = nil,
        translationMemory: [QwenMTMemoryPair] = []
    ) throws -> Data {
        let request = QwenMTRequest(
            model: model.rawValue,
            messages: [.init(role: "user", content: text)],
            stream: stream,
            translationOptions: .init(
                sourceLanguage: sourceLanguage.qwenMTName,
                targetLanguage: targetLanguage.qwenMTName,
                domains: domainHint,
                translationMemory: translationMemory.isEmpty ? nil : translationMemory
            )
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        return try encoder.encode(request)
    }
}

public enum QwenMTStreamDecoder {
    public static func decodeChunk(_ data: Data) throws -> String? {
        let response: QwenMTStreamResponse
        do {
            response = try JSONDecoder().decode(QwenMTStreamResponse.self, from: data)
        } catch {
            throw QwenMTProtocolError.invalidJSON
        }
        return response.choices.first?.delta.content
    }
}

public enum QwenMTResponseDecoder {
    public static func decode(_ data: Data) throws -> String {
        let response: QwenMTResponse
        do {
            response = try JSONDecoder().decode(QwenMTResponse.self, from: data)
        } catch {
            throw QwenMTProtocolError.invalidJSON
        }

        guard let content = response.choices.first?.message.content.trimmedNonempty else {
            throw QwenMTProtocolError.missingTranslation
        }
        return content
    }
}

private struct QwenMTRequest: Encodable {
    let model: String
    let messages: [Message]
    let stream: Bool
    let translationOptions: TranslationOptions

    enum CodingKeys: String, CodingKey {
        case model
        case messages
        case stream
        case translationOptions = "translation_options"
    }

    struct Message: Encodable {
        let role: String
        let content: String
    }

    struct TranslationOptions: Encodable {
        let sourceLanguage: String
        let targetLanguage: String
        let domains: String?
        let translationMemory: [QwenMTMemoryPair]?

        enum CodingKeys: String, CodingKey {
            case sourceLanguage = "source_lang"
            case targetLanguage = "target_lang"
            case domains
            case translationMemory = "tm_list"
        }
    }
}

private struct QwenMTResponse: Decodable {
    let choices: [Choice]

    struct Choice: Decodable {
        let message: Message
    }

    struct Message: Decodable {
        let content: String
    }
}

private struct QwenMTStreamResponse: Decodable {
    let choices: [Choice]

    struct Choice: Decodable {
        let delta: Delta
    }

    struct Delta: Decodable {
        let content: String?
    }
}

private extension SourceLanguage {
    var qwenMTName: String {
        switch self {
        case .automatic:
            "auto"
        case .chinese:
            "Chinese"
        case .english:
            "English"
        case .japanese:
            "Japanese"
        case .korean:
            "Korean"
        }
    }
}

private extension String {
    var trimmedNonempty: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
