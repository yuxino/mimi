import MimiCore

func runDraftPreviewTrackerTests(using runner: inout TestRunner) {
    runner.run("draft preview tracker suppresses repeated text") {
        var tracker = DraftPreviewTracker()

        let firstGeneration = tracker.update("今日は")
        let repeatedGeneration = tracker.update("今日は")

        try expect(firstGeneration != nil, "the first draft should start a preview")
        try expectEqual(repeatedGeneration, nil)
    }

    runner.run("draft preview tracker rejects a stale result") {
        var tracker = DraftPreviewTracker()
        let firstGeneration = try requiredGeneration(tracker.update("今日は"))
        let latestGeneration = try requiredGeneration(tracker.update("今日は晴れです"))

        try expect(
            !tracker.accepts(text: "今日は", generation: firstGeneration),
            "an older translation must not replace the latest draft"
        )
        try expect(
            tracker.accepts(text: "今日は晴れです", generation: latestGeneration),
            "the latest translation should remain valid"
        )
    }

    runner.run("draft preview tracker reset invalidates in-flight work") {
        var tracker = DraftPreviewTracker()
        let generation = try requiredGeneration(tracker.update("途中まで"))

        tracker.reset()

        try expect(
            !tracker.accepts(text: "途中まで", generation: generation),
            "reset should reject callbacks from cancelled work"
        )
    }
}

private func requiredGeneration(_ generation: Int?) throws -> Int {
    guard let generation else {
        throw TestFailure(description: "expected a preview generation")
    }
    return generation
}
