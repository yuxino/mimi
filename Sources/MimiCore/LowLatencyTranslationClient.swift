import Foundation

public actor LowLatencyTranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private let asrClient: RealtimeASRClient
    private let mtClient: QwenMTClient
    private var eventHandler: EventHandler?
    private var translationTask: Task<Void, Never>?
    private var translationGeneration = 0
    private var lastDraftText = ""

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        session: URLSession = .shared
    ) throws {
        self.asrClient = try RealtimeASRClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            session: session
        )
        self.mtClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            session: session
        )
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        cancelPendingTranslation()
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
        translationTask?.cancel()
        translationTask = nil
        await asrClient.finish()
        eventHandler = nil
        lastDraftText = ""
    }

    public func disconnect() async {
        cancelPendingTranslation()
        eventHandler = nil
        await asrClient.disconnect()
    }

    private func handleASREvent(_ event: LiveTranslateServerEvent) async {
        switch event {
        case let .sourceDraft(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            if lastDraftText.isEmpty {
                await emit(.translationDraft(""))
            }
            await emit(.sourceDraft(text: text, language: language))
            scheduleDraftTranslation(for: text)

        case let .sourceFinal(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            if lastDraftText.isEmpty {
                await emit(.translationDraft(""))
            }
            await emit(.sourceFinal(text: text, language: language))
            await translateFinal(text)

        default:
            await emit(event)
        }
    }

    private func scheduleDraftTranslation(for text: String) {
        guard text != lastDraftText else { return }
        lastDraftText = text
        translationGeneration += 1
        let generation = translationGeneration

        translationTask?.cancel()
        translationTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(200))
            } catch {
                return
            }
            guard !Task.isCancelled else { return }
            await self?.translateDraft(text, generation: generation)
        }
    }

    private func translateDraft(_ text: String, generation: Int) async {
        do {
            let translation = try await mtClient.translateStreaming(text) { [weak self] partial in
                await self?.emitStreamingDraft(partial, generation: generation)
            }
            guard generation == translationGeneration, !Task.isCancelled else { return }
            await emit(.translationDraft(translation))
        } catch is CancellationError {
            return
        } catch {
            await handleTranslationFailure(error)
        }
    }

    private func translateFinal(_ text: String) async {
        translationGeneration += 1
        let generation = translationGeneration
        translationTask?.cancel()
        translationTask = nil
        lastDraftText = ""

        do {
            let translation = try await mtClient.translateStreaming(text) { [weak self] partial in
                await self?.emitStreamingDraft(partial, generation: generation)
            }
            guard generation == translationGeneration else { return }
            await emit(.translationFinal(translation))
        } catch is CancellationError {
            return
        } catch {
            await handleTranslationFailure(error)
        }
    }

    private func emitStreamingDraft(_ translation: String, generation: Int) async {
        guard generation == translationGeneration else { return }
        await emit(.translationDraft(translation))
    }

    private func handleTranslationFailure(_ error: Error) async {
        guard
            let clientError = error as? QwenMTClientError,
            clientError.isAuthenticationFailure
        else {
            return
        }

        await emit(
            .error(
                code: "translation_authentication_failed",
                message: clientError.localizedDescription
            )
        )
    }

    private func cancelPendingTranslation() {
        translationGeneration += 1
        translationTask?.cancel()
        translationTask = nil
        lastDraftText = ""
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}
