---
status: passed
phase: 20
verified: 2026-02-25
---

# Phase 20: Transcript Fix Hardening - Verification

## Phase Goal
Caption timing is protected against word count drift when fix_transcription rewrites words.

## Must-Haves Verification

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User sees mismatch warning with diff context | PASS | `print_mismatch_context()` at caption.rs:281 shows count + first 5 changed words |
| 2 | TTY users get interactive prompt (originals/fixed/retry) | PASS | `dialoguer::Select` at caption.rs:456 with 3 choices |
| 3 | Non-TTY causes hard pipeline failure | PASS | `TranscriptMismatch` error returned at caption.rs:444 when `!stdin().is_terminal()` |
| 4 | Retry uses enhanced prompt with word count constraint | PASS | `invoke_claude_fix(words, verbose, true)` appends "MUST return EXACTLY N entries" at caption.rs:323-327 |
| 5 | Retry failure shows 3 versions, user picks | PASS | `offer_post_retry_menu()` at caption.rs:504 shows original/first/retry, 4-choice Select |
| 6 | Caption timing never silently corrupts | PASS | No code path modifies word.start or word.end; `apply_fixes()` only touches word.word |

## Requirement Coverage

| ID | Description | Status | Evidence |
|----|-------------|--------|----------|
| META-03 | Transcript fix prompt enforces exact word count | PASS | `enforce_count` parameter in `invoke_claude_fix()` adds word count constraint to retry prompt |

## Artifact Verification

| Artifact | Exists | Contains Expected |
|----------|--------|-------------------|
| Cargo.toml | Yes | `dialoguer = "0.12"` |
| src/error.rs | Yes | `TranscriptMismatch` variant + `format_error` arm |
| src/commands/caption.rs | Yes | `invoke_claude_fix`, `print_mismatch_context`, `apply_fixes`, `handle_retry`, `offer_post_retry_menu` |

## Build Verification

- cargo check: PASS (1 pre-existing warning: SAFE_MARGIN_TOP)
- cargo clippy: PASS
- cargo test: PASS (16/16)

## Score: 6/6 must-haves verified
