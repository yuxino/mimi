import Foundation
import MimiCore

func runRealtimeASRProtocolTests(using runner: inout TestRunner) {
    runner.run("ASR endpoint builds the dedicated realtime URL") {
        let endpoint = try RealtimeASREndpoint(workspaceID: "ws-abc123")
        try expectEqual(
            endpoint.url.absoluteString,
            "wss://ws-abc123.cn-beijing.maas.aliyuncs.com/api-ws/v1/realtime?model=qwen3-asr-flash-realtime"
        )
    }

    runner.run("ASR session uses a balanced noise threshold with low-latency silence") {
        let data = try RealtimeASRRequestEncoder.sessionUpdate(
            sourceLanguage: .japanese,
            eventID: "event-session"
        )
        let json = try asrJSONObject(data)
        let session = try asrRequiredObject(json["session"])
        let transcription = try asrRequiredObject(session["input_audio_transcription"])
        let turnDetection = try asrRequiredObject(session["turn_detection"])

        try expectEqual(json["type"] as? String, "session.update")
        try expectEqual(session["sample_rate"] as? Int, 16_000)
        try expectEqual(session["input_audio_format"] as? String, "pcm")
        try expectEqual(transcription["language"] as? String, "ja")
        try expectEqual(turnDetection["type"] as? String, "server_vad")
        try expectEqual(turnDetection["threshold"] as? Double, 0.2)
        try expectEqual(turnDetection["silence_duration_ms"] as? Int, 400)
    }

    runner.run("automatic ASR omits the language hint") {
        let data = try RealtimeASRRequestEncoder.sessionUpdate(
            sourceLanguage: .automatic,
            eventID: "event-auto"
        )
        let json = try asrJSONObject(data)
        let session = try asrRequiredObject(json["session"])
        let transcription = try asrRequiredObject(session["input_audio_transcription"])

        try expect(transcription["language"] == nil, "automatic ASR must not lock a language")
    }

    runner.run("ASR audio and finish requests use realtime event types") {
        let audio = try asrJSONObject(
            try RealtimeASRRequestEncoder.audioAppend(Data([0x00, 0x7F, 0xFF]), eventID: "event-audio")
        )
        let finish = try asrJSONObject(
            try RealtimeASRRequestEncoder.finish(eventID: "event-finish")
        )

        try expectEqual(audio["type"] as? String, "input_audio_buffer.append")
        try expectEqual(audio["audio"] as? String, "AH//")
        try expectEqual(finish["type"] as? String, "session.finish")
    }

    runner.run("ASR preview combines confirmed text and stash") {
        let event = try RealtimeASRServerEventDecoder.decode(
            #"{"type":"conversation.item.input_audio_transcription.text","text":"今日は","stash":"晴れです","language":"ja"}"#
        )

        try expectEqual(event, .sourceDraft(text: "今日は晴れです", language: "ja"))
    }

    runner.run("ASR completion maps to a final source event") {
        let event = try RealtimeASRServerEventDecoder.decode(
            #"{"type":"conversation.item.input_audio_transcription.completed","transcript":"今日は晴れです。","language":"ja"}"#
        )

        try expectEqual(event, .sourceFinal(text: "今日は晴れです。", language: "ja"))
    }
}

private func asrJSONObject(_ data: Data) throws -> [String: Any] {
    guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw TestFailure(description: "expected a JSON object")
    }
    return object
}

private func asrRequiredObject(_ value: Any?) throws -> [String: Any] {
    guard let object = value as? [String: Any] else {
        throw TestFailure(description: "expected a nested JSON object")
    }
    return object
}
