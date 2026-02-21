# Phase 12: Comprehensive README - Context

**Gathered:** 2026-02-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Write `README.md` from live `--help` output: prerequisites, Homebrew install, pipeline-first usage, flag reference tables, troubleshooting. The README is the single source a user needs to understand, install, and run contentops.

</domain>

<decisions>
## Implementation Decisions

### Document structure and flow
- Lead with a one-liner description and a complete `contentops pipeline` example (copy-paste ready)
- Structure: hero example -> prerequisites -> install -> usage (pipeline first, then individual subcommands) -> flag reference tables -> troubleshooting
- Keep it scannable: tables and code blocks over prose paragraphs
- Target ~50 lines for the "get started" path (everything before flag reference tables)

### Tone and depth
- Terse, practical, developer-facing -- assume the reader knows the terminal
- No marketing language or feature comparison tables
- Every section has actionable content: commands to run, flags to use, errors to fix
- Brief "why" context only where it prevents confusion (e.g., why whisper-cli not whisper.cpp)

### Code examples
- Every subcommand gets one copy-paste example showing the most common invocation
- Pipeline example is the hero -- show the full input-to-output workflow
- Show actual output filenames so users know what to expect
- No explanatory prose between flags in the reference table -- the flag name and description are enough

### Troubleshooting scope
- Map common failure messages from `contentops doctor` output to resolution steps
- Map error hints already built into the CLI to resolution commands
- Cover: missing FFmpeg, missing whisper-cli, missing Claude CLI (for overlay), file not found, codec errors
- Keep each troubleshooting entry to 2-3 lines: error message -> cause -> fix command

### Claude's Discretion
- Exact Markdown formatting and heading hierarchy
- Whether to include badges (build status, version)
- Section dividers and visual spacing
- Whether to include a TOC for navigation

</decisions>

<specifics>
## Specific Ideas

- Flag reference tables must match `contentops <subcommand> --help` output exactly -- no undocumented or phantom flags
- Include both `brew install darrelldoesdevops/tap/contentops` and direct-download install paths
- Prerequisites: FFmpeg, whisper-cli, Claude CLI (optional)
- The README should be generated from live `--help` output to prevent flag drift

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 12-comprehensive-readme*
*Context gathered: 2026-02-21*
