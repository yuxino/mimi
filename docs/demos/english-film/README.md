# English film documentation capture

These files are an isolated documentation harness, not application code or a supported native test mode. They load the unchanged production frontend and the attributed film response fixture; they never call a provider or accept credentials.

Use the frontend and 31-second Sintel excerpt from preparation run 34038261225, artifact 9990866707. The preparation source is recorded in Git history at b048092cbe76d5d4fb5836400c7e377a1e3e273e. The exact movie hash is retained in response.json. If rebuilding the film excerpt produces different bytes, review that source and its timing before changing any hash guard.

Install Python 3.12, Playwright 1.55.0, Chrome, ffmpeg and Noto Sans CJK. From the repository root, set MIMI_PREPARED to the prepared directory containing dist/, sintel-excerpt.mp4 and source-commit.txt. Set MIMI_RESPONSE to the absolute path of docs/demos/english-film/response.json, then run python -I docs/demos/english-film/record.py. Outputs are written to english-film-review/ and english-film-delivery/.

The recorder verifies the sample hashes, normal and transparent overlay states, pause/resume, full film playback, final immersive mode, media decoding and output dimensions. Only local fixture URLs are allowed in the browser. Captured compositor frames are ordered by timestamp; the source video is not replaced or edited into the UI afterward. The original film audio is aligned to measured playback start.

See the parent README for film credit, licensing and recording boundaries.
