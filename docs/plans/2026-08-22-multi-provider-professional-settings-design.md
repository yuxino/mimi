# Multi-provider profiles and professional settings

## Goal

Make mimi behave like a maintainable open-source application rather than an
Alibaba-only form. The first release of this work keeps Alibaba Cloud as the
default, adds an OpenAI Realtime end-to-end provider, and gives users a clear,
professional place to manage more than one service profile.

This change must preserve mimi's privacy boundaries: capture system audio only,
never persist audio or subtitle text, keep API keys in the operating-system
credential store, and keep diagnostics content-free.

## Product decisions

- A **service profile** is a named, built-in provider configuration. Users may
  create several profiles, but exactly one is active for the next session.
- Existing installations migrate to a stable `alibaba-default` profile and keep
  using Alibaba Cloud without asking the user to choose again.
- Alibaba Cloud supports the existing Turbo, Low latency, and High quality
  modes and the existing source/target language matrix. Automatic source
  detection exposes only Turbo and Low latency because High quality requires
  an explicit source-language hint.
- OpenAI Realtime is the second complete provider. It uses automatic source
  detection, Simplified Chinese/English/Japanese targets, and Turbo mode. It is
  not presented as a generic "OpenAI-compatible" endpoint because realtime
  audio protocols are not interchangeable.
- Profiles cannot be changed while a session is connecting, listening, paused,
  or stopping. A running session owns an immutable resolved configuration.
- Provider support remains a typed, built-in registry. This is intentionally not
  a runtime plug-in loader or a free-form endpoint/schema editor.

## Data and credential model

Public profile metadata is persisted separately from general preferences:

```json
{
  "schemaVersion": 1,
  "activeProfileId": "alibaba-default",
  "profiles": [
    {
      "id": "alibaba-default",
      "name": "Alibaba Cloud",
      "provider": "alibabaCloud"
    }
  ]
}
```

Profile IDs are stable, bounded identifiers. Names are user-facing only and
never appear in diagnostics. API keys are not part of this JSON, persisted or
global frontend state, events, snapshots, logs, or error messages. A replacement
exists only briefly in the local write-only editor draft and its dedicated save
command.

Each secret is isolated by both profile and provider:

```text
provider-profile:<profile-id>:<provider-id>:api-key
```

The frontend receives only `present`, `missing`, or `unavailable`. The API-key
field is always an empty replacement draft; saving clears it immediately.
Changing a profile's provider is not supported, preventing an existing secret
from being sent to a different service.

The legacy Alibaba key migrates only to `alibaba-default`. A credential-store
tombstone is authoritative after successful migration or explicit removal, so
clearing preferences cannot resurrect an old key. Migration verifies a readable,
non-empty destination before writing the tombstone. Unreadable, blank, or
concurrently changed values fail closed and are never automatically deleted.
An empty legacy scan does not write a tombstone, so a key created later by an
older installed version remains eligible for migration.
Verified legacy values remain available for rollback after migration. An
explicit removal of the default Alibaba credential/profile writes the
tombstone first, then strictly removes the legacy slots before deleting the
current profile slot; a failure leaves the current credential reachable for a
retry.

## Provider boundary

The session layer resolves the active profile natively, obtains its secret only
for session construction, validates provider capabilities, and builds the
matching client. Frontend start requests never carry credentials or provider
configuration.

Alibaba keeps its 16 kHz mono PCM pipeline. OpenAI Realtime uses 24 kHz mono
PCM, so audio capture exposes the provider-requested format and performs bounded
resampling before network transmission. The OpenAI adapter:

- waits for session configuration acknowledgement before accepting audio;
- sends bounded audio frames and ignores output audio;
- treats recoverable provider errors as non-terminal and sanitizes all errors;
- pairs source and translation finals atomically;
- distinguishes graceful close from timeout/failure and does not persist an
  unconfirmed tail;
- bounds connection, send, and finish operations; and
- cancels stale generations so an old connection cannot affect a new session.

## Settings experience

The settings window becomes a responsive control surface instead of a long form.
Its default size is 900×760 with a 560×620 minimum. At 720 points and above it
uses a restrained two-column layout; below that it becomes one scrollable column.

1. A compact session hero shows readiness, the active provider/profile, and the
   primary Start/Stop action.
2. Subtitle preferences group source language, target language, translation
   mode, typography, and overlay behavior. Controls reflect the active provider's
   capabilities and never silently accept unsupported values.
3. Service profiles have a scannable list and a focused editor. Users can add,
   rename, select, and delete profiles, and replace or remove a credential
   without ever reading it back.
4. Credential state is communicated with icon, text, and detail—not color alone.
   Missing fields focus the relevant control; unavailable secure storage is not
   mislabeled as a missing key.
5. Mutating controls are disabled while a session is active and re-check session
   state when an action executes, including confirmation dialogs.
6. Each window installs settings/session listeners before requesting its boot
   snapshots. Events received while those snapshots are in flight are buffered
   and take precedence, so opening a window cannot roll back a concurrent tray,
   shortcut, or profile change. Partial initialization is cleaned up and
   retried; until then credential readiness remains unavailable rather than
   optimistically configured.

The visual language is native and quiet: semantic surfaces, clear type hierarchy,
subtle borders/material, one calm teal accent, consistent spacing, visible keyboard
focus, and reduced-motion support. New strings are added in Simplified Chinese,
English, and Japanese with identical key sets.

## Migration and compatibility

- A missing catalog creates `alibaba-default` and marks it active.
- The old single Alibaba credential is imported once according to the credential
  rules above. Existing language, mode, font, overlay, locale, and startup
  preferences remain unchanged.
- A catalog is written atomically. Invalid or unknown data is reported without
  overwriting the original file.
- At least one profile must remain. Deleting the active profile requires another
  valid selection.
- Profile deletion removes the scoped credential before committing the catalog;
  a process interruption can therefore leave a visible profile that needs a new
  key, but never an unreachable orphan credential.
- The general settings save command persists only non-secret preferences. Profile
  and credential mutations use dedicated native commands.

## Verification

Automated coverage must include:

- profile validation, JSON round trips, active-profile invariants, and atomic
  migration;
- provider/profile credential isolation, tombstone behavior, unavailable-store
  handling, write-only IPC, and secret redaction;
- provider factory selection and capability normalization;
- OpenAI session acknowledgement, framing/resampling, recoverable errors,
  atomic finals, cancellation, and bounded finish;
- frontend profile CRUD, incomplete/unavailable states, active-session guards,
  capability-aware controls, localization key parity, and absence of secrets in
  rendered state; and
- responsive settings layouts at narrow and wide sizes, keyboard focus, long
  localized strings, reduced motion, and normal/empty/error/paused/listening
  states.

Run `./scripts/check.sh`, then launch the packaged development app through
`./scripts/dev-app.sh` and inspect the real settings window. Real provider smoke
tests use local credential-store entries and must not record or log content.
