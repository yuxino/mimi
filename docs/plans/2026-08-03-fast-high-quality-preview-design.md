# Fast High-Quality Preview Design

## Goal

Make high-quality translation feel immediate without replacing the accurate Qwen-MT Plus final translation.

## Design

Audio 3 interim recognition remains provisional. Each changed, uncommitted draft is sent to Qwen-MT Flash as a streaming preview. The first returned translation token updates the visible draft subtitle, and later tokens extend it. These preview events never enter subtitle history.

Audio 3 server finals and the existing 1.2-second stable / 4.5-second maximum-wait fallback continue through the ordered Qwen-MT Plus queue. Before a source chunk becomes final, its preview request is cancelled. The Plus result then replaces the preview and is the only translation stored in history or translation memory.

Draft requests use a generation token. A newer ASR draft invalidates callbacks from an older request, and an in-flight stale request is preempted after 450 milliseconds if it has not completed. This prevents an older translation from flashing over newer speech while still giving the first request time to return. If a previous Plus sentence finishes while the next sentence already has a valid preview, that preview is restored immediately after the previous final is recorded.

Authentication failures remain terminal. Timeout, cancellation, and other transient preview failures are silent because the accurate Plus final path remains active. Connect, pause, stop, and language switching cancel both preview and final work.

## Alternatives

- Shortening the local finalization delay would reduce latency but fragment sentences and weaken Plus accuracy.
- Replacing Plus with Flash would be faster but would directly trade away the accuracy the user prefers.
- Translating every draft without generation checks would waste requests and display stale source/translation pairs.

## Verification

Tests cover duplicate draft suppression, stale generation rejection, reset behavior, and the rule that draft translations never enter history. The full core suite and warnings-as-errors release build must pass before packaging and signing.
