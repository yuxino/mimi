import Foundation

public actor Audio3ASRClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private let endpoint: Audio3ASREndpoint
    private let apiKey: String
    private let sourceLanguage: SourceLanguage
    private let session: URLSession

    private var socket: URLSessionWebSocketTask?
    private var receiveTask: Task<Void, Never>?
    private var eventHandler: EventHandler?
    private var taskID: String?
    private var taskStarted = false
    private var taskFinished = false
    private var terminalError: Error?

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        session: URLSession = .shared
    ) throws {
        let trimmedKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedKey.isEmpty else {
            throw LiveTranslateClientError.missingAPIKey
        }

        self.endpoint = try Audio3ASREndpoint(workspaceID: workspaceID)
        self.apiKey = trimmedKey
        self.sourceLanguage = sourceLanguage
        self.session = session
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        disconnect()

        var request = URLRequest(url: endpoint.url)
        request.setValue("Bearer \(apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("mimi-macos", forHTTPHeaderField: "User-Agent")
        request.timeoutInterval = 15

        let socket = session.webSocketTask(with: request)
        let taskID = UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
        self.socket = socket
        self.eventHandler = onEvent
        self.taskID = taskID
        self.taskStarted = false
        self.taskFinished = false
        self.terminalError = nil
        socket.resume()

        receiveTask = Task { [weak self] in
            await self?.receiveLoop()
        }

        let command = try Audio3ASRRequestEncoder.runTask(
            taskID: taskID,
            sourceLanguage: sourceLanguage,
            context: Audio3ASRContext.audiovisualDialogue(for: sourceLanguage)
        )
        try await sendText(command, on: socket)
        try await waitForTaskStart(timeout: .seconds(10))
    }

    public func sendAudio(_ pcmData: Data) async throws {
        guard let socket, taskStarted else {
            throw LiveTranslateClientError.notConnected
        }
        guard !pcmData.isEmpty else { return }

        try await socket.send(.data(pcmData))
    }

    public func ping(timeout: Duration = .seconds(4)) async throws {
        guard let socket else {
            throw LiveTranslateClientError.notConnected
        }

        let completion = Audio3PingCompletion()
        try await withCheckedThrowingContinuation { continuation in
            socket.sendPing { error in
                if let error {
                    completion.resume(throwing: error, continuation: continuation)
                } else {
                    completion.resume(returning: (), continuation: continuation)
                }
            }

            Task {
                try? await Task.sleep(for: timeout)
                completion.resume(
                    throwing: LiveTranslateClientError.healthCheckTimedOut,
                    continuation: continuation
                )
            }
        }
    }

    public func finish(timeout: Duration = .seconds(3)) async {
        guard let socket, let taskID else { return }

        do {
            let command = try Audio3ASRRequestEncoder.finishTask(taskID: taskID)
            try await sendText(command, on: socket)

            let clock = ContinuousClock()
            let deadline = clock.now.advanced(by: timeout)
            while !taskFinished, terminalError == nil, clock.now < deadline {
                try? await Task.sleep(for: .milliseconds(50))
            }
        } catch {
            await emit(.error(code: "task_finish_failed", message: error.localizedDescription))
        }

        disconnect()
    }

    public func disconnect() {
        receiveTask?.cancel()
        receiveTask = nil
        socket?.cancel(with: .goingAway, reason: nil)
        socket = nil
        eventHandler = nil
        taskID = nil
        taskStarted = false
        taskFinished = false
        terminalError = nil
    }

    private func waitForTaskStart(timeout: Duration) async throws {
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: timeout)
        while !taskStarted, terminalError == nil, clock.now < deadline {
            try await Task.sleep(for: .milliseconds(25))
        }
        if let terminalError {
            throw terminalError
        }
        guard taskStarted else {
            throw LiveTranslateClientError.healthCheckTimedOut
        }
    }

    private func sendText(_ data: Data, on socket: URLSessionWebSocketTask) async throws {
        try await socket.send(.string(String(decoding: data, as: UTF8.self)))
    }

    private func receiveLoop() async {
        guard let socket else { return }

        while !Task.isCancelled {
            do {
                let message = try await socket.receive()
                let event: Audio3ASRServerEvent

                switch message {
                case let .string(text):
                    event = try Audio3ASRServerEventDecoder.decode(text)
                case let .data(data):
                    event = try Audio3ASRServerEventDecoder.decode(data)
                @unknown default:
                    throw LiveTranslateClientError.unsupportedMessage
                }

                switch event {
                case .taskStarted:
                    taskStarted = true
                case .taskFinished:
                    taskFinished = true
                case let .taskFailed(code, message):
                    terminalError = Audio3ASRTaskError(code: code, message: message)
                default:
                    break
                }
                await emit(event.subtitleEvent(sourceLanguage: sourceLanguage))
                if case .taskFailed = event {
                    return
                }
            } catch {
                guard !Task.isCancelled else { return }
                terminalError = error
                await emit(.error(code: "transport_error", message: error.localizedDescription))
                return
            }
        }
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}

private struct Audio3ASRTaskError: LocalizedError, Sendable {
    let code: String
    let message: String

    var errorDescription: String? {
        message.isEmpty ? "Alibaba Cloud speech recognition failed (\(code))." : message
    }
}

private final class Audio3PingCompletion: @unchecked Sendable {
    private let lock = NSLock()
    private var didResume = false

    func resume(
        returning value: Void,
        continuation: CheckedContinuation<Void, any Error>
    ) {
        guard claimContinuation() else { return }
        continuation.resume(returning: value)
    }

    func resume(
        throwing error: any Error,
        continuation: CheckedContinuation<Void, any Error>
    ) {
        guard claimContinuation() else { return }
        continuation.resume(throwing: error)
    }

    private func claimContinuation() -> Bool {
        lock.withLock {
            guard !didResume else { return false }
            didResume = true
            return true
        }
    }
}
