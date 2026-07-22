import Foundation

var runner = TestRunner()
runSubtitleReducerTests(using: &runner)

print("\n\(runner.passed) passed, \(runner.failed) failed")
if runner.failed > 0 {
    exit(EXIT_FAILURE)
}
