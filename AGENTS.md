# AGENTS.md

## Project

Mimi is a Tauri v2 desktop app (Rust backend + React/TypeScript frontend) that listens to system audio playing on macOS or Windows and shows live translated subtitles in a floating always-on-top overlay. It supports built-in Alibaba Cloud and OpenAI Realtime service profiles and never records audio.

Preserve these product constraints:

- Capture system audio only. Do not add microphone capture unless the task explicitly requires it.
- Do not persist audio or subtitle content.
- Store API credentials in the OS keychain only (macOS Keychain / Windows Credential Manager via `keyring`). Never add plaintext, source-controlled, or environment-variable credential fallbacks.
- Keep diagnostics content-free: timing, counts, language codes, status codes, and sanitized error labels are acceptable; recognized or translated text is not.

## Repository map

- `src-tauri/src/core/`: UI-independent models, configuration, wire protocols, subtitle assembly, text segmentation, and pipeline diagnostics. Pure Rust, fully unit-tested.
- `src-tauri/src/clients/`: tokio network clients (Alibaba live translate/Audio 3.0/Qwen-MT pipelines and OpenAI Realtime translation).
- `src-tauri/src/audio/`: system-audio capture (macOS ScreenCaptureKit via `screen-capture-kit`, Windows WASAPI loopback via `cpal` + `rubato`) and the bounded PCM send pipeline.
- `src-tauri/src/session_manager.rs`: session lifecycle — start/stop/pause/resume, language/mode switching, health checks, automatic reconnection, state events.
- `src-tauri/src/settings_store.rs`: preferences/profile JSON in the app config directory + provider/profile-scoped keychain credential storage.
- `src-tauri/src/{commands,windows,lib}.rs`: IPC commands, overlay/tray-panel window management, tray/shortcut wiring.
- `src/`: React frontend — `src/windows/{overlay,tray-panel,settings}/` contain the three product surfaces; `src/lib/{ipc,store,types,i18n}.ts` define the IPC contract.
- The product website lives in the separate `yuxino-labs/mimi-web`
  repository. Never add a website copy, subtree, or generated site assets to
  this application repository.
- `docs/plans/`: current accepted design records; completed checklists and
  superseded designs stay in Git history.
- `docs/development/common-regressions.md`: required macOS signing, permission,
  Keychain, overlay, and local-testing pitfalls. Read it before packaging or
  diagnosing a repeated system prompt.
- `scripts/check.sh`: canonical automated test and strict-build entry point.
- `scripts/package-app.sh`: release-shaped local build via `tauri build`, signed with the stable local identity; it is not an identity-compatible update for a GitHub Release build.
- `scripts/codesign-identity.sh`: honors an explicit `MIMI_CODESIGN_IDENTITY`; otherwise it selects the exact fingerprint of the unique `mimi Local Development` identity or reports unavailable. macOS packaging and development launch fail closed rather than use ad-hoc signing.
- `scripts/verify-macos-install-identity.sh`: compares the complete designated requirement before a formal app is replaced.
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
- On macOS use `/Applications/mimi-dev.app` for all pre-push testing. Never overwrite `/Applications/mimi.app` with a locally signed package when its designated requirement differs from the installed GitHub Release. An intentional certificate migration must be explicit and is expected to require one final Screen Recording and Keychain authorization.
- A normal credential snapshot may read each profile API key once, but must not touch migration-only Keychain items after a profile-scoped key exists. Keep non-secret migration bookkeeping off the steady-state authorization path.
- Never delete and recreate a Keychain credential to refresh its ACL, widen an item or keychain to allow-all, or fabricate a Team ID for a self-signed build. Preserve the same service/account and update its secret in place. Password-free Keychain continuity across rebuilt binaries requires an Apple-issued signing identity with a stable Team ID; the current self-signed identities guarantee a stable designated requirement for TCC, not that stronger Keychain property.

## Verification

Run the repository check from the repository root:

```bash
./scripts/check.sh
```

It runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and the frontend typecheck/lint/test/build pipeline, plus whitespace/error checks on the diff.

Additional checks by change type:

- UI changes: on macOS run `./scripts/dev-app.sh` and inspect the settings window, tray panel, and overlay in normal, empty, error, paused, collapsed, translating, and long-subtitle states. This launches a signed bundle from one canonical path so macOS does not treat every rebuild as a new app and repeat privacy prompts. Use `./scripts/dev-app.sh --ui-only` for credential-free UI smoke tests; UI-test mode must not access provider networks or start system-audio capture. On Windows, use `npm run tauri:dev`.
- Latency or streaming changes: measure against a real session for the affected provider (user-supplied OS-keychain credentials) and report timing diagnostics as well as correctness tests.
- Packaging or signing changes: read `docs/development/common-regressions.md`, run `./scripts/package-app.sh`, and verify the resulting app opens without replacing an installed app of a different designated requirement. Windows packaging is verified on a Windows machine (or CI). Never commit `dist/`, `src-tauri/target/`, or signing identities.

Before committing, inspect the diff for credentials, recordings, subtitle content, personal paths, and build artifacts.
