import Foundation
import MimiCore

func runLiveTranslateProtocolTests(using runner: inout TestRunner) {
    runner.run("endpoint builds the Beijing realtime URL") {
        let endpoint = try LiveTranslateEndpoint(workspaceID: "ws-abc123")
        try expectEqual(
            endpoint.url.absoluteString,
            "wss://ws-abc123.cn-beijing.maas.aliyuncs.com/api-ws/v1/realtime?model=qwen3.5-livetranslate-flash-realtime"
        )
    }

    runner.run("endpoint rejects host injection") {
        do {
            _ = try LiveTranslateEndpoint(workspaceID: "bad.example.com")
            throw TestFailure(description: "expected invalid workspace ID")
        } catch LiveTranslateProtocolError.invalidWorkspaceID {
            // Expected.
        }
    }

    runner.run("session update requests text-only Chinese translation and source transcript") {
        let data = try LiveTranslateRequestEncoder.sessionUpdate(
            sourceLanguage: .japanese,
            eventID: "event-session"
        )
        let json = try jsonObject(data)
        let session = try requiredObject(json["session"])
        let transcription = try requiredObject(session["input_audio_transcription"])
        let translation = try requiredObject(session["translation"])

        try expectEqual(json["type"] as? String, "session.update")
        try expectEqual(session["modalities"] as? [String], ["text"])
        try expectEqual(session["sample_rate"] as? Int, 16_000)
        try expectEqual(session["input_audio_format"] as? String, "pcm")
        try expectEqual(transcription["model"] as? String, "qwen3-asr-flash-realtime")
        try expectEqual(transcription["language"] as? String, "ja")
        try expectEqual(translation["language"] as? String, "zh")
    }

    runner.run("audio append Base64 encodes PCM bytes") {
        let data = try LiveTranslateRequestEncoder.audioAppend(
            Data([0x00, 0x7F, 0xFF]),
            eventID: "event-audio"
        )
        let json = try jsonObject(data)

        try expectEqual(json["type"] as? String, "input_audio_buffer.append")
        try expectEqual(json["audio"] as? String, "AH//")
    }

    runner.run("session update selects an explicit target language") {
        let data = try LiveTranslateRequestEncoder.sessionUpdate(
            sourceLanguage: .english,
            targetLanguage: .japanese,
            eventID: "event-target"
        )
        let json = try jsonObject(data)
        let session = try requiredObject(json["session"])
        let translation = try requiredObject(session["translation"])

        try expectEqual(translation["language"] as? String, "ja")
    }

    runner.run("finish event uses the documented type") {
        let json = try jsonObject(try LiveTranslateRequestEncoder.finish(eventID: "event-finish"))
        try expectEqual(json["type"] as? String, "session.finish")
    }

    runner.run("source preview combines confirmed and tentative text") {
        let event = try LiveTranslateServerEvent.decode(
            """
            {"type":"conversation.item.input_audio_transcription.text","text":"Hello","stash":" world","language":"en"}
            """
        )
        try expectEqual(event, .sourceDraft(text: "Hello world", language: "en"))
    }

    runner.run("source completion decodes the final transcript") {
        let event = try LiveTranslateServerEvent.decode(
            """
            {"type":"conversation.item.input_audio_transcription.completed","transcript":"Hello world.","language":"en"}
            """
        )
        try expectEqual(event, .sourceFinal(text: "Hello world.", language: "en"))
    }

    runner.run("translation preview and completion decode") {
        let preview = try LiveTranslateServerEvent.decode(
            #"{"type":"response.text.text","text":"你好","stash":"，世界"}"#
        )
        let final = try LiveTranslateServerEvent.decode(
            #"{"type":"response.text.done","text":"你好，世界。"}"#
        )

        try expectEqual(preview, .translationDraft("你好，世界"))
        try expectEqual(final, .translationFinal("你好，世界。"))
    }

    runner.run("session and error events decode") {
        let updated = try LiveTranslateServerEvent.decode(#"{"type":"session.updated"}"#)
        let finished = try LiveTranslateServerEvent.decode(#"{"type":"session.finished"}"#)
        let failure = try LiveTranslateServerEvent.decode(
            #"{"type":"error","error":{"code":"invalid_value","message":"Bad language"}}"#
        )

        try expectEqual(updated, .sessionUpdated)
        try expectEqual(finished, .sessionFinished)
        try expectEqual(failure, .error(code: "invalid_value", message: "Bad language"))
    }

    runner.run("unknown events are ignored without failing the receive loop") {
        let event = try LiveTranslateServerEvent.decode(#"{"type":"response.created"}"#)
        try expectEqual(event, .ignored(type: "response.created"))
    }
}

private func jsonObject(_ data: Data) throws -> [String: Any] {
    guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        throw TestFailure(description: "expected a JSON object")
    }
    return object
}

private func requiredObject(_ value: Any?) throws -> [String: Any] {
    guard let object = value as? [String: Any] else {
        throw TestFailure(description: "expected a nested JSON object")
    }
    return object
}
