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
