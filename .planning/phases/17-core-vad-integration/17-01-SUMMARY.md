---
phase: 17-core-vad-integration
plan: 01
subsystem: audio-processing
tags: [rust, vad, silero, onnx, silence-detection]

requires: [16-01]
provides:
  - src/vad.rs module with run_vad() returning Vec<SpeechInterval>
  - ffmpeg::extract_16k_wav() shared helper
  - cut command using VAD instead of silencedetect
affects: [17-02]

tech-stack:
  added: [hound 3.5.1]
  patterns: [LabelIterator for VAD chunk labeling, shared WAV extraction helper]

key-files:
  created:
    - src/vad.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/ffmpeg.rs
    - src/main.rs
    - src/commands/cut.rs
    - src/silence.rs

key-decisions:
  - "VAD threshold 0.5, padding_chunks 0 for aggressive cutting"
  - "Created src/vad.rs as separate module (not inline in ffmpeg.rs)"
  - "Used hound for WAV reading (i16 samples)"

patterns-established:
  - "extract_16k_wav() in ffmpeg.rs as single entry point for 16kHz mono WAV extraction"
  - "total_silence_from_speeches() computes silence from speech intervals directly"

requirements-completed: [VAD-01, VAD-03]

duration: 3min
completed: 2026-02-24
---

# Phase 17 Plan 01: Core VAD Module + Cut Command

**Created vad.rs with Silero VAD inference via LabelIterator, added extract_16k_wav helper to ffmpeg.rs, replaced silencedetect with VAD in cut command**

## Performance

- **Duration:** 3 min
- **Tasks:** 2
- **Files modified:** 7 (1 created)

## Accomplishments
- Created `src/vad.rs` with `run_vad()` function using `LabelIterator` from `voice_activity_detector` crate to detect speech segments
- Added `extract_16k_wav()` to `ffmpeg.rs` as shared helper for 16kHz mono WAV extraction
- Replaced silencedetect with VAD in `cut.rs`: extract WAV, run VAD, build concat filter from speech intervals directly
- Added `total_silence_from_speeches()` helper to `silence.rs`
- Added `hound = "3.5"` dependency for WAV file reading
- Old silencedetect code in cut.rs commented out with `// DEPRECATED: Phase 18 removes` markers

## Task Commits

1. **Task 1+2: VAD module + cut command** - `c5f392b` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy io_other_error lint**
- **Found during:** clippy verification
- **Issue:** `std::io::Error::new(ErrorKind::Other, ...)` should use `std::io::Error::other()`
- **Fix:** Used `std::io::Error::other()` in extract_16k_wav
- **Verification:** `cargo clippy -- -D warnings` exits 0

---

*Phase: 17-core-vad-integration*
*Completed: 2026-02-24*
