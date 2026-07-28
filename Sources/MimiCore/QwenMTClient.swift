import Foundation

public enum QwenMTClientError: Error, LocalizedError, Equatable, Sendable {
    case missingAPIKey
    case invalidHTTPResponse
    case requestFailed(statusCode: Int, message: String)

    public var errorDescription: String? {
        switch self {
        case .missingAPIKey:
            "Add an Alibaba Cloud Model Studio API key in Settings."
        case .invalidHTTPResponse:
            "Qwen-MT returned an invalid HTTP response."
        case let .requestFailed(statusCode, message):
            message.isEmpty ? "Qwen-MT request failed with HTTP \(statusCode)." : message
        }
    }

    public var isAuthenticationFailure: Bool {
        switch self {
        case let .requestFailed(statusCode, _):
            statusCode == 401 || statusCode == 403
        case .missingAPIKey:
            true
        case .invalidHTTPResponse:
            false
        }
    }
}

public actor QwenMTClient {
    private let endpoint: QwenMTEndpoint
    private let apiKey: String
    private let sourceLanguage: SourceLanguage
    private let targetLanguage: TargetLanguage
    private let model: QwenMTModel
    private let domainHint: String?
    private let session: URLSession

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage = .simplifiedChinese,
        model: QwenMTModel = .lite,
        domainHint: String? = nil,
        session: URLSession = .shared
    ) throws {
        let trimmedKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedKey.isEmpty else {
            throw QwenMTClientError.missingAPIKey
        }

        self.endpoint = try QwenMTEndpoint(workspaceID: workspaceID)
        self.apiKey = trimmedKey
        self.sourceLanguage = sourceLanguage
        self.targetLanguage = targetLanguage
        self.model = model
        self.domainHint = domainHint
        self.session = session
    }

    public func translate(
        _ text: String,
        sourceLanguageOverride: SourceLanguage? = nil,
        translationMemory: [QwenMTMemoryPair] = []
    ) async throws -> String {
        let request = try makeRequest(
            text: text,
            sourceLanguageOverride: sourceLanguageOverride,
            stream: false,
            translationMemory: translationMemory
        )

        let (data, response) = try await session.data(for: request)
        try Self.validate(response: response, errorData: data)

        return try QwenMTResponseDecoder.decode(data)
    }

    public func translateStreaming(
        _ text: String,
        sourceLanguageOverride: SourceLanguage? = nil,
        translationMemory: [QwenMTMemoryPair] = [],
        onPartial: @escaping @Sendable (String) async -> Void
    ) async throws -> String {
        var request = try makeRequest(
            text: text,
            sourceLanguageOverride: sourceLanguageOverride,
            stream: true,
            translationMemory: translationMemory
        )
        request.setValue("text/event-stream", forHTTPHeaderField: "Accept")

        let (bytes, response) = try await session.bytes(for: request)
        try Self.validate(response: response, errorData: Data())

        var translation = ""
        for try await line in bytes.lines {
            try Task.checkCancellation()
            guard line.hasPrefix("data:") else { continue }

            let payload = String(line.dropFirst(5))
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !payload.isEmpty, payload != "[DONE]" else {
                if payload == "[DONE]" { break }
                continue
            }

            guard let data = payload.data(using: .utf8) else {
                throw QwenMTProtocolError.invalidJSON
            }
            if let content = try QwenMTStreamDecoder.decodeChunk(data), !content.isEmpty {
                translation += content
                await onPartial(translation)
            }
        }

        let trimmedTranslation = translation.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedTranslation.isEmpty else {
            throw QwenMTProtocolError.missingTranslation
        }
        return trimmedTranslation
    }

    private func makeRequest(
        text: String,
        sourceLanguageOverride: SourceLanguage?,
        stream: Bool,
        translationMemory: [QwenMTMemoryPair]
    ) throws -> URLRequest {
        let trimmedText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedText.isEmpty else {
            throw QwenMTProtocolError.missingTranslation
        }

        var request = URLRequest(url: endpoint.url)
        request.httpMethod = "POST"
        request.timeoutInterval = 10
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try QwenMTRequestEncoder.request(
            text: trimmedText,
            sourceLanguage: sourceLanguageOverride ?? sourceLanguage,
            targetLanguage: targetLanguage,
            model: model,
            stream: stream,
            domainHint: domainHint,
            translationMemory: translationMemory
        )
        return request
    }

    private static func validate(response: URLResponse, errorData: Data) throws {
        guard let httpResponse = response as? HTTPURLResponse else {
            throw QwenMTClientError.invalidHTTPResponse
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            throw QwenMTClientError.requestFailed(
                statusCode: httpResponse.statusCode,
                message: errorMessage(in: errorData)
            )
        }
    }

    private static func errorMessage(in data: Data) -> String {
        guard
            let object = try? JSONSerialization.jsonObject(with: data),
            let json = object as? [String: Any],
            let error = json["error"] as? [String: Any],
            let message = error["message"] as? String
        else {
            return ""
        }
        return message
    }
}
