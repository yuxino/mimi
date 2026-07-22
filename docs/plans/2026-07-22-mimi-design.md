# mimi MVP Design

## Product goal

`mimi` is a native macOS menu-bar app that listens to audio already playing on the Mac and displays live bilingual subtitles in a floating, always-on-top panel. The first release targets people watching foreign-language videos in browsers or desktop players. It prioritizes readable Chinese and stable subtitles over the lowest possible latency.

Success for the MVP means a user can add Alibaba Cloud Model Studio credentials, grant macOS capture permission, choose a source language, start listening, and see the original sentence plus a Simplified Chinese translation without installing a browser extension.

## MVP scope

- Native macOS app built with SwiftUI and AppKit.
- Menu-bar controls for start, stop, source language, settings, and quit.
- System-audio capture through ScreenCaptureKit; microphone capture is excluded.
- English, Japanese, and Korean source-language choices; English is the default.
- Simplified Chinese text-only output from Alibaba Cloud Model Studio.
- Original and translated subtitles in a movable, always-on-top, click-through-capable panel.
- Draft text is visually muted; confirmed text is prominent.
- API key stored in macOS Keychain; Workspace ID and preferences stored in UserDefaults.
- Clear permission, credential, network, and service error states.

The MVP does not include Windows support, browser extensions, translated speech, video-frame input, subtitle export, accounts, analytics, or cloud storage.

## Architecture

The repository uses Swift Package Manager so it can compile and test with the installed Apple command-line toolchain. `MimiCore` contains provider-independent state, configuration, protocol messages, and subtitle assembly. `MimiApp` contains macOS-specific capture, Keychain storage, window management, and SwiftUI views. Keeping protocol and state code in a library makes it deterministic to test without screen-capture permission or Alibaba credentials.

At runtime, `SystemAudioCapture` configures an `SCStream` for 16 kHz mono audio and excludes mimi's own process. Audio sample buffers are converted to signed 16-bit little-endian PCM and delivered to `LiveTranslateClient`. The client connects to the China (Beijing) Model Studio realtime WebSocket endpoint using the user's Workspace ID and an `Authorization: Bearer` header. It sends a text-only `session.update`, streams Base64 audio via `input_audio_buffer.append`, and sends `session.finish` before closing.

Incoming original-language and translated events feed `SubtitleReducer`, which maintains draft and confirmed text separately. The observable `AppModel` publishes connection state and subtitle snapshots to both the menu and overlay window.

## User flow

On first launch, mimi opens Settings because credentials are missing. The API key field saves to Keychain, while Workspace ID and source language save locally as preferences. Pressing Start validates settings, requests Screen Recording permission through ScreenCaptureKit, opens the Alibaba WebSocket, and then begins audio capture. The menu icon and status label progress through Idle, Connecting, Listening, Stopping, or Error.

The subtitle panel sits near the bottom center of the active screen. It has a translucent dark background, rounded corners, a smaller source line, and a larger Chinese line. Users can drag the panel while it is unlocked. A menu toggle locks the panel and makes it ignore mouse events so clicks pass through to the video beneath it. Font size is adjustable in Settings.

Stopping first halts capture, then sends `session.finish`, waits briefly for `session.finished`, and disconnects. The final subtitle remains visible until Clear or another session starts. Closing the settings window does not quit the menu-bar app.

## Failure handling

- Missing credentials: keep capture stopped and open Settings with a specific validation message.
- Capture permission denied: explain how to enable Screen & System Audio Recording in System Settings and offer a retry.
- WebSocket authentication or configuration failure: stop capture, retain the server message where safe, and return to Error state.
- Network interruption: stop capture immediately so audio is never queued without a destination; the user can retry manually.
- Malformed or unknown server events: ignore unknown event types, surface explicit `error` events, and keep the receive loop alive where possible.
- Shutdown: attempt `session.finish`, cancel outstanding receive work, stop `SCStream`, and clear in-memory audio buffers.

## Testing and verification

Unit tests cover endpoint construction, session configuration encoding, audio event encoding, server-event decoding, subtitle draft/final transitions, and settings validation. The capture layer exposes narrow protocols so the app model can be tested with fakes. `swift test` is the required automated gate.

The build is packaged into a minimal `.app` bundle by a script that adds `Info.plist`, the system-audio usage description, and an ad-hoc signature. Manual verification checks first-run permission handling, menu commands, overlay dragging/locking, silence behavior, start/stop cleanup, and a real Alibaba session once the user supplies a Workspace ID and API key locally. No credential is committed, logged, or included in the bundle.

