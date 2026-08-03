import AppKit
import Foundation
import MimiCore

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var state = TranslationSessionState()
    @Published private(set) var isOverlayCollapsed = false
    @Published private(set) var isPaused = false

    private var controller = TranslationSessionController()
    private let audioCapture = SystemAudioCapture()
    private var client: TranslationClient?
    private var audioSender: AudioSendPipeline?
    private var overlayController: OverlayWindowController?
    private var settingsController: SettingsWindowController?
    private weak var activeSettings: AppSettings?
    private var healthCheckTask: Task<Void, Never>?
    private var recoveryTask: Task<Void, Never>?
    private var isRecovering = false
    private let isUITestMode = ProcessInfo.processInfo.environment["MIMI_UI_TEST"] == "1"

    var isActive: Bool { state.status.isActive }
    var showsOverlayControlsForUITesting: Bool { isUITestMode }

    func attachOverlay(settings: AppSettings) {
        guard overlayController == nil else { return }
        overlayController = OverlayWindowController(model: self, settings: settings)
        settingsController = SettingsWindowController(model: self, settings: settings)
        overlayController?.updateLocked(settings.isOverlayLocked)

        if isUITestMode {
            seedUITestSubtitles(targetLanguage: settings.targetLanguage)
            if ProcessInfo.processInfo.environment["MIMI_UI_TEST_PAUSED"] == "1" {
                isPaused = true
                controller.didPause()
                publishState()
            }
            if ProcessInfo.processInfo.environment["MIMI_UI_TEST_COLLAPSED"] == "1" {
                setOverlayCollapsed(true)
            }
            overlayController?.show()
        }
    }

    func start(using settings: AppSettings) async {
        guard !state.status.isActive else { return }
        isPaused = false
        PipelineDiagnostics.log(
            "session start requested source=\(settings.sourceLanguage.rawValue) target=\(settings.targetLanguage.rawValue) mode=\(settings.translationMode.rawValue)"
        )

        do {
            try settings.save()
        } catch {
            PipelineDiagnostics.log(
                "session settings failed error=\(PipelineDiagnostics.errorLabel(error))"
            )
            controller.didFail(error.localizedDescription)
            publishState()
            return
        }

        activeSettings = settings
        _ = await establishSession(using: settings, clearSubtitles: true)
    }

    @discardableResult
    private func establishSession(
        using settings: AppSettings,
        clearSubtitles: Bool
    ) async -> Bool {
        stopHealthChecks()

        do {
            let configuration = try settings.configuration()
            PipelineDiagnostics.log("session connecting clear=\(clearSubtitles ? 1 : 0)")
            if clearSubtitles {
                controller.clearSubtitles()
            }
            controller.beginConnecting()
            publishState()

            let newClient = try TranslationClient(configuration: configuration)
            client = newClient

            try await newClient.connect { [weak self] event in
                await self?.receive(event)
            }
            PipelineDiagnostics.log("asr websocket connected")

            let newAudioSender = AudioSendPipeline(
                client: newClient,
                onError: { [weak self] error in
                    Task { @MainActor in
                        await self?.handleAudioTransportFailure(error)
                    }
                }
            )
            audioSender = newAudioSender
            try await audioCapture.start(
                onAudio: { [weak newAudioSender] data in
                    newAudioSender?.enqueue(data)
                },
                onError: { [weak self] error in
                    Task { @MainActor in
                        await self?.handleCaptureFailure(error)
                    }
                }
            )
            PipelineDiagnostics.log("audio capture started")

            controller.didConnect()
            activeSettings = settings
            publishState()
            overlayController?.show()
            startHealthChecks()
            PipelineDiagnostics.log("session listening")
            return true
        } catch {
            PipelineDiagnostics.log(
                "session establish failed error=\(PipelineDiagnostics.errorLabel(error))"
            )
            audioSender?.stop()
            audioSender = nil
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
            if !isRecovering {
                activeSettings = nil
            }
            controller.didFail(error.localizedDescription)
            publishState()
            return false
        }
    }

    func stop() async {
        guard state.status.isActive || client != nil else { return }
        PipelineDiagnostics.log("session stop requested")

        stopHealthChecks()
        recoveryTask?.cancel()
        recoveryTask = nil
        activeSettings = nil
        isRecovering = false
        isPaused = false
        controller.beginStopping()
        publishState()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.finish()
        client = nil
        controller.didStop()
        publishState()
        PipelineDiagnostics.log("session stopped")
    }

    func clearSubtitles() {
        controller.clearSubtitles()
        publishState()
    }

    func togglePaused(using settings: AppSettings) async {
        if isUITestMode {
            isPaused.toggle()
            if isPaused {
                controller.didPause()
            } else {
                controller.didConnect()
            }
            publishState()
            return
        }

        if isPaused {
            await resume(using: settings)
        } else {
            await pause()
        }
    }

    func pause() async {
        guard !isPaused, state.status == .listening else { return }
        PipelineDiagnostics.log("session pause requested")

        isPaused = true
        stopHealthChecks()
        recoveryTask?.cancel()
        recoveryTask = nil
        isRecovering = false
        controller.didPause()
        publishState()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        controller.didPause()
        publishState()
        PipelineDiagnostics.log("session paused")
    }

    func resume(using settings: AppSettings) async {
        guard isPaused else { return }
        PipelineDiagnostics.log("session resume requested")

        isPaused = false
        activeSettings = settings
        let resumed = await establishSession(using: settings, clearSubtitles: false)
        guard !resumed else {
            PipelineDiagnostics.log("session resumed")
            return
        }

        isPaused = true
        activeSettings = settings
        controller.didPause()
        publishState()
        overlayController?.show()
        PipelineDiagnostics.log("session resume failed; remaining paused")
    }

    func switchSourceLanguage(
        to language: SourceLanguage,
        using settings: AppSettings
    ) async {
        guard language != .automatic else { return }

        let targetLanguage = language.targetLanguageAfterQuickSwitch(
            from: settings.sourceLanguage,
            currentTarget: settings.targetLanguage
        )
        let needsReconnect = state.status == .listening
            && (settings.sourceLanguage != language
                || settings.targetLanguage != targetLanguage
                || settings.translationMode != .highQuality)
        settings.sourceLanguage = language
        settings.targetLanguage = targetLanguage
        settings.translationMode = .highQuality
        settings.persistPreferences()
        if isPaused {
            activeSettings = settings
            return
        }
        guard needsReconnect else { return }

        PipelineDiagnostics.log(
            "session language switch source=\(language.rawValue) target=\(targetLanguage.rawValue) mode=highQuality"
        )
        stopHealthChecks()
        recoveryTask?.cancel()
        recoveryTask = nil
        activeSettings = settings
        controller.beginConnecting()
        publishState()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        _ = await establishSession(using: settings, clearSubtitles: false)
    }

    func setOverlayLocked(_ locked: Bool) {
        overlayController?.updateLocked(locked)
    }

    func showOverlay() {
        guard state.status == .listening else { return }
        overlayController?.show()
    }

    func toggleOverlayCollapsed() {
        setOverlayCollapsed(!isOverlayCollapsed)
    }

    func setOverlayCollapsed(_ collapsed: Bool) {
        guard collapsed != isOverlayCollapsed else { return }
        overlayController?.setCollapsed(collapsed)
        isOverlayCollapsed = collapsed
    }

    func showSettings() {
        settingsController?.show()
    }

    private func receive(_ event: LiveTranslateServerEvent) async {
        guard !isPaused else { return }

        if case let .error(code, message) = event, code == "transport_error" {
            PipelineDiagnostics.log("session transport error code=\(code)")
            controller.beginConnecting()
            publishState()
            queueRecovery(after: message)
            return
        }

        controller.handle(event)
        publishState()

        if case .error = event {
            PipelineDiagnostics.log("session terminal error")
            stopHealthChecks()
            recoveryTask?.cancel()
            recoveryTask = nil
            audioSender?.stop()
            audioSender = nil
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
            activeSettings = nil
        }
    }

    private func handleCaptureFailure(_ error: Error) async {
        guard !isPaused else { return }
        PipelineDiagnostics.log(
            "audio capture failed error=\(PipelineDiagnostics.errorLabel(error))"
        )
        stopHealthChecks()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        activeSettings = nil
        controller.didFail(error.localizedDescription)
        publishState()
    }

    private func handleAudioTransportFailure(_ error: Error) async {
        guard !isPaused, !isRecovering, activeSettings != nil else { return }
        PipelineDiagnostics.log(
            "audio transport failed error=\(PipelineDiagnostics.errorLabel(error))"
        )
        queueRecovery(after: error.localizedDescription)
    }

    private func startHealthChecks() {
        stopHealthChecks()
        healthCheckTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(10))
                guard !Task.isCancelled else { return }
                if await self?.checkConnectionHealth() == false {
                    return
                }
            }
        }
    }

    private func stopHealthChecks() {
        healthCheckTask?.cancel()
        healthCheckTask = nil
    }

    private func checkConnectionHealth() async -> Bool {
        guard !isPaused, !isRecovering, let client else { return false }

        do {
            try await client.ping()
            return true
        } catch {
            PipelineDiagnostics.log(
                "connection health failed error=\(PipelineDiagnostics.errorLabel(error))"
            )
            healthCheckTask = nil
            queueRecovery(after: error.localizedDescription)
            return false
        }
    }

    private func queueRecovery(after failureMessage: String) {
        guard !isPaused, recoveryTask == nil, activeSettings != nil else { return }
        recoveryTask = Task { @MainActor [weak self] in
            await self?.recoverConnection(after: failureMessage)
            self?.recoveryTask = nil
        }
    }

    private func recoverConnection(after failureMessage: String) async {
        guard !isPaused, !isRecovering, let settings = activeSettings else { return }

        PipelineDiagnostics.log("session recovery started")
        isRecovering = true
        stopHealthChecks()
        controller.beginConnecting()
        publishState()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.disconnect()
        client = nil

        var recovered = false
        for (attempt, delay) in [1, 2].enumerated() {
            PipelineDiagnostics.log("session recovery attempt=\(attempt + 1)")
            do {
                try await Task.sleep(for: .seconds(delay))
            } catch {
                isRecovering = false
                return
            }
            guard !Task.isCancelled, activeSettings != nil else {
                isRecovering = false
                return
            }
            recovered = await establishSession(using: settings, clearSubtitles: false)
            if recovered {
                break
            }
        }

        guard !Task.isCancelled, activeSettings != nil else {
            isRecovering = false
            return
        }
        if !recovered {
            PipelineDiagnostics.log("session recovery exhausted")
            activeSettings = nil
            controller.didFail(failureMessage)
            publishState()
        }
        isRecovering = false
    }

    private func publishState() {
        state = controller.state
        if !state.status.isActive {
            overlayController?.hide()
        }
    }

    private func seedUITestSubtitles(targetLanguage: TargetLanguage) {
        controller.didConnect()

        switch targetLanguage {
        case .english:
            seedUITestHistory(
                sourceLanguage: "ja",
                sources: [
                    "今日は映画について話しましょう。",
                    "主人公は駅で友達を待っています。",
                    "電車が遅れているので少し心配になりました。"
                ],
                translations: [
                    "Let's talk about the film today.",
                    "The protagonist is waiting for a friend at the station.",
                    "The delayed train has her a little worried."
                ]
            )
        case .japanese:
            seedUITestHistory(
                sourceLanguage: "en",
                sources: [
                    "Let's talk about the film today.",
                    "The main character is waiting for a friend at the station.",
                    "The train is late, so she's getting a little worried."
                ],
                translations: [
                    "今日は映画の話をしましょう。",
                    "主人公は駅で友達を待っています。",
                    "電車が遅れていて、少し心配になってきました。"
                ]
            )
        case .simplifiedChinese:
            seedUITestHistory(
                sourceLanguage: "ja",
                sources: [
                    "今日は映画について話しましょう。",
                    "主人公は駅で友達を待っています。",
                    "電車が遅れているので少し心配になりました。"
                ],
                translations: [
                    "今天咱们聊聊电影吧。",
                    "主人公正在车站等朋友呢。",
                    "因为电车晚点了，我有点担心。"
                ]
            )
        case .original:
            let sources = [
                "今日は映画について話しましょう。",
                "主人公は駅で友達を待っています。",
                "電車が遅れているので少し心配になりました。"
            ]
            seedUITestHistory(
                sourceLanguage: "ja",
                sources: sources,
                translations: sources
            )
        }

        if ProcessInfo.processInfo.environment["MIMI_UI_TEST_LONG_SUBTITLE"] == "1" {
            controller.handle(
                .sourceDraft(
                    text: "話し手が長いあいだ途切れずに話し続けても字幕は読みやすい長さで区切られ最後の部分だけが更新されます",
                    language: "ja"
                )
            )
            controller.handle(
                .translationDraft(
                    "即使说话者长时间连续讲话字幕也会保持容易阅读的长度已经完成的小句不会反复重排只有最后一小段继续更新"
                )
            )
        }

        if ProcessInfo.processInfo.environment["MIMI_UI_TEST_TRANSLATING"] == "1" {
            controller.handle(.translationStarted)
        }

        publishState()
    }

    private func seedUITestHistory(
        sourceLanguage: String,
        sources: [String],
        translations: [String]
    ) {
        for (source, translation) in zip(sources, translations) {
            controller.handle(.sourceFinal(text: source, language: sourceLanguage))
            controller.handle(.translationFinal(translation))
        }
    }
}

