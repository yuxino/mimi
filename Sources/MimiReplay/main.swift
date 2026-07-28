@preconcurrency import AVFoundation
import Foundation
import MimiCore

private struct Metrics: Encodable {
    let audioSeconds: Double
    let firstSourceDraft: Double?
    let firstTranslationDraft: Double?
    let firstTranslationFinal: Double?
    let sourceDrafts: Int
    let translationDrafts: Int
    let translationFinals: Int
    let longestEventGap: Double
    let sourceFinalTexts: [String]
    let translationFinalTexts: [String]
}

private actor Recorder {
    private let clock = ContinuousClock()
    private let start: ContinuousClock.Instant
    private var firstSource: Double?
    private var firstDraft: Double?
    private var firstFinal: Double?
    private var sourceCount = 0
    private var draftCount = 0
    private var finalCount = 0
    private var previous: Double?
    private var longestGap = 0.0
    private var sourceFinalTexts: [String] = []
    private var translationFinalTexts: [String] = []

    init() {
        start = clock.now
    }

    func record(_ event: LiveTranslateServerEvent) {
        let now = elapsed()
        if let previous {
            longestGap = max(longestGap, now - previous)
        }
        previous = now

        switch event {
        case .sourceDraft:
            sourceCount += 1
            firstSource = firstSource ?? now
        case let .sourceFinal(text, _):
            sourceFinalTexts.append(text)
        case let .translationDraft(text) where !text.isEmpty:
            draftCount += 1
            firstDraft = firstDraft ?? now
        case let .translationFinal(text):
            finalCount += 1
            firstFinal = firstFinal ?? now
            translationFinalTexts.append(text)
        default:
            break
        }
    }

    func metrics(audioSeconds: Double) -> Metrics {
        Metrics(
            audioSeconds: audioSeconds,
            firstSourceDraft: firstSource,
            firstTranslationDraft: firstDraft,
            firstTranslationFinal: firstFinal,
            sourceDrafts: sourceCount,
            translationDrafts: draftCount,
            translationFinals: finalCount,
            longestEventGap: longestGap,
            sourceFinalTexts: sourceFinalTexts,
            translationFinalTexts: translationFinalTexts
        )
    }

    private func elapsed() -> Double {
        let duration = start.duration(to: clock.now)
        return Double(duration.components.seconds)
            + Double(duration.components.attoseconds) / 1_000_000_000_000_000_000
    }
}

@main
private struct MimiReplay {
    static func main() async {
        do {
            let environment = ProcessInfo.processInfo.environment
            guard
                CommandLine.arguments.count == 2,
                let workspaceID = environment["MIMI_WORKSPACE_ID"],
                let apiKey = environment["MIMI_API_KEY"]
            else {
                throw ReplayError.missingConfiguration
            }

            let source = SourceLanguage(
                rawValue: environment["MIMI_SOURCE_LANGUAGE"] ?? "auto"
            ) ?? .automatic
            let target = TargetLanguage(
                rawValue: environment["MIMI_TARGET_LANGUAGE"] ?? "zh"
            ) ?? .simplifiedChinese
            let configuration = try LiveTranslationConfiguration(
                workspaceID: workspaceID,
                apiKey: apiKey,
                sourceLanguage: source,
                targetLanguage: target,
                translationMode: .lowLatency
            ).validated()
            let pcm = try loadPCM16Mono16k(
                from: URL(fileURLWithPath: CommandLine.arguments[1])
            )
            let bytesPerSecond = 32_000
            let recorder = Recorder()
            let client = try TranslationClient(configuration: configuration)

            try await client.connect { event in
                await recorder.record(event)
            }

            let chunkSize = bytesPerSecond / 10
            var offset = 0
            while offset < pcm.count {
                let end = min(offset + chunkSize, pcm.count)
                try await client.sendAudio(pcm.subdata(in: offset..<end))
                offset = end
                try await Task.sleep(for: .milliseconds(100))
            }

            try await Task.sleep(for: .seconds(3))
            await client.finish()

            let result = await recorder.metrics(
                audioSeconds: Double(pcm.count) / Double(bytesPerSecond)
            )
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            print(String(decoding: try encoder.encode(result), as: UTF8.self))
        } catch {
            print("REPLAY_ERROR: \(error.localizedDescription)")
            Foundation.exit(1)
        }
    }

    private static func loadPCM16Mono16k(from url: URL) throws -> Data {
        let file = try AVAudioFile(forReading: url)
        guard
            let format = AVAudioFormat(standardFormatWithSampleRate: 16_000, channels: 1),
            let converter = AVAudioConverter(from: file.processingFormat, to: format),
            let input = AVAudioPCMBuffer(
                pcmFormat: file.processingFormat,
                frameCapacity: AVAudioFrameCount(file.length)
            )
        else {
            throw ReplayError.audioConversionFailed
        }
        try file.read(into: input)

        let ratio = format.sampleRate / file.processingFormat.sampleRate
        let capacity = AVAudioFrameCount(ceil(Double(input.frameLength) * ratio)) + 1
        guard let output = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: capacity) else {
            throw ReplayError.audioConversionFailed
        }

        let inputProvider = ReplayAudioInputProvider(buffer: input)
        var conversionError: NSError?
        _ = converter.convert(to: output, error: &conversionError) { _, status in
            guard let buffer = inputProvider.take() else {
                status.pointee = .endOfStream
                return nil
            }
            status.pointee = .haveData
            return buffer
        }
        if let conversionError {
            throw conversionError
        }
        guard let channel = output.floatChannelData?[0] else {
            throw ReplayError.audioConversionFailed
        }
        return PCM16Encoder.encode(
            channels: [Array(UnsafeBufferPointer(start: channel, count: Int(output.frameLength)))]
        )
    }
}

private final class ReplayAudioInputProvider: @unchecked Sendable {
    private let lock = NSLock()
    private var buffer: AVAudioPCMBuffer?

    init(buffer: AVAudioPCMBuffer) {
        self.buffer = buffer
    }

    func take() -> AVAudioPCMBuffer? {
        lock.lock()
        defer { lock.unlock() }
        defer { buffer = nil }
        return buffer
    }
}

private enum ReplayError: Error, LocalizedError {
    case missingConfiguration
    case audioConversionFailed

    var errorDescription: String? {
        switch self {
        case .missingConfiguration:
            "Provide the audio path and Mimi configuration environment variables."
        case .audioConversionFailed:
            "The replay audio could not be converted to 16 kHz mono PCM."
        }
    }
}
