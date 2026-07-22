import AppKit
import Foundation
import MimiCore

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var state = TranslationSessionState()

    private var controller = TranslationSessionController()
    private let audioCapture = SystemAudioCapture()
    private var client: LiveTranslateClient?
    private var overlayController: OverlayWindowController?
    private weak var activeSettings: AppSettings?
    private var recoveryMonitor = TranslationRecoveryMonitor()
    private var recoveryTask: Task<Void, Never>?
    private var isRecovering = false
    private let isUITestMode = ProcessInfo.processInfo.environment["MIMI_UI_TEST"] == "1"

    var isActive: Bool { state.status.isActive }

    func attachOverlay(settings: AppSettings) {
        guard overlayController == nil else { return }
        overlayController = OverlayWindowController(model: self, settings: settings)
        overlayController?.updateLocked(settings.isOverlayLocked)

        if isUITestMode {
            controller.handle(.sourceFinal(text: "The future is already here.", language: "en"))
            controller.handle(.translationFinal("未来已在眼前。"))
            publishState()
            overlayController?.show()
            DispatchQueue.main.async {
                NSApplication.shared.sendAction(
                    Selector(("showSettingsWindow:")),
                    to: nil,
                    from: nil
                )
            }
        }
    }

    func start(using settings: AppSettings) async {
        guard !state.status.isActive else { return }

        activeSettings = settings
        recoveryMonitor = TranslationRecoveryMonitor()
        _ = await establishSession(using: settings, clearSubtitles: true)
    }

    @discardableResult
    private func establishSession(
        using settings: AppSettings,
        clearSubtitles: Bool
    ) async -> Bool {
        stopRecoveryWatchdog()

        do {
            let configuration = try settings.configuration()
            if clearSubtitles {
                controller.clearSubtitles()
            }
            controller.beginConnecting()
            publishState()

            let newClient = try LiveTranslateClient(
                workspaceID: configuration.workspaceID,
                apiKey: configuration.apiKey,
                sourceLanguage: configuration.sourceLanguage
            )
            client = newClient

            try await newClient.connect { [weak self] event in
                await self?.receive(event)
            }

            try await audioCapture.start(
                onAudio: { [weak newClient] data in
                    Task {
                        try? await newClient?.sendAudio(data)
                    }
                },
                onActivity: { [weak self] in
                    Task { @MainActor in
                        self?.noteActiveAudio()
                    }
                },
                onError: { [weak self] error in
                    Task { @MainActor in
                        await self?.handleCaptureFailure(error)
                    }
                }
            )

            controller.didConnect()
            activeSettings = settings
            recoveryMonitor.reset(at: currentTimestamp)
            publishState()
            overlayController?.show()
            startRecoveryWatchdog()
            return true
        } catch {
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

        stopRecoveryWatchdog()
        activeSettings = nil
        isRecovering = false
        controller.beginStopping()
        publishState()
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

    private func receive(_ event: LiveTranslateServerEvent) async {
        recoveryMonitor.noteServerActivity(at: currentTimestamp)
        controller.handle(event)
        publishState()

        if case .error = event {
            stopRecoveryWatchdog()
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
            activeSettings = nil
        }
    }

    private func handleCaptureFailure(_ error: Error) async {
        stopRecoveryWatchdog()
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        activeSettings = nil
        controller.didFail(error.localizedDescription)
        publishState()
    }

    private func noteActiveAudio() {
        guard state.status.isActive else { return }
        recoveryMonitor.noteActiveAudio(at: currentTimestamp)
    }

    private func startRecoveryWatchdog() {
        stopRecoveryWatchdog()
        recoveryTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(1))
                guard !Task.isCancelled else { return }
                if await self?.recoverIfNeeded() == true {
                    return
                }
            }
        }
    }

    private func stopRecoveryWatchdog() {
        recoveryTask?.cancel()
        recoveryTask = nil
    }

    private func recoverIfNeeded() async -> Bool {
        let now = currentTimestamp
        guard
            !isRecovering,
            recoveryMonitor.shouldRecover(at: now),
            let settings = activeSettings
        else {
            return false
        }

        isRecovering = true
        recoveryMonitor.markRecovery(at: now)
        recoveryTask = nil
        await audioCapture.stop()
        await client?.disconnect()
        client = nil

        var recovered = false
        for delay in [1, 2] {
            try? await Task.sleep(for: .seconds(delay))
            recovered = await establishSession(using: settings, clearSubtitles: false)
            if recovered {
                break
            }
        }

        if !recovered {
            activeSettings = nil
        }
        isRecovering = false
        return true
    }

    private var currentTimestamp: TimeInterval {
        ProcessInfo.processInfo.systemUptime
    }

    private func publishState() {
        state = controller.state
    }
}
