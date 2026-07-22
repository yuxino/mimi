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
    }

    runner.run("service errors move the session to error") {
        var controller = TranslationSessionController()
        controller.beginConnecting()
        controller.handle(.error(code: "invalid_value", message: "Bad language"))

        try expectEqual(controller.state.status, .error("Bad language"))
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

    runner.run("unknown server events leave state unchanged") {
        var controller = TranslationSessionController()
        let before = controller.state
        controller.handle(.ignored(type: "response.created"))
        try expectEqual(controller.state, before)
    }
}
