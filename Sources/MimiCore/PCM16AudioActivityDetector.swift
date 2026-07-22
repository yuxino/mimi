import Foundation

public enum PCM16AudioActivityDetector {
    public static func isActive(
        _ data: Data,
        rmsThreshold: Double = 0.02
    ) -> Bool {
        let sampleCount = data.count / MemoryLayout<Int16>.size
        guard sampleCount > 0 else { return false }

        var sumOfSquares = 0.0
        for offset in stride(from: 0, to: sampleCount * 2, by: 2) {
            let low = UInt16(data[offset])
            let high = UInt16(data[offset + 1]) << 8
            let sample = Int16(bitPattern: low | high)
            let normalized = Double(sample) / 32_768.0
            sumOfSquares += normalized * normalized
        }

        let rms = (sumOfSquares / Double(sampleCount)).squareRoot()
        return rms >= rmsThreshold
    }
}
