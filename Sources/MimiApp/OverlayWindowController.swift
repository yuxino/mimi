import AppKit
import SwiftUI

enum SubtitleOverlayMetrics {
    static let referenceSize = NSSize(width: 640, height: 136)
    static let minimumSize = NSSize(width: 360, height: 100)
    static let maximumSize = NSSize(width: 1_200, height: 600)
}

@MainActor
final class OverlayWindowController {
    private static let frameAutosaveName = "mimi.subtitle-overlay"
    private static let frameLayoutVersionKey = "subtitleOverlayFrameLayoutVersion"
    private static let frameLayoutVersion = 4
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
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.title = "mimi Subtitles"
        panel.setAccessibilityRole(.window)
        panel.isOpaque = false
        panel.acceptsMouseMovedEvents = true
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.contentView = ResizeCursorHostingView(
            rootView: AnyView(
                SubtitleOverlayView()
                .environmentObject(model)
                .environmentObject(settings)
            )
        )

        panel.setFrameAutosaveName(Self.frameAutosaveName)
        var restoredFrame = panel.frame
        let defaults = UserDefaults.standard
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
        panel.isMovableByWindowBackground = false
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

private final class ResizeCursorHostingView: NSHostingView<AnyView> {
    private enum Region {
        case top
        case left
        case bottom
        case right
        case topLeft
        case topRight
        case bottomLeft
        case bottomRight
    }

    private struct CursorRegion {
        let rect: NSRect
        let region: Region
    }

    private static let edgeThickness: CGFloat = 14
    private static let cornerSize: CGFloat = 24

    private var pointerTrackingArea: NSTrackingArea?

    override func updateTrackingAreas() {
        if let pointerTrackingArea {
            removeTrackingArea(pointerTrackingArea)
        }

        let area = NSTrackingArea(
            rect: bounds,
            options: [
                .activeAlways,
                .inVisibleRect,
                .mouseEnteredAndExited,
                .mouseMoved
            ],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        pointerTrackingArea = area
        super.updateTrackingAreas()
    }

    override func mouseMoved(with event: NSEvent) {
        super.mouseMoved(with: event)
        let point = convert(event.locationInWindow, from: nil)
        guard let region = cursorRegions.first(where: { $0.rect.contains(point) }) else {
            return
        }
        cursor(for: region.region).set()
    }

    override func mouseExited(with event: NSEvent) {
        super.mouseExited(with: event)
        NSCursor.arrow.set()
    }

    override func resetCursorRects() {
        super.resetCursorRects()

        for region in cursorRegions {
            addCursorRect(region.rect, cursor: cursor(for: region.region))
        }
    }

    private var cursorRegions: [CursorRegion] {
        let edge = Self.edgeThickness
        let corner = Self.cornerSize
        let width = bounds.width
        let height = bounds.height

        return [
            CursorRegion(
                rect: NSRect(x: 0, y: height - corner, width: corner, height: corner),
                region: .topLeft
            ),
            CursorRegion(
                rect: NSRect(x: width - corner, y: height - corner, width: corner, height: corner),
                region: .topRight
            ),
            CursorRegion(
                rect: NSRect(x: 0, y: 0, width: corner, height: corner),
                region: .bottomLeft
            ),
            CursorRegion(
                rect: NSRect(x: width - corner, y: 0, width: corner, height: corner),
                region: .bottomRight
            ),
            CursorRegion(
                rect: NSRect(x: corner, y: height - edge, width: max(0, width - corner * 2), height: edge),
                region: .top
            ),
            CursorRegion(
                rect: NSRect(x: 0, y: corner, width: edge, height: max(0, height - corner * 2)),
                region: .left
            ),
            CursorRegion(
                rect: NSRect(x: corner, y: 0, width: max(0, width - corner * 2), height: edge),
                region: .bottom
            ),
            CursorRegion(
                rect: NSRect(x: width - edge, y: corner, width: edge, height: max(0, height - corner * 2)),
                region: .right
            )
        ]
    }

    private func cursor(for region: Region) -> NSCursor {
        if #available(macOS 15.0, *) {
            let position: NSCursor.FrameResizePosition = switch region {
            case .top: .top
            case .left: .left
            case .bottom: .bottom
            case .right: .right
            case .topLeft: .topLeft
            case .topRight: .topRight
            case .bottomLeft: .bottomLeft
            case .bottomRight: .bottomRight
            }
            return .frameResize(position: position, directions: .all)
        }

        return switch region {
        case .top: .resizeUp
        case .left: .resizeLeft
        case .bottom: .resizeDown
        case .right: .resizeRight
        case .topLeft, .bottomRight:
            diagonalCursor(systemName: "arrow.up.left.and.arrow.down.right")
        case .topRight, .bottomLeft:
            diagonalCursor(systemName: "arrow.up.right.and.arrow.down.left")
        }
    }

    private func diagonalCursor(systemName: String) -> NSCursor {
        guard let image = NSImage(systemSymbolName: systemName, accessibilityDescription: nil) else {
            return .resizeLeftRight
        }
        image.size = NSSize(width: 18, height: 18)
        return NSCursor(image: image, hotSpot: NSPoint(x: 9, y: 9))
    }
}

private extension NSRect {
    var area: CGFloat {
        guard !isNull else { return 0 }
        return width * height
    }
}
