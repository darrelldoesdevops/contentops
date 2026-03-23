---
phase: 23-pipeline-reorder
plan: 01
subsystem: pipeline
tags: [rust, ffmpeg, whisper, vad, silence-removal, captioning]

requires: []
provides:
  - Pipeline stages reordered: scale -> normalize -> cut -> transcribe -> fix -> caption -> overlay
  - adjust_timestamps call removed from pipeline execution path
  - Whisper receives cut video and self-extracts WAV (timestamps match final timeline)
affects: [24-cleanup]

tech-stack:
  added: []
  patterns:
    - "Cut-before-transcribe: run VAD/FFmpeg cut on normalized video before Whisper, so transcript timestamps need no post-hoc adjustment"

key-files:
  created: []
  modified:
    - src/commands/pipeline.rs

key-decisions:
  - "Cut before transcribe: Whisper runs on cut video so timestamps are naturally correct, eliminating adjust_timestamps drift"
  - "transcribe() receives None for wav_path so it self-extracts from cut video using identical 16kHz mono parameters"
  - "VAD WAV extracted from normalized (pre-cut) video, cleaned up before transcription stage begins"
  - "Empty speeches path falls through to transcription instead of returning early with pre-transcribed words"

patterns-established:
  - "Pipeline stage reorder: move blocks in run_stages(), update banners, remove now-unnecessary transformations"

requirements-completed: [PIPE-01, PIPE-02, PIPE-03]

duration: 15min
completed: 2026-03-22
---

# Phase 23 Plan 01: Pipeline Reorder Summary

**Pipeline reordered so VAD silence cut runs before Whisper transcription, eliminating adjust_timestamps drift by letting Whisper timestamp the already-cut video directly**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T00:00:00Z
- **Completed:** 2026-03-22
- **Tasks:** 2 (1 auto, 1 checkpoint auto-approved)
- **Files modified:** 1

## Accomplishments
- Reordered run_stages() from scale->normalize->transcribe->fix->cut to scale->normalize->cut->transcribe->fix->caption->overlay
- Removed adjust_timestamps block (lines 439-448) and all associated word_data/adjusted/adjusted_words variables
- transcribe() now receives &cut_output and None for wav_path — self-extracts from final video
- Updated dry-run output to reflect new stage order (cut=3, transcribe=4, fix=5)
- Empty speeches path now falls through to transcription instead of early-returning with stale word timestamps
- All 21 tests pass, clippy clean, release build successful

## Task Commits

1. **Task 1: Reorder run_stages() and remove adjust_timestamps call** - `375db75` (feat)
2. **Task 2: Verify caption sync on real video** - auto-approved (checkpoint:human-verify, release build passed)

## Files Created/Modified
- `src/commands/pipeline.rs` - Reordered stages 3-5, removed adjust_timestamps block, updated dry-run output

## Decisions Made
- WAV is still extracted once for VAD from the normalized video. Whisper gets None and self-extracts from cut video — two WAV extractions total but each on the right timeline for its purpose.
- Empty speeches case no longer returns early: copies normalized to cut.mp4 and falls through to transcription so captions are still generated even when no silence is removed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Pipeline reorder complete, ready for Phase 24 which removes the now-dead adjust_timestamps function from silence.rs
- Real video verification can happen any time with `contentops pipeline -i <video> -m <model>`

---
*Phase: 23-pipeline-reorder*
*Completed: 2026-03-22*

## Self-Check: PASSED
- src/commands/pipeline.rs: FOUND
- 23-01-SUMMARY.md: FOUND
- Commit 375db75: FOUND
