# Top-tier translation and drama-dialogue prompt

Date: 2026-08-12

## Goal

mimi is used mostly for watching TV dramas, Japanese dramas, and adult videos,
where filler particles and vocal sounds carry meaning. Translation should use
the best available model and must keep those particles instead of smoothing
them away.

## Change

1. **Flagship model everywhere for finals.** `qwen-mt-plus` is the current
   flagship Qwen3-based translation model on Alibaba Cloud Model Studio
   (92 languages). High-quality mode already used it for finals; low-latency
   mode finals now also use `qwen-mt-plus`. Transient draft previews stay on
   `qwen-mt-flash` so the live line still appears quickly.
2. **Drama-dialogue domain hint.** `QwenMTDomainHint.spokenDialogue` now
   explicitly:
   - keeps Chinese particles (嗯、啊、呢、吧、嘛、哦、唉) and maps Japanese
     (えっと、あの、うーん、あぁ) and English (um, uh, oh, hmm) fillers to
     natural equivalents instead of dropping them;
   - renders polite/formal Japanese as naturally courteous Chinese, never stiff;
   - prefers short, complete sentences that fit one subtitle line;
   - keeps deliberate repetition for emphasis and collapses only accidental ASR
     repetition;
   - preserves every vocalization (interjections, breaths, gasps, moans, cries)
     and does not sanitize or censor explicit dialogue;
   - outputs only the translation text, without quotes or explanations.
3. **Glossary force-maps tone words.** Prose guidance alone was not enough:
   Qwen-MT still flattened えっと/うーん/あぁ into nothing. The domain hint now
   also emits `translation_options.terms`, a forced glossary whose entries pin
   filler sounds to natural counterparts (`えっと→那个`, `うーん→嗯`,
   `あぁ→啊`, `まあ→嘛`, `um→嗯`, `oh→哦`, and so on). Terms are selected per
   source→target pair; when the source is set to automatic, the Japanese,
   English, and Korean glossaries are combined because the scripts cannot
   cross-match.

## Trade-off

Low-latency mode finals now take slightly longer (Plus is slower than Flash).
Quality and particle fidelity improve; users who want maximum speed can switch
to the original-subtitle mode, which does no translation.

## Verification

- Protocol tests assert the guidance keeps Chinese particles, Japanese fillers,
  polite-Japanese handling, output-only constraint, vocal sounds, and explicit
  dialogue; the filler glossary pins えっと→那个 and うーん→嗯, combines
  languages for automatic source, encodes into `translation_options.terms`,
  and is omitted when no terms apply.
- The full core suite and strict release build pass via `./scripts/check.sh`.
