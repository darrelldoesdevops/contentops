# Phase 20: Transcript Fix Hardening - Context

**Gathered:** 2026-02-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Guard caption timing against word count corruption when fix_transcription rewrites words. No new caption features -- just defensive validation and user-facing mismatch handling.

</domain>

<decisions>
## Implementation Decisions

### Warning output
- Show diff context: mismatch count plus the first few changed words so user can spot what Claude rewrote
- Same detail regardless of --verbose flag
- Claude's discretion on warning destination (stderr vs file) and visual style

### Fallback behavior
- Claude's discretion on full fallback vs partial alignment approach
- On mismatch: pause pipeline and present interactive prompt with three choices:
  - Use originals (keep original words, timing stays safe)
  - Use fixed (accept Claude's version despite word count change)
  - Retry (re-run the fix_transcription call)
- In non-TTY environments (CI, scripts): fail the pipeline with error code -- never silently continue

### Retry policy
- Maximum 1 retry allowed when user picks "Retry"
- Claude's discretion on whether retry uses same prompt or enhanced prompt with word count constraint
- If retry also mismatches: show all three versions (original, first fix, retry fix) and let user pick any
- After retry exhausted, only offer: use originals, use first fix, use retry fix, or abort

### Claude's Discretion
- Warning destination (stderr only vs stderr + log file)
- Warning visual style (match existing CLI patterns)
- Full fallback vs partial alignment strategy
- Retry prompt enhancement strategy

</decisions>

<specifics>
## Specific Ideas

- User wants to see both attempts when retry fails -- present original, first fix, and retry fix side by side
- Interactive prompt follows the same pattern as Phase 21's title approval (dialoguer-based)
- Non-TTY must fail hard, not silently degrade

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 20-transcript-fix-hardening*
*Context gathered: 2026-02-25*
