# Windows audio-format and dual-architecture CI design

## Problem

Windows loopback capture reads the default playback endpoint's mix format, but
the callback currently accepts only F32 and I16. CPAL 0.18 can expose ten other
PCM or floating-point formats, so an otherwise valid playback device can be
rejected before capture starts. The existing Windows CI also uses an implicit
x64 runner and has no ARM64 compile, test, or launch coverage.

## Design

- Keep the usual F32 callback as a zero-copy fast path.
- Convert I8, I16, I24, I32, I64, U8, U16, U24, U32, U64, and F64 through
  CPAL's sample conversion contract into normalized F32 before the existing
  downmix/resample/PCM16 pipeline.
- Reuse one normalization buffer owned by the capture processor so integer or
  F64 callbacks do not allocate a new vector every time.
- Reject DsdU8, DsdU16, DsdU32, and future unknown formats before stream
  creation. DSD is a bitstream rather than PCM and cannot use the PCM
  resampler safely.
- Pin x64 CI to Windows Server 2025 and add a native Windows 11 ARM64 job. Both
  architectures run Rust tests, compile the complete Tauri app, verify the PE
  machine type, and keep a credential-free UI-test process alive briefly.

## Verification boundary

Windows unit tests cover support classification, DSD rejection, normalization
of every signed, unsigned, 24-bit, and floating-point representation, and
buffer reuse. CI startup uses Mimi's synthetic UI-test mode, which never reads
credentials, opens provider connections, or starts audio capture. It proves
the x64/ARM64 executable can start on GitHub runners, but does not prove real
WASAPI loopback, tray interaction, installer acceptance, or physical x64/ARM64
audio-driver behavior.
