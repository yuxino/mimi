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

    runner.run("duplicate finals do not duplicate history") {
        var reducer = SubtitleReducer()
        reducer.apply(.sourceFinal("Hello."))
        reducer.apply(.translationFinal("你好。"))
        reducer.apply(.translationFinal("你好。"))

        try expectEqual(reducer.snapshot.history.count, 1)
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
