---
phase: 20-transcript-fix-hardening
plan: 01
status: complete
---

# Plan 20-01 Summary: Harden fix_transcription

## What Was Built

Replaced the silent fallback in `fix_transcription()` with an interactive mismatch handler. When Claude returns a different word count, the user now sees diff context and chooses how to proceed. Non-TTY environments fail hard instead of silently corrupting.

## Key Changes

| File | Change |
|------|--------|
| Cargo.toml | Added `dialoguer = "0.12"` |
| src/error.rs | Added `TranscriptMismatch` error variant + `format_error` arm |
| src/commands/caption.rs | Extracted `invoke_claude_fix()`, `print_mismatch_context()`, `apply_fixes()` helpers; refactored `fix_transcription()` with TTY-gated interactive prompt and 1-retry policy |

## Architecture

```
fix_transcription()
  -> invoke_claude_fix(enforce_count=false)
  -> if match: apply_fixes (happy path, unchanged)
  -> if mismatch:
       -> print_mismatch_context()
       -> non-TTY: Err(TranscriptMismatch)
       -> TTY: dialoguer::Select (originals / fixed / retry)
         -> retry: invoke_claude_fix(enforce_count=true)
           -> if match: apply_fixes
           -> if mismatch: offer_post_retry_menu (3-way comparison, 4 choices)
```

## Decisions

- Warning goes to stderr only (no log file) -- matches existing CLI pattern
- `apply_fixes` uses `min(words.len(), corrected.len())` to safely handle mismatched lengths when user chooses "Use fixed"
- Retry prompt appends "CRITICAL: You MUST return EXACTLY N entries" constraint
- Three-way comparison shows first 20 words of each version

## Self-Check: PASSED

- [x] cargo check passes
- [x] cargo clippy passes (only pre-existing SAFE_MARGIN_TOP warning)
- [x] cargo test passes (16/16)
- [x] TranscriptMismatch variant used in non-TTY and abort paths
- [x] dialoguer::Select used for interactive prompts
- [x] IsTerminal gates all interactive paths
- [x] Spinner cleared before any output/prompt
- [x] Word timing (start/end) never modified -- only word field updated

## Commits

1. `feat(20-01): add dialoguer dependency and TranscriptMismatch error variant`
2. `feat(20-01): harden fix_transcription with interactive mismatch handling and retry`
