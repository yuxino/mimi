import AppKit
import SwiftUI

@MainActor
final class OverlayWindowController {
    private static let frameAutosaveName = "mimi.subtitle-overlay"
    private static let defaultSize = NSSize(width: 820, height: 210)

    private let panel: NSPanel

    init(model: AppModel, settings: AppSettings) {
        let screenFrame = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let origin = NSPoint(
            x: screenFrame.midX - Self.defaultSize.width / 2,
            y: screenFrame.minY + 72
        )

        panel = NSPanel(
            contentRect: NSRect(origin: origin, size: Self.defaultSize),
            styleMask: [.borderless, .nonactivatingPanel, .resizable],
            backing: .buffered,
            defer: false
        )
        panel.minSize = NSSize(width: 420, height: 190)
        panel.contentMinSize = panel.minSize
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

        if panel.setFrameUsingName(Self.frameAutosaveName) {
            var restoredFrame = panel.frame
            restoredFrame.size.width = max(restoredFrame.width, panel.minSize.width)
            restoredFrame.size.height = max(restoredFrame.height, panel.minSize.height)
            panel.setFrame(
                panel.constrainFrameRect(restoredFrame, to: panel.screen),
                display: false
            )
        }
        panel.setFrameAutosaveName(Self.frameAutosaveName)
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
