# Codex Project Guidance Design

## Goal

Give Codex and other coding agents durable, repository-specific context so they can change Mimi safely and verify their work consistently.

## Design

Add a root `AGENTS.md`, which Codex discovers automatically at project scope. Keep it concise and focused on facts that are easy to miss from a single file: the Swift package layout, the custom executable test runner, concurrency boundaries, streaming draft/final semantics, bounded-latency expectations, and Mimi's privacy rules.

Add `scripts/check.sh` as the canonical automated verification entry point. It runs the complete core suite, the warnings-as-errors release build, and whitespace checks on both staged and unstaged diffs. Packaging remains separate because it assembles and signs a local app bundle and is only required for UI, packaging, or release work.

Update the contribution guide to use the canonical check command. Do not add `.codex/config.toml`: model choice, approval policy, sandboxing, and local tools are environment concerns, while this repository only needs portable project guidance.

## Alternatives

- Adding only `AGENTS.md` would document the correct commands but leave agents to reproduce a multi-command verification sequence.
- Adding a project `.codex/config.toml` could standardize runtime preferences, but it would unnecessarily impose local security and model choices on contributors.
- Adding CI would improve remote enforcement, but it expands scope beyond preparing the repository for Codex and requires a separate workflow decision.

## Verification

Run `bash -n scripts/check.sh`, then run `./scripts/check.sh`. Review the resulting diff and confirm no personal paths, generated output, credentials, or runtime configuration were added.
