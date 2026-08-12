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
    case plus = "qwen-mt-plus"
}

public enum QwenMTDomainHint {
    public static func spokenDialogue(
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage
    ) -> String {
        let languageGuidance = switch targetLanguage {
        case .original:
            ""
        case .simplifiedChinese:
            """
            Use concise, idiomatic Simplified Chinese, like subtitles for a TV \
            drama, and keep every natural particle: 嗯、啊、呢、吧、嘛、哦、唉. \
            Render Japanese fillers (えっと、あの、うーん、あぁ) and English \
            fillers (um, uh, oh, hmm) with their natural Chinese equivalents; \
            never drop a meaningful filler.
            """
        case .english:
            """
            Use concise, idiomatic conversational English with natural \
            contractions and interjections (um, uh, oh, hmm, yeah). Keep \
            Japanese fillers (えっと、あの、うーん、あぁ) as natural English \
            equivalents; never drop a meaningful filler.
            """
        case .japanese:
            """
            Use natural conversational Japanese with appropriate register, \
            particles, and sentence endings (ね、よ、な、じゃん). Keep Chinese \
            and English fillers as natural Japanese equivalents; never drop a \
            meaningful filler.
            """
        }
        let sourceGuidance = sourceGuidance(
            source: sourceLanguage,
            target: targetLanguage
        )
        return """
        Natural spoken dialogue for watching TV dramas and films. \(languageGuidance) \
        \(sourceGuidance) Preserve the speaker's tone, emotion, and implied subjects \
        from context. The translation memory (tm_list) is the ongoing dialogue: \
        each pair is one finished subtitle line and its translation. Use it as \
        context — keep names, pronouns, and implied subjects consistent with \
        earlier lines, keep the same speaker's tone and register, and resolve \
        ambiguous or truncated phrases from what came before. Never repeat or \
        re-translate anything already in the memory; translate only the new input. \
        Render polite or formal Japanese (です/ます、敬語) as naturally courteous \
        Chinese, never stiff or literal. Avoid literal, explanatory, or translation-like \
        wording. Prefer short, complete sentences that fit one subtitle line. \
        Faithfully preserve every vocalization represented in the source, including \
        interjections, hesitation, breaths, gasps, moans, and cries, rendered \
        naturally in the target language. Do not sanitize, euphemize, censor, or \
        omit explicit dialogue or vocal sounds. Keep deliberate repetition for \
        emphasis; collapse only clear accidental ASR repetition. Output only the \
        translation text.
        """
    }

    /// Returns a glossary that forces common Japanese, English, and Korean
    /// fillers to keep a natural counterpart in the target language. Prose
    /// instructions alone are not enough — Qwen-MT flattens えっと/うーん/あぁ
    /// into nothing, so these exact mappings pin them down.
    public static func fillerTerms(
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage
    ) -> [QwenMTTerm] {
        let sources: [SourceLanguage] = sourceLanguage == .automatic
            ? [.japanese, .english, .korean]
            : [sourceLanguage]
        return sources.flatMap { source in
            fillerTerms(source: source, target: targetLanguage)
        }
    }

