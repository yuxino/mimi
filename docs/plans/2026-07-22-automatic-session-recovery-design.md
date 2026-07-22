# Automatic Session Recovery Design

## Goal and approach

mimi should recover after a video switch without inspecting browser URLs, window titles, or playback controls. System-audio capture remains display-wide. A lightweight watchdog observes two local signals: recent non-silent PCM audio and recent Alibaba Cloud server events. When audio is active but no server event arrives for eight seconds, the current translation connection is considered stale and mimi rebuilds the WebSocket session while keeping the last subtitle visible.

This is safer than detecting browser tabs because it works with any video application and does not require Accessibility permission. It is also more precise than restarting on every silent gap: silence alone never triggers recovery. A natural pause can last indefinitely; recovery begins only after audible content resumes and the service still produces no recognition or translation event.

## Components and data flow

`PCM16AudioActivityDetector` computes the RMS level of captured 16-bit mono PCM and classifies meaningful system audio. `TranslationRecoveryMonitor` is a deterministic state machine containing the latest server-event time, latest active-audio time, stall timeout, activity window, and recovery cooldown. Both live in `MimiCore` so they can be tested without ScreenCaptureKit or a real network.

`SystemAudioCapture` reports throttled activity callbacks in addition to PCM chunks. `AppModel` feeds activity and server-event timestamps into the monitor and runs a one-second watchdog task while listening. A stale decision stops capture, disconnects the old client, changes the visible state to Connecting, then reconnects without clearing the last subtitle. Manual Stop cancels the watchdog immediately.

## Failure handling and verification

Only one automatic recovery can run at a time, and a cooldown prevents loops. A failed recovery uses the existing error state rather than retrying forever. Tests cover silence, audible PCM, recent server traffic, stale active audio, cooldown, and recovery after a new video resumes. Existing reducer, protocol, capture conversion, packaging, signing, and live-service checks must remain green.
