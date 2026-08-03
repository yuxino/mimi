import Foundation

public enum LiveTranslateProtocolError: Error, Equatable, Sendable {
    case invalidWorkspaceID
    case invalidEndpoint
    case invalidJSON
    case missingEventType
}

public struct LiveTranslateEndpoint: Equatable, Sendable {
    public static let model = "qwen3.5-livetranslate-flash-realtime"

    public let url: URL

    public init(workspaceID: String) throws {
        let pattern = "^[A-Za-z0-9][A-Za-z0-9-]{1,126}[A-Za-z0-9]$"
        guard workspaceID.range(of: pattern, options: .regularExpression) != nil else {
            throw LiveTranslateProtocolError.invalidWorkspaceID
        }

        var components = URLComponents()
        components.scheme = "wss"
        components.host = "\(workspaceID).cn-beijing.maas.aliyuncs.com"
        components.path = "/api-ws/v1/realtime"
        components.queryItems = [URLQueryItem(name: "model", value: Self.model)]

        guard let url = components.url else {
            throw LiveTranslateProtocolError.invalidEndpoint
        }
        self.url = url
    }
}

public enum LiveTranslateRequestEncoder {
    public static func sessionUpdate(
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage = .simplifiedChinese,
        hotwords: [String: String] = [:],
        eventID: String? = nil
    ) throws -> Data {
        let request = SessionUpdateRequest(
            eventID: eventID ?? EventID.next(),
            session: .init(
                modalities: ["text"],
                sampleRate: 16_000,
                inputAudioFormat: "pcm",
                inputAudioTranscription: .init(
                    model: "qwen3-asr-flash-realtime",
                    language: sourceLanguage.rawValue
                ),
                translation: .init(
                    language: targetLanguage.rawValue,
                    corpus: hotwords.isEmpty ? nil : .init(phrases: hotwords)
                )
            )
        )
        return try encoder.encode(request)
    }

    public static func audioAppend(
        _ pcmData: Data,
        eventID: String? = nil
    ) throws -> Data {
        try encoder.encode(
            AudioAppendRequest(eventID: eventID ?? EventID.next(), audio: pcmData.base64EncodedString())
        )
    }

    public static func finish(eventID: String? = nil) throws -> Data {
        try encoder.encode(FinishRequest(eventID: eventID ?? EventID.next()))
    }

    private static var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        return encoder
    }
}

public enum LiveTranslateServerEvent: Equatable, Sendable {
    case sessionCreated
    case sessionUpdated
    case sourceDraft(text: String, language: String?)
    case sourceFinal(text: String, language: String?)
    case translationStarted
    case translationDraft(String)
    case translationFinal(String)
    case sessionFinished
    case error(code: String, message: String)
    case ignored(type: String)

    public static func decode(_ text: String) throws -> Self {
        guard let data = text.data(using: .utf8) else {
            throw LiveTranslateProtocolError.invalidJSON
        }
        return try decode(data)
    }

    public static func decode(_ data: Data) throws -> Self {
        guard
            let object = try? JSONSerialization.jsonObject(with: data),
            let json = object as? [String: Any]
        else {
            throw LiveTranslateProtocolError.invalidJSON
        }
        guard let type = json["type"] as? String else {
            throw LiveTranslateProtocolError.missingEventType
        }

        switch type {
        case "session.created":
            return .sessionCreated
        case "session.updated":
            return .sessionUpdated
        case "session.finished":
            return .sessionFinished

        case "conversation.item.input_audio_transcription.text":
            return .sourceDraft(
                text: combinedText(in: json),
                language: json["language"] as? String
            )

        case "conversation.item.input_audio_transcription.completed":
            return .sourceFinal(
                text: (json["transcript"] as? String ?? "").trimmed,
                language: json["language"] as? String
            )

        case "response.text.text", "response.audio_transcript.text":
            return .translationDraft(combinedText(in: json))

        case "response.text.done":
            return .translationFinal((json["text"] as? String ?? "").trimmed)

        case "response.audio_transcript.done":
            return .translationFinal((json["transcript"] as? String ?? "").trimmed)

        case "error":
            let error = json["error"] as? [String: Any]
            return .error(
                code: error?["code"] as? String ?? "unknown_error",
                message: error?["message"] as? String ?? "Alibaba Cloud returned an unknown error."
            )

        default:
            return .ignored(type: type)
        }
    }

    private static func combinedText(in json: [String: Any]) -> String {
        let confirmed = json["text"] as? String ?? ""
        let tentative = json["stash"] as? String ?? ""
        return (confirmed + tentative).trimmed
    }
}

private struct SessionUpdateRequest: Encodable {
    let eventID: String
    let type = "session.update"
    let session: Session

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case session
    }

    struct Session: Encodable {
        let modalities: [String]
        let sampleRate: Int
        let inputAudioFormat: String
        let inputAudioTranscription: Transcription
        let translation: Translation

        enum CodingKeys: String, CodingKey {
            case modalities
            case sampleRate = "sample_rate"
            case inputAudioFormat = "input_audio_format"
            case inputAudioTranscription = "input_audio_transcription"
            case translation
        }
    }

    struct Transcription: Encodable {
        let model: String
        let language: String
    }

    struct Translation: Encodable {
        let language: String
        let corpus: Corpus?
    }

    struct Corpus: Encodable {
        let phrases: [String: String]
    }
}

private struct AudioAppendRequest: Encodable {
    let eventID: String
    let type = "input_audio_buffer.append"
    let audio: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case audio
    }
}

private struct FinishRequest: Encodable {
    let eventID: String
    let type = "session.finish"

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
    }
}

private enum EventID {
    static func next() -> String {
        "event_\(UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased())"
    }
}

private extension String {
    var trimmed: String {
        trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
