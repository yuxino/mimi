import MimiCore

func runSubtitleReducerTests(using runner: inout TestRunner) {
    runner.run("subtitle reducer starts empty") {
        let reducer = SubtitleReducer()
        try expectEqual(reducer.snapshot, .empty)
    }

    runner.run("drafts remain visibly unconfirmed") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceDraft("Hello wor"))
        reducer.apply(.translationDraft("你好，世"))

        try expectEqual(reducer.snapshot.source, SubtitleLine(text: "Hello wor", isFinal: false))
        try expectEqual(reducer.snapshot.translation, SubtitleLine(text: "你好，世", isFinal: false))
    }

    runner.run("final translation creates a history pair") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("Hello world."))
        reducer.apply(.translationFinal("你好，世界。"))

        try expectEqual(reducer.snapshot.source, SubtitleLine(text: "Hello world.", isFinal: true))
        try expectEqual(reducer.snapshot.translation, SubtitleLine(text: "你好，世界。", isFinal: true))
        try expectEqual(
            reducer.snapshot.history,
            [SubtitlePair(source: "Hello world.", translation: "你好，世界。")]
        )
    }

    runner.run("a new draft keeps confirmed history available") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("Hello."))
        reducer.apply(.translationFinal("你好。"))
        reducer.apply(.sourceDraft("How are"))
        reducer.apply(.translationDraft("你最近"))

        try expectEqual(
            reducer.snapshot.history,
            [SubtitlePair(source: "Hello.", translation: "你好。")]
        )
        try expectEqual(
            reducer.snapshot.translation,
            SubtitleLine(text: "你最近", isFinal: false)
        )
    }

    runner.run("a Plus final replaces its preview and alone enters history") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceDraft("今日は晴れです"))
        reducer.apply(.translationDraft("今天晴天"))

        try expectEqual(reducer.snapshot.history, [])

        reducer.apply(.sourceFinal("今日は晴れです。"))
        reducer.apply(.translationFinal("今天天气很好。"))

        try expectEqual(
            reducer.snapshot.history,
            [SubtitlePair(source: "今日は晴れです。", translation: "今天天气很好。")]
        )
        try expectEqual(
            reducer.snapshot.translation,
            SubtitleLine(text: "今天天气很好。", isFinal: true)
        )
    }

    runner.run("a delayed final translation stays paired with its original source") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("First sentence."))
        reducer.apply(.sourceDraft("Second sentence"))
        reducer.apply(.translationFinal("第一句。"))

        try expectEqual(
            reducer.snapshot.history,
            [SubtitlePair(source: "First sentence.", translation: "第一句。")]
        )
        try expectEqual(
            reducer.snapshot.source,
            SubtitleLine(text: "Second sentence", isFinal: false)
        )
    }

    runner.run("duplicate finals do not duplicate history") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("Hello."))
        reducer.apply(.translationFinal("你好。"))
        reducer.apply(.translationFinal("你好。"))

        try expectEqual(reducer.snapshot.history.count, 1)
    }

    runner.run("revoking removes only the last confirmed pair") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("First."))
        reducer.apply(.translationFinal("第一句。"))
        reducer.apply(.sourceFinal("Second."))
        reducer.apply(.translationFinal("第二句。"))

        reducer.apply(.revokeLastConfirmed)

        try expectEqual(reducer.snapshot.history.count, 1)
        try expectEqual(reducer.snapshot.history[0].translation, "第一句。")
    }

    runner.run("revoking an empty history is a no-op") {
        var reducer = SubtitleReducer()
        reducer.apply(.revokeLastConfirmed)
        try expectEqual(reducer.snapshot.history, [])
    }

    runner.run("identical source and translation remain in history") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("嗯啊"))
        reducer.apply(.translationFinal("嗯啊"))

        try expectEqual(
            reducer.snapshot.history,
            [SubtitlePair(source: "嗯啊", translation: "嗯啊")]
        )
    }

    runner.run("history is bounded") {
        var reducer = SubtitleReducer(maxHistoryCount: 2)
        for index in 1...3 {
            reducer.apply(.sourceFinal("source \(index)"))
            reducer.apply(.translationFinal("translation \(index)"))
        }

        try expectEqual(
            reducer.snapshot.history,
            [
                SubtitlePair(source: "source 2", translation: "translation 2"),
                SubtitlePair(source: "source 3", translation: "translation 3")
            ]
        )
    }

    runner.run("clear resets all subtitle state") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("Hello."))
        reducer.apply(.translationFinal("你好。"))
        reducer.apply(.clear)

        try expectEqual(reducer.snapshot, .empty)
    }
}
