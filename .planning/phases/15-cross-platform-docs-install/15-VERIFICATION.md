---
phase: 15-cross-platform-docs-install
status: passed
verified: 2026-02-21
---

# Phase 15: Cross-Platform Docs & Install -- Verification

## Goal
README covers Linux and Windows users with install instructions and platform-specific prerequisites

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| XPLAT-06 | PASSED | README has Linux curl one-liner, Windows PowerShell one-liner, three-platform prerequisites table, platform-specific troubleshooting fixes |

## Success Criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | README includes Linux and Windows install paths | PASSED | curl one-liner for Linux, Invoke-WebRequest for Windows |
| 2 | Prerequisites shows platform-specific commands | PASSED | 4-column table: macOS/Linux/Windows |
| 3 | Doctor hints are platform-aware | PASSED | Implemented in Phase 13 error.rs; README notes doctor for verification |

## Score

**3/3 must-haves verified**
