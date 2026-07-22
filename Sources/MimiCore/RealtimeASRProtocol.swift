import Foundation

public struct RealtimeASREndpoint: Equatable, Sendable {
    public static let model = "qwen3-asr-flash-realtime"

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

public enum RealtimeASRRequestEncoder {
    public static func sessionUpdate(
        sourceLanguage: SourceLanguage,
        eventID: String? = nil
    ) throws -> Data {
        try encoder.encode(
            ASRSessionUpdateRequest(
                eventID: eventID ?? ASREventID.next(),
                session: .init(
                    inputAudioFormat: "pcm",
                    sampleRate: 16_000,
                    inputAudioTranscription: .init(language: sourceLanguage.rawValue),
                    turnDetection: .init(
                        type: "server_vad",
                        threshold: 0.0,
                        silenceDurationMilliseconds: 400
                    )
                )
            )
        )
    }

    public static func audioAppend(
        _ pcmData: Data,
        eventID: String? = nil
    ) throws -> Data {
        try encoder.encode(
            ASRAudioAppendRequest(
                eventID: eventID ?? ASREventID.next(),
                audio: pcmData.base64EncodedString()
            )
        )
    }

    public static func finish(eventID: String? = nil) throws -> Data {
        try encoder.encode(ASRFinishRequest(eventID: eventID ?? ASREventID.next()))
    }

    private static var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        return encoder
    }
}

public enum RealtimeASRServerEventDecoder {
    public static func decode(_ text: String) throws -> LiveTranslateServerEvent {
        try LiveTranslateServerEvent.decode(text)
    }

    public static func decode(_ data: Data) throws -> LiveTranslateServerEvent {
        try LiveTranslateServerEvent.decode(data)
    }
}

private struct ASRSessionUpdateRequest: Encodable {
    let eventID: String
    let type = "session.update"
    let session: Session

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case session
    }

    struct Session: Encodable {
        let inputAudioFormat: String
        let sampleRate: Int
        let inputAudioTranscription: Transcription
        let turnDetection: TurnDetection

        enum CodingKeys: String, CodingKey {
            case inputAudioFormat = "input_audio_format"
            case sampleRate = "sample_rate"
            case inputAudioTranscription = "input_audio_transcription"
            case turnDetection = "turn_detection"
        }
    }

    struct Transcription: Encodable {
        let language: String
    }

    struct TurnDetection: Encodable {
        let type: String
        let threshold: Double
        let silenceDurationMilliseconds: Int

        enum CodingKeys: String, CodingKey {
            case type
            case threshold
            case silenceDurationMilliseconds = "silence_duration_ms"
        }
    }
}

private struct ASRAudioAppendRequest: Encodable {
    let eventID: String
    let type = "input_audio_buffer.append"
    let audio: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
        case audio
    }
}

private struct ASRFinishRequest: Encodable {
    let eventID: String
    let type = "session.finish"

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case type
    }
}

private enum ASREventID {
    static func next() -> String {
        "event_\(UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased())"
    }
}
