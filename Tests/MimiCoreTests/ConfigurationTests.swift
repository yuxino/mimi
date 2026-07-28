import MimiCore

func runConfigurationTests(using runner: inout TestRunner) {
    runner.run("automatic language has a clear display name") {
        try expectEqual(SourceLanguage.automatic.displayName, "自动识别")
    }

    runner.run("automatic language resolves high quality to low latency") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "sk-test",
            sourceLanguage: .automatic,
            translationMode: .highQuality
        )

        try expectEqual(configuration.effectiveTranslationMode, .lowLatency)
    }

    runner.run("configuration defaults to low-latency translation") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "sk-test",
            sourceLanguage: .japanese
        )

        try expectEqual(configuration.translationMode, .lowLatency)
    }

    runner.run("configuration preserves an explicit translation mode") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "sk-test",
            sourceLanguage: .japanese,
            translationMode: .highQuality
        )
        let validated = try configuration.validated()

        try expectEqual(validated.translationMode, .highQuality)
    }

    runner.run("configuration requires an API key") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "   ",
            sourceLanguage: .english
        )

        do {
            try configuration.validate()
            throw TestFailure(description: "expected a missing API key error")
        } catch LiveTranslationConfigurationError.missingAPIKey {
            // Expected.
        }
    }

    runner.run("configuration validates the Workspace ID") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "bad.example.com",
            apiKey: "sk-test",
            sourceLanguage: .english
        )

        do {
            try configuration.validate()
            throw TestFailure(description: "expected an invalid Workspace ID error")
        } catch LiveTranslationConfigurationError.invalidWorkspaceID {
            // Expected.
        }
    }

    runner.run("configuration trims valid credentials") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "  ws-abc123  ",
            apiKey: "  sk-test  ",
            sourceLanguage: .korean
        )
        let validated = try configuration.validated()

        try expectEqual(validated.workspaceID, "ws-abc123")
        try expectEqual(validated.apiKey, "sk-test")
        try expectEqual(validated.sourceLanguage, .korean)
    }
}
