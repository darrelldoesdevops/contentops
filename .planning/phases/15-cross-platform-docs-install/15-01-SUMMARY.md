---
phase: 15-cross-platform-docs-install
plan: 01
subsystem: docs
tags: [readme, cross-platform, install, documentation]

requires:
  - phase: 13-platform-portable-code
    provides: Platform-aware error hints and doctor output
  - phase: 14-linux-windows-ci-release
    provides: Linux and Windows binaries in GitHub Releases
provides:
  - Cross-platform README with Linux/Windows install paths
  - Platform-specific prerequisite documentation
affects: []

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - README.md

key-decisions:
  - "Prerequisites table expanded to 4 columns (macOS, Linux, Windows) instead of single Install column"
  - "whisper-cli links to build-from-source on Linux/Windows since no canonical package exists"

patterns-established: []

requirements-completed: [XPLAT-06]

duration: 1min
completed: 2026-02-21
---

# Phase 15: Cross-Platform Docs & Install Summary

**README updated with Linux curl one-liner, Windows PowerShell one-liner, and three-platform prerequisite table**

## Performance

- **Duration:** 1 min
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Prerequisites table shows macOS/Linux/Windows install commands side by side
- Install section includes Linux x86_64 curl one-liner and Windows PowerShell Invoke-WebRequest one-liner
- Troubleshooting table shows platform-specific fix commands

## Task Commits

1. **Task 1: Add cross-platform install and prerequisites to README** - `05037cb` (feat)

## Files Created/Modified
- `README.md` - Added Linux/Windows install paths, expanded prerequisites table, updated troubleshooting fixes

## Decisions Made
- Expanded prerequisites to 4-column table (Tool, macOS, Linux, Windows) for clarity
- whisper-cli uses build-from-source link on Linux/Windows

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
None

## Next Phase Readiness
- v1.3 milestone complete -- all cross-platform work done

---
*Phase: 15-cross-platform-docs-install*
*Completed: 2026-02-21*
