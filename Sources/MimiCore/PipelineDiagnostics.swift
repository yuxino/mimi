import Foundation
import OSLog

public enum PipelineDiagnostics {
    private static let logger = Logger(
        subsystem: "app.yuxino.mimi",
        category: "pipeline"
    )

    // Diagnostics contain timing, counts, language codes, and sanitized error
    // labels only. Keep them on by default so intermittent pipeline failures
    // can be diagnosed without recording subtitle or translation content.
    public static let isEnabled =
        ProcessInfo.processInfo.environment["MIMI_PIPELINE_DIAGNOSTICS"] != "0"

    public static func log(_ message: @autoclosure () -> String) {
        guard isEnabled else { return }
        let value = message()
        logger.notice("\(value, privacy: .public)")
    }

    public static func errorLabel(_ error: Error) -> String {
        if let error = error as? QwenMTClientError {
            return switch error {
            case .missingAPIKey:
                "QwenMTClientError.missingAPIKey"
            case .invalidHTTPResponse:
                "QwenMTClientError.invalidHTTPResponse"
            case .requestTimedOut:
                "QwenMTClientError.requestTimedOut"
            case let .requestFailed(statusCode, _):
                "QwenMTClientError.requestFailed(status=\(statusCode))"
            }
        }
        if let error = error as? QwenMTProtocolError {
            return "QwenMTProtocolError.\(String(describing: error))"
        }
        let nsError = error as NSError
        return "\(String(describing: type(of: error)))[\(nsError.domain):\(nsError.code)]"
    }

    public static func milliseconds(_ duration: Duration) -> Int {
        let components = duration.components
        let seconds = components.seconds * 1_000
        let milliseconds = components.attoseconds / 1_000_000_000_000_000
        return Int(seconds + milliseconds)
    }
}
