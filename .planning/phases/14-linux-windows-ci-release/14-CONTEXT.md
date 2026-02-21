# Phase 14: Linux & Windows CI/Release - Context

**Gathered:** 2026-02-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Add Linux and Windows targets to the existing release.yml build matrix and CI pipeline. Tag push produces downloadable Linux and Windows binaries alongside existing macOS binaries. CI runs tests on all three platforms.

</domain>

<decisions>
## Implementation Decisions

### Build matrix targets
- Add `x86_64-unknown-linux-gnu` (ubuntu-latest runner)
- Add `x86_64-pc-windows-msvc` (windows-latest runner)
- Keep existing macOS ARM64, x86_64, and universal builds unchanged
- x86_64 only for Linux/Windows (no ARM Linux or ARM Windows)

### Binary packaging
- Linux: raw binary named `contentops-x86_64-unknown-linux-gnu` (matches existing macOS naming convention)
- Windows: raw `.exe` named `contentops-x86_64-pc-windows-msvc.exe`
- SHA256 checksums for each, same pattern as macOS
- No tar.gz or zip wrapping -- keep flat binary pattern consistent with macOS

### CI test matrix
- Expand ci.yml to run on ubuntu-latest and windows-latest alongside existing macos-latest
- All three platforms run: fmt check, clippy, test, audit
- Use matrix strategy to avoid duplicating job definitions

### Release job updates
- Add linux and windows build jobs parallel to existing macOS jobs
- Release job `needs:` expands to include new build jobs
- Linux build runs on `ubuntu-latest`
- Windows build runs on `windows-latest`
- Release job stays on `macos-latest` (needs lipo for universal binary)
- Checksum generation uses `sha256sum` on Linux runner or stays on macOS release job

### Claude's Discretion
- Whether to use a matrix strategy in release.yml for the new platform builds or keep separate jobs (matching existing macOS pattern)
- Cross-compilation vs native runner builds
- Exact checksum command differences between platforms

</decisions>

<specifics>
## Specific Ideas

- Keep the existing per-architecture job pattern (build-arm64, build-x86_64) rather than refactoring to matrix -- less risk to working macOS flow
- The update-tap job should remain unchanged (Homebrew is macOS-only)

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 14-linux-windows-ci-release*
*Context gathered: 2026-02-21*