    private static func fillerTerms(
        source: SourceLanguage,
        target: TargetLanguage
    ) -> [QwenMTTerm] {
        switch (source, target) {
        case (.japanese, .simplifiedChinese):
            return [
                .init(source: "えっと", target: "那个"),
                .init(source: "えーと", target: "那个"),
                .init(source: "ええと", target: "那个"),
                .init(source: "あの", target: "那个"),
                .init(source: "あのー", target: "那个"),
                .init(source: "あのう", target: "那个"),
                .init(source: "うーん", target: "嗯"),
                .init(source: "う〜ん", target: "嗯"),
                .init(source: "あぁ", target: "啊"),
                .init(source: "ああ", target: "啊"),
                .init(source: "あっ", target: "啊"),
                .init(source: "えっ", target: "诶"),
                .init(source: "ふふ", target: "呵呵"),
                .init(source: "うふふ", target: "嘿嘿"),
                .init(source: "まあ", target: "嘛"),
                .init(source: "ねえ", target: "那个"),
                .init(source: "あら", target: "哎呀"),
                .init(source: "おや", target: "哎呀"),
                .init(source: "うわ", target: "哇"),
                .init(source: "きゃっ", target: "呀"),
                .init(source: "はぁ", target: "唉"),
                .init(source: "んー", target: "嗯")
            ]
        case (.english, .simplifiedChinese):
            return [
                .init(source: "um", target: "嗯"),
                .init(source: "uh", target: "呃"),
                .init(source: "oh", target: "哦"),
                .init(source: "hmm", target: "嗯"),
                .init(source: "ah", target: "啊"),
                .init(source: "wow", target: "哇"),
                .init(source: "hey", target: "喂"),
                .init(source: "yikes", target: "哎呀")
            ]
        case (.korean, .simplifiedChinese):
            return [
                .init(source: "어", target: "嗯"),
                .init(source: "아", target: "啊"),
                .init(source: "음", target: "嗯"),
                .init(source: "어우", target: "哎哟"),
                .init(source: "헐", target: "不是吧"),
                .init(source: "야", target: "喂")
            ]
        case (.japanese, .english):
            return [
                .init(source: "えっと", target: "Um"),
                .init(source: "えーと", target: "Um"),
                .init(source: "ええと", target: "Um"),
                .init(source: "あの", target: "Um"),
                .init(source: "うーん", target: "Hmm"),
                .init(source: "う〜ん", target: "Hmm"),
                .init(source: "あぁ", target: "Ah"),
                .init(source: "あっ", target: "Oh"),
                .init(source: "えっ", target: "Huh"),
                .init(source: "ふふ", target: "Heh"),
                .init(source: "まあ", target: "Well"),
                .init(source: "ねえ", target: "Hey"),
                .init(source: "あら", target: "Oh"),
                .init(source: "うわ", target: "Wow"),
                .init(source: "きゃっ", target: "Eek")
            ]
        case (.english, .japanese):
            return [
                .init(source: "um", target: "うーん"),
                .init(source: "uh", target: "あの"),
                .init(source: "oh", target: "あっ"),
                .init(source: "hmm", target: "うーん"),
                .init(source: "ah", target: "ああ"),
                .init(source: "wow", target: "わあ"),
                .init(source: "hey", target: "ねえ"),
                .init(source: "yikes", target: "ひえっ")
            ]
        default:
            return []
        }
    }

    private static func sourceGuidance(
        source: SourceLanguage,
        target: TargetLanguage
    ) -> String {
        switch source {
        case .japanese:
            switch target {
            case .simplifiedChinese:
                return """
                For every Japanese filler use its natural Chinese counterpart: \
                えっと/あの→那个，うーん→嗯，あぁ→啊，まあ→嘛，ねえ→那个。 Sentence-final \
                particles need a counterpart too: ね→呢/吧，よ→啊/哦，な→啊，じゃん→嘛。 \
                Dropping a filler or particle is an error.
                """
            case .english:
                return """
                For every Japanese filler use its natural English counterpart: \
                えっと/あの→Um，うーん→Hmm，あぁ→Ah，まあ→Well，ねえ→Hey。 Sentence-final \
                particles need a counterpart too: ね→huh/right，よ→you know。 Dropping a \
                filler or particle is an error.
                """
            default:
                return ""
            }
        case .english:
            switch target {
            case .simplifiedChinese:
                return """
                For every English filler use its natural Chinese counterpart: \
                um→嗯，uh→呃，oh→哦，hmm→嗯，ah→啊，wow→哇。 Dropping a filler is an error.
                """
            case .japanese:
                return """
                For every English filler use its natural Japanese counterpart: \
                um→うーん，uh→あの，oh→あっ，hmm→うーん，wow→わあ。 Dropping a filler is an error.
                """
            default:
                return ""
            }
        case .korean:
            if target == .simplifiedChinese {
                return "For every Korean filler use its natural Chinese counterpart: 어→嗯，아→啊，음→嗯。 Dropping a filler is an error."
            }
            return ""
        case .chinese, .automatic:
            return ""
        }
    }
}

public struct QwenMTTerm: Equatable, Sendable, Encodable {
    public let source: String
    public let target: String

    public init(source: String, target: String) {
        self.source = source
        self.target = target
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
        terms: [QwenMTTerm] = [],
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
                terms: terms.isEmpty ? nil : terms,
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
        let terms: [QwenMTTerm]?
        let translationMemory: [QwenMTMemoryPair]?

        enum CodingKeys: String, CodingKey {
            case sourceLanguage = "source_lang"
            case targetLanguage = "target_lang"
            case domains
            case terms
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
