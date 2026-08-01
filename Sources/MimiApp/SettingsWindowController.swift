import AppKit
import SwiftUI

@MainActor
final class SettingsWindowController {
    private static let frameAutosaveName = "mimi.settings-window"

    private let window: NSWindow

    init(model: AppModel, settings: AppSettings) {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 560, height: 610),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = "mimi 设置"
        window.contentMinSize = NSSize(width: 520, height: 550)
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(
            rootView: SettingsView()
                .environmentObject(model)
                .environmentObject(settings)
        )

        if !window.setFrameUsingName(Self.frameAutosaveName) {
            window.center()
        }
        window.setFrameAutosaveName(Self.frameAutosaveName)
    }

    func show() {
        NSApplication.shared.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }
}
