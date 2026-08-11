import MimiCore

func runSessionControllerTests(using runner: inout TestRunner) {
    runner.run("session follows the happy-path lifecycle") {
        var controller = TranslationSessionController()

        controller.beginConnecting()
        try expectEqual(controller.state.status, .connecting)

        controller.didConnect()
        try expectEqual(controller.state.status, .listening)

        controller.beginStopping()
        try expectEqual(controller.state.status, .stopping)

        controller.didStop()
        try expectEqual(controller.state.status, .idle)
    }

    runner.run("server events update subtitle state") {
        var controller = TranslationSessionController()
        controller.handle(.sourceDraft(text: "Hello wor", language: "en"))
        controller.handle(.translationDraft("你好，世"))

        try expectEqual(controller.state.subtitles.source.text, "Hello wor")
        try expectEqual(controller.state.subtitles.source.isFinal, false)
        try expectEqual(controller.state.subtitles.translation.text, "你好，世")

        controller.handle(.sourceFinal(text: "Hello world.", language: "en"))
        controller.handle(.translationFinal("你好，世界。"))

        try expectEqual(controller.state.subtitles.history.count, 1)
        try expectEqual(controller.state.subtitles.translation.isFinal, true)
        try expectEqual(controller.state.detectedLanguage?.displayName, "English")
    }

    runner.run("a new connection clears the previously detected language") {
        var controller = TranslationSessionController()
        controller.handle(.sourceDraft(text: "こんにちは", language: "ja"))
        try expectEqual(controller.state.detectedLanguage?.displayName, "日本語")

        controller.beginConnecting()

        try expectEqual(controller.state.detectedLanguage, nil)
    }

    runner.run("service errors move the session to error") {
        var controller = TranslationSessionController()
        controller.beginConnecting()
        controller.handle(.error(code: "invalid_value", message: "Bad language"))

        try expectEqual(controller.state.status, .error("Bad language"))
    }

    runner.run("translation activity follows the real Plus request lifecycle") {
        var controller = TranslationSessionController()
        controller.didConnect()

        controller.handle(.translationStarted)
        try expectEqual(controller.state.isTranslationPending, true)

        controller.handle(.translationFinal("翻译完成。"))
        try expectEqual(controller.state.isTranslationPending, false)

        controller.handle(.translationStarted)
        controller.didFail("Request failed")
        try expectEqual(controller.state.isTranslationPending, false)
    }

    runner.run("pausing clears translation activity without discarding subtitles") {
        var controller = TranslationSessionController()
        controller.didConnect()
        controller.handle(.sourceFinal(text: "Please wait.", language: "en"))
        controller.handle(.translationFinal("请稍等。"))
        controller.handle(.translationStarted)
        let subtitlesBeforePause = controller.state.subtitles

        controller.didPause()

        try expectEqual(controller.state.status, .listening)
        try expectEqual(controller.state.isTranslationPending, false)
        try expectEqual(controller.state.subtitles, subtitlesBeforePause)
    }

    runner.run("clearing subtitles does not change session status") {
        var controller = TranslationSessionController()
        controller.didConnect()
        controller.handle(.sourceFinal(text: "Hello.", language: "en"))
        controller.handle(.translationFinal("你好。"))

        controller.clearSubtitles()

        try expectEqual(controller.state.status, .listening)
        try expectEqual(controller.state.subtitles, .empty)
    }

    runner.run("stopping ignores flushed tail subtitles") {
        var controller = TranslationSessionController()
        controller.didConnect()
        controller.handle(.sourceFinal(text: "Last real line.", language: "en"))
        controller.handle(.translationFinal("最后一句正常字幕。"))
        let subtitlesBeforeStopping = controller.state.subtitles

        controller.beginStopping()
        controller.handle(.sourceFinal(text: "Translation mode ended.", language: "en"))
        controller.handle(.translationFinal("翻译模式已结束。"))
        controller.handle(.sessionFinished)

        try expectEqual(controller.state.status, .stopping)
        try expectEqual(controller.state.subtitles, subtitlesBeforeStopping)
    }

    runner.run("unknown server events leave state unchanged") {
        var controller = TranslationSessionController()
        let before = controller.state
        controller.handle(.ignored(type: "response.created"))
        try expectEqual(controller.state, before)
    }

    runner.run("subtitle revocation removes the last history pair") {
        var controller = TranslationSessionController()
        controller.didConnect()
        controller.handle(.sourceFinal(text: "Hello.", language: "en"))
        controller.handle(.translationFinal("你好。"))

        controller.handle(.subtitleRevoked)

        try expectEqual(controller.state.subtitles.history, [])
    }
}
