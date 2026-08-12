import MimiCore

func runConfigurationTests(using runner: inout TestRunner) {
    runner.run("automatic language has a clear display name") {
        try expectEqual(SourceLanguage.automatic.displayName, "自动识别")
    }

    runner.run("manual source languages are available for high-quality switching") {
        try expectEqual(
            SourceLanguage.manualCases,
            [.japanese, .english, .korean, .chinese]
        )
    }

    runner.run("Chinese quick switch shows original subtitles") {
        try expectEqual(
            SourceLanguage.chinese.targetLanguageAfterQuickSwitch(
                from: .japanese,
                currentTarget: .simplifiedChinese
            ),
            .original
        )
    }

    runner.run("leaving Chinese original mode restores Chinese translation") {
        try expectEqual(
            SourceLanguage.japanese.targetLanguageAfterQuickSwitch(
                from: .chinese,
                currentTarget: .original
            ),
            .simplifiedChinese
        )
    }

    runner.run("ordinary language switches preserve a custom target") {
        try expectEqual(
            SourceLanguage.english.targetLanguageAfterQuickSwitch(
                from: .japanese,
                currentTarget: .english
            ),
            .english
        )
    }

    runner.run("automatic language status includes the detected language") {
        let japanese = DetectedLanguage(reportedLanguage: "ja-JP")

        try expectEqual(
            SourceLanguage.automatic.statusDisplayName(
                detectedLanguage: japanese,
                targetLanguage: .simplifiedChinese
            ),
            "自动识别（日本語）"
        )
        try expectEqual(
            SourceLanguage.automatic.statusDisplayName(
                detectedLanguage: nil,
                targetLanguage: .simplifiedChinese
            ),
            "自动识别中"
        )
        try expectEqual(
            SourceLanguage.automatic.statusDisplayName(
                detectedLanguage: DetectedLanguage(reportedLanguage: "zh"),
                targetLanguage: .simplifiedChinese
            ),
            "自动识别中"
        )
        try expectEqual(
            SourceLanguage.automatic.statusDisplayName(
                detectedLanguage: DetectedLanguage(reportedLanguage: "zh"),
                targetLanguage: .english
            ),
            "自动识别（中文）"
        )
        try expectEqual(
            SourceLanguage.japanese.statusDisplayName(
                detectedLanguage: DetectedLanguage(reportedLanguage: "en"),
                targetLanguage: .simplifiedChinese
            ),
            "日本語"
        )
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
        try expectEqual(configuration.targetLanguage, .simplifiedChinese)
    }

    runner.run("target languages expose service codes and display names") {
        try expectEqual(TargetLanguage.original.translatesAudio, false)
        try expectEqual(TargetLanguage.original.displayName, "原文（不翻译）")
        try expectEqual(TargetLanguage.simplifiedChinese.rawValue, "zh")
        try expectEqual(TargetLanguage.english.qwenMTName, "English")
        try expectEqual(TargetLanguage.japanese.displayName, "日本語")
    }

    runner.run("detected languages normalize service codes for display") {
        try expectEqual(
            DetectedLanguage(reportedLanguage: "ja-JP")?.displayName,
            "日本語"
        )
        try expectEqual(
            DetectedLanguage(reportedLanguage: "yue")?.displayName,
            "粤语"
        )
        try expectEqual(
            DetectedLanguage(reportedLanguage: "unknown")?.displayName,
            "UNKNOWN"
        )
    }

    runner.run("original subtitles preserve the strongest recognition backend") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "workspace",
            apiKey: "secret",
            sourceLanguage: .japanese,
            targetLanguage: .original,
            translationMode: .highQuality
        )

        try expectEqual(configuration.effectiveTranslationMode, .highQuality)
    }

    runner.run("configuration preserves an explicit translation mode") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "sk-test",
            sourceLanguage: .japanese,
            targetLanguage: .english,
            translationMode: .highQuality
        )
        let validated = try configuration.validated()

        try expectEqual(validated.translationMode, .highQuality)
        try expectEqual(validated.targetLanguage, .english)
    }

    runner.run("turbo mode stays turbo even with automatic source") {
        let configuration = LiveTranslationConfiguration(
            workspaceID: "ws-abc123",
            apiKey: "sk-test",
            sourceLanguage: .automatic,
            translationMode: .turbo
        )

        try expectEqual(configuration.effectiveTranslationMode, .turbo)
    }

    runner.run("translation modes expose short display names") {
        try expectEqual(TranslationMode.turbo.displayName, "极速")
        try expectEqual(TranslationMode.lowLatency.displayName, "低延迟")
        try expectEqual(TranslationMode.highQuality.displayName, "高质量")
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
