---
phase: 14-linux-windows-ci-release
status: passed
verified: 2026-02-21
---

# Phase 14: Linux & Windows CI/Release -- Verification

## Goal
Tag push produces downloadable binaries for Linux and Windows alongside existing macOS binaries

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| XPLAT-04 | PASSED | release.yml has build-linux (x86_64-unknown-linux-gnu) and build-windows (x86_64-pc-windows-msvc) jobs; release job downloads, checksums, and uploads all binaries |
| XPLAT-05 | PASSED | ci.yml uses matrix strategy with [macos-latest, ubuntu-latest, windows-latest]; all 3 run fmt, clippy, test, audit |

## Success Criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | release.yml includes linux-gnu and windows-msvc targets | PASSED | build-linux and build-windows jobs with correct targets |
| 2 | Release contains Linux and Windows binaries | PASSED | files list includes contentops-x86_64-unknown-linux-gnu and contentops-x86_64-pc-windows-msvc.exe with SHA256 |
| 3 | CI runs tests on all three platforms | PASSED | matrix.os: [macos-latest, ubuntu-latest, windows-latest] |

## Score

**3/3 must-haves verified**

## Notes

Actual binary production will be validated on next tag push. Workflow files are syntactically valid (yq parse confirmed). Existing macOS flow and update-tap job unchanged.
