import CoreGraphics
import Foundation
import MimiCore

func runOverlayResizeInteractionTests(using runner: inout TestRunner) {
    let bounds = CGRect(x: 0, y: 0, width: 640, height: 136)
    let hitTester = OverlayResizeHitTester()

    runner.run("all four overlay corners have a resize region") {
        try expectEqual(hitTester.region(at: CGPoint(x: 0, y: 136), in: bounds), .topLeft)
        try expectEqual(hitTester.region(at: CGPoint(x: 640, y: 136), in: bounds), .topRight)
        try expectEqual(hitTester.region(at: CGPoint(x: 0, y: 0), in: bounds), .bottomLeft)
        try expectEqual(hitTester.region(at: CGPoint(x: 640, y: 0), in: bounds), .bottomRight)
    }

    runner.run("corner resize regions take priority over edge regions") {
        try expectEqual(hitTester.region(at: CGPoint(x: 8, y: 130), in: bounds), .topLeft)
        try expectEqual(hitTester.region(at: CGPoint(x: 632, y: 130), in: bounds), .topRight)
        try expectEqual(hitTester.region(at: CGPoint(x: 8, y: 6), in: bounds), .bottomLeft)
        try expectEqual(hitTester.region(at: CGPoint(x: 632, y: 6), in: bounds), .bottomRight)
    }

    runner.run("overlay edges remain independently resizable") {
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: 130), in: bounds), .top)
        try expectEqual(hitTester.region(at: CGPoint(x: 8, y: 68), in: bounds), .left)
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: 6), in: bounds), .bottom)
        try expectEqual(hitTester.region(at: CGPoint(x: 632, y: 68), in: bounds), .right)
    }

    runner.run("subtitle content does not claim a resize cursor") {
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: 68), in: bounds), nil)
    }

    runner.run("resize hit testing includes exact top and right boundaries") {
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: bounds.maxY), in: bounds), .top)
        try expectEqual(hitTester.region(at: CGPoint(x: bounds.maxX, y: 68), in: bounds), .right)
    }

    runner.run("flipped hosting-view coordinates preserve visual corner directions") {
        try expectEqual(
            hitTester.region(at: CGPoint(x: 0, y: 0), in: bounds, isFlipped: true),
            .topLeft
        )
        try expectEqual(
            hitTester.region(at: CGPoint(x: 640, y: 0), in: bounds, isFlipped: true),
            .topRight
        )
        try expectEqual(
            hitTester.region(at: CGPoint(x: 0, y: 136), in: bounds, isFlipped: true),
            .bottomLeft
        )
        try expectEqual(
            hitTester.region(at: CGPoint(x: 640, y: 136), in: bounds, isFlipped: true),
            .bottomRight
        )
    }

    runner.run("flipped hosting-view cursor rectangles match visual regions") {
        for region in OverlayResizeRegion.allCases {
            let rect = hitTester.rect(for: region, in: bounds, isFlipped: true)
            try expectEqual(
                hitTester.region(
                    at: CGPoint(x: rect.midX, y: rect.midY),
                    in: bounds,
                    isFlipped: true
                ),
                region
            )
        }
    }

    runner.run("resize regions adapt to minimum and maximum overlay sizes") {
        let sizes = [
            CGSize(width: 360, height: 100),
            CGSize(width: 1_200, height: 600)
        ]

        for size in sizes {
            let currentBounds = CGRect(origin: .zero, size: size)
            for region in OverlayResizeRegion.allCases {
                let rect = hitTester.rect(for: region, in: currentBounds)
                try expect(
                    currentBounds.contains(CGPoint(x: rect.midX, y: rect.midY)),
                    "\(region) must remain inside \(size)"
                )
                try expectEqual(
                    hitTester.region(at: CGPoint(x: rect.midX, y: rect.midY), in: currentBounds),
                    region
                )
            }
        }
    }

    runner.run("points outside the overlay never claim a resize cursor") {
        try expectEqual(hitTester.region(at: CGPoint(x: -1, y: 68), in: bounds), nil)
        try expectEqual(hitTester.region(at: CGPoint(x: 641, y: 68), in: bounds), nil)
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: -1), in: bounds), nil)
        try expectEqual(hitTester.region(at: CGPoint(x: 320, y: 137), in: bounds), nil)
    }

    runner.run("resize cursor state sets switches and restores the cursor") {
        var state = OverlayResizeCursorState()

        try expectEqual(state.update(region: .topLeft), .set(.topLeft))
        try expectEqual(
            state.update(region: .topLeft),
            .set(.topLeft),
            "AppKit may reset the cursor between mouse-moved events"
        )
        try expectEqual(state.update(region: .bottomRight), .set(.bottomRight))
        try expectEqual(state.update(region: nil), .restoreDefault)
        try expectEqual(state.update(region: nil), .none)
    }

    runner.run("collapsing keeps the overlay centered and bottom anchored") {
        let expanded = CGRect(x: 200, y: 72, width: 640, height: 136)
        let collapsed = SubtitleOverlayCollapseLayout.collapsedFrame(
            from: expanded,
            compactSize: CGSize(width: 280, height: 54)
        )

        try expectEqual(collapsed.midX, expanded.midX)
        try expectEqual(collapsed.maxY, expanded.maxY)
        try expectEqual(collapsed.size, CGSize(width: 280, height: 54))
    }

    runner.run("expanding follows a compact bar moved by the user") {
        let movedCompactBar = CGRect(x: 80, y: 180, width: 280, height: 54)
        let expanded = SubtitleOverlayCollapseLayout.expandedFrame(
            from: movedCompactBar,
            expandedSize: CGSize(width: 700, height: 180)
        )

        try expectEqual(expanded.midX, movedCompactBar.midX)
        try expectEqual(expanded.maxY, movedCompactBar.maxY)
        try expectEqual(expanded.size, CGSize(width: 700, height: 180))
    }

    runner.run("collapsing and expanding restores the original frame") {
        let expanded = CGRect(x: 200, y: 72, width: 640, height: 136)
        let collapsed = SubtitleOverlayCollapseLayout.collapsedFrame(
            from: expanded,
            compactSize: CGSize(width: 280, height: 54)
        )
        let restored = SubtitleOverlayCollapseLayout.expandedFrame(
            from: collapsed,
            expandedSize: expanded.size
        )

        try expectEqual(restored, expanded)
    }

    runner.run("collapsing a bottom-docked overlay keeps the compact bar on screen") {
        let expanded = CGRect(x: 400, y: 10, width: 640, height: 136)
        let collapsed = SubtitleOverlayCollapseLayout.collapsedFrame(
            from: expanded,
            compactSize: CGSize(width: 280, height: 54)
        )

        try expect(collapsed.minY >= 0, "compact bar should stay on screen at the bottom")
        try expectEqual(collapsed.maxY, expanded.maxY)
    }

    runner.run("expanding from a bar at the screen top stays on screen") {
        let screenHeight = 982.0
        let movedBar = CGRect(x: 400, y: screenHeight - 54, width: 280, height: 54)
        let expanded = SubtitleOverlayCollapseLayout.expandedFrame(
            from: movedBar,
            expandedSize: CGSize(width: 640, height: 136)
        )

        try expect(expanded.maxY <= screenHeight, "expanded window should stay on screen at the top")
        try expectEqual(expanded.maxY, movedBar.maxY)
    }
}
