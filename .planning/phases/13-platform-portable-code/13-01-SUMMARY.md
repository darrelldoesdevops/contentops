---
phase: 13-platform-portable-code
plan: 01
subsystem: infra
tags: [cross-platform, cfg, ffmpeg, font-resolution]

requires:
  - phase: 12-comprehensive-readme
    provides: completed macOS-only codebase ready for cross-platform porting
provides:
  - Platform-conditional default font resolution (macOS/Windows/Linux)
  - Cross-platform null muxer for ffmpeg normalize pass
  - Platform-aware install hints in error messages
affects: [14-linux-windows-ci-release, 15-cross-platform-docs-install]

tech-stack:
  added: []
  patterns: [cfg-gated platform constants, runtime font probing on Linux]

key-files:
  created: []
  modified:
    - src/commands/overlay.rs
    - src/commands/normalize.rs
    - src/error.rs

key-decisions:
  - "Use #[cfg(target_os)] compile-time branching for font constants on macOS/Windows, runtime probe on Linux"
  - "Use -f null - instead of /dev/null for cross-platform ffmpeg null muxer"
  - "whisper-cli non-macOS hint links to build-from-source since no canonical apt/choco package exists"

patterns-established:
  - "Platform branching: #[cfg(target_os)] for compile-time, cfg!() macro for runtime"
  - "Linux font resolution: ordered candidate list with exists() probe and fallback"

requirements-completed: [XPLAT-01, XPLAT-02, XPLAT-03]

duration: 3min
completed: 2026-02-21
---

# Phase 13: Platform-Portable Code Summary

**Platform-conditional font paths, cross-platform null muxer, and OS-aware install hints replacing three macOS-only assumptions**

## Performance

- **Duration:** 3 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Default font resolves per-platform: macOS Impact.ttf, Windows impact.ttf, Linux runtime probe across 6 candidate paths
- Normalize command uses `-f null -` instead of `/dev/null` for cross-platform ffmpeg compatibility
- Error hints show brew on macOS, choco on Windows, apt on Linux for ffmpeg; build-from-source link for whisper on non-macOS

## Task Commits

1. **Task 1: Platform-conditional font path and cross-platform null muxer** - `78fbb31` (feat)
2. **Task 2: Platform-aware error hints for ffmpeg and whisper** - `024a05d` (feat)

## Files Created/Modified
- `src/commands/overlay.rs` - Platform-conditional DEFAULT_FONT with Linux runtime probe via resolve_default_font()
- `src/commands/normalize.rs` - Cross-platform null muxer (- instead of /dev/null)
- `src/error.rs` - ffmpeg_install_hint() and whisper_install_hint() helper functions with cfg!() branching

## Decisions Made
- Used compile-time `#[cfg(target_os)]` for macOS/Windows font constants, runtime `cfg!()` for error hints
- Linux font probe checks 6 candidate paths (msttcorefonts, Liberation, DejaVu) and falls back to first candidate
- whisper-cli uses build-from-source link on non-macOS since no canonical package manager distribution exists

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
None

## Next Phase Readiness
- Code compiles on macOS, ready for cross-compilation CI in Phase 14
- No blockers

---
*Phase: 13-platform-portable-code*
*Completed: 2026-02-21*
