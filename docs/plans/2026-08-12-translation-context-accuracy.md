# Translation context and accuracy pass

Date: 2026-08-12

## Goal

Improve subtitle translation quality by making Qwen-MT use the ongoing
conversation as context, without changing latency or the confirmed-final-only
display behavior.

## Change

1. **More translation memory for finals.** High-quality mode already passed the
   last 3 source→target pairs as `translation_options.tm_list`; it now passes
   the last 6, and the in-session memory cap grows from 6 to 12 pairs.
   Low-latency mode finals now pass the last 5 pairs for the same detected
   language. Qwen-MT-plus has a 16K-token window, so the extra pairs are
   negligible in cost and latency.
2. **Pin the detected source language.** When the configured source is
   automatic, the language reported by ASR is resolved and sent as
   `source_lang` instead of `auto`. Alibaba's docs state that specifying the
   source language improves translation accuracy.
3. **Tell the model the memory is the ongoing dialogue.** The spoken-dialogue
   domain prompt now explains that `tm_list` is the recent conversation: keep
   names, pronouns, and implied subjects consistent with earlier lines, keep
   the speaker's tone and register, resolve ambiguous or truncated phrases
   from what came before, and never repeat or re-translate remembered lines.

## Verification

- Protocol tests assert the guidance frames `tm_list` as the ongoing dialogue
  and forbids re-translating remembered lines.
- The full core suite and the strict warnings-as-errors release build pass.
- The packaged app is manually inspected in high-quality translated mode.
