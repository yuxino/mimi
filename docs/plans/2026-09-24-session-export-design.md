# Optional session history and system-audio export

Issue #34 requests timestamped transcript and audio export. The accepted product
boundary is explicit opt-in: two independent Session export settings, both false for new
and existing installations. The non-secret booleans are remembered; content is
never automatically written to disk. Settings can change only while inactive.
Only the settings window may change these options or request an export.

## Lifetime and limits

A manual session start resets both buffers and snapshots the two preferences.
Only confirmed pairs that pass the existing reducer enter transcript retention;
streaming previews never do. The export archive is separate from the overlay's
20-pair history and stops growing at 10,000 pairs or 2 MiB of UTF-8 text. An
oversized pair stops further retention rather than silently producing a gap.
Clearing subtitles also clears the retained transcript. Reconnects and pauses
preserve already confirmed content; a new manual start and app exit erase it.

When opted in, the audio sender copies provider-format PCM (16-bit mono,
16/24 kHz) into a 64 MiB memory buffer before network submission. No disk I/O is
added to native capture callbacks, and recording does not add a second capture
source. The bounded sender's existing backpressure policy remains intact.
Recording is limited to live translation capture; pauses and reconnect gaps are
omitted. A format change or invalid PCM stops retention instead of corrupting a
WAV. UI-only mode never captures or records system audio. When an export option is
explicitly enabled, its synthetic session supplies a fixed sample text pair or
one second of a generated tone for native export QA.

Turning either option off clears its corresponding buffer immediately. Clear
session content removes both buffers without deleting any files exported earlier.
Counts, byte sizes and limit flags are settings-only metadata; archives are not
broadcast in normal session snapshots. Limits are surfaced explicitly in the UI.

## Export

After stopping, the user chooses TXT or WAV and a destination in the native save
dialog. TXT contains UTC final-confirmation timestamps and elapsed session time,
original text and translation. These are not speech onset or media timestamps.
WAV contains the captured PCM timeline; it cannot be aligned directly with TXT
because capture pauses and reconnect gaps are omitted.

A single export snapshot is allowed at a time. Snapshot allocation can temporarily
add up to one 64 MiB audio copy. The lifecycle lock protects snapshot acquisition
but is released while the user chooses a path. A buffer revision invalidates the
snapshot if a new session, clear, or opt-out occurs while the picker is open. The
final revision check and file write serialize with session/settings mutations.
Cancel leaves no file and retains the buffer for retry. Save failures report a
fixed content-free error and preserve the buffer and prior destination. A private
temporary file beside the selected destination is synced and atomically replaces
the destination; failure removes the temporary file. No recovery cache is created.
An abrupt process/OS crash during that final write can leave the temporary file
in the user-selected destination directory; normal cancellation writes nothing.

The official Tauri dialog plugin supplies cross-platform native save dialogs;
no generic filesystem or dialog capability is granted to the frontend. `time`
formats UTC timestamps and `tempfile` safely replaces destinations; both already
exist in the dependency graph. Credentials remain exclusively in the OS keychain,
and diagnostics never include audio, subtitles or destination paths.

## Verification

Core tests cover default-off behavior, legacy preference loading, limits, Unicode
content, timestamp clock reversal, WAV headers/sample preservation, opt-out,
new-session reset, preview exclusion, duplicate finals and retention past overlay
history. File tests cover replacement and failed destinations without residue.
Frontend tests cover serial polling, stale replies, teardown and retries.
Run the full repository check and the signed UI-only development bundle. Real
capture acceptance requires an authorized provider session on each platform;
unit tests and UI-only fixtures do not establish that acceptance.
