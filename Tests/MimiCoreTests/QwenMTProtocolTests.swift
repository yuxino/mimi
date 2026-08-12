import Foundation
import MimiCore

func runQwenMTProtocolTests(using runner: inout TestRunner) {
    runner.run("Qwen-MT endpoint builds the workspace chat-completions URL") {
        let endpoint = try QwenMTEndpoint(workspaceID: "ws-abc123")
        try expectEqual(
            endpoint.url.absoluteString,
            "https://ws-abc123.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions"
        )
    }

    runner.run("Qwen-MT request selects Lite and full language names") {
        let data = try QwenMTRequestEncoder.request(
            text: "今日は晴れです。",
            sourceLanguage: .japanese
        )
        let json = try mtJSONObject(data)
        let messages = json["messages"] as? [[String: Any]]
        let options = try mtRequiredObject(json["translation_options"])

        try expectEqual(json["model"] as? String, "qwen-mt-lite")
        try expectEqual(json["stream"] as? Bool, false)
        try expectEqual(messages?.first?["role"] as? String, "user")
        try expectEqual(messages?.first?["content"] as? String, "今日は晴れです。")
        try expectEqual(options["source_lang"] as? String, "Japanese")
        try expectEqual(options["target_lang"] as? String, "Chinese")
    }

    runner.run("Qwen-MT request can enable incremental streaming") {
        let data = try QwenMTRequestEncoder.request(
            text: "今日は晴れです。",
            sourceLanguage: .japanese,
            stream: true
        )
        let json = try mtJSONObject(data)

        try expectEqual(json["stream"] as? Bool, true)
    }

    runner.run("Qwen-MT request selects an explicit target language") {
        let data = try QwenMTRequestEncoder.request(
            text: "今日は晴れです。",
            sourceLanguage: .japanese,
            targetLanguage: .english
        )
        let json = try mtJSONObject(data)
        let options = try mtRequiredObject(json["translation_options"])

        try expectEqual(options["target_lang"] as? String, "English")
    }

    runner.run("Qwen-MT request preserves vocal sounds in natural-dialogue guidance") {
        let guidance = QwenMTDomainHint.spokenDialogue(for: .simplifiedChinese)
        let data = try QwenMTRequestEncoder.request(
            text: "今日は晴れです。",
            sourceLanguage: .japanese,
            model: .flash,
            stream: true,
            domainHint: guidance
        )
        let json = try mtJSONObject(data)
        let options = try mtRequiredObject(json["translation_options"])

        try expectEqual(json["model"] as? String, "qwen-mt-flash")
        try expectEqual(options["domains"] as? String, guidance)
        try expect(
            guidance.contains("gasps, moans, and cries"),
            "guidance should preserve vocal sounds"
        )
        try expect(
            guidance.contains("嗯、啊、呢、吧、嘛"),
            "guidance should preserve Chinese particles"
        )
        try expect(
            guidance.contains("えっと"),
            "guidance should preserve Japanese fillers"
        )
        try expect(
            guidance.contains("polite or formal Japanese"),
            "guidance should keep polite Japanese natural"
        )
        try expect(
            guidance.contains("Output only the translation text"),
            "guidance should constrain the response to the translation"
        )
        try expect(
            guidance.contains("Do not sanitize, euphemize, censor, or omit"),
            "guidance should preserve explicit dialogue"
        )
        try expect(
            !guidance.contains("do not mechanically translate every filler"),
            "guidance should not filter filler sounds"
        )
    }

    runner.run("Qwen-MT request includes bounded translation memory pairs") {
        let data = try QwenMTRequestEncoder.request(
            text: "そうなんですね。",
            sourceLanguage: .japanese,
            model: .flash,
            translationMemory: [
                .init(source: "今日は晴れです。", target: "今天天气很好。")
            ]
        )
        let json = try mtJSONObject(data)
        let options = try mtRequiredObject(json["translation_options"])
        let memory = options["tm_list"] as? [[String: Any]]

        try expectEqual(memory?.first?["source"] as? String, "今日は晴れです。")
        try expectEqual(memory?.first?["target"] as? String, "今天天气很好。")
    }

    runner.run("Qwen-MT request can select the highest-quality Plus model") {
        let data = try QwenMTRequestEncoder.request(
            text: "今日はいい天気ですね。",
            sourceLanguage: .japanese,
            targetLanguage: .simplifiedChinese,
            model: .plus
        )
        let json = try mtJSONObject(data)

        try expectEqual(json["model"] as? String, "qwen-mt-plus")
    }

    runner.run("Qwen-MT automatically detects the source language") {
        let data = try QwenMTRequestEncoder.request(
            text: "Hello, world.",
            sourceLanguage: .automatic
        )
        let json = try mtJSONObject(data)
        let options = try mtRequiredObject(json["translation_options"])

        try expectEqual(options["source_lang"] as? String, "auto")
    }

    runner.run("ASR language reports resolve to explicit Qwen-MT languages") {
        try expectEqual(SourceLanguage(detectedLanguage: "ja-JP"), .japanese)
        try expectEqual(SourceLanguage(detectedLanguage: "English"), .english)
        try expectEqual(SourceLanguage(detectedLanguage: "ko"), .korean)
        try expectEqual(SourceLanguage(detectedLanguage: "zh-CN"), .chinese)
        try expectEqual(SourceLanguage(detectedLanguage: "unknown"), nil)
    }

    runner.run("Qwen-MT response decodes and trims translated content") {
        let translation = try QwenMTResponseDecoder.decode(
            Data(
                #"{"choices":[{"message":{"role":"assistant","content":"  今天天气晴朗。  "}}]}"#.utf8
            )
        )

        try expectEqual(translation, "今天天气晴朗。")
    }

    runner.run("Qwen-MT response requires translated content") {
        do {
            _ = try QwenMTResponseDecoder.decode(Data(#"{"choices":[]}"#.utf8))
            throw TestFailure(description: "expected a missing translation error")
        } catch QwenMTProtocolError.missingTranslation {
            // Expected.
        }
    }

    runner.run("Qwen-MT stream chunk decodes incremental content") {
        let content = try QwenMTStreamDecoder.decodeChunk(
            Data(#"{"choices":[{"delta":{"role":"assistant","content":"今天"}}]}"#.utf8)
        )
        let terminalContent = try QwenMTStreamDecoder.decodeChunk(
            Data(#"{"choices":[]}"#.utf8)
        )

        try expectEqual(content, "今天")
        try expectEqual(terminalContent, nil)
    }

    runner.run("Qwen-MT timeout has a useful error message") {
        try expectEqual(
            QwenMTClientError.requestTimedOut.localizedDescription,
            "Qwen-MT took too long to respond."
        )
    }

    runner.run("Qwen-MT diagnostics retain status without response content") {
        try expectEqual(
            PipelineDiagnostics.errorLabel(
                QwenMTClientError.requestFailed(
                    statusCode: 429,
                    message: "sensitive response detail"
                )
            ),
            "QwenMTClientError.requestFailed(status=429)"
        )
    }

    runner.run("Qwen-MT retry policy backs off only for transient failures") {
        try expectEqual(
            QwenMTRetryPolicy.delay(
                after: .requestTimedOut,
                attempt: 1
            ),
            .milliseconds(600)
        )
        try expectEqual(
            QwenMTRetryPolicy.delay(
                after: .requestFailed(statusCode: 429, message: "busy"),
                attempt: 3
            ),
            .milliseconds(2_400)
        )
        try expectEqual(
            QwenMTRetryPolicy.delay(
                after: .requestFailed(statusCode: 503, message: "down"),
                attempt: 8
            ),
            .seconds(8)
        )
        try expectEqual(
            QwenMTRetryPolicy.delay(
                after: .requestFailed(statusCode: 401, message: "bad key"),
                attempt: 1
            ),
            nil
        )
        try expectEqual(
            QwenMTRetryPolicy.delay(
                after: .requestFailed(statusCode: 400, message: "bad request"),
                attempt: 1
            ),
            nil
        )
    }
}

private func mtJSONObject(_ data: Data) throws -> [String: Any] {
    guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw TestFailure(description: "expected a JSON object")
    }
    return object
}

private func mtRequiredObject(_ value: Any?) throws -> [String: Any] {
    guard let object = value as? [String: Any] else {
        throw TestFailure(description: "expected a nested JSON object")
    }
    return object
}
