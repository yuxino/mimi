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

    runner.run("Qwen-MT final request selects Flash with natural-dialogue guidance") {
        let guidance = """
        Natural spoken dialogue. Preserve meaningful interjections and hesitation \
        as natural Chinese particles such as 嗯、啊、呢、吧、嘛.
        """
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
