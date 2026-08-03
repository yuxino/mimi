# Codex Project Guidance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add portable repository guidance and one canonical automated verification command for Codex-driven changes.

**Architecture:** Put durable project expectations in the root `AGENTS.md`, which Codex discovers automatically. Put repeatable automated checks in `scripts/check.sh`, while keeping app packaging and signing as an explicit change-dependent step.

**Tech Stack:** Markdown, Bash, Swift 6.1, Swift Package Manager.

---

### Task 1: Add repository guidance

**Files:**
- Create: `AGENTS.md`

1. Document the product privacy constraints and repository map.
2. Document Swift concurrency, architecture, and streaming pipeline expectations.
3. Document the canonical check command and change-specific manual verification.

### Task 2: Add the canonical check command

**Files:**
- Create: `scripts/check.sh`
- Modify: `CONTRIBUTING.md`

1. Add a strict Bash script that resolves the repository root from its own location.
2. Run `swift run mimi-core-tests`.
3. Run `swift build -c release -Xswiftc -warnings-as-errors`.
4. Run `git diff --check` and `git diff --cached --check`.
5. Make the script executable and replace duplicated contribution-guide commands with `./scripts/check.sh`.

### Task 3: Verify and deliver

**Files:**
- Verify: `AGENTS.md`, `scripts/check.sh`, `CONTRIBUTING.md`, and the plan files

1. Run `bash -n scripts/check.sh` and expect no output.
2. Run `./scripts/check.sh` and expect all core tests and the strict release build to pass.
3. Review `git diff` and `git status` for accidental files or sensitive data.
4. Commit the focused change and push `main` to `origin`.
