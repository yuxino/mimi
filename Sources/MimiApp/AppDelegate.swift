import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var showSettingsHandler: (() -> Void)?
    private var shouldShowSettingsWhenReady = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        requestSettings()
    }

    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        requestSettings()
        return true
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    func installShowSettingsHandler(_ handler: @escaping () -> Void) {
        showSettingsHandler = handler
        if shouldShowSettingsWhenReady {
            shouldShowSettingsWhenReady = false
            handler()
        }
    }

    private func requestSettings() {
        if let showSettingsHandler {
            showSettingsHandler()
        } else {
            shouldShowSettingsWhenReady = true
        }
    }
}
