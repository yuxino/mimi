import AppKit
import MimiCore
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
    private static let collapsedSize = NSSize(width: 280, height: 54)

    private let panel: ResizeCursorPanel
    private let hostingView: ResizeCursorHostingView
    private var expandedSize = SubtitleOverlayMetrics.referenceSize
    private var frameSettleTask: Task<Void, Never>?

    init(model: AppModel, settings: AppSettings) {
        let screenFrame = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let origin = NSPoint(
            x: screenFrame.midX - Self.defaultSize.width / 2,
            y: screenFrame.minY + 72
        )

        panel = ResizeCursorPanel(
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
        hostingView = ResizeCursorHostingView(
            rootView: AnyView(
                SubtitleOverlayView()
                .environmentObject(model)
                .environmentObject(settings)
            )
        )
        panel.contentView = hostingView

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
        expandedSize = panel.frame.size
    }

    func show() {
        panel.orderFrontRegardless()
    }

    func hide() {
        panel.resetResizeCursor()
        panel.orderOut(nil)
    }

    func updateLocked(_ locked: Bool) {
        if locked {
            panel.resetResizeCursor()
        }
        panel.ignoresMouseEvents = locked
        panel.isMovable = !locked
        panel.isMovableByWindowBackground = false
    }

    func setCollapsed(_ collapsed: Bool) {
        panel.resetResizeCursor()
        frameSettleTask?.cancel()

        if collapsed {
            expandedSize = panel.frame.size
            panel.saveFrame(usingName: Self.frameAutosaveName)
            panel.setFrameAutosaveName("")
            setResizeInteractionEnabled(false)
            panel.minSize = Self.collapsedSize
            panel.maxSize = Self.collapsedSize
            panel.contentMinSize = Self.collapsedSize
            panel.contentMaxSize = Self.collapsedSize

            let frame = SubtitleOverlayCollapseLayout.collapsedFrame(
                from: panel.frame,
                compactSize: Self.collapsedSize
            )
            settleFrame(frame)
        } else {
            panel.minSize = Self.minimumSize
            panel.maxSize = Self.maximumSize
            panel.contentMinSize = Self.minimumSize
            panel.contentMaxSize = Self.maximumSize

            let frame = SubtitleOverlayCollapseLayout.expandedFrame(
                from: panel.frame,
                expandedSize: expandedSize
            )
            panel.setFrameAutosaveName(Self.frameAutosaveName)
            settleFrame(frame)
            panel.saveFrame(usingName: Self.frameAutosaveName)
            setResizeInteractionEnabled(true)
        }
    }

    /// Animates to `frame`, then re-asserts the exact target once the animation
    /// has had time to finish. The collapse/expand animation can be interrupted
    /// (for example by a live SwiftUI layout pass or an in-flight window drag),
    /// which used to leave the panel stuck at an intermediate height such as a
    /// taller-than-expected collapsed bar. Settling the frame keeps the panel at
    /// exactly the compact or expanded size.
    private func settleFrame(_ frame: NSRect) {
        let target = constrainedFrame(frame)
        panel.setFrame(target, display: true, animate: true)

        frameSettleTask = Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: 400_000_000)
            guard let self, !Task.isCancelled else { return }
            if self.panel.frame != target {
                self.panel.setFrame(target, display: true, animate: false)
            }
        }
    }

    private func setResizeInteractionEnabled(_ enabled: Bool) {
        panel.resizeInteractionEnabled = enabled
        hostingView.resizeInteractionEnabled = enabled
    }

    private func constrainedFrame(_ frame: NSRect) -> NSRect {
        let targetScreen = NSScreen.screens.max { lhs, rhs in
            lhs.visibleFrame.intersection(frame).area
                < rhs.visibleFrame.intersection(frame).area
        } ?? NSScreen.main
        return Self.constrain(
            frame,
            to: targetScreen?.visibleFrame
                ?? NSScreen.main?.visibleFrame
                ?? frame
        )
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

private final class ResizeCursorPanel: NSPanel {
    private let hitTester = OverlayResizeHitTester()
    private var cursorState = OverlayResizeCursorState()
    nonisolated(unsafe) private var localMouseMonitor: Any?
    nonisolated(unsafe) private var globalMouseMonitor: Any?
    private var cursorRefreshIsScheduled = false
    var resizeInteractionEnabled = true {
        didSet {
            if !resizeInteractionEnabled {
                resetResizeCursor()
            }
            if let contentView {
                invalidateCursorRects(for: contentView)
            }
        }
    }

    override init(
        contentRect: NSRect,
        styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType,
        defer flag: Bool
    ) {
        super.init(
            contentRect: contentRect,
            styleMask: style,
            backing: backingStoreType,
            defer: flag
        )
        installMouseMonitors()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("ResizeCursorPanel does not support NSCoding")
    }

    deinit {
        if let localMouseMonitor {
            NSEvent.removeMonitor(localMouseMonitor)
        }
        if let globalMouseMonitor {
            NSEvent.removeMonitor(globalMouseMonitor)
        }
    }

    override func sendEvent(_ event: NSEvent) {
        super.sendEvent(event)

        guard resizeInteractionEnabled else {
            resetResizeCursor()
            return
        }

        switch event.type {
        case .mouseMoved, .cursorUpdate:
            updateResizeCursor(for: event)
        case .mouseExited:
            apply(cursorState.update(region: nil))
        default:
            break
        }
    }

    func resetResizeCursor() {
        apply(cursorState.update(region: nil))
    }

    private func updateResizeCursor(for event: NSEvent) {
        guard let contentView else { return }
        let point = contentView.convert(event.locationInWindow, from: nil)
        apply(
            cursorState.update(
                region: hitTester.region(
                    at: point,
                    in: contentView.bounds,
                    isFlipped: contentView.isFlipped
                )
            )
        )
    }

    private func installMouseMonitors() {
        localMouseMonitor = NSEvent.addLocalMonitorForEvents(matching: .mouseMoved) {
            [weak self] event in
            self?.scheduleCursorRefresh()
            return event
        }
        globalMouseMonitor = NSEvent.addGlobalMonitorForEvents(matching: .mouseMoved) {
            [weak self] _ in
            Task { @MainActor in
                self?.scheduleCursorRefresh()
            }
        }
    }

    private func scheduleCursorRefresh() {
        guard resizeInteractionEnabled, isVisible, !ignoresMouseEvents else { return }
        guard !cursorRefreshIsScheduled else { return }
        cursorRefreshIsScheduled = true

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.cursorRefreshIsScheduled = false
            self.refreshCursorAtCurrentMouseLocation()
        }
    }

    private func refreshCursorAtCurrentMouseLocation() {
        guard let contentView else { return }
        let windowPoint = convertPoint(fromScreen: NSEvent.mouseLocation)
        let contentPoint = contentView.convert(windowPoint, from: nil)
        apply(
            cursorState.update(
                region: hitTester.region(
                    at: contentPoint,
                    in: contentView.bounds,
                    isFlipped: contentView.isFlipped
                )
            )
        )
    }

    private func apply(_ action: OverlayResizeCursorAction) {
        switch action {
        case let .set(region):
            resizeCursor(for: region).set()
        case .restoreDefault:
            NSCursor.arrow.set()
        case .none:
            break
        }
    }
}

