import AppKit
import SwiftUI

enum SubtitleOverlayMetrics {
    static let referenceSize = NSSize(width: 640, height: 136)
    static let minimumScale: CGFloat = 0.75
    static let maximumScale: CGFloat = 1.5

    static var minimumSize: NSSize {
        scaledSize(minimumScale)
    }

    static var maximumSize: NSSize {
        scaledSize(maximumScale)
    }

    static func normalizedSize(_ size: NSSize) -> NSSize {
        let widthScale = size.width / referenceSize.width
        let heightScale = size.height / referenceSize.height
        let scale = min(max(min(widthScale, heightScale), minimumScale), maximumScale)
        return scaledSize(scale)
    }

    private static func scaledSize(_ scale: CGFloat) -> NSSize {
        NSSize(
            width: referenceSize.width * scale,
            height: referenceSize.height * scale
        )
    }
}

@MainActor
final class OverlayWindowController {
    private static let frameAutosaveName = "mimi.subtitle-overlay"
    private static let frameLayoutVersionKey = "subtitleOverlayFrameLayoutVersion"
    private static let frameLayoutVersion = 3
    private static let defaultSize = SubtitleOverlayMetrics.referenceSize
    private static let minimumSize = SubtitleOverlayMetrics.minimumSize
    private static let maximumSize = SubtitleOverlayMetrics.maximumSize

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
        panel.contentAspectRatio = SubtitleOverlayMetrics.referenceSize
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
        let defaults = UserDefaults.standard
        if defaults.integer(forKey: Self.frameLayoutVersionKey) < Self.frameLayoutVersion,
           restoredFrame.height <= 220 {
            restoredFrame.size.height = Self.defaultSize.height
        }
        if defaults.integer(forKey: Self.frameLayoutVersionKey) < Self.frameLayoutVersion {
            restoredFrame.size = SubtitleOverlayMetrics.normalizedSize(restoredFrame.size)
        }
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
        defaults.set(Self.frameLayoutVersion, forKey: Self.frameLayoutVersionKey)
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
