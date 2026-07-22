import Foundation

struct TestFailure: Error, CustomStringConvertible {
    let description: String
}

struct TestRunner {
    private(set) var passed = 0
    private(set) var failed = 0

    mutating func run(_ name: String, _ body: () throws -> Void) {
        do {
            try body()
            passed += 1
            print("✓ \(name)")
        } catch {
            failed += 1
            print("✗ \(name): \(error)")
        }
    }

}

func expectEqual<T: Equatable>(_ actual: T, _ expected: T, _ message: String = "") throws {
    guard actual == expected else {
        let suffix = message.isEmpty ? "" : " (\(message))"
        throw TestFailure(description: "expected \(expected), got \(actual)\(suffix)")
    }
}

func expect(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    guard condition() else {
        throw TestFailure(description: message)
    }
}
