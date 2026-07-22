import AppKit
import SwiftUI

@MainActor
final class OverlayWindowController {
    private let panel: NSPanel

    init(model: AppModel, settings: AppSettings) {
        let size = NSSize(width: 820, height: 170)
        let screenFrame = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let origin = NSPoint(
            x: screenFrame.midX - size.width / 2,
            y: screenFrame.minY + 72
        )

        panel = NSPanel(
            contentRect: NSRect(origin: origin, size: size),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.contentView = NSHostingView(
            rootView: SubtitleOverlayView()
                .environmentObject(model)
                .environmentObject(settings)
        )
    }

    func show() {
        panel.orderFrontRegardless()
    }

    func hide() {
        panel.orderOut(nil)
    }

    func updateLocked(_ locked: Bool) {
        panel.ignoresMouseEvents = locked
        panel.isMovable = !locked
        panel.isMovableByWindowBackground = !locked
    }
}
