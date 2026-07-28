import Foundation

public actor LowLatencyTranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private let asrClient: RealtimeASRClient
    private let mtClient: QwenMTClient
    private var eventHandler: EventHandler?
    private var draftWorker: Task<Void, Never>?
    private var preemptionTask: Task<Void, Never>?
    private var pendingDraft: String?
    private var finalQueue: [String] = []
    private var finalWorker: Task<Void, Never>?
    private var draftTranslationGeneration = 0
    private var draftWorkerGeneration = 0
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
        cancelPendingTranslation()
        cancelFinalTranslations()
        await asrClient.finish()
        eventHandler = nil
    }

    public func disconnect() async {
        cancelPendingTranslation()
        cancelFinalTranslations()
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
            enqueueDraftTranslation(text)

        case let .sourceFinal(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            if lastDraftText.isEmpty {
                await emit(.translationDraft(""))
            }
            await emit(.sourceFinal(text: text, language: language))
            enqueueFinalTranslation(text)

        default:
            await emit(event)
        }
    }

    private func enqueueDraftTranslation(_ text: String) {
        guard text != lastDraftText else { return }
        lastDraftText = text
        pendingDraft = text

        guard draftWorker != nil else {
            startDraftWorker()
            return
        }

        guard preemptionTask == nil else { return }
        preemptionTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(450))
            guard !Task.isCancelled else { return }
            await self?.preemptStaleDraft()
        }
    }

    private func startDraftWorker() {
        draftWorkerGeneration += 1
        let workerGeneration = draftWorkerGeneration
        draftWorker = Task { [weak self] in
            await self?.runDraftWorker(workerGeneration: workerGeneration)
        }
    }

    private func runDraftWorker(workerGeneration: Int) async {
        while !Task.isCancelled, let text = pendingDraft {
            pendingDraft = nil
            preemptionTask?.cancel()
            preemptionTask = nil
            draftTranslationGeneration += 1
            let generation = draftTranslationGeneration

            do {
                let translation = try await mtClient.translateStreaming(text) { _ in }
                guard !Task.isCancelled else { break }
                await emitStreamingDraft(translation, generation: generation)
            } catch is CancellationError {
                break
            } catch {
                await handleTranslationFailure(error)
            }
        }

        guard workerGeneration == draftWorkerGeneration else { return }
        draftWorker = nil
        preemptionTask?.cancel()
        preemptionTask = nil

        if pendingDraft != nil {
            startDraftWorker()
        }
    }

    private func preemptStaleDraft() {
        preemptionTask = nil
        guard pendingDraft != nil else { return }
        draftTranslationGeneration += 1
        draftWorker?.cancel()
        startDraftWorker()
    }

    private func enqueueFinalTranslation(_ text: String) {
        cancelPendingTranslation()
        finalQueue.append(text)
        guard finalWorker == nil else { return }

        finalWorker = Task { [weak self] in
            await self?.runFinalWorker()
        }
    }

    private func runFinalWorker() async {
        while !Task.isCancelled, !finalQueue.isEmpty {
            let text = finalQueue.removeFirst()
            do {
                let translation = try await mtClient.translateStreaming(text) { _ in }
                guard !Task.isCancelled else { return }
                await emit(.translationFinal(translation))
            } catch is CancellationError {
                return
            } catch {
                await handleTranslationFailure(error)
            }
        }

        finalWorker = nil
        if !finalQueue.isEmpty {
            enqueueFinalTranslationWorker()
        }
    }

    private func enqueueFinalTranslationWorker() {
        guard finalWorker == nil else { return }
        finalWorker = Task { [weak self] in
            await self?.runFinalWorker()
        }
    }

    private func emitStreamingDraft(_ translation: String, generation: Int) async {
        guard
            generation == draftTranslationGeneration,
            pendingDraft == nil
        else {
            return
        }
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
        draftTranslationGeneration += 1
        draftWorker?.cancel()
        draftWorker = nil
        preemptionTask?.cancel()
        preemptionTask = nil
        pendingDraft = nil
        lastDraftText = ""
    }

    private func cancelFinalTranslations() {
        finalWorker?.cancel()
        finalWorker = nil
        finalQueue = []
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}
