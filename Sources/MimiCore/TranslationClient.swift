import Foundation

public actor TranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private enum Backend {
        case lowLatency(LowLatencyTranslationClient)
        case highQuality(LiveTranslateClient)
    }

    private let backend: Backend

    public init(
        configuration: LiveTranslationConfiguration,
        session: URLSession = .shared
    ) throws {
        switch configuration.effectiveTranslationMode {
        case .lowLatency:
            self.backend = .lowLatency(
                try LowLatencyTranslationClient(
                    workspaceID: configuration.workspaceID,
                    apiKey: configuration.apiKey,
                    sourceLanguage: configuration.sourceLanguage,
                    session: session
                )
            )
        case .highQuality:
            self.backend = .highQuality(
                try LiveTranslateClient(
                    workspaceID: configuration.workspaceID,
                    apiKey: configuration.apiKey,
                    sourceLanguage: configuration.sourceLanguage,
                    session: session
                )
            )
        }
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        switch backend {
        case let .lowLatency(client):
            try await client.connect(onEvent: onEvent)
        case let .highQuality(client):
            try await client.connect(onEvent: onEvent)
        }
    }

    public func sendAudio(_ pcmData: Data) async throws {
        switch backend {
        case let .lowLatency(client):
            try await client.sendAudio(pcmData)
        case let .highQuality(client):
            try await client.sendAudio(pcmData)
        }
    }

    public func ping(timeout: Duration = .seconds(4)) async throws {
        switch backend {
        case let .lowLatency(client):
            try await client.ping(timeout: timeout)
        case let .highQuality(client):
            try await client.ping(timeout: timeout)
        }
    }

    public func finish() async {
        switch backend {
        case let .lowLatency(client):
            await client.finish()
        case let .highQuality(client):
            await client.finish()
        }
    }

    public func disconnect() async {
        switch backend {
        case let .lowLatency(client):
            await client.disconnect()
        case let .highQuality(client):
            await client.disconnect()
        }
    }
}
