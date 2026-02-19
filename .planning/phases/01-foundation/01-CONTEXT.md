# Phase 1: Foundation - Context

**Gathered:** 2026-02-19
**Status:** Ready for planning

<domain>
## Phase Boundary

CLI skeleton, FFmpeg subprocess wrapper, error handling, and temp file lifecycle. User can invoke the CLI, it validates FFmpeg is available, and the subprocess/error/temp-file infrastructure is solid enough to build every pipeline stage on top of.

</domain>

<decisions>
## Implementation Decisions

### CLI structure
- Subcommand-per-function pattern (not generic `process`): `contentops cut input.mp4`, `contentops caption input.mp4`, etc.
- Verb-based subcommand names: `cut` for silence removal (Phase 2), `caption` for captioning, `overlay` for overlays
- Phase 1 implements the `cut` subcommand as the first FFmpeg-exercising command
- Bare `contentops` (no subcommand) shows help with available subcommands and one-line descriptions
- clap derive macros for argument parsing
- Default output: same directory as input, subcommand-specific suffix (`input_cut.mp4`, `input_captioned.mp4`, `input_overlay.mp4`)
- `-o` flag overrides output path
- Overwrite existing output files silently (matches FFmpeg `-y` behavior)

### Error presentation
- Colored + structured errors (like rustc/cargo): red `error:` prefix, bold stage name, indented FFmpeg stderr
- On FFmpeg failure: show last 10-20 lines of stderr. Full log saved to file.
- FFmpeg not found: actionable error with install hint (`brew install ffmpeg`)
- Every error always identifies which pipeline stage failed (e.g., "error in stage 'audio extraction': ...")

### FFmpeg output handling
- Spinner + status line during processing ("Processing input.mp4...") using indicatif crate
- Phase 5 upgrades spinner to real progress bar (PIPE-05)
- `--verbose` flag available from Phase 1: streams raw FFmpeg stderr in real-time
- Success message shows output path + file size: "✓ Created input_cut.mp4 (12.3 MB)"

### Temp file behavior
- Temp files created next to input file (same directory), dot-prefixed
- Naming pattern: `.contentops_tmp_<random>.ext`
- Cleaned up after both successful and failed runs
- Signal handling for Ctrl+C cleanup: Claude's discretion on implementation approach for Phase 1

### Claude's Discretion
- Signal handling strategy for Ctrl+C cleanup (best-effort vs full SIGINT handler)
- Exact spinner style from indicatif
- Internal module organization and error type hierarchy
- Color crate choice (anyhow, miette, or manual colored output)

</decisions>

<specifics>
## Specific Ideas

- CLI should feel like DevOps tools (kubectl, docker, terraform) — subcommands map to specific functions
- Each subcommand gets its own output suffix so you can tell what was done to a file at a glance

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-02-19*
