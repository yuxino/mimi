import MimiCore

func runConnectionHealthTests(using runner: inout TestRunner) {
    runner.run("websocket health timeout has a useful message") {
        try expectEqual(
            LiveTranslateClientError.healthCheckTimedOut.errorDescription,
            "The live translation connection stopped responding."
        )
    }
}
