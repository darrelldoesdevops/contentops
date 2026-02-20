---
phase: 06-audit-cleanup
plan: 02
subsystem: refactor
tags: [dead-code, spinner, indicatif, rust]

requires:
  - phase: 06-audit-cleanup
    provides: Findings report (06-01) documenting all issues
provides:
  - Shared spinner factory in src/ui.rs
  - Zero dead code suppressions
  - Clean clippy baseline
affects: [06-03, 07-doctor, 08-pipeline]

tech-stack:
  added: []
  patterns:
    - "Shared UI utilities in src/ui.rs for cross-module progress indicators"

key-files:
  created:
    - src/ui.rs
  modified:
    - src/temp.rs
    - src/main.rs
    - src/commands/cut.rs
    - src/commands/caption.rs
    - src/commands/overlay.rs
    - src/commands/normalize.rs
    - src/ffmpeg.rs

key-decisions:
  - "Deleted cleanup_all() -- Pipeline will share TempFileRegistry directly, no standalone cleanup needed"
  - "Used impl Into<String> for make_spinner parameter -- accepts both String and &str without caller conversion"

patterns-established:
  - "ui::make_spinner for all spinner creation -- future commands must use crate::ui::make_spinner"

requirements-completed: [AUDIT-01, AUDIT-02, AUDIT-03]

duration: 2min
completed: 2026-02-20
---

# Phase 6 Plan 2: Dead Code Removal + Spinner Extraction Summary

**Deleted cleanup_all() dead code, extracted 5 duplicate spinner factories into shared src/ui.rs module**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-20T09:05:57Z
- **Completed:** 2026-02-20T09:08:12Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Removed `TempFileRegistry::cleanup_all()` and its `#[allow(dead_code)]` suppression from temp.rs
- Created `src/ui.rs` with `pub fn make_spinner(message: impl Into<String>)` shared utility
- Replaced all 5 duplicate spinner implementations across cut.rs, caption.rs, overlay.rs, normalize.rs, and ffmpeg.rs
- Removed unused `Duration`, `ProgressBar`, `ProgressStyle` imports from files that no longer need them

## Task Commits

1. **Task 1+2: Delete dead code and replace spinners** - `0142399` (refactor)

## Files Created/Modified
- `src/ui.rs` - New shared spinner factory module
- `src/temp.rs` - Removed cleanup_all() and #[allow(dead_code)]
- `src/main.rs` - Added mod ui declaration
- `src/commands/cut.rs` - Replaced local make_spinner with ui::make_spinner
- `src/commands/caption.rs` - Replaced local make_spinner with ui::make_spinner
- `src/commands/overlay.rs` - Replaced inline spinner with ui::make_spinner
- `src/commands/normalize.rs` - Replaced inline spinner with ui::make_spinner
- `src/ffmpeg.rs` - Replaced inline spinner with crate::ui::make_spinner

## Decisions Made
- Deleted cleanup_all() rather than keeping with justification (Phase 8 won't need it)
- Used `impl Into<String>` for make_spinner to avoid `.to_string()` at every call site

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Codebase is clean with zero clippy warnings and zero dead code suppressions
- Ready for Plan 06-03 (consistent AppError error handling)

---
*Phase: 06-audit-cleanup*
*Completed: 2026-02-20*
