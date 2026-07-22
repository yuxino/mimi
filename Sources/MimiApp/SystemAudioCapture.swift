import AVFoundation
import CoreMedia
import Foundation
import MimiCore
import ScreenCaptureKit

enum SystemAudioCaptureError: LocalizedError {
    case alreadyRunning
    case noDisplay
    case unsupportedAudioFormat

    var errorDescription: String? {
        switch self {
        case .alreadyRunning:
            "System audio capture is already running."
        case .noDisplay:
            "mimi could not find a display to use for system audio capture."
        case .unsupportedAudioFormat:
            "macOS returned an unsupported system audio format."
        }
    }
}

final class SystemAudioCapture: NSObject, @unchecked Sendable {
    typealias AudioHandler = @Sendable (Data) -> Void
    typealias ErrorHandler = @Sendable (Error) -> Void

    private let audioQueue = DispatchQueue(label: "app.yuxino.mimi.system-audio")
    private let lock = NSLock()

    private var stream: SCStream?
    private var audioHandler: AudioHandler?
    private var errorHandler: ErrorHandler?

    func start(
        onAudio: @escaping AudioHandler,
        onError: @escaping ErrorHandler
    ) async throws {
        guard stream == nil else {
            throw SystemAudioCaptureError.alreadyRunning
        }

        let content = try await SCShareableContent.excludingDesktopWindows(
            false,
            onScreenWindowsOnly: false
        )
        guard let display = content.displays.first(where: { $0.displayID == CGMainDisplayID() })
            ?? content.displays.first
        else {
            throw SystemAudioCaptureError.noDisplay
        }

        let ownBundleID = Bundle.main.bundleIdentifier
        let excludedApplications = content.applications.filter {
            $0.bundleIdentifier == ownBundleID
        }
        let filter = SCContentFilter(
            display: display,
            excludingApplications: excludedApplications,
            exceptingWindows: []
        )

        let configuration = SCStreamConfiguration()
        configuration.capturesAudio = true
        configuration.excludesCurrentProcessAudio = true
        configuration.sampleRate = 16_000
        configuration.channelCount = 1
        configuration.width = 2
        configuration.height = 2
        configuration.minimumFrameInterval = CMTime(value: 1, timescale: 1)
        configuration.queueDepth = 3
        configuration.showsCursor = false

        let newStream = SCStream(filter: filter, configuration: configuration, delegate: self)
        try newStream.addStreamOutput(self, type: .audio, sampleHandlerQueue: audioQueue)

        lock.withLock {
            audioHandler = onAudio
            errorHandler = onError
            stream = newStream
        }

        do {
            try await newStream.startCapture()
        } catch {
            clearState()
            throw error
        }
    }

    func stop() async {
        let activeStream = lock.withLock {
            let activeStream = stream
            stream = nil
            audioHandler = nil
            errorHandler = nil
            return activeStream
        }

        guard let activeStream else { return }
        try? await activeStream.stopCapture()
        try? activeStream.removeStreamOutput(self, type: .audio)
    }

    private func clearState() {
        lock.withLock {
            stream = nil
            audioHandler = nil
            errorHandler = nil
        }
    }

    private func currentAudioHandler() -> AudioHandler? {
        lock.withLock { audioHandler }
    }

    private func currentErrorHandler() -> ErrorHandler? {
        lock.withLock { errorHandler }
    }
}

extension SystemAudioCapture: SCStreamOutput {
    func stream(
        _ stream: SCStream,
        didOutputSampleBuffer sampleBuffer: CMSampleBuffer,
        of outputType: SCStreamOutputType
    ) {
        guard outputType == .audio, sampleBuffer.isValid else { return }

        do {
            if let data = try Self.makePCM16Data(from: sampleBuffer), !data.isEmpty {
                currentAudioHandler()?(data)
            }
        } catch {
            currentErrorHandler()?(error)
        }
    }

    private static func makePCM16Data(from sampleBuffer: CMSampleBuffer) throws -> Data? {
        var result: Data?

        try sampleBuffer.withAudioBufferList { audioBufferList, _ in
            guard
                let description = sampleBuffer.formatDescription?.audioStreamBasicDescription,
                let format = AVAudioFormat(
                    standardFormatWithSampleRate: description.mSampleRate,
                    channels: description.mChannelsPerFrame
                ),
                let samples = AVAudioPCMBuffer(
                    pcmFormat: format,
                    bufferListNoCopy: audioBufferList.unsafePointer
                ),
                let floatChannels = samples.floatChannelData
            else {
                throw SystemAudioCaptureError.unsupportedAudioFormat
            }

            let frameCount = Int(samples.frameLength)
            let channels = (0..<Int(format.channelCount)).map { channelIndex in
                Array(
                    UnsafeBufferPointer(
                        start: floatChannels[channelIndex],
                        count: frameCount
                    )
                )
            }
            result = PCM16Encoder.encode(channels: channels)
        }

        return result
    }
}

extension SystemAudioCapture: SCStreamDelegate {
    func stream(_ stream: SCStream, didStopWithError error: any Error) {
        currentErrorHandler()?(error)
        clearState()
    }
}
