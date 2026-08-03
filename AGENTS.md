# AGENTS.md

## Project

Mimi is a native macOS 14+ live-subtitle app written in Swift 6.1. It captures system audio, streams speech recognition, translates recognized text, and renders a floating subtitle overlay.

Preserve these product constraints:

- Capture system audio only. Do not add microphone capture unless the task explicitly requires it.
- Do not persist audio or subtitle content.
- Store API credentials in macOS Keychain only. Never add plaintext, source-controlled, or environment-variable credential fallbacks.
- Keep diagnostics content-free: timing, counts, language codes, status codes, and sanitized error labels are acceptable; recognized or translated text is not.

## Repository map

- `Sources/MimiCore/`: UI-independent models, protocols, streaming clients, reducers, text segmentation, and pipeline diagnostics.
- `Sources/MimiApp/`: SwiftUI/AppKit lifecycle, settings, Keychain access, system-audio capture, and overlay UI.
- `Sources/MimiReplay/`: command-line replay tool for latency-sensitive pipeline investigation.
- `Tests/MimiCoreTests/`: custom executable test suite for `MimiCore`.
- `Resources/`: app metadata and icons.
- `scripts/check.sh`: canonical automated test and strict-build entry point.
- `scripts/package-app.sh`: release build, app-bundle assembly, and local signing.
- `docs/plans/`: accepted design notes and implementation plans.

## Working agreements

- Read the relevant source and tests before changing behavior. For non-trivial behavior changes, add or update a design note in `docs/plans/`.
- Keep reusable logic in `MimiCore`; keep AppKit, SwiftUI, ScreenCaptureKit, UserDefaults, and Keychain integration in `MimiApp`.
- Preserve Swift 6 concurrency safety. Prefer value types and pure transformations; isolate mutable network or lifecycle state in actors or `@MainActor` types.
- Treat streaming drafts as replaceable previews and final events as durable subtitle history. Do not let preview work block, reorder, or overwrite final translations.
- Keep queues and on-screen draft growth bounded. Latency fixes must account for cancellation, reconnects, stale generations, empty results, and out-of-order completions.
- Add focused coverage to the custom test runner when changing `MimiCore`. Do not assume `swift test` runs this repository's suite.
- Do not introduce a dependency, external service, or credential requirement unless the task needs it and the trade-off is documented.

## Verification

Run the repository check from the repository root:

```bash
./scripts/check.sh
```

It runs the complete core suite, a release build with warnings treated as errors, and whitespace/error checks on the diff.

Additional checks by change type:

- UI changes: run `./scripts/package-app.sh`, launch `dist/mimi.app`, and inspect normal, empty, error, paused, collapsed, translating, and long-subtitle states as applicable.
- Latency or streaming changes: use `mimi-replay` with non-sensitive fixtures and report measurements as well as correctness tests.
- Packaging or signing changes: run `./scripts/package-app.sh` and verify the resulting app opens. Never commit `dist/` or signing identities.

Before committing, inspect the diff for credentials, recordings, subtitle content, personal paths, and build artifacts.
