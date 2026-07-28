import Foundation

public enum PCM16Encoder {
    public static func encode(channels: [[Float]]) -> Data {
        guard
            !channels.isEmpty,
            let frameCount = channels.map(\.count).min(),
            frameCount > 0
        else {
            return Data()
        }

        var data = Data(capacity: frameCount * MemoryLayout<Int16>.size)
        let channelCount = Float(channels.count)

        for frame in 0..<frameCount {
            let mixed = channels.reduce(Float.zero) { partial, channel in
                partial + channel[frame]
            } / channelCount
            var sample = quantize(mixed).littleEndian
            withUnsafeBytes(of: &sample) { bytes in
                data.append(contentsOf: bytes)
            }
        }

        return data
    }

    private static func quantize(_ sample: Float) -> Int16 {
        let clamped = min(max(sample, -1), 1)
        if clamped >= 0 {
            return Int16((clamped * Float(Int16.max)).rounded())
        }
        return Int16((clamped * 32_768).rounded())
    }
}
