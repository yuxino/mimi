import AppKit
import Foundation
import MimiCore

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var state = TranslationSessionState()

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
            seedUITestSubtitles()
            overlayController?.show()
        }
    }

    func start(using settings: AppSettings) async {
        guard !state.status.isActive else { return }

        do {
            try settings.save()
        } catch {
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

            controller.didConnect()
            activeSettings = settings
            publishState()
            overlayController?.show()
            startHealthChecks()
            return true
        } catch {
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

        stopHealthChecks()
        recoveryTask?.cancel()
        recoveryTask = nil
        activeSettings = nil
        isRecovering = false
        controller.beginStopping()
        publishState()
        audioSender?.stop()
        audioSender = nil
        await audioCapture.stop()
        await client?.finish()
        client = nil
        controller.didStop()
        publishState()
    }

    func clearSubtitles() {
        controller.clearSubtitles()
        publishState()
    }

    func setOverlayLocked(_ locked: Bool) {
        overlayController?.updateLocked(locked)
    }

    func showOverlay() {
        overlayController?.show()
    }

    func showSettings() {
        settingsController?.show()
    }

    private func receive(_ event: LiveTranslateServerEvent) async {
        if case let .error(code, message) = event, code == "transport_error" {
            controller.beginConnecting()
            publishState()
            queueRecovery(after: message)
            return
        }

        controller.handle(event)
        publishState()

        if case .error = event {
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
        guard !isRecovering, activeSettings != nil else { return }
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
        guard !isRecovering, let client else { return false }

        do {
            try await client.ping()
            return true
        } catch {
            healthCheckTask = nil
            queueRecovery(after: error.localizedDescription)
            return false
        }
    }

    private func queueRecovery(after failureMessage: String) {
        guard recoveryTask == nil, activeSettings != nil else { return }
        recoveryTask = Task { @MainActor [weak self] in
            await self?.recoverConnection(after: failureMessage)
            self?.recoveryTask = nil
        }
    }

    private func recoverConnection(after failureMessage: String) async {
        guard !isRecovering, let settings = activeSettings else { return }

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
        for delay in [1, 2] {
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
            activeSettings = nil
            controller.didFail(failureMessage)
            publishState()
        }
        isRecovering = false
    }

    private func publishState() {
        state = controller.state
    }

    private func seedUITestSubtitles() {
        controller.didConnect()
        controller.handle(.sourceFinal(text: "今日は映画について話しましょう。", language: "ja"))
        controller.handle(.translationFinal("今天咱们聊聊电影吧。"))
        controller.handle(.sourceFinal(text: "主人公は駅で友達を待っています。", language: "ja"))
        controller.handle(.translationFinal("主人公正在车站等朋友呢。"))
        controller.handle(.sourceFinal(text: "電車が遅れているので少し心配になりました。", language: "ja"))
        controller.handle(.translationFinal("因为电车晚点了，我有点担心。"))
        controller.handle(.sourceDraft(text: "でも、もうすぐ来るでしょう。", language: "ja"))
        controller.handle(.translationDraft("嗯……不过应该很快就到了吧。"))
        publishState()
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
            do {
                for await data in pair.stream {
                    try Task.checkCancellation()
                    try await client.sendAudio(data)
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
