# mimi demonstration

Starts from the real tray controls, sends original synthetic English speech to the realtime translation service, and shows progressive translated subtitles in the actual floating-overlay frontend. The final subtitle and pause state are real UI states. The existing settings tour follows.

This is the actual production frontend with **real Alibaba Cloud responses**. The documentation harness substitutes the native desktop API boundary; it does not establish OS audio-capture, keychain, filesystem or native-window acceptance. The recording is not a latency benchmark.

Original sample material is used throughout. No personal audio, private PDF, microphone recording, API key or fabricated provider answer is included. Only the user-authorized test material was sent to the configured provider.

The earlier navigation/settings actions retain **10x** speed with **0.8-second** result holds. The new live-result segment is replayed at **3x** with a short final hold, allowing the answer to remain readable. MP4 and GIF timing match.

- `demo.mp4`: full H.264 video, without audio.
- `preview.gif`: inline README animation.
- `poster.png`: an actual result screen.
- `provenance.json`: source commits, service-call metadata, segment timing and media hashes.

No application code, dependencies, update feeds or releases were changed for this demo.
