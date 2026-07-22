import Foundation
import MimiCore

func runPCMConversionTests(using runner: inout TestRunner) {
    runner.run("PCM conversion clamps and scales float samples") {
        let data = PCM16Encoder.encode(
            channels: [[-1.5, -1.0, -0.5, 0, 0.5, 1.0, 1.5]]
        )

        try expectEqual(
            decodeInt16LittleEndian(data),
            [-32_768, -32_768, -16_384, 0, 16_384, 32_767, 32_767]
        )
    }

    runner.run("PCM conversion mixes channels to mono") {
        let data = PCM16Encoder.encode(
            channels: [
                [1.0, -1.0, 0.5],
                [-1.0, 1.0, 0.5]
            ]
        )

        try expectEqual(decodeInt16LittleEndian(data), [0, 0, 16_384])
    }

    runner.run("PCM conversion uses the shortest channel safely") {
        let data = PCM16Encoder.encode(
            channels: [
                [0.25, 0.5],
                [0.25]
            ]
        )

        try expectEqual(decodeInt16LittleEndian(data), [8_192])
    }

    runner.run("PCM conversion handles empty input") {
        try expectEqual(PCM16Encoder.encode(channels: []), Data())
        try expectEqual(PCM16Encoder.encode(channels: [[]]), Data())
    }
}

private func decodeInt16LittleEndian(_ data: Data) -> [Int16] {
    stride(from: 0, to: data.count - (data.count % 2), by: 2).map { offset in
        let low = UInt16(data[offset])
        let high = UInt16(data[offset + 1]) << 8
        return Int16(bitPattern: low | high)
    }
}
