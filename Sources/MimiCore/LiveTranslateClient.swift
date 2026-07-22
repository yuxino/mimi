import Foundation

public enum LiveTranslateClientError: Error, LocalizedError, Equatable, Sendable {
    case missingAPIKey
    case notConnected
    case unsupportedMessage

    public var errorDescription: String? {
        switch self {
        case .missingAPIKey:
            "Add an Alibaba Cloud Model Studio API key in Settings."
        case .notConnected:
            "The live translation session is not connected."
        case .unsupportedMessage:
            "The live translation service returned an unsupported WebSocket message."
        }
    }
}

public actor LiveTranslateClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private let endpoint: LiveTranslateEndpoint
    private let apiKey: String
    private let sourceLanguage: SourceLanguage
    private let hotwords: [String: String]
    private let session: URLSession

    private var socket: URLSessionWebSocketTask?
    private var receiveTask: Task<Void, Never>?
    private var eventHandler: EventHandler?
    private var receivedSessionFinished = false

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        hotwords: [String: String] = [:],
        session: URLSession = .shared
    ) throws {
        let trimmedKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedKey.isEmpty else {
            throw LiveTranslateClientError.missingAPIKey
        }

        self.endpoint = try LiveTranslateEndpoint(workspaceID: workspaceID)
        self.apiKey = trimmedKey
        self.sourceLanguage = sourceLanguage
        self.hotwords = hotwords
        self.session = session
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        disconnect()

        var request = URLRequest(url: endpoint.url)
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.timeoutInterval = 15

        let socket = session.webSocketTask(with: request)
        self.socket = socket
        self.eventHandler = onEvent
        self.receivedSessionFinished = false
        socket.resume()

        let configuration = try LiveTranslateRequestEncoder.sessionUpdate(
            sourceLanguage: sourceLanguage,
            hotwords: hotwords
        )
        try await send(configuration, on: socket)

        receiveTask = Task { [weak self] in
            await self?.receiveLoop()
        }
    }

    public func sendAudio(_ pcmData: Data) async throws {
        guard let socket else {
            throw LiveTranslateClientError.notConnected
        }
        guard !pcmData.isEmpty else { return }

        let message = try LiveTranslateRequestEncoder.audioAppend(pcmData)
        try await send(message, on: socket)
    }

    public func finish(timeout: Duration = .seconds(2)) async {
        guard let socket else { return }

        do {
            let message = try LiveTranslateRequestEncoder.finish()
            try await send(message, on: socket)

            let clock = ContinuousClock()
            let deadline = clock.now.advanced(by: timeout)
            while !receivedSessionFinished, clock.now < deadline {
                try? await Task.sleep(for: .milliseconds(50))
            }
        } catch {
            await emit(.error(code: "session_finish_failed", message: error.localizedDescription))
        }

        disconnect()
    }

    public func disconnect() {
        receiveTask?.cancel()
        receiveTask = nil
        socket?.cancel(with: .goingAway, reason: nil)
        socket = nil
        eventHandler = nil
        receivedSessionFinished = false
    }

    private func send(_ data: Data, on socket: URLSessionWebSocketTask) async throws {
        try await socket.send(.string(String(decoding: data, as: UTF8.self)))
    }

    private func receiveLoop() async {
        guard let socket else { return }

        while !Task.isCancelled {
            do {
                let message = try await socket.receive()
                let event: LiveTranslateServerEvent

                switch message {
                case let .string(text):
                    event = try LiveTranslateServerEvent.decode(text)
                case let .data(data):
                    event = try LiveTranslateServerEvent.decode(data)
                @unknown default:
                    throw LiveTranslateClientError.unsupportedMessage
                }

                if event == .sessionFinished {
                    receivedSessionFinished = true
                }
                await emit(event)
            } catch {
                guard !Task.isCancelled else { return }
                await emit(.error(code: "transport_error", message: error.localizedDescription))
                return
            }
        }
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}
