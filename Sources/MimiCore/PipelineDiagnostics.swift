import Foundation

public enum PipelineDiagnostics {
    public static let isEnabled =
        ProcessInfo.processInfo.environment["MIMI_PIPELINE_DIAGNOSTICS"] == "1"

    public static func log(_ message: @autoclosure () -> String) {
        guard isEnabled else { return }
        NSLog("mimi-pipeline %@", message())
    }

    public static func milliseconds(_ duration: Duration) -> Int {
        let components = duration.components
        let seconds = components.seconds * 1_000
        let milliseconds = components.attoseconds / 1_000_000_000_000_000
        return Int(seconds + milliseconds)
    }
}
