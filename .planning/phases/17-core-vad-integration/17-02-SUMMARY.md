---
phase: 17-core-vad-integration
plan: 02
subsystem: audio-processing
tags: [rust, vad, pipeline, whisper, wav-sharing]

requires: [17-01]
provides:
  - Pipeline Stage 3 using VAD instead of silencedetect
  - caption::transcribe() with wav_path parameter for WAV sharing
  - Single WAV extraction shared between Whisper (Stage 1) and VAD (Stage 3)
affects: [18-cleanup]

tech-stack:
  added: []
  patterns: [shared WAV between pipeline stages, optional wav_path parameter]

key-files:
  created: []
  modified:
    - src/commands/caption.rs
    - src/commands/pipeline.rs
    - src/ffmpeg.rs

key-decisions:
  - "transcribe() takes wav_path: Option<&Path> -- None for standalone, Some for pipeline WAV sharing"
  - "WAV extracted before Stage 1 and cleaned up after Stage 3"
  - "run_silencedetect marked #[allow(dead_code)] with DEPRECATED comment"

patterns-established:
  - "Optional pre-extracted resource pattern: if Some use provided, if None create own"

requirements-completed: [VAD-02, VAD-03]

duration: 3min
completed: 2026-02-24
---

# Phase 17 Plan 02: Pipeline VAD Integration + WAV Sharing

**Wired VAD into pipeline Stage 3, updated transcribe() to accept optional WAV path, pipeline extracts WAV once for both Whisper and VAD**

## Performance

- **Duration:** 3 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Updated `caption::transcribe()` with `wav_path: Option<&Path>` parameter; when `Some`, skips audio extraction and uses provided WAV
- Pipeline extracts shared 16kHz WAV before Stage 1, passes to both `transcribe()` and `vad::run_vad()`
- Replaced entire silencedetect block in pipeline.rs with VAD: no more amplitude threshold, word filtering, or silence_to_speech conversion
- Old silencedetect code in pipeline.rs commented out with `// DEPRECATED: Phase 18 removes` markers
- `run_silencedetect()` in ffmpeg.rs marked with `#[allow(dead_code)]` and DEPRECATED comment
- Dry-run output updated: "VAD-based silence removal" instead of "silencedetect + word-protected cut"
- `breaths` parameter renamed to `_breaths` (silently ignored)

## Task Commits

1. **Task 1+2: caption.rs + pipeline.rs** - `51aaf3c` (feat)

## Deviations from Plan

None.

---

*Phase: 17-core-vad-integration*
*Completed: 2026-02-24*
