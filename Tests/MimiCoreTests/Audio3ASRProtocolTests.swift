import Foundation
import MimiCore

func runAudio3ASRProtocolTests(using runner: inout TestRunner) {
    runner.run("Audio 3 ASR endpoint uses the binary inference WebSocket") {
        let endpoint = try Audio3ASREndpoint(workspaceID: "ws-abc123")
        try expectEqual(
            endpoint.url.absoluteString,
            "wss://ws-abc123.cn-beijing.maas.aliyuncs.com/api-ws/v1/inference"
        )
    }

    runner.run("Audio 3 run-task favors accurate sentence boundaries") {
        let data = try Audio3ASRRequestEncoder.runTask(
            taskID: "task-123",
            sourceLanguage: .japanese,
            context: "日本語の自然な会話"
        )
        let json = try audio3JSONObject(data)
        let header = try audio3RequiredObject(json["header"])
        let payload = try audio3RequiredObject(json["payload"])
        let parameters = try audio3RequiredObject(payload["parameters"])
        let input = try audio3RequiredObject(payload["input"])

        try expectEqual(header["action"] as? String, "run-task")
        try expectEqual(header["task_id"] as? String, "task-123")
        try expectEqual(payload["model"] as? String, "qwen-audio-3.0-asr-flash-streaming")
        try expectEqual(parameters["format"] as? String, "pcm")
        try expectEqual(parameters["sample_rate"] as? Int, 16_000)
        try expectEqual(parameters["semantic_punctuation_enabled"] as? Bool, true)
        try expectEqual(parameters["heartbeat"] as? Bool, true)
        try expectEqual(parameters["language_hints"] as? [String], ["ja"])
        try expect(input["context"] != nil, "dialogue context should improve recognition")
        try expect(parameters["special_word_filter"] == nil, "sensitive filtering must stay disabled")
    }

    runner.run("Audio 3 automatic recognition omits language hints") {
        let data = try Audio3ASRRequestEncoder.runTask(
            taskID: "task-auto",
            sourceLanguage: .automatic
        )
        let json = try audio3JSONObject(data)
        let payload = try audio3RequiredObject(json["payload"])
        let parameters = try audio3RequiredObject(payload["parameters"])

        try expect(parameters["language_hints"] == nil, "automatic recognition must not lock a language")
    }

    runner.run("Audio 3 finish-task preserves the task identifier") {
        let data = try Audio3ASRRequestEncoder.finishTask(taskID: "task-finish")
        let json = try audio3JSONObject(data)
        let header = try audio3RequiredObject(json["header"])
        let payload = try audio3RequiredObject(json["payload"])

        try expectEqual(header["action"] as? String, "finish-task")
        try expectEqual(header["task_id"] as? String, "task-finish")
        try expect(payload["input"] is [String: Any], "finish-task requires an empty input object")
    }

    runner.run("Audio 3 interim and final results map to subtitle events") {
        let draft = try Audio3ASRServerEventDecoder.decode(
            #"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"今日は","heartbeat":false,"sentence_end":false}}}}"#
        )
        let final = try Audio3ASRServerEventDecoder.decode(
            #"{"header":{"event":"result-generated"},"payload":{"output":{"sentence":{"text":"今日は晴れです。","heartbeat":false,"sentence_end":true}}}}"#
        )

        try expectEqual(
            draft.subtitleEvent(sourceLanguage: .japanese),
            .sourceDraft(text: "今日は", language: "ja")
        )
        try expectEqual(
            final.subtitleEvent(sourceLanguage: .japanese),
            .sourceFinal(text: "今日は晴れです。", language: "ja")
        )
    }

    runner.run("Audio 3 lifecycle and failures decode") {
        let started = try Audio3ASRServerEventDecoder.decode(
            #"{"header":{"event":"task-started"},"payload":{}}"#
        )
        let failed = try Audio3ASRServerEventDecoder.decode(
            #"{"header":{"event":"task-failed","error_code":"CLIENT_ERROR","error_message":"Bad request"},"payload":{}}"#
        )

        try expectEqual(started, .taskStarted)
        try expectEqual(failed, .taskFailed(code: "CLIENT_ERROR", message: "Bad request"))
    }
}

private func audio3JSONObject(_ data: Data) throws -> [String: Any] {
    guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw TestFailure(description: "expected a JSON object")
    }
    return object
}

private func audio3RequiredObject(_ value: Any?) throws -> [String: Any] {
    guard let object = value as? [String: Any] else {
        throw TestFailure(description: "expected a nested JSON object")
    }
    return object
}
