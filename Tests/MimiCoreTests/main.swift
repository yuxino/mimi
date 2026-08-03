import Foundation

var runner = TestRunner()
runSubtitleReducerTests(using: &runner)
runSubtitleTextSegmenterTests(using: &runner)
runLiveTranslateProtocolTests(using: &runner)
runRealtimeASRProtocolTests(using: &runner)
runAudio3ASRProtocolTests(using: &runner)
runQwenMTProtocolTests(using: &runner)
runSessionControllerTests(using: &runner)
runPCMConversionTests(using: &runner)
runConnectionHealthTests(using: &runner)
runConfigurationTests(using: &runner)
runOverlayResizeInteractionTests(using: &runner)

print("\n\(runner.passed) passed, \(runner.failed) failed")
if runner.failed > 0 {
    exit(EXIT_FAILURE)
}
