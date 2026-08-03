import Foundation

public struct Audio3ASREndpoint: Equatable, Sendable {
    public static let model = "qwen-audio-3.0-asr-flash-streaming"

    public let url: URL

    public init(workspaceID: String) throws {
        let pattern = "^[A-Za-z0-9][A-Za-z0-9-]{1,126}[A-Za-z0-9]$"
        guard workspaceID.range(of: pattern, options: .regularExpression) != nil else {
            throw LiveTranslateProtocolError.invalidWorkspaceID
        }

        var components = URLComponents()
        components.scheme = "wss"
        components.host = "\(workspaceID).cn-beijing.maas.aliyuncs.com"
        components.path = "/api-ws/v1/inference"

        guard let url = components.url else {
            throw LiveTranslateProtocolError.invalidEndpoint
        }
        self.url = url
    }
}

public enum Audio3ASRRequestEncoder {
    public static func runTask(
        taskID: String,
        sourceLanguage: SourceLanguage,
        context: String? = nil
    ) throws -> Data {
        let trimmedContext = context?.trimmingCharacters(in: .whitespacesAndNewlines)
        let request = Audio3RunTaskRequest(
            header: .init(taskID: taskID),
            payload: .init(
                parameters: .init(
                    languageHints: sourceLanguage == .automatic
                        ? nil
                        : [sourceLanguage.rawValue]
                ),
                input: .init(
                    context: trimmedContext.flatMap { text in
                        text.isEmpty ? nil : [.user(text)]
                    }
                )
            )
        )
        return try encoder.encode(request)
    }

    public static func finishTask(taskID: String) throws -> Data {
        try encoder.encode(Audio3FinishTaskRequest(header: .init(taskID: taskID)))
    }

    private static var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        return encoder
    }
}

public enum Audio3ASRServerEvent: Equatable, Sendable {
    case taskStarted
    case transcription(text: String, isFinal: Bool)
    case heartbeat
    case taskFinished
    case taskFailed(code: String, message: String)
    case ignored(type: String)

    public func subtitleEvent(sourceLanguage: SourceLanguage) -> LiveTranslateServerEvent {
        let reportedLanguage = sourceLanguage == .automatic ? nil : sourceLanguage.rawValue
        switch self {
        case .taskStarted:
            return .sessionCreated
        case let .transcription(text, isFinal):
            return isFinal
                ? .sourceFinal(text: text, language: reportedLanguage)
                : .sourceDraft(text: text, language: reportedLanguage)
        case .heartbeat:
            return .ignored(type: "heartbeat")
        case .taskFinished:
            return .sessionFinished
        case let .taskFailed(code, message):
            return .error(code: code, message: message)
        case let .ignored(type):
            return .ignored(type: type)
        }
    }
}

public enum Audio3ASRServerEventDecoder {
    public static func decode(_ text: String) throws -> Audio3ASRServerEvent {
        guard let data = text.data(using: .utf8) else {
            throw LiveTranslateProtocolError.invalidJSON
        }
        return try decode(data)
    }

    public static func decode(_ data: Data) throws -> Audio3ASRServerEvent {
        guard
            let object = try? JSONSerialization.jsonObject(with: data),
            let json = object as? [String: Any],
            let header = json["header"] as? [String: Any],
            let event = header["event"] as? String
        else {
            throw LiveTranslateProtocolError.invalidJSON
        }

        switch event {
        case "task-started":
            return .taskStarted
        case "task-finished":
            return .taskFinished
        case "task-failed":
            return .taskFailed(
                code: header["error_code"] as? String ?? "asr_task_failed",
                message: header["error_message"] as? String
                    ?? "Alibaba Cloud speech recognition failed."
            )
        case "result-generated":
            guard
                let payload = json["payload"] as? [String: Any],
                let output = payload["output"] as? [String: Any],
                let sentence = output["sentence"] as? [String: Any]
            else {
                return .ignored(type: event)
            }
            if sentence["heartbeat"] as? Bool == true {
                return .heartbeat
            }
            let text = (sentence["text"] as? String ?? "")
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else {
                return .ignored(type: "empty-result")
            }
            return .transcription(
                text: text,
                isFinal: sentence["sentence_end"] as? Bool == true
            )
        default:
            return .ignored(type: event)
        }
    }
}

public enum Audio3ASRContext {
    public static func audiovisualDialogue(for language: SourceLanguage) -> String {
        switch language {
        case .automatic:
            "Natural audiovisual dialogue, including interjections, breaths, gasps, moans, cries, laughter, and other vocalizations."
        case .chinese:
            "中文影视口语对白，包括语气词、停顿、喘息、呻吟、哭声、笑声和其他发声。"
        case .english:
            "Natural English audiovisual dialogue, including interjections, hesitations, breaths, gasps, moans, cries, laughter, and other vocalizations."
        case .japanese:
            "日本語の映像作品の自然な口語会話。感動詞、間投詞、息遣い、喘ぎ声、うめき声、泣き声、笑い声などの発声を含む。"
        case .korean:
            "한국어 영상 작품의 자연스러운 구어 대화. 감탄사, 머뭇거림, 숨소리, 신음, 울음, 웃음 등 발성을 포함함."
        }
    }
}

private struct Audio3RunTaskRequest: Encodable {
    let header: Header
    let payload: Payload

    struct Header: Encodable {
        let action = "run-task"
        let taskID: String
        let streaming = "duplex"

        enum CodingKeys: String, CodingKey {
            case action
            case taskID = "task_id"
            case streaming
        }
    }

    struct Payload: Encodable {
        let taskGroup = "audio"
        let task = "asr"
        let function = "recognition"
        let model = Audio3ASREndpoint.model
        let parameters: Parameters
        let input: Input

        enum CodingKeys: String, CodingKey {
            case taskGroup = "task_group"
            case task
            case function
            case model
            case parameters
            case input
        }
    }

    struct Parameters: Encodable {
        let format = "pcm"
        let sampleRate = 16_000
        let languageHints: [String]?
        let semanticPunctuationEnabled = true
        let heartbeat = true

        enum CodingKeys: String, CodingKey {
            case format
            case sampleRate = "sample_rate"
            case languageHints = "language_hints"
            case semanticPunctuationEnabled = "semantic_punctuation_enabled"
            case heartbeat
        }
    }

    struct Input: Encodable {
        let context: [ContextMessage]?
    }

    struct ContextMessage: Encodable {
        let role: String
        let content: [Content]

        static func user(_ text: String) -> Self {
            .init(role: "user", content: [.init(type: "input_text", text: text)])
        }

        struct Content: Encodable {
            let type: String
            let text: String
        }
    }
}

private struct Audio3FinishTaskRequest: Encodable {
    let header: Header
    let payload = Payload()

    struct Header: Encodable {
        let action = "finish-task"
        let taskID: String
        let streaming = "duplex"

        enum CodingKeys: String, CodingKey {
            case action
            case taskID = "task_id"
            case streaming
        }
    }

    struct Payload: Encodable {
        let input = Input()
    }

    struct Input: Encodable {}
}
