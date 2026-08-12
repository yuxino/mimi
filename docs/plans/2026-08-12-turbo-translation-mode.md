# Turbo translation mode

Date: 2026-08-12

## Goal

Add a speed-first "极速" (turbo) translation mode as a third option in
Settings, alongside 低延迟 and 高质量. Recognition is already streaming and
fast; the perceived delay comes from waiting for a full sentence and then
running the slow flagship Plus model. Turbo removes both waits.

## Change

1. **New mode.** `TranslationMode.turbo` with display name 极速. It is exposed
   in the Settings 翻译模式 picker, persisted, shown in the overlay language
   pill and menu bar, and survives language switching.
2. **Fast model for finals.** Turbo uses the high-quality pipeline
   (`Audio3ASR` + local sentence commits) but translates finals with
   `qwen-mt-flash`, which supports incremental streaming. The final translation
   streams onto the live line as it is generated, then locks into history.
3. **Short commit windows.** Stable-draft commit drops from 1.2s to 500ms and
   the maximum wait from 4.5s to 2s; long incomplete speech commits earlier
   (12 characters instead of 20). Subtitles appear close to the spoken moment.
4. **Mode is user-chosen.** `prepareForListening` and language switching no
   longer force high-quality mode, so the selected mode sticks.

## Trade-off

Flash is noticeably faster than Plus at the cost of some translation
fidelity. 低延迟 keeps Plus finals (preview + correct), and 高质量 stays
the most accurate option.

## Verification

- Configuration tests assert turbo survives automatic source and validation,
  and modes expose short display names.
- The full core suite and strict warnings-as-errors release build pass.
- The packaged app is manually inspected with turbo selected.
