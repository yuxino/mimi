import AppKit
import SwiftUI

@MainActor
final class OverlayWindowController {
    private static let frameAutosaveName = "mimi.subtitle-overlay"
    private static let defaultSize = NSSize(width: 640, height: 190)
    private static let minimumSize = NSSize(width: 460, height: 150)
    private static let maximumSize = NSSize(width: 960, height: 360)

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
        panel.minSize = Self.minimumSize
        panel.maxSize = Self.maximumSize
        panel.contentMinSize = panel.minSize
        panel.contentMaxSize = panel.maxSize
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.title = "mimi Subtitles"
        panel.setAccessibilityRole(.window)
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

        panel.setFrameAutosaveName(Self.frameAutosaveName)
        var restoredFrame = panel.frame
        restoredFrame.size.width = restoredFrame.width > panel.maxSize.width
            ? Self.defaultSize.width
            : max(restoredFrame.width, panel.minSize.width)
        restoredFrame.size.height = restoredFrame.height > panel.maxSize.height
            ? Self.defaultSize.height
            : max(restoredFrame.height, panel.minSize.height)
        let targetScreen = NSScreen.screens.max { lhs, rhs in
            lhs.visibleFrame.intersection(restoredFrame).area
                < rhs.visibleFrame.intersection(restoredFrame).area
        } ?? NSScreen.main
        panel.setFrame(
            Self.constrain(restoredFrame, to: targetScreen?.visibleFrame ?? screenFrame),
            display: false
        )
        panel.saveFrame(usingName: Self.frameAutosaveName)
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

    private static func constrain(_ frame: NSRect, to visibleFrame: NSRect) -> NSRect {
        var result = frame
        result.size.width = min(result.width, visibleFrame.width)
        result.size.height = min(result.height, visibleFrame.height)
        result.origin.x = min(
            max(result.minX, visibleFrame.minX),
            visibleFrame.maxX - result.width
        )
        result.origin.y = min(
            max(result.minY, visibleFrame.minY),
            visibleFrame.maxY - result.height
        )
        return result
    }
}

private extension NSRect {
    var area: CGFloat {
        guard !isNull else { return 0 }
        return width * height
    }
}
