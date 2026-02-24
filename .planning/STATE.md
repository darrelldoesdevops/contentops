# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-24)

**Core value:** Take a raw video file and remove dead air automatically
**Current focus:** v1.4 Silero VAD -- Phase 16: Build & CI Verification

## Current Position

Milestone: v1.4 Silero VAD
Phase: 16 of 18 (Build & CI Verification)
Plan: 1 of 1 in current phase
Status: Phase complete
Last activity: 2026-02-24 -- Phase 16 complete (voice_activity_detector + 4-platform CI with ORT cache)

Progress: [█░░░░░░░░░] 11% (v1.4)

## Performance Metrics

**Velocity:**
- Total plans completed: 19 (v1.0: 8, v1.1: 3 formal + 3 quick, v1.2: 3, v1.3: 3)
- Total execution time: 5 days
- Milestones shipped: 4 (v1.0, v1.1, v1.2, v1.3)

**By Milestone:**

| Milestone | Phases | Plans | Duration |
|-----------|--------|-------|----------|
| v1.0 MVP | 5 | 8 | 2 days |
| v1.1 Polish & Pipeline | 4 | 3+3 quick | 1 day |
| v1.2 Distribution & Docs | 3 | 3 | 5 min |
| v1.3 Cross-Platform | 3 | 3 | 2 days |

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

### Pending Todos

None.

### Blockers/Concerns

- Phase 17: voice_activity_detector does not expose get_speech_timestamps(); must implement chunk accumulation loop -- verify exact iterator API from docs.rs before coding
- Phase 17: Audio format must be 16kHz mono f32le before VAD or inference returns garbage -- assert format at VAD boundary
- Phase 16 follow-up: Windows CI ORT cache with ~/.ort-cache should work (GHA expands ~ on Windows) but verify on first cold runner run

## Session Continuity

Last session: 2026-02-24
Stopped at: Phase 16 complete -- ready for Phase 17 (VAD Integration)
Resume file: None
