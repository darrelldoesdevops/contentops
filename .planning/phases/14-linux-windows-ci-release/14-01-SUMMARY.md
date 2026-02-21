---
phase: 14-linux-windows-ci-release
plan: 01
subsystem: infra
tags: [github-actions, ci-cd, cross-platform, release]

requires:
  - phase: 13-platform-portable-code
    provides: Platform-portable Rust code that compiles on Linux and Windows
provides:
  - Linux x86_64 binary in GitHub Releases
  - Windows x86_64 binary in GitHub Releases
  - Three-platform CI test matrix
affects: [15-cross-platform-docs-install]

tech-stack:
  added: []
  patterns: [per-architecture build jobs, matrix CI strategy]

key-files:
  created: []
  modified:
    - .github/workflows/release.yml
    - .github/workflows/ci.yml

key-decisions:
  - "Kept per-architecture job pattern for release builds instead of matrix strategy -- less risk to working macOS flow"
  - "Used matrix strategy for CI since all platforms run identical steps"
  - "Release job stays on macos-latest for lipo universal binary creation"

patterns-established:
  - "Release: separate build job per platform, single release job collects all artifacts"
  - "CI: matrix strategy with os array for uniform test coverage"

requirements-completed: [XPLAT-04, XPLAT-05]

duration: 2min
completed: 2026-02-21
---

# Phase 14: Linux & Windows CI/Release Summary

**Linux and Windows build jobs in release.yml with SHA256 checksums, plus three-platform CI matrix**

## Performance

- **Duration:** 2 min
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- release.yml produces Linux (x86_64-unknown-linux-gnu) and Windows (x86_64-pc-windows-msvc.exe) binaries alongside existing macOS builds
- All binaries get SHA256 checksums, uploaded to GitHub Releases
- ci.yml runs fmt, clippy, test, and audit on macOS, Ubuntu, and Windows via matrix strategy

## Task Commits

1. **Task 1: Add Linux and Windows build jobs to release.yml** - `16a6970` (feat)
2. **Task 2: Expand CI to three-platform matrix** - `45af1db` (feat)

## Files Created/Modified
- `.github/workflows/release.yml` - Added build-linux and build-windows jobs, expanded release needs/downloads/checksums/files
- `.github/workflows/ci.yml` - Refactored to matrix strategy with 3 OS targets

## Decisions Made
- Kept separate build jobs for release (matching existing macOS pattern) rather than matrix to minimize risk
- Used matrix strategy for CI since all platforms run identical check steps
- Release job runs on macos-latest (required for lipo universal binary)

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
None

## Next Phase Readiness
- Binaries will be available on next tag push for Linux and Windows
- Phase 15 can reference download URLs for README install instructions

---
*Phase: 14-linux-windows-ci-release*
*Completed: 2026-02-21*
