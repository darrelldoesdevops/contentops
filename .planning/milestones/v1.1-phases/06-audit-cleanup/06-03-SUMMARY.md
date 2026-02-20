---
phase: 06-audit-cleanup
plan: 03
subsystem: error-handling
tags: [thiserror, anyhow, AppError, rust]

requires:
  - phase: 06-audit-cleanup
    provides: Clean codebase with shared spinner utility (06-02)
provides:
  - NoSpeechDetected, ClaudeFailed, ParseFailed AppError variants
  - Consistent colored format_error() output for all error paths
  - Zero bare anyhow::bail! or anyhow::anyhow! in command files
affects: [07-doctor, 08-pipeline]

tech-stack:
  added: []
  patterns:
    - "All errors use typed AppError variants with format_error() for colored output"

key-files:
  created: []
  modified:
    - src/error.rs
    - src/commands/cut.rs
    - src/commands/caption.rs
    - src/commands/overlay.rs
    - src/commands/normalize.rs

key-decisions:
  - "Used ParseFailed { stage, message } for JSON errors -- preserves context without needing serde_json in AppError"
  - "ClaudeFailed reuses FfmpegFailed-style indented stderr formatting for consistency"

patterns-established:
  - "All new error paths must use AppError variants, never bare anyhow macros"

requirements-completed: [AUDIT-04]

duration: 2min
completed: 2026-02-20
---

# Phase 6 Plan 3: Consistent AppError Error Handling Summary

**Added NoSpeechDetected, ClaudeFailed, and ParseFailed variants; converted all 8 bare anyhow calls to typed AppError with colored format_error() output**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-20T09:09:09Z
- **Completed:** 2026-02-20T09:11:03Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added 3 new AppError variants: NoSpeechDetected, ClaudeFailed, ParseFailed
- Added 3 matching format_error() arms with consistent colored output using owo_colors
- Converted 4 bare anyhow::bail! and 4 bare anyhow::anyhow! calls to typed AppError variants
- Every error path now flows through format_error() for consistent user-facing messages

## Task Commits

1. **Task 1+2: Add variants and convert all anyhow calls** - `a977f1b` (refactor)

## Files Created/Modified
- `src/error.rs` - Added NoSpeechDetected, ClaudeFailed, ParseFailed variants + format_error() arms
- `src/commands/cut.rs` - anyhow::bail! -> AppError::NoSpeechDetected
- `src/commands/caption.rs` - anyhow::anyhow! -> AppError::ParseFailed (2 sites)
- `src/commands/overlay.rs` - anyhow::bail! -> AppError::ClaudeFailed (2), InputNotFound (1); anyhow::anyhow! -> ParseFailed (1)
- `src/commands/normalize.rs` - anyhow::anyhow! -> AppError::ParseFailed

## Decisions Made
- Used ParseFailed { stage, message } with String message rather than embedding serde_json::Error
- Reused InputNotFound for missing transcription file (same semantics as missing input video)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 6 complete -- all 5 AUDIT requirements satisfied
- Codebase ready for Phase 7 (Doctor subcommand)

---
*Phase: 06-audit-cleanup*
*Completed: 2026-02-20*
