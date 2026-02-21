# Phase 15: Cross-Platform Docs & Install - Context

**Gathered:** 2026-02-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Update README with Linux and Windows install paths, platform-specific prerequisites, and ensure doctor subcommand hints are platform-aware. No code changes -- documentation only.

</domain>

<decisions>
## Implementation Decisions

### Install paths
- Linux: curl one-liner downloading binary from GitHub Releases, chmod +x, move to /usr/local/bin
- Windows: PowerShell Invoke-WebRequest one-liner downloading .exe from GitHub Releases, move to user PATH
- Keep existing macOS Homebrew + curl one-liners unchanged

### Prerequisites section
- Show platform-specific install commands side by side (brew, apt, choco)
- ffmpeg: brew install ffmpeg / apt install ffmpeg / choco install ffmpeg
- whisper-cli: brew install whisper-cli / build from source link (no apt/choco package)

### Doctor hints
- Already platform-aware from Phase 13 error.rs changes
- Just document that doctor output varies by platform

### Claude's Discretion
- Exact README section ordering and formatting
- Whether to use tabs or platform headers for multi-platform commands

</decisions>

<specifics>
## Specific Ideas

No specific requirements -- open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 15-cross-platform-docs-install*
*Context gathered: 2026-02-21*
