# AGENTS.md

## Project

Mimi is a Tauri v2 desktop app (Rust backend + React/TypeScript frontend) that listens to system audio playing on macOS or Windows and shows live translated subtitles in a floating always-on-top overlay. It streams speech recognition and translation from Alibaba Cloud Model Studio and never records audio.

Preserve these product constraints:

- Capture system audio only. Do not add microphone capture unless the task explicitly requires it.
- Do not persist audio or subtitle content.
- Store API credentials in the OS keychain only (macOS Keychain / Windows Credential Manager via `keyring`). Never add plaintext, source-controlled, or environment-variable credential fallbacks.
- Keep diagnostics content-free: timing, counts, language codes, status codes, and sanitized error labels are acceptable; recognized or translated text is not.

## Repository map

- `src-tauri/src/core/`: UI-independent models, configuration, wire protocols, subtitle assembly, text segmentation, and pipeline diagnostics. Pure Rust, fully unit-tested.
- `src-tauri/src/clients/`: tokio network clients (low-latency live translate WebSocket, Audio 3.0 ASR WebSocket, Qwen-MT HTTP/SSE, and the high-quality pipeline).
- `src-tauri/src/audio/`: system-audio capture (macOS ScreenCaptureKit via `screen-capture-kit`, Windows WASAPI loopback via `cpal` + `rubato`) and the bounded PCM send pipeline.
- `src-tauri/src/session_manager.rs`: session lifecycle — start/stop/pause/resume, language/mode switching, health checks, automatic reconnection, state events.
- `src-tauri/src/settings_store.rs`: preferences JSON in the app config directory + keychain credential storage.
- `src-tauri/src/{commands,windows,lib}.rs`: IPC commands, overlay/tray-panel window management, tray/shortcut wiring.
- `src/`: React frontend — `src/windows/{overlay,tray-panel,settings}/` replicate the original SwiftUI windows; `src/lib/{ipc,store,types,i18n}.ts` define the IPC contract.
- `mimi-web/`: the product website (marketing site, not the app).
- `docs/plans/`: accepted design notes and implementation plans.
- `scripts/check.sh`: canonical automated test and strict-build entry point.
- `scripts/package-app.sh`: release build via `tauri build`, re-signed with the stable local identity.
- `scripts/codesign-identity.sh`: picks the code-signing identity for local builds (`MIMI_CODESIGN_IDENTITY` → `mimi Local Development` → ad-hoc). Stable identity keeps Screen Recording / keychain grants valid across rebuilds.
- `.github/workflows/ci.yml`: CI (Rust fmt/clippy/test on macOS and Windows, frontend checks).

## Working agreements

- Read the relevant source and tests before changing behavior. For non-trivial behavior changes, add or update a design note in `docs/plans/`.
- Keep UI-independent logic in `src-tauri/src/core/`; keep Tauri, window, keyring, and OS-audio integration in the app-layer modules. Never import `tauri` types in `core/` or `clients/`.
- Preserve Rust concurrency safety. Isolate mutable network or lifecycle state behind `Arc<Mutex<…>>` or actors; never hold a `std::sync::MutexGuard` across an `.await`.
- Treat streaming drafts as replaceable previews and final events as durable subtitle history. Do not let preview work block, reorder, or overwrite final translations.
- Keep queues and on-screen draft growth bounded. Latency fixes must account for cancellation, reconnects, stale generations, empty results, and out-of-order completions.
- Add focused `#[cfg(test)]` coverage when changing `core/`. `cargo test` runs the repository's suite.
- The wire protocols (JSON shapes, model names, domain prompts, filler glossaries) mirror the upstream services exactly; do not reword the translation prompts.
- Do not introduce a dependency, external service, or credential requirement unless the task needs it and the trade-off is documented.

## Verification

Run the repository check from the repository root:

```bash
./scripts/check.sh
```

It runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and the frontend typecheck/lint/test/build pipeline, plus whitespace/error checks on the diff.

Additional checks by change type:

- UI changes: run `npm run tauri dev` and inspect the settings window, tray panel, and overlay in normal, empty, error, paused, collapsed, translating, and long-subtitle states. `MIMI_UI_TEST=1` seeds demo credentials; `MIMI_AUTO_START=1` additionally starts a session on launch so the failure path (fake credentials) can be exercised without interaction. For the icon-correct macOS dev run (a real .app bundle with the masked Dock icon, window titles/tooltip marked "(dev)"), use `./scripts/dev-app.sh` instead: it builds a release binary with `--features tauri/custom-protocol` (exactly what `tauri build` does — Tauri's `cfg(dev)` is `!custom-protocol`, so without that feature even a release build replaces the Dock icon at runtime with an unmasked square; see `docs/plans/2026-08-14-tauri-dev-dock-icon-design.md`), wraps it in `target/release/mimi-dev.app`, and launches it. The bare `npm run tauri dev` binary always triggers that runtime override, so its Dock icon cannot be masked.
- Latency or streaming changes: measure against a real Alibaba Cloud session (user-supplied credentials) and report timing diagnostics as well as correctness tests.
- Packaging or signing changes: run `./scripts/package-app.sh` and verify the resulting app opens. Windows packaging is verified on a Windows machine (or CI). Never commit `dist/`, `src-tauri/target/`, or signing identities.

Before committing, inspect the diff for credentials, recordings, subtitle content, personal paths, and build artifacts.
