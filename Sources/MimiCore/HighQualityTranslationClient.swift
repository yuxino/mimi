import Foundation

public actor HighQualityTranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private struct TranslationRequest: Sendable {
        let text: String
        let language: String?
        let enqueuedAt: ContinuousClock.Instant
    }

    private let asrClient: Audio3ASRClient
    private let mtClient: QwenMTClient
    private let sourceLanguage: SourceLanguage
    private let translatesAudio: Bool
    private let clock = ContinuousClock()
    private var eventHandler: EventHandler?
    private var finalQueue: [TranslationRequest] = []
    private var translationMemory: [QwenMTMemoryPair] = []
    private var finalWorker: Task<Void, Never>?

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage = .simplifiedChinese,
        session: URLSession = .shared
    ) throws {
        self.asrClient = try Audio3ASRClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            session: session
        )
        self.mtClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            model: .plus,
            domainHint: QwenMTDomainHint.spokenDialogue(for: targetLanguage),
            session: session
        )
        self.sourceLanguage = sourceLanguage
        self.translatesAudio = targetLanguage.translatesAudio
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        cancelFinalTranslations()
        eventHandler = onEvent
        try await asrClient.connect { [weak self] event in
            await self?.handleASREvent(event)
        }
    }

    public func sendAudio(_ pcmData: Data) async throws {
        try await asrClient.sendAudio(pcmData)
    }

    public func ping(timeout: Duration = .seconds(4)) async throws {
        try await asrClient.ping(timeout: timeout)
    }

    public func finish() async {
        await asrClient.finish()
        await waitForFinalTranslations(timeout: .seconds(35))
        cancelFinalTranslations()
        eventHandler = nil
    }

    public func disconnect() async {
        cancelFinalTranslations()
        eventHandler = nil
        await asrClient.disconnect()
    }

    private func handleASREvent(_ event: LiveTranslateServerEvent) async {
        switch event {
        case let .sourceDraft(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            PipelineDiagnostics.log(
                "audio3 asr draft length=\(text.count) language=\(language ?? sourceLanguage.rawValue)"
            )
            if !translatesAudio {
                await emit(.sourceDraft(text: text, language: language))
                await emit(.translationDraft(text))
            } else if finalWorker == nil, finalQueue.isEmpty {
                // While Plus is translating a confirmed sentence, keep that
                // sentence visible instead of pairing its translation with a
                // draft from the next sentence.
                await emit(.sourceDraft(text: text, language: language))
            }

        case let .sourceFinal(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            PipelineDiagnostics.log(
                "audio3 asr final length=\(text.count) language=\(language ?? sourceLanguage.rawValue) queuedFinals=\(finalQueue.count)"
            )
            guard translatesAudio else {
                await emit(.sourceFinal(text: text, language: language))
                await emit(.translationFinal(text))
                return
            }
            finalQueue.append(.init(text: text, language: language, enqueuedAt: clock.now))
            PipelineDiagnostics.log("mt plus final enqueued depth=\(finalQueue.count)")
            startFinalWorkerIfNeeded()

        default:
            await emit(event)
        }
    }

    private func startFinalWorkerIfNeeded() {
        guard finalWorker == nil else { return }
        finalWorker = Task { [weak self] in
            await self?.runFinalWorker()
        }
    }

    private func runFinalWorker() async {
        while !Task.isCancelled, !finalQueue.isEmpty {
            let request = finalQueue.removeFirst()
            let startedAt = clock.now
            PipelineDiagnostics.log(
                "mt plus final started waitMs=\(PipelineDiagnostics.milliseconds(request.enqueuedAt.duration(to: startedAt))) remaining=\(finalQueue.count)"
            )
            await emit(.sourceFinal(text: request.text, language: request.language))

            do {
                let translation = try await translateWithOneRetry(request.text)
                guard !Task.isCancelled else { return }
                PipelineDiagnostics.log(
                    "mt plus final completed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) remaining=\(finalQueue.count)"
                )
                await emit(.translationFinal(translation))
                remember(source: request.text, translation: translation)
            } catch is CancellationError {
                return
            } catch {
                PipelineDiagnostics.log(
                    "mt plus final failed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) error=\(PipelineDiagnostics.errorLabel(error))"
                )
                if await handleTranslationFailure(error) {
                    return
                }
                await emit(.translationFinal(""))
            }
        }

        finalWorker = nil
        if !finalQueue.isEmpty {
            startFinalWorkerIfNeeded()
        }
    }

    private func handleTranslationFailure(_ error: Error) async -> Bool {
        guard
            let clientError = error as? QwenMTClientError,
            clientError.isAuthenticationFailure
        else {
            return false
        }
        await emit(
            .error(
                code: "translation_authentication_failed",
                message: clientError.localizedDescription
            )
        )
        return true
    }

    private func translateWithOneRetry(_ text: String) async throws -> String {
        let memory = Array(translationMemory.suffix(3))
        do {
            return try await mtClient.translate(text, translationMemory: memory)
        } catch {
            guard shouldRetry(error) else { throw error }
            PipelineDiagnostics.log(
                "mt plus final retrying error=\(PipelineDiagnostics.errorLabel(error))"
            )
            try await Task.sleep(for: .milliseconds(600))
            return try await mtClient.translate(text, translationMemory: memory)
        }
    }

    private func shouldRetry(_ error: Error) -> Bool {
        guard let clientError = error as? QwenMTClientError else { return false }
        switch clientError {
        case .requestTimedOut, .invalidHTTPResponse:
            return true
        case let .requestFailed(statusCode, _):
            return statusCode == 408 || statusCode == 429 || statusCode >= 500
        case .missingAPIKey:
            return false
        }
    }

    private func remember(source: String, translation: String) {
        translationMemory.removeAll { $0.source == source }
        translationMemory.append(.init(source: source, target: translation))
        if translationMemory.count > 6 {
            translationMemory.removeFirst(translationMemory.count - 6)
        }
    }

    private func cancelFinalTranslations() {
        finalWorker?.cancel()
        finalWorker = nil
        finalQueue = []
        translationMemory = []
    }

    private func waitForFinalTranslations(timeout: Duration) async {
        let deadline = clock.now.advanced(by: timeout)
        while finalWorker != nil, clock.now < deadline {
            try? await Task.sleep(for: .milliseconds(50))
        }
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}
