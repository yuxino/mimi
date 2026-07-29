import MimiCore

func runSubtitleTextSegmenterTests(using runner: inout TestRunner) {
    runner.run("short subtitle remains a single segment") {
        try expectEqual(
            SubtitleTextSegmenter.segments(in: "今天的天气很好。", maximumCharacters: 28),
            ["今天的天气很好。"]
        )
    }

    runner.run("sentence punctuation creates stable subtitle segments") {
        try expectEqual(
            SubtitleTextSegmenter.segments(
                in: "第一句话已经说完。第二句话也说完了！第三句还在继续",
                maximumCharacters: 28
            ),
            ["第一句话已经说完。", "第二句话也说完了！", "第三句还在继续"]
        )
    }

    runner.run("continuous CJK speech is bounded without losing text") {
        let text = "这是一段完全没有句号而且会持续不断增长的字幕内容用来模拟视频里人物一直讲话的情况"
        let segments = SubtitleTextSegmenter.segments(in: text, maximumCharacters: 14)

        try expect(segments.count > 1, "long continuous speech should be split")
        try expect(
            segments.allSatisfy { $0.count <= 14 },
            "every CJK segment should respect the requested limit"
        )
        try expectEqual(segments.joined(), text)
    }

    runner.run("English subtitles prefer word boundaries") {
        let text = "This is a continuous English subtitle that should never split a normal word."
        let segments = SubtitleTextSegmenter.segments(in: text, maximumCharacters: 24)

        try expect(segments.count > 1, "long English speech should be split")
        try expectEqual(segments.joined(separator: " "), text)
        try expect(
            segments.dropLast().allSatisfy { !$0.hasSuffix(" ") },
            "segments should not retain boundary whitespace"
        )
    }

    runner.run("extending a long draft preserves completed segment prefixes") {
        let first = SubtitleTextSegmenter.segments(
            in: "持续讲话时字幕会不断增长直到超过一行然后继续",
            maximumCharacters: 12
        )
        let extended = SubtitleTextSegmenter.segments(
            in: "持续讲话时字幕会不断增长直到超过一行然后继续显示后面的新增内容",
            maximumCharacters: 12
        )

        try expectEqual(Array(extended.prefix(first.count - 1)), Array(first.dropLast()))
    }
}
