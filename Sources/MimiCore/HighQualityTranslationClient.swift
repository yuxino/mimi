import Foundation

public actor HighQualityTranslationClient {
    public typealias EventHandler = @Sendable (LiveTranslateServerEvent) async -> Void

    private struct TranslationRequest: Sendable {
        let text: String
        let language: String?
        let enqueuedAt: ContinuousClock.Instant
    }

    private struct DraftTranslationRequest: Sendable {
        let text: String
        let language: String?
        let generation: Int
        let enqueuedAt: ContinuousClock.Instant
    }

    private struct DraftPreview: Sendable {
        let request: DraftTranslationRequest
        let translation: String
    }

    private let asrClient: Audio3ASRClient
    private let draftMTClient: QwenMTClient
    private let mtClient: QwenMTClient
    private let sourceLanguage: SourceLanguage
    private let translatesAudio: Bool
    private let clock = ContinuousClock()
    private var eventHandler: EventHandler?
    private var draftCommitter = ASRDraftCommitter()
    private var latestDraftLanguage: String?
    private var draftStabilityTask: Task<Void, Never>?
    private var draftMaximumWaitTask: Task<Void, Never>?
    private var draftPreviewTracker = DraftPreviewTracker()
    private var pendingDraftTranslation: DraftTranslationRequest?
    private var draftWorker: Task<Void, Never>?
    private var draftPreemptionTask: Task<Void, Never>?
    private var draftWorkerGeneration = 0
    private var activeDraftPreview: DraftPreview?
    private var finalQueue: [TranslationRequest] = []
    private var translationMemory: [QwenMTMemoryPair] = []
    private var finalWorker: Task<Void, Never>?
    private var pendingRevokeCount = 0

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
        let domainHint = QwenMTDomainHint.spokenDialogue(
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage
        )
        let fillerTerms = QwenMTDomainHint.fillerTerms(
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage
        )
        self.draftMTClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            model: .flash,
            domainHint: domainHint,
            terms: fillerTerms,
            streamingTimeout: .seconds(5),
            session: session
        )
        self.mtClient = try QwenMTClient(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            model: .plus,
            domainHint: domainHint,
            terms: fillerTerms,
            session: session
        )
        self.sourceLanguage = sourceLanguage
        self.translatesAudio = targetLanguage.translatesAudio
    }

    public func connect(onEvent: @escaping EventHandler) async throws {
        resetDraftFinalization()
        cancelDraftTranslations()
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
        cancelDraftTranslations()
        await asrClient.finish()
        await commitPendingDraft(boundary: "session-finish")
        await waitForFinalTranslations(timeout: .seconds(35))
        resetDraftFinalization()
        cancelFinalTranslations()
        eventHandler = nil
    }

    public func disconnect() async {
        resetDraftFinalization()
        cancelDraftTranslations()
        cancelFinalTranslations()
        eventHandler = nil
        await asrClient.disconnect()
    }

    private func handleASREvent(_ event: LiveTranslateServerEvent) async {
        switch event {
        case let .sourceDraft(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            let uncommittedText = draftCommitter.updateDraft(text)
            latestDraftLanguage = language
            PipelineDiagnostics.log(
                "audio3 asr draft length=\(text.count) pendingLength=\(uncommittedText.count) language=\(language ?? sourceLanguage.rawValue)"
            )
            guard draftCommitter.hasPendingText else { return }
            scheduleDraftFinalization()
            if !translatesAudio {
                await emit(.sourceDraft(text: uncommittedText, language: language))
                await emit(.translationDraft(uncommittedText))
            } else {
                // High-quality mode shows only confirmed finals. The flash
                // draft preview rewrote the visible line continuously and was
                // then replaced by the higher-quality final, which read as
                // subtitles that kept changing until they became correct.
                if finalWorker == nil, finalQueue.isEmpty {
                    await emit(.sourceDraft(text: uncommittedText, language: language))
                }
            }

        case let .sourceFinal(text, language):
            let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return }
            cancelDraftTimers()
            cancelDraftTranslations()
            latestDraftLanguage = nil
            let outcome = draftCommitter.finishSentence(text)
            let uncommittedText: String?
            switch outcome {
            case .none:
                uncommittedText = nil
            case let .appended(newText), let .replaced(newText):
                uncommittedText = newText
            }
            PipelineDiagnostics.log(
                "audio3 asr final length=\(text.count) pendingLength=\(uncommittedText?.count ?? 0) language=\(language ?? sourceLanguage.rawValue) queuedFinals=\(finalQueue.count)"
            )
            guard let uncommittedText else {
                PipelineDiagnostics.log("audio3 asr final deduplicated")
                return
            }
            if case .replaced = outcome {
                PipelineDiagnostics.log("audio3 asr final superseded provisional")
                if translatesAudio {
                    // Translated mode commits through the serial final worker;
                    // revoke there, right before the authoritative replacement,
                    // so the provisional history entry has already landed.
                    pendingRevokeCount += 1
                } else {
                    // Original-subtitle mode emits finals synchronously in event
                    // order, so the revocation belongs immediately before the
                    // replacement pair.
                    await emit(.subtitleRevoked)
                }
            }
            await enqueueConfirmedSource(
                uncommittedText,
                language: language,
                boundary: "server-final"
            )

        default:
            await emit(event)
        }
    }

    private func scheduleDraftFinalization() {
        // Keep one running timer instead of resetting it on every draft so
        // complete sentences are confirmed on a steady cadence while the
        // speaker keeps talking. commitPendingDraft clears the timer, and the
        // next draft schedules a fresh one.
        guard draftStabilityTask == nil else { return }
        draftStabilityTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(1_200))
                guard !Task.isCancelled else { return }
                await self?.commitPendingDraft(boundary: "stable-draft")
            } catch {
                return
            }
        }

        guard draftMaximumWaitTask == nil else { return }
        draftMaximumWaitTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(4_500))
                guard !Task.isCancelled else { return }
                await self?.commitPendingDraft(boundary: "maximum-wait")
            } catch {
                return
            }
        }
    }

    private func commitPendingDraft(boundary: String) async {
        cancelDraftTimers()
        let text: String?
        if boundary == "maximum-wait" || boundary == "session-finish" {
            text = draftCommitter.commitLatestDraft(commitLongIncomplete: true)
        } else {
            text = draftCommitter.commitCompleteSentences()
        }
        guard let text else { return }
        cancelDraftTranslations()
        let language = latestDraftLanguage
        PipelineDiagnostics.log(
            "audio3 asr local final boundary=\(boundary) length=\(text.count) language=\(language ?? sourceLanguage.rawValue)"
        )
        await enqueueConfirmedSource(text, language: language, boundary: boundary)
    }

    private func enqueueConfirmedSource(
        _ text: String,
        language: String?,
        boundary: String
    ) async {
        guard translatesAudio else {
            await emit(.sourceFinal(text: text, language: language))
            await emit(.translationFinal(text))
            return
        }

        finalQueue.append(.init(text: text, language: language, enqueuedAt: clock.now))
        PipelineDiagnostics.log(
            "mt plus final enqueued boundary=\(boundary) depth=\(finalQueue.count)"
        )
        startFinalWorkerIfNeeded()
    }

    private func enqueueDraftTranslation(_ text: String, language: String?) {
        guard let generation = draftPreviewTracker.update(text) else { return }
        activeDraftPreview = nil
        pendingDraftTranslation = .init(
            text: text,
            language: language,
            generation: generation,
            enqueuedAt: clock.now
        )

        guard draftWorker != nil else {
            startDraftWorker()
            return
        }

        guard draftPreemptionTask == nil else { return }
        draftPreemptionTask = Task { [weak self] in
            do {
                try await Task.sleep(for: .milliseconds(450))
                guard !Task.isCancelled else { return }
                await self?.preemptStaleDraft()
            } catch {
                return
            }
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
        while !Task.isCancelled, let request = pendingDraftTranslation {
            pendingDraftTranslation = nil
            draftPreemptionTask?.cancel()
            draftPreemptionTask = nil
            let startedAt = clock.now
            let memory = Array(translationMemory.suffix(2))
            PipelineDiagnostics.log(
                "mt flash preview started waitMs=\(PipelineDiagnostics.milliseconds(request.enqueuedAt.duration(to: startedAt))) length=\(request.text.count)"
            )

            do {
                let translation = try await draftMTClient.translateStreaming(
                    request.text,
                    translationMemory: memory
                ) { [weak self] partial in
                    await self?.emitDraftPreview(partial, for: request)
                }
                guard !Task.isCancelled else { break }
                PipelineDiagnostics.log(
                    "mt flash preview completed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) pending=\(pendingDraftTranslation == nil ? 0 : 1)"
                )
                await emitDraftPreview(translation, for: request)
            } catch is CancellationError {
                PipelineDiagnostics.log("mt flash preview cancelled")
                break
            } catch {
                PipelineDiagnostics.log(
                    "mt flash preview failed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) error=\(PipelineDiagnostics.errorLabel(error))"
                )
                if let clientError = error as? QwenMTClientError,
                    clientError.isAuthenticationFailure {
                    await handleTranslationFailure(clientError)
                    return
                }
            }
        }

        guard workerGeneration == draftWorkerGeneration else { return }
        draftWorker = nil
        draftPreemptionTask?.cancel()
        draftPreemptionTask = nil

        if pendingDraftTranslation != nil {
            startDraftWorker()
        }
    }

    private func emitDraftPreview(
        _ translation: String,
        for request: DraftTranslationRequest
    ) async {
        let trimmed = translation.trimmingCharacters(in: .whitespacesAndNewlines)
        guard
            !trimmed.isEmpty,
            pendingDraftTranslation == nil,
            draftPreviewTracker.accepts(
                text: request.text,
                generation: request.generation
            )
        else {
            return
        }

        activeDraftPreview = .init(request: request, translation: trimmed)
        await emit(.sourceDraft(text: request.text, language: request.language))
        await emit(.translationDraft(trimmed))
    }

    private func preemptStaleDraft() {
        draftPreemptionTask = nil
        guard pendingDraftTranslation != nil else { return }
        PipelineDiagnostics.log("mt flash preview preempted for newer ASR text")
        draftWorker?.cancel()
        startDraftWorker()
    }

    private func restoreActiveDraftPreview() async {
        guard
            let activeDraftPreview,
            draftPreviewTracker.accepts(
                text: activeDraftPreview.request.text,
                generation: activeDraftPreview.request.generation
            )
        else {
            return
        }
        await emit(
            .sourceDraft(
                text: activeDraftPreview.request.text,
                language: activeDraftPreview.request.language
            )
        )
        await emit(.translationDraft(activeDraftPreview.translation))
    }

    private func cancelDraftTranslations() {
        draftPreviewTracker.reset()
        pendingDraftTranslation = nil
        activeDraftPreview = nil
        draftPreemptionTask?.cancel()
        draftPreemptionTask = nil
        draftWorker?.cancel()
        draftWorker = nil
    }

    private func cancelDraftTimers() {
        draftStabilityTask?.cancel()
        draftStabilityTask = nil
        draftMaximumWaitTask?.cancel()
        draftMaximumWaitTask = nil
    }

    private func resetDraftFinalization() {
        cancelDraftTimers()
        draftCommitter.reset()
        latestDraftLanguage = nil
        pendingRevokeCount = 0
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
            await emit(.translationStarted)
            await emit(.sourceFinal(text: request.text, language: request.language))

            do {
                let translation = try await translateWithRetry(request.text)
                guard !Task.isCancelled else { return }
                if pendingRevokeCount > 0 {
                    // The previous item was a provisional local commit that the
                    // server final superseded. Revoke it immediately before the
                    // authoritative replacement lands so history holds the
                    // sentence once and the line never visibly disappears.
                    pendingRevokeCount -= 1
                    await emit(.subtitleRevoked)
                }
                PipelineDiagnostics.log(
                    "mt plus final completed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) remaining=\(finalQueue.count)"
                )
                await emit(.translationFinal(translation))
                remember(source: request.text, translation: translation)
                await restoreActiveDraftPreview()
            } catch is CancellationError {
                return
            } catch {
                PipelineDiagnostics.log(
                    "mt plus final failed requestMs=\(PipelineDiagnostics.milliseconds(startedAt.duration(to: clock.now))) error=\(PipelineDiagnostics.errorLabel(error))"
                )
                await handleTranslationFailure(error)
                return
            }
        }

        finalWorker = nil
        if !finalQueue.isEmpty {
            startFinalWorkerIfNeeded()
        }
    }

    private func handleTranslationFailure(_ error: Error) async {
        let code: String
        if let clientError = error as? QwenMTClientError,
            clientError.isAuthenticationFailure {
            code = "translation_authentication_failed"
        } else {
            code = "translation_failed"
        }
        await emit(
            .error(
                code: code,
                message: error.localizedDescription
            )
        )
    }

    private func translateWithRetry(_ text: String) async throws -> String {
        let memory = Array(translationMemory.suffix(3))
        var attempt = 1

        while true {
            do {
                return try await mtClient.translate(text, translationMemory: memory)
            } catch is CancellationError {
                throw CancellationError()
            } catch let clientError as QwenMTClientError {
                guard let delay = QwenMTRetryPolicy.delay(
                    after: clientError,
                    attempt: attempt
                ) else {
                    throw clientError
                }
                PipelineDiagnostics.log(
                    "mt plus final retrying attempt=\(attempt) delayMs=\(PipelineDiagnostics.milliseconds(delay)) error=\(PipelineDiagnostics.errorLabel(clientError))"
                )
                try await Task.sleep(for: delay)
                attempt += 1
            } catch {
                throw error
            }
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
        pendingRevokeCount = 0
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
