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
}
