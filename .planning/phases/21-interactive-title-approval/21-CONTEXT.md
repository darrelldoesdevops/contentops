# Phase 21: Interactive Title Approval - Context

**Gathered:** 2026-02-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Present Claude-generated title options to the user before burning the overlay. User selects and optionally edits a title during pipeline execution. Non-TTY environments auto-select the first option. This phase modifies `overlay.rs` (title generation) and `pipeline.rs` (approval flow), not caption.rs.

</domain>

<decisions>
## Implementation Decisions

### Title generation flow
- `generate_title()` in overlay.rs currently returns a single title string from Claude haiku
- Change prompt to request 3 title options (numbered, newline-separated blocks)
- Parse Claude response into `Vec<String>` of 3 options
- If parsing fails or returns <2 options, fall back to treating entire response as single option
- Claude model stays as haiku (fast, cheap, sufficient for titles)

### Interactive selection
- Use `dialoguer::Select` (same library as Phase 20 mismatch handling)
- Display 3 options with arrow-key selection + "Custom..." as 4th choice
- If user picks "Custom...", use `dialoguer::Input` for freeform title entry
- Spinner must `finish_and_clear()` before prompt renders (known indicatif + dialoguer friction from STATE.md blocker note)
- After selection, print chosen title with checkmark prefix (match existing CLI style)

### Non-TTY behavior
- Check `std::io::stdin().is_terminal()` (same pattern as Phase 20)
- Non-TTY: auto-select first option, print info message, continue without prompting
- No error/abort in non-TTY -- title approval is not safety-critical like transcript mismatch

### Pipeline integration
- Title approval happens during Stage 7 (overlay) in pipeline.rs
- Current flow: `overlay::run()` calls `generate_title()` internally
- New flow: extract title approval out of overlay::run() so pipeline can control the interactive step
- Pipeline passes the approved title as `text` to overlay::run() (existing --text path), skipping generate_title
- Standalone `contentops overlay --auto` also gets the interactive flow (not just pipeline)

### CLI flag
- Add `--no-interactive` flag to PipelineArgs (already decided in STATE.md)
- When set, behaves like non-TTY: auto-selects first title, no prompts
- Maps conceptually to IsTerminal check but is an explicit override
- Also add to OverlayArgs for standalone usage

### Claude's Discretion
- Exact prompt wording for generating 3 options
- Response parsing strategy (numbered list, delimiter-separated, etc.)
- How to display options in the terminal (numbered list, quoted, etc.)
- Whether to show a preview of line wrapping for each option

</decisions>

<specifics>
## Specific Ideas

- Title approval follows the same dialoguer pattern proven in Phase 20 (transcript mismatch handling)
- The spinner-to-prompt sequencing concern from STATE.md blockers must be tested: `finish_and_clear()` + `Select::new().interact()` in a real terminal
- Keep the "Custom..." option lightweight -- single-line input, not a full editor

</specifics>

<deferred>
## Deferred Ideas

- TikTok description generation and sidecar file -- Phase 22
- Title style customization (font, color, position from prompt) -- out of scope, use existing overlay flags

</deferred>

---

*Phase: 21-interactive-title-approval*
*Context gathered: 2026-02-25*
