import AppKit
import Foundation
import MimiCore

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var state = TranslationSessionState()

    private var controller = TranslationSessionController()
    private let audioCapture = SystemAudioCapture()
    private var client: TranslationClient?
    private var overlayController: OverlayWindowController?
    private var settingsController: SettingsWindowController?
    private weak var activeSettings: AppSettings?
    private var healthCheckTask: Task<Void, Never>?
    private var isRecovering = false
    private let isUITestMode = ProcessInfo.processInfo.environment["MIMI_UI_TEST"] == "1"

    var isActive: Bool { state.status.isActive }

    func attachOverlay(settings: AppSettings) {
        guard overlayController == nil else { return }
        overlayController = OverlayWindowController(model: self, settings: settings)
        settingsController = SettingsWindowController(model: self, settings: settings)
        overlayController?.updateLocked(settings.isOverlayLocked)

        if isUITestMode {
            overlayController?.show()
        }
    }

    func start(using settings: AppSettings) async {
        guard !state.status.isActive else { return }

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

            try await audioCapture.start(
                onAudio: { [weak newClient] data in
                    Task {
                        try? await newClient?.sendAudio(data)
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
            publishState()
            overlayController?.show()
            startHealthChecks()
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

        stopHealthChecks()
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

    func showSettings() {
        settingsController?.show()
    }

    private func receive(_ event: LiveTranslateServerEvent) async {
        if case let .error(code, message) = event, code == "transport_error" {
            controller.beginConnecting()
            publishState()
            Task { @MainActor [weak self] in
                await self?.recoverConnection(after: message)
            }
            return
        }

        controller.handle(event)
        publishState()

        if case .error = event {
            stopHealthChecks()
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
            activeSettings = nil
        }
    }

    private func handleCaptureFailure(_ error: Error) async {
        stopHealthChecks()
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        activeSettings = nil
        controller.didFail(error.localizedDescription)
        publishState()
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
            await recoverConnection(after: error.localizedDescription)
            return false
        }
    }

    private func recoverConnection(after failureMessage: String) async {
        guard !isRecovering, let settings = activeSettings else { return }

        isRecovering = true
        stopHealthChecks()
        controller.beginConnecting()
        publishState()
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
            controller.didFail(failureMessage)
            publishState()
        }
        isRecovering = false
    }

    private func publishState() {
        state = controller.state
    }
}
