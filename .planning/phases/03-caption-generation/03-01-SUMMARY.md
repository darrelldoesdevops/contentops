---
phase: 03-caption-generation
plan: 01
subsystem: cli
tags: [whisper-cpp, ffmpeg, srt, serde, captions, transcription]

requires:
  - phase: 01-foundation
    provides: CLI skeleton, FFmpeg wrapper, error handling, temp file management
provides:
  - Caption subcommand with full pipeline (audio extraction, transcription, SRT + JSON output)
  - CaptionArgs CLI struct with input, output, model, lang flags
  - WhisperNotFound, WhisperFailed, ModelNotFound error variants
  - require_whisper() detection function
  - SRT generation with word grouping (3-5 words per entry)
  - JSON sidecar with per-word timestamps
affects: [04-caption-rendering]

tech-stack:
  added: [serde, serde_json]
  patterns: [whisper-cpp shell-out, timestamp parsing, SRT formatting, word grouping]

key-files:
  created:
    - src/commands/caption.rs
  modified:
    - Cargo.toml
    - src/cli.rs
    - src/error.rs
    - src/commands/mod.rs
    - src/main.rs

key-decisions:
  - "Shell out to whisper-cpp binary rather than whisper-rs binding for stability"
  - "Word grouping at 3-5 words per SRT entry, breaking on punctuation"
  - "Validation order: ffmpeg -> whisper-cpp -> input file -> model file"

patterns-established:
  - "Caption command pattern: validate deps, extract audio, transcribe, generate outputs"
  - "derive_caption_output helper for multi-extension output file naming"
  - "Timestamp parsing from HH:MM:SS.mmm string to f64 seconds"

requirements-completed: [CAP-01, CAP-02]

duration: 3min
completed: 2026-02-20
---

# Phase 3 Plan 1: Caption Generation Summary

**whisper-cpp caption pipeline with audio extraction, SRT word-grouped output, and JSON sidecar with per-word timestamps**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-19T16:13:40Z
- **Completed:** 2026-02-19T16:16:14Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Caption subcommand registered in CLI with --model and --lang flags
- Full pipeline: FFmpeg audio extraction -> whisper-cpp transcription -> SRT + JSON output
- Actionable error messages for missing whisper-cpp, missing model, and missing input
- SRT word grouping (3-5 words, break on punctuation) with proper timestamp formatting

## Task Commits

Each task was committed atomically:

1. **Task 1: CLI scaffold, error variants, and whisper-cpp detection** - `18fd268` (feat)
2. **Task 2: Full caption pipeline** - `5f9cf3f` (feat)

## Files Created/Modified
- `src/commands/caption.rs` - Full caption pipeline (393 lines): audio extraction, whisper-cpp invocation, JSON parsing, SRT + JSON output
- `src/cli.rs` - CaptionArgs with input, output, model, lang flags; Caption variant in Commands enum
- `src/error.rs` - WhisperNotFound, WhisperFailed, ModelNotFound variants; require_whisper() function; format_error arms
- `src/commands/mod.rs` - Added `pub mod caption`
- `src/main.rs` - Caption match arm dispatching to caption::run
- `Cargo.toml` - Added serde (with derive) and serde_json dependencies

## Decisions Made
- Shell out to whisper-cpp binary (not whisper-rs) for stability, matching blockers noted in STATE.md
- Validate dependencies in order: ffmpeg -> whisper-cpp -> input -> model (fail fast on missing tools)
- Group words into SRT entries of 3-5 words, breaking early on punctuation for natural reading

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. Users need whisper-cpp installed (`brew install whisper-cpp`) and a model file downloaded from huggingface.

## Next Phase Readiness
- Caption generation complete, produces SRT + JSON files ready for Phase 4 (caption rendering/overlay)
- SRT format compatible with FFmpeg ass/subtitles filter for overlay

## Self-Check: PASSED

All files verified present. All commit hashes verified in git log.

---
*Phase: 03-caption-generation*
*Completed: 2026-02-20*