private final class ResizeCursorHostingView: NSHostingView<AnyView> {
    private let hitTester = OverlayResizeHitTester()
    private var pointerTrackingArea: NSTrackingArea?
    private var cursorState = OverlayResizeCursorState()
    var resizeInteractionEnabled = true {
        didSet {
            if !resizeInteractionEnabled {
                clearResizeCursorIfNeeded()
            }
            window?.invalidateCursorRects(for: self)
        }
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()

        if let pointerTrackingArea {
            removeTrackingArea(pointerTrackingArea)
        }

        let area = NSTrackingArea(
            rect: bounds,
            options: [
                .activeAlways,
                .inVisibleRect,
                .mouseEnteredAndExited,
                .mouseMoved,
                .cursorUpdate
            ],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        pointerTrackingArea = area
    }

    override func layout() {
        super.layout()
        window?.invalidateCursorRects(for: self)
    }

    override func mouseMoved(with event: NSEvent) {
        super.mouseMoved(with: event)
        guard resizeInteractionEnabled else { return }
        updateCursor(for: event)
    }

    override func cursorUpdate(with event: NSEvent) {
        guard resizeInteractionEnabled else {
            clearResizeCursorIfNeeded()
            return
        }
        updateCursor(for: event)
    }

    override func mouseExited(with event: NSEvent) {
        super.mouseExited(with: event)
        clearResizeCursorIfNeeded()
    }

    override func resetCursorRects() {
        super.resetCursorRects()

        guard resizeInteractionEnabled else { return }

        for region in OverlayResizeRegion.allCases {
            addCursorRect(
                hitTester.rect(for: region, in: bounds, isFlipped: isFlipped),
                cursor: resizeCursor(for: region)
            )
        }
    }

    private func updateCursor(for event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        applyCursorAction(
            cursorState.update(
                region: hitTester.region(
                    at: point,
                    in: bounds,
                    isFlipped: isFlipped
                )
            )
        )
    }

    private func clearResizeCursorIfNeeded() {
        applyCursorAction(cursorState.update(region: nil))
    }

    private func applyCursorAction(_ action: OverlayResizeCursorAction) {
        switch action {
        case let .set(region):
            resizeCursor(for: region).set()
        case .restoreDefault:
            NSCursor.arrow.set()
        case .none:
            break
        }
    }

}

private func resizeCursor(for region: OverlayResizeRegion) -> NSCursor {
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

private extension NSRect {
    var area: CGFloat {
        guard !isNull else { return 0 }
        return width * height
    }
}
