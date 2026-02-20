---
status: testing
phase: 09-ci-cd
source: [09-VERIFICATION.md, .github/workflows/ci.yml, .github/workflows/release.yml]
started: 2026-02-20T18:30:00Z
updated: 2026-02-20T18:30:00Z
---

## Current Test

number: 1
name: CI workflow triggers on push/PR
expected: |
  `.github/workflows/ci.yml` triggers on push to main AND pull_request to main.
awaiting: user response

## Tests

### 1. CI workflow triggers on push/PR
expected: `.github/workflows/ci.yml` triggers on push to main AND pull_request to main.
result: [pending]

### 2. CI runs all four quality gates
expected: CI job runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `cargo audit` as separate steps.
result: [pending]

### 3. Release triggers on version tag
expected: `.github/workflows/release.yml` triggers on tags matching `v*` (e.g., `v1.1.0`).
result: [pending]

### 4. Release builds both architectures
expected: Release has separate jobs for ARM64 (`macos-latest`, `aarch64-apple-darwin`) and Intel (`macos-13`, `x86_64-apple-darwin`).
result: [pending]

### 5. Universal binary created
expected: Release job combines both binaries with `lipo -create` to produce `contentops-universal-apple-darwin`.
result: [pending]

### 6. SHA256 checksums for all artifacts
expected: Release generates `.sha256` checksum files for all three binaries (aarch64, x86_64, universal).
result: [pending]

### 7. Release artifacts uploaded to GitHub Release
expected: `softprops/action-gh-release` attaches all 6 files (3 binaries + 3 checksums) to the GitHub Release.
result: [pending]

## Summary

total: 7
passed: 0
issues: 0
pending: 7
skipped: 0

## Gaps

[none yet]
