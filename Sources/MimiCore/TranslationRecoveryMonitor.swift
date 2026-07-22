import Foundation

public struct TranslationRecoveryMonitor: Sendable {
    public let stallTimeout: TimeInterval
    public let recentAudioWindow: TimeInterval
    public let responseGrace: TimeInterval
    public let recoveryCooldown: TimeInterval

    private var lastServerActivity: TimeInterval?
    private var lastActiveAudio: TimeInterval?
    private var probeStartedAt: TimeInterval?
    private var lastRecovery: TimeInterval?

    public init(
        stallTimeout: TimeInterval = 8,
        recentAudioWindow: TimeInterval = 2,
        responseGrace: TimeInterval = 3,
        recoveryCooldown: TimeInterval = 5
    ) {
        self.stallTimeout = stallTimeout
        self.recentAudioWindow = recentAudioWindow
        self.responseGrace = responseGrace
        self.recoveryCooldown = recoveryCooldown
    }

    public mutating func reset(at timestamp: TimeInterval) {
        lastServerActivity = timestamp
        lastActiveAudio = nil
        probeStartedAt = nil
    }

    public mutating func noteServerActivity(at timestamp: TimeInterval) {
        lastServerActivity = timestamp
        probeStartedAt = nil
    }

    public mutating func noteActiveAudio(at timestamp: TimeInterval) {
        if let lastActiveAudio,
           timestamp < lastActiveAudio || timestamp - lastActiveAudio > recentAudioWindow {
            probeStartedAt = nil
        }
        lastActiveAudio = timestamp

        if probeStartedAt == nil,
           let lastServerActivity,
           timestamp >= lastServerActivity,
           timestamp - lastServerActivity >= stallTimeout {
            probeStartedAt = timestamp
        }
    }

    public func shouldRecover(at timestamp: TimeInterval) -> Bool {
        guard
            let lastServerActivity,
            let lastActiveAudio,
            let probeStartedAt,
            timestamp >= lastServerActivity,
            timestamp >= lastActiveAudio,
            timestamp >= probeStartedAt,
            timestamp - lastServerActivity >= stallTimeout,
            timestamp - lastActiveAudio <= recentAudioWindow,
            timestamp - probeStartedAt >= responseGrace
        else {
            return false
        }

        if let lastRecovery {
            return timestamp >= lastRecovery
                && timestamp - lastRecovery >= recoveryCooldown
        }
        return true
    }

    public mutating func markRecovery(at timestamp: TimeInterval) {
        lastRecovery = timestamp
        lastServerActivity = timestamp
        lastActiveAudio = nil
        probeStartedAt = nil
    }
}
