---
phase: 21-interactive-title-approval
plan: 01
status: complete
---

# Plan 21-01 Summary

## What Was Built

Interactive title approval for the overlay and pipeline workflows.

## Changes

| File | Change |
|------|--------|
| `src/cli.rs` | Added `--no-interactive` flag to PipelineArgs and OverlayArgs |
| `src/commands/overlay.rs` | Replaced `generate_title()` with `generate_title_options()` (pub, returns Vec<String>); added `parse_title_options()` helper; added `approve_title()` with dialoguer Select + Input + non-TTY fallback; 3 new parse tests |
| `src/commands/pipeline.rs` | Pipeline calls `overlay::generate_title_options()` + `overlay::approve_title()` before Stage 7; `no_interactive` threaded through run -> run_stages -> finish_stages; dry_run output updated |

## Key Decisions

- Claude prompt uses `---` delimiter to separate 3 title options
- Parsing falls back to single option if <2 delimited sections found
- `approve_title()` auto-selects first option when `no_interactive || !is_terminal()`
- Pipeline handles title generation + approval, then passes approved title via `text` arg to overlay (skipping overlay's internal generation)

## Commits

1. `feat(21-01): add --no-interactive flag and multi-option title generation with approval`
2. `feat(21-01): integrate title approval into pipeline Stage 7`
