import MimiCore

func runASRDraftCommitterTests(using runner: inout TestRunner) {
    runner.run("ASR draft committer keeps an incomplete tail pending") {
        var committer = ASRDraftCommitter()

        try expectEqual(committer.updateDraft("今日は"), "今日は")
        try expectEqual(committer.updateDraft("今日は晴れ"), "今日は晴れ")
        // No sentence-ending punctuation yet: nothing is committed.
        try expectEqual(committer.commitCompleteSentences(), nil)
        try expectEqual(committer.updateDraft("今日は晴れです"), "今日は晴れです")
        try expectEqual(committer.commitCompleteSentences(), nil)
        try expect(committer.hasPendingText, "the incomplete tail should stay pending")
    }

    runner.run("ASR draft committer commits only complete sentences") {
        var committer = ASRDraftCommitter()

        try expectEqual(
            committer.updateDraft("こんにちは。今日は天気が"),
            "こんにちは。今日は天気が"
        )
        try expectEqual(committer.commitCompleteSentences(), "こんにちは。")
        // The incomplete tail remains pending and is not committed.
        try expectEqual(committer.commitCompleteSentences(), nil)
        try expectEqual(committer.updateDraft("こんにちは。今日は天気がいいですね。"), "今日は天気がいいですね。")
        try expectEqual(committer.commitCompleteSentences(), "今日は天気がいいですね。")
    }

    runner.run("ASR draft committer splits multiple sentences per draft") {
        var committer = ASRDraftCommitter()

        try expectEqual(
            committer.updateDraft("あ！え？うん。まだ"),
            "あ！え？うん。まだ"
        )
        try expectEqual(committer.commitCompleteSentences(), "あ！え？うん。")
        try expectEqual(committer.updateDraft("あ！え？うん。まだ続きます"), "まだ続きます")
    }

    runner.run("ASR draft committer supports English and Chinese delimiters") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("Hello there. How are you?")
        try expectEqual(committer.commitCompleteSentences(), "Hello there. How are you?")

        var chinese = ASRDraftCommitter()
        _ = chinese.updateDraft("你好！今天天气不错。明天")
        try expectEqual(chinese.commitCompleteSentences(), "你好！今天天气不错。")
    }

    runner.run("ASR draft committer suppresses an already committed server final") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("こんにちは。今日は")
        try expectEqual(committer.commitCompleteSentences(), "こんにちは。")
        // The same sentence arriving later as a server final must be dropped.
        try expectEqual(committer.finishSentence("こんにちは。"), nil)
        // A genuinely new final is committed.
        try expectEqual(committer.finishSentence("今日は天気がいいですね。"), "今日は天気がいいですね。")
    }

    runner.run("ASR draft committer commits a clean server final after drafts") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("今日は晴れ")
        _ = committer.commitCompleteSentences()
        try expectEqual(
            committer.finishSentence("今日は晴れですが、寒いです"),
            "今日は晴れですが、寒いです"
        )
        try expectEqual(committer.updateDraft("次の文です"), "次の文です")
    }

    runner.run("ASR draft committer strips overlapping suffix from late final") {
        var committer = ASRDraftCommitter()

        // Long-incomplete fallback committed a mid-sentence chunk.
        let longDraft = String(repeating: "あいうえお", count: 10)
        _ = committer.updateDraft(longDraft)
        try expectEqual(committer.commitLatestDraft(commitLongIncomplete: true), longDraft)
        // The server final overlaps the committed chunk; only the new tail is returned.
        try expectEqual(
            committer.finishSentence(longDraft + "かきくけこ"),
            "かきくけこ"
        )
    }

    runner.run("ASR draft committer ignores punctuation-only finals") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("今日は晴れです")
        _ = committer.commitCompleteSentences()
        try expectEqual(committer.finishSentence("。"), nil)
    }

    runner.run("ASR draft committer keeps subtitles flowing with long incomplete speech") {
        var committer = ASRDraftCommitter()

        let shortIncomplete = "まだ話してる途中"
        _ = committer.updateDraft(shortIncomplete)
        try expectEqual(
            committer.commitLatestDraft(commitLongIncomplete: true),
            nil,
            "short incomplete text should stay pending even on max-wait"
        )

        let longIncomplete = "話し手が長いあいだ途切れずに話し続けても字幕は読みやすい長さで"
        _ = committer.updateDraft(longIncomplete)
        try expectEqual(
            committer.commitLatestDraft(commitLongIncomplete: true),
            longIncomplete
        )
    }

    runner.run("ASR draft committer resets all sentence state") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("こんにちは。途中まで")
        _ = committer.commitCompleteSentences()
        committer.reset()

        try expectEqual(committer.updateDraft("こんにちは。途中まで別の文"), "こんにちは。途中まで別の文")
        try expectEqual(committer.commitCompleteSentences(), "こんにちは。")
        try expect(committer.hasPendingText, "a new draft should be pending after reset")
    }
}
