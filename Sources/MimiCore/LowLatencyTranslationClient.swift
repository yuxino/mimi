import Foundation

public actor LowLatencyTranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private struct TranslationRequest: Sendable {
        let text: String
        let detectedLanguage: SourceLanguage?
    }

    private struct TranslationMemoryEntry: Sendable {
        let language: SourceLanguage?
        let pair: QwenMTMemoryPair
    }

    private let asrClient: RealtimeASRClient
    private let draftMTClient: QwenMTClient
    private let finalMTClient: QwenMTClient
    private let configuredSourceLanguage: SourceLanguage
    private let translatesAudio: Bool
    private var eventHandler: EventHandler?
    private var draftWorker: Task<Void, Never>?
    private var preemptionTask: Task<Void, Never>?
    private var pendingDraft: TranslationRequest?
    private var finalQueue: [TranslationRequest] = []
    private var translationMemory: [TranslationMemoryEntry] = []
    private var finalWorker: Task<Void, Never>?
    private var draftTranslationGeneration = 0
    private var draftWorkerGeneration = 0
    private var lastDraftText = ""

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        targetLanguage: TargetLanguage = .simplifiedChinese,
        session: URLSession = .shared
    ) throws {
        self.asrClient = try RealtimeASRClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            session: session
        )
        self.configuredSourceLanguage = sourceLanguage
        self.translatesAudio = targetLanguage.translatesAudio
        self.draftMTClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            model: .flash,
            streamingTimeout: .seconds(5),
            session: session
        )
        self.finalMTClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            model: .flash,
            domainHint: Self.finalDomainHint(for: targetLanguage),
            session: session
        )
    }

    private static func finalDomainHint(for targetLanguage: TargetLanguage) -> String {
        let languageGuidance = switch targetLanguage {
        case .original:
            ""
        case .simplifiedChinese:
            "Use concise, idiomatic Simplified Chinese and preserve natural particles such as 嗯、啊、呢、吧、嘛."
        case .english:
            "Use concise, idiomatic conversational English with natural contractions and interjections."
        case .japanese:
            "Use natural conversational Japanese with appropriate register, particles, and sentence endings."
        }
        return """
        Natural spoken dialogue. \(languageGuidance) Preserve the speaker's tone and \
        implied subjects from context. Avoid literal, explanatory, or translation-like \
        wording. Preserve meaningful interjections and hesitation. Collapse accidental \
        ASR repetition, and do not mechanically translate every filler.
        """
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
        await asrClient.finish()
        await waitForFinalTranslations(timeout: .seconds(5))
        cancelFinalTranslations()
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
            guard translatesAudio else {
                await emit(.translationDraft(text))
                return
            }
            enqueueDraftTranslation(text, language: language)

        case let .sourceFinal(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            if lastDraftText.isEmpty {
                await emit(.translationDraft(""))
            }
            await emit(.sourceFinal(text: text, language: language))
            guard translatesAudio else {
                await emit(.translationFinal(text))
                return
            }
            enqueueFinalTranslation(text, language: language)

        default:
            await emit(event)
        }
    }

    private func enqueueDraftTranslation(_ text: String, language: String?) {
        guard text != lastDraftText else { return }
        lastDraftText = text
        pendingDraft = TranslationRequest(
            text: text,
            detectedLanguage: resolvedSourceLanguage(language)
        )

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
        while !Task.isCancelled, let request = pendingDraft {
            pendingDraft = nil
            preemptionTask?.cancel()
            preemptionTask = nil
            draftTranslationGeneration += 1
            let generation = draftTranslationGeneration

            do {
                let translation = try await draftMTClient.translateStreaming(
                    request.text,
                    sourceLanguageOverride: request.detectedLanguage
                ) { _ in }
                guard !Task.isCancelled else { break }
                await emitStreamingDraft(translation, generation: generation)
            } catch is CancellationError {
                break
            } catch {
                if await handleTranslationFailure(error) {
                    break
                }
                await emitStreamingDraft(request.text, generation: generation)
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

    private func enqueueFinalTranslation(_ text: String, language: String?) {
        cancelPendingTranslation()
        finalQueue.append(
            TranslationRequest(
                text: text,
                detectedLanguage: resolvedSourceLanguage(language)
            )
        )
        guard finalWorker == nil else { return }

        finalWorker = Task { [weak self] in
            await self?.runFinalWorker()
        }
    }

    private func runFinalWorker() async {
        while !Task.isCancelled, !finalQueue.isEmpty {
            let request = finalQueue.removeFirst()
            do {
                let recentMemory = translationMemory
                    .filter { $0.language == request.detectedLanguage }
                    .suffix(3)
                    .map(\.pair)
                let translation = try await finalMTClient.translateStreaming(
                    request.text,
                    sourceLanguageOverride: request.detectedLanguage,
                    translationMemory: recentMemory
                ) { _ in }
                guard !Task.isCancelled else { return }
                await emit(.translationFinal(translation))
                remember(
                    source: request.text,
                    translation: translation,
                    language: request.detectedLanguage
                )
            } catch is CancellationError {
                return
            } catch {
                if await handleTranslationFailure(error) {
                    return
                }
                await emit(.translationFinal(request.text))
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
        translationMemory = []
    }

    private func waitForFinalTranslations(timeout: Duration) async {
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: timeout)
        while finalWorker != nil, clock.now < deadline {
            try? await Task.sleep(for: .milliseconds(50))
        }
    }

    private func resolvedSourceLanguage(_ reportedLanguage: String?) -> SourceLanguage? {
        guard configuredSourceLanguage == .automatic else { return nil }
        return SourceLanguage(detectedLanguage: reportedLanguage)
    }

    private func remember(
        source: String,
        translation: String,
        language: SourceLanguage?
    ) {
        translationMemory.removeAll {
            $0.language == language && $0.pair.source == source
        }
        translationMemory.append(
            TranslationMemoryEntry(
                language: language,
                pair: QwenMTMemoryPair(source: source, target: translation)
            )
        )
        if translationMemory.count > 9 {
            translationMemory.removeFirst(translationMemory.count - 9)
        }
    }

    private func emit(_ event: LiveTranslateServerEvent) async {
        await eventHandler?(event)
    }
}
