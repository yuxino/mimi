# Stop duplicate subtitles from local commits racing server finals

Date: 2026-08-11

## Problem

Live pipeline logs from a real session (Chinese/English audio, high-quality
mode) showed the stable-draft local committer racing the server's semantic
finals:

- A 15-character sentence was committed locally at 22:43:43.059, and the server
  final for the same speech arrived 28 ms later at 22:43:43.087 as a
  23-character result. The overlap was committed as a separate "tail", so one
  spoken sentence produced two history entries.
- A sentence committed locally as 11 characters was followed 0.5 s later by a
  25-character revision from ASR, then further fragments. The provisional lines
  stayed in history while the server's corrected final was committed again.

Users see the same speech twice (or chopped into fragments): the local commit is
based on an in-flight partial that the server is still revising, so history gets
the provisional wording and later the authoritative wording as separate rows.

## Change

Local commits are treated as replaceable previews; server finals remain the
single source of durable history.

1. **Supersede and revoke** (`ASRDraftCommitter.finishSentence` now returns
   `FinishOutcome`): when a server final structurally covers the last local
   commit (extends it, or contains it with additional leading words), the
   committer reports `.replaced(fullFinal)` instead of appending a tail.
2. **Revoke plumbing**: a new client-internal `LiveTranslateServerEvent`
   (`subtitleRevoked`) maps to `SubtitleEvent.revokeLastConfirmed`, which
   removes only the last history pair. The high-quality client defers the
   revoke until the replacement translation is ready, so the provisional entry
   has already entered history and the replacement lands immediately after —
   the line updates in place instead of blinking away. Exact duplicates remain
   deduplicated without a revoke, so confirmed lines do not flicker.

An earlier draft of this change added a "stability gate" that only committed a
complete sentence after it appeared in two consecutive drafts. Live testing
showed it added seconds of latency for utterances that arrive as a single
partial, so it was removed. Local commits keep the normal 1.2 s cadence; the
supersede-and-revoke path fixes the duplicate after the fact, and the 4.5 s
maximum-wait and session-finish paths remain the last-resort fallback for
stalled ASR.

## Verification

- Unit tests cover exact server-final dedup, extension supersede, leading-word
  supersede, revised wording falling back to append, and revoke semantics in
  the reducer and session controller.
- The full core suite and release build with warnings treated as errors pass via
  `./scripts/check.sh`.

Residual case: a server final that rewrites a stable local commit with
completely different wording cannot be detected structurally and is appended
like new content. The stability gate makes that rare by keeping still-revising
sentences out of history in the first place.

## 2026-08-14 追加：积压下的队列合并与降载

真实会话反馈（高音量连读）：翻译延迟随语音量持续增长、偶发重复字幕，
像是队列积压。定位到高质量管线的串行 final 队列：稳定草稿本地提交与
服务端 final 都入同一 FIFO，语音爆发时入队速度快于 MT 翻译速度，深度
无上界 → 延迟无上界；已入队但尚未展示的本地提交随后被服务端 final
覆盖时仍会先翻译一遍 → 重复行。

修复（`high_quality_client`）：

1. **合并（coalesce）**：服务端 final 入队时，若结构上覆盖队尾的本地
   提交（复用 committer 的 `final_covers_chunk` 覆盖规则），直接原位替换
   队尾文本——该临时提交从未展示过，省一次翻译往返、少一条重复历史行；
   同时回退为其预留的 revoke 计数（revoke 只在临时行已上屏时使用）。
2. **降载（shed）**：队列深度 ≥ 3 时丢弃最老的仍未开始的 `stable-draft`
   项；`server-final`/`maximum-wait`/`session-finish` 永不丢弃。被丢的
   本地提交若 ASR 后续没有 final，maximum-wait 兜底路径仍会按原机制提交，
   内容不丢；高音量下服务端 final 几乎总是紧随其后，延迟被限制在
   ~3 × 单次翻译时长。

`final_covers_chunk` 抽出为 committer 共享纯函数并有单元测试；原文模式下
语言胶囊的模式槽位显示"原文"占位（不再无声消失，用户不再困惑）。
