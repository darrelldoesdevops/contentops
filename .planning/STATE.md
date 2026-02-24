# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-24)

**Core value:** Take a raw video file and remove dead air automatically
**Current focus:** v1.4 Silero VAD -- Phase 18: Cleanup & Doctor Updates

## Current Position

Milestone: v1.4 Silero VAD
Phase: 17 of 18 (Core VAD Integration) -- COMPLETE
Plan: 2 of 2 in Phase 17
Status: Phase 17 complete, ready for Phase 18
Last activity: 2026-02-24 -- Phase 17 complete (both plans executed)

Progress: [██████░░░░] 60% (v1.4)

## Performance Metrics

**Velocity:**
- Total plans completed: 21 (v1.0: 8, v1.1: 3 formal + 3 quick, v1.2: 3, v1.3: 3, v1.4: 2)
- Total execution time: 5 days
- Milestones shipped: 4 (v1.0, v1.1, v1.2, v1.3)

**By Milestone:**

| Milestone | Phases | Plans | Duration |
|-----------|--------|-------|----------|
| v1.0 MVP | 5 | 8 | 2 days |
| v1.1 Polish & Pipeline | 4 | 3+3 quick | 1 day |
| v1.2 Distribution & Docs | 3 | 3 | 5 min |
| v1.3 Cross-Platform | 3 | 3 | 2 days |
| v1.4 Silero VAD | 2/3 | 4 | in progress |

## Accumulated Context

### Decisions

All decisions logged in PROJECT.md Key Decisions table.

Recent v1.4 decisions:
- Use voice_activity_detector 0.2.1 (bundles Silero V5 ONNX, pins ort =2.0.0-rc.10 -- never upgrade ort independently)
- Bundle ONNX model in binary (zero user setup; 1.8MB increase acceptable)
- Remove --breaths flag (VAD detects all non-speech inherently)
- ORT_CACHE_DIR: ~/.ort-cache normalizes ORT binary cache path across all platforms (Linux/macOS/Windows)
- Cross-compile x86_64-apple-darwin on ARM64 macos-latest runner; skip tests for that target (exec format error)
- Security audit conditioned to aarch64-apple-darwin only to avoid 4x redundant cargo install per CI run
- VAD threshold 0.5, padding_chunks 0 for aggressive cutting
- Shared extract_16k_wav helper in ffmpeg.rs used by cut, pipeline, and caption
- transcribe() accepts wav_path: Option<&Path> for pipeline WAV sharing

### Pending Todos

None.

### Blockers/Concerns

- Windows CI ORT cache with ~/.ort-cache should work (GHA expands ~ on Windows) but verify on first cold runner run

## Session Continuity

Last session: 2026-02-24
Stopped at: Phase 17 complete, ready for Phase 18 (Cleanup & Doctor Updates)
Resume file: None
