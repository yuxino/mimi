import MimiCore

func runASRDraftCommitterTests(using runner: inout TestRunner) {
    runner.run("ASR draft committer commits the latest uncommitted text") {
        var committer = ASRDraftCommitter()

        try expectEqual(committer.updateDraft("今日は"), "今日は")
        try expectEqual(committer.updateDraft("今日は晴れ"), "今日は晴れ")
        try expectEqual(committer.commitLatestDraft(), "今日は晴れ")
        try expectEqual(committer.updateDraft("今日は晴れです"), "です")
    }

    runner.run("ASR draft committer suppresses an exact late final") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("今日は晴れです")
        try expectEqual(committer.commitLatestDraft(), "今日は晴れです")
        try expectEqual(committer.finishSentence("今日は晴れです"), nil)
        try expectEqual(committer.updateDraft("次の文です"), "次の文です")
    }

    runner.run("ASR draft committer preserves a late final suffix") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("今日は晴れ")
        try expectEqual(committer.commitLatestDraft(), "今日は晴れ")
        try expectEqual(committer.finishSentence("今日は晴れですが、寒いです"), "ですが、寒いです")
    }

    runner.run("ASR draft committer ignores punctuation-only late tails") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("今日は晴れです")
        _ = committer.commitLatestDraft()
        try expectEqual(committer.finishSentence("今日は晴れです。"), nil)
    }

    runner.run("ASR draft committer resets all sentence state") {
        var committer = ASRDraftCommitter()

        _ = committer.updateDraft("途中まで")
        _ = committer.commitLatestDraft()
        committer.reset()

        try expectEqual(committer.updateDraft("途中まで別の文"), "途中まで別の文")
        try expect(committer.hasPendingText, "a new draft should be pending after reset")
    }
}