private enum AudioSendPipelineError: LocalizedError {
    case fellBehind

    var errorDescription: String? {
        "Audio streaming fell behind. mimi is reconnecting."
    }
}

private final class AudioSendPipeline: @unchecked Sendable {
    private let continuation: AsyncStream<Data>.Continuation
    private let onError: @Sendable (Error) -> Void
    private let lock = NSLock()
    private var worker: Task<Void, Never>?
    private var hasFailed = false

    init(
        client: TranslationClient,
        onError: @escaping @Sendable (Error) -> Void
    ) {
        let pair = AsyncStream<Data>.makeStream(
            bufferingPolicy: .bufferingNewest(20)
        )
        self.continuation = pair.continuation
        self.onError = onError
        self.worker = Task {
            let clock = ContinuousClock()
            var sentBufferCount = 0
            var sentByteCount = 0
            do {
                for await data in pair.stream {
                    try Task.checkCancellation()
                    let startedAt = clock.now
                    try await client.sendAudio(data)
                    sentBufferCount += 1
                    sentByteCount += data.count
                    if sentBufferCount == 1 || sentBufferCount.isMultiple(of: 100) {
                        PipelineDiagnostics.log(
                            "audio sent buffers=\(sentBufferCount) bytes=\(sentByteCount)"
                        )
                    }
                    let sendMilliseconds = PipelineDiagnostics.milliseconds(
                        startedAt.duration(to: clock.now)
                    )
                    if sendMilliseconds > 200 {
                        PipelineDiagnostics.log(
                            "audio send blockedMs=\(sendMilliseconds) bytes=\(data.count)"
                        )
                    }
                }
            } catch is CancellationError {
                return
            } catch {
                self.failOnce(with: error)
            }
        }
    }

    func enqueue(_ data: Data) {
        switch continuation.yield(data) {
        case .enqueued:
            break
        case .dropped:
            PipelineDiagnostics.log("audio queue dropped newest buffer")
            failOnce(with: AudioSendPipelineError.fellBehind)
        case .terminated:
            break
        @unknown default:
            failOnce(with: AudioSendPipelineError.fellBehind)
        }
    }

    func stop() {
        lock.withLock {
            hasFailed = true
        }
        continuation.finish()
        worker?.cancel()
        worker = nil
    }

    private func failOnce(with error: Error) {
        let shouldReport = lock.withLock {
            guard !hasFailed else { return false }
            hasFailed = true
            return true
        }
        guard shouldReport else { return }
        continuation.finish()
        onError(error)
    }
}
