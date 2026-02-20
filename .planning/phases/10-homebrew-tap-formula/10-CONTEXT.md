# Phase 10: Homebrew Tap + Formula - Context

**Gathered:** 2026-02-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Create `darrelldoesdevops/homebrew-tap` repo with an architecture-conditional formula so users can install contentops via `brew install darrelldoesdevops/tap/contentops` on ARM and Intel Macs. Auto-update wiring is Phase 11; README documentation is Phase 12.

</domain>

<decisions>
## Implementation Decisions

### Dependency handling
- FFmpeg declared as `depends_on "ffmpeg"` — auto-installs with the formula
- whisper-cli handled via caveats only (not in Homebrew, can't be a dependency)
- Add `depends_on :macos` guard to prevent Linuxbrew install attempts
- Claude CLI mentioned in caveats as optional for AI-powered caption generation

### Post-install caveats
- Moderate verbosity — actionable but not a full guide
- Standard Homebrew caveats formatting (plain text, indented)
- Content: list prerequisites with install hints, mention whisper model is needed (without specifying which model), note Claude CLI as optional
- Include `contentops doctor` hint at the end so users can self-diagnose
- Don't include specific model recommendations or download commands

### Tap repo presentation
- Repo: `darrelldoesdevops/homebrew-tap` — generic, reusable for future tools
- No README — formula only
- No LICENSE file
- GitHub repo description: "Homebrew tap" (short, generic)

### Sentinel comment design
- Visually obvious markers that stand out in the formula (e.g., `# === AUTO-UPDATE: VERSION ===`)
- Include a brief 2-3 line header comment explaining sentinel system for future maintainers
- Style is Claude's discretion — pick whatever works best for sed-based patching (inline vs block)
- Which values to mark is Claude's discretion — determine what needs sentinels for reliable auto-update

### Claude's Discretion
- Sentinel comment style (inline vs block markers) — pick what's most sed-friendly
- Which specific values get sentinel markers (version, SHAs, URLs)
- Formula test block implementation details
- brew audit compliance specifics

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard Homebrew formula approaches. Follow Homebrew conventions for formula structure and naming.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-homebrew-tap-formula*
*Context gathered: 2026-02-20*
