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

        do {
            let configuration = try settings.configuration()
            controller.clearSubtitles()
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
                onError: { [weak self] error in
                    Task { @MainActor in
                        await self?.handleCaptureFailure(error)
                    }
                }
            )

            controller.didConnect()
            publishState()
            overlayController?.show()
        } catch {
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
            controller.didFail(error.localizedDescription)
            publishState()
        }
    }

    func stop() async {
        guard state.status.isActive || client != nil else { return }

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
        controller.handle(event)
        publishState()

        if case .error = event {
            await audioCapture.stop()
            await client?.disconnect()
            client = nil
        }
    }

    private func handleCaptureFailure(_ error: Error) async {
        await audioCapture.stop()
        await client?.disconnect()
        client = nil
        controller.didFail(error.localizedDescription)
        publishState()
    }

    private func publishState() {
        state = controller.state
    }
}
