import Foundation
import MimiCore

func runRecoveryMonitorTests(using runner: inout TestRunner) {
    runner.run("audio activity ignores empty and silent PCM") {
        try expect(!PCM16AudioActivityDetector.isActive(Data()), "empty PCM should be inactive")
        try expect(
            !PCM16AudioActivityDetector.isActive(pcm16Data([0, 0, 0, 0])),
            "silent PCM should be inactive"
        )
    }

    runner.run("audio activity ignores low-level noise") {
        try expect(
            !PCM16AudioActivityDetector.isActive(pcm16Data([100, -100, 80, -80])),
            "low-amplitude PCM should be inactive"
        )
    }

    runner.run("audio activity detects speech-like signal") {
        try expect(
            PCM16AudioActivityDetector.isActive(pcm16Data([8_000, -9_000, 10_000, -8_500])),
            "speech-like PCM should be active"
        )
    }

    runner.run("recovery monitor does not restart during silence") {
        var monitor = TranslationRecoveryMonitor()
        monitor.reset(at: 0)

        try expect(!monitor.shouldRecover(at: 60), "silence alone should not trigger recovery")
    }

    runner.run("recovery monitor respects recent server activity") {
        var monitor = TranslationRecoveryMonitor()
        monitor.reset(at: 0)
        monitor.noteServerActivity(at: 9)
        monitor.noteActiveAudio(at: 10)

        try expect(!monitor.shouldRecover(at: 10), "recent events mean the session is healthy")
    }

    runner.run("recovery monitor restarts stale session when audio resumes") {
        var monitor = TranslationRecoveryMonitor(responseGrace: 3)
        monitor.reset(at: 0)
        monitor.noteServerActivity(at: 1)

        try expect(!monitor.shouldRecover(at: 20), "long silence should remain idle")

        monitor.noteActiveAudio(at: 20)
        try expect(!monitor.shouldRecover(at: 20), "new audio should get a response grace period")

        monitor.noteActiveAudio(at: 21)
        monitor.noteActiveAudio(at: 22)
        monitor.noteActiveAudio(at: 23)
        try expect(monitor.shouldRecover(at: 23), "continued audio without events should recover")
    }

    runner.run("recovery monitor cancels recovery when server responds") {
        var monitor = TranslationRecoveryMonitor(responseGrace: 3)
        monitor.reset(at: 0)
        monitor.noteActiveAudio(at: 10)
        monitor.noteActiveAudio(at: 11)
        monitor.noteServerActivity(at: 12)
        monitor.noteActiveAudio(at: 13)

        try expect(!monitor.shouldRecover(at: 14), "a server response should clear the pending probe")
    }

    runner.run("recovery monitor suppresses recovery during cooldown") {
        var monitor = TranslationRecoveryMonitor(responseGrace: 0, recoveryCooldown: 5)
        monitor.reset(at: 0)
        monitor.noteActiveAudio(at: 10)
        try expect(monitor.shouldRecover(at: 10), "the initial stall should recover")

        monitor.markRecovery(at: 10)
        monitor.noteActiveAudio(at: 12)
        try expect(!monitor.shouldRecover(at: 12), "cooldown should suppress a reconnect loop")
    }
}

private func pcm16Data(_ samples: [Int16]) -> Data {
    var data = Data(capacity: samples.count * 2)
    for sample in samples {
        let bits = UInt16(bitPattern: sample)
        data.append(UInt8(truncatingIfNeeded: bits))
        data.append(UInt8(truncatingIfNeeded: bits >> 8))
    }
    return data
}
