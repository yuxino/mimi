import CoreGraphics
import Foundation

public enum OverlayResizeRegion: CaseIterable, Equatable, Sendable {
    case top
    case left
    case bottom
    case right
    case topLeft
    case topRight
    case bottomLeft
    case bottomRight
}

public enum OverlayResizeCursorAction: Equatable, Sendable {
    case set(OverlayResizeRegion)
    case restoreDefault
    case none
}

public struct OverlayResizeCursorState: Sendable {
    public private(set) var activeRegion: OverlayResizeRegion?

    public init() {}

    public mutating func update(region: OverlayResizeRegion?) -> OverlayResizeCursorAction {
        if region == activeRegion {
            return region.map(OverlayResizeCursorAction.set) ?? .none
        }

        let previousRegion = activeRegion
        activeRegion = region

        if let region {
            return .set(region)
        }
        return previousRegion == nil ? .none : .restoreDefault
    }
}

public struct OverlayResizeHitTester: Sendable {
    public let edgeThickness: CGFloat
    public let cornerSize: CGFloat

    public init(edgeThickness: CGFloat = 14, cornerSize: CGFloat = 32) {
        self.edgeThickness = max(0, edgeThickness)
        self.cornerSize = max(edgeThickness, cornerSize)
    }

    public func region(
        at point: CGPoint,
        in bounds: CGRect,
        isFlipped: Bool = false
    ) -> OverlayResizeRegion? {
        guard
            bounds.width > 0,
            bounds.height > 0,
            point.x >= bounds.minX,
            point.x <= bounds.maxX,
            point.y >= bounds.minY,
            point.y <= bounds.maxY
        else {
            return nil
        }

        let nearLeftCorner = point.x <= bounds.minX + cornerSize
        let nearRightCorner = point.x >= bounds.maxX - cornerSize
        let nearTopCorner = isFlipped
            ? point.y <= bounds.minY + cornerSize
            : point.y >= bounds.maxY - cornerSize
        let nearBottomCorner = isFlipped
            ? point.y >= bounds.maxY - cornerSize
            : point.y <= bounds.minY + cornerSize

        if nearLeftCorner, nearTopCorner { return .topLeft }
        if nearRightCorner, nearTopCorner { return .topRight }
        if nearLeftCorner, nearBottomCorner { return .bottomLeft }
        if nearRightCorner, nearBottomCorner { return .bottomRight }

        if isFlipped
            ? point.y <= bounds.minY + edgeThickness
            : point.y >= bounds.maxY - edgeThickness {
            return .top
        }
        if point.x <= bounds.minX + edgeThickness { return .left }
        if isFlipped
            ? point.y >= bounds.maxY - edgeThickness
            : point.y <= bounds.minY + edgeThickness {
            return .bottom
        }
        if point.x >= bounds.maxX - edgeThickness { return .right }
        return nil
    }

    public func rect(
        for region: OverlayResizeRegion,
        in bounds: CGRect,
        isFlipped: Bool = false
    ) -> CGRect {
        let edge = min(edgeThickness, min(bounds.width, bounds.height))
        let corner = min(cornerSize, min(bounds.width / 2, bounds.height / 2))
        let topY = isFlipped ? bounds.minY : bounds.maxY - corner
        let bottomY = isFlipped ? bounds.maxY - corner : bounds.minY
        let topEdgeY = isFlipped ? bounds.minY : bounds.maxY - edge
        let bottomEdgeY = isFlipped ? bounds.maxY - edge : bounds.minY

        return switch region {
        case .topLeft:
            CGRect(x: bounds.minX, y: topY, width: corner, height: corner)
        case .topRight:
            CGRect(x: bounds.maxX - corner, y: topY, width: corner, height: corner)
        case .bottomLeft:
            CGRect(x: bounds.minX, y: bottomY, width: corner, height: corner)
        case .bottomRight:
            CGRect(x: bounds.maxX - corner, y: bottomY, width: corner, height: corner)
        case .top:
            CGRect(
                x: bounds.minX + corner,
                y: topEdgeY,
                width: max(0, bounds.width - corner * 2),
                height: edge
            )
        case .left:
            CGRect(
                x: bounds.minX,
                y: bounds.minY + corner,
                width: edge,
                height: max(0, bounds.height - corner * 2)
            )
        case .bottom:
            CGRect(
                x: bounds.minX + corner,
                y: bottomEdgeY,
                width: max(0, bounds.width - corner * 2),
                height: edge
            )
        case .right:
            CGRect(
                x: bounds.maxX - edge,
                y: bounds.minY + corner,
                width: edge,
                height: max(0, bounds.height - corner * 2)
            )
        }
    }
}

public enum SubtitleOverlayCollapseLayout {
    public static func collapsedFrame(
        from expandedFrame: CGRect,
        compactSize: CGSize
    ) -> CGRect {
        CGRect(
            x: expandedFrame.midX - compactSize.width / 2,
            y: expandedFrame.minY,
            width: compactSize.width,
            height: compactSize.height
        )
    }

    public static func expandedFrame(
        from collapsedFrame: CGRect,
        expandedSize: CGSize
    ) -> CGRect {
        CGRect(
            x: collapsedFrame.midX - expandedSize.width / 2,
            y: collapsedFrame.minY,
            width: expandedSize.width,
            height: expandedSize.height
        )
    }
}
