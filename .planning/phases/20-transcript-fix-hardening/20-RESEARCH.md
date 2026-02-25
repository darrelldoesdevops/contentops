# Phase 20: Transcript Fix Hardening - Research

**Researched:** 2026-02-25
**Domain:** Defensive validation in LLM-powered transcription correction
**Confidence:** HIGH

## Summary

Phase 20 hardens `fix_transcription()` in `src/commands/caption.rs` against word count drift when Claude rewrites words. The current code (lines 358-367) silently falls back to original words on mismatch with a single-line warning. The phase replaces this with an interactive prompt (use originals / use fixed / retry) and a hard fail in non-TTY environments.

The scope is narrow: one function (`fix_transcription`), one new dependency (`dialoguer` 0.12), and one new error variant. No new caption features, no changes to the pipeline flow beyond the fix stage.

**Primary recommendation:** Add `dialoguer` 0.12 (already decided by user), gate prompts behind `std::io::IsTerminal`, implement 1-retry policy with three-way comparison on retry failure.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Show diff context on mismatch: count plus first few changed words
- Same detail regardless of --verbose flag
- On mismatch: pause pipeline with interactive prompt (use originals / use fixed / retry)
- Non-TTY: fail pipeline with error code, never silently continue
- Maximum 1 retry when user picks "Retry"
- If retry also mismatches: show all three versions (original, first fix, retry fix) and let user pick any
- After retry exhausted: offer use originals, use first fix, use retry fix, or abort
- Interactive prompt follows dialoguer pattern (same as Phase 21)

### Claude's Discretion
- Warning destination (stderr only vs stderr + log file)
- Warning visual style (match existing CLI patterns)
- Full fallback vs partial alignment strategy
- Retry prompt enhancement strategy

### Deferred Ideas (OUT OF SCOPE)
None
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| META-03 | Transcript fix prompt enforces exact word count to prevent timing corruption | Word count validation already exists at line 358; needs interactive handling instead of silent fallback |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| dialoguer | 0.12 | Interactive terminal prompts (Select) | Shares `console` crate with indicatif (already in deps), no terminal conflict |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::io::IsTerminal | stable (1.70+) | TTY detection for non-interactive guard | Gate all stdin reads |
| console (transitive) | via dialoguer+indicatif | Terminal abstraction | Already present via indicatif |

## Architecture Patterns

### Current fix_transcription Flow (caption.rs:280-388)
```
1. Serialize words to JSON
2. Send to Claude CLI (haiku model)
3. Parse response
4. Check word count match (line 358)
5. If mismatch → eprintln warning → return Ok(()) (CURRENT: silent fallback)
6. If match → apply corrections
```

### Target Flow
```
1. Serialize words to JSON
2. Send to Claude CLI (haiku model)
3. Parse response
4. Check word count match
5. If mismatch:
   a. Print diff context (count + first changed words)
   b. If TTY → interactive prompt (use originals / use fixed / retry)
   c. If not TTY → return error (hard fail)
   d. If "retry" → re-invoke Claude with enhanced prompt
   e. If retry also mismatches → show 3 versions → user picks
6. If match → apply corrections
```

### Spinner Lifecycle
The existing code already manages spinner correctly: `finish_and_clear()` before any user-facing output. Must follow same pattern: clear spinner BEFORE showing mismatch prompt.

### Pattern: dialoguer + indicatif Coexistence
```rust
// 1. Finish spinner BEFORE prompt
if let Some(pb) = spinner {
    pb.finish_and_clear();
}

// 2. Now safe to use dialoguer
let selection = dialoguer::Select::new()
    .with_prompt("Word count mismatch")
    .items(&["Use originals", "Use fixed", "Retry"])
    .default(0)
    .interact()?;
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal prompts | Custom stdin reading | dialoguer::Select | Handles raw mode, arrow keys, rendering |
| TTY detection | Manual fd checks | std::io::IsTerminal | Standard library, cross-platform |

## Common Pitfalls

### Pitfall 1: Spinner Still Ticking During Prompt
**What goes wrong:** dialoguer prompt renders while indicatif spinner is still active, corrupting terminal output
**Why it happens:** Forgetting to `finish_and_clear()` before any interactive prompt
**How to avoid:** Always clear spinner before dialoguer calls; existing code already follows this pattern
**Warning signs:** Garbled terminal output, prompt text overwritten by spinner

### Pitfall 2: Non-TTY Hang
**What goes wrong:** Pipeline hangs in CI/scripts waiting for stdin that never comes
**Why it happens:** dialoguer::Select blocks on stdin without TTY check
**How to avoid:** Gate with `std::io::IsTerminal::is_terminal(&std::io::stdin())` BEFORE any dialoguer call
**Warning signs:** `contentops pipeline` hangs in GitHub Actions

### Pitfall 3: Retry Reusing Same Prompt
**What goes wrong:** Retry produces same word count mismatch because Claude receives identical prompt
**Why it happens:** No instruction to preserve word count in retry prompt
**How to avoid:** Enhanced retry prompt explicitly states "You MUST return exactly N words"
**Warning signs:** Both attempts fail identically

## Code Examples

### TTY Guard Pattern
```rust
use std::io::IsTerminal;

if std::io::stdin().is_terminal() {
    // Interactive: show dialoguer prompt
} else {
    // Non-TTY: hard fail
    return Err(AppError::TranscriptMismatch { ... }.into());
}
```

### Diff Context Display
```rust
fn print_mismatch_context(original: &[Word], corrected: &[Word]) {
    eprintln!(
        "Warning: LLM returned {} words, expected {}",
        corrected.len(),
        original.len()
    );
    // Show first few differences
    let diffs: Vec<_> = original.iter().zip(corrected.iter())
        .filter(|(o, c)| o.word != c.word)
        .take(5)
        .collect();
    for (orig, fixed) in &diffs {
        eprintln!("  \"{}\" -> \"{}\"", orig.word, fixed.word);
    }
    if original.len() != corrected.len() {
        let min_len = original.len().min(corrected.len());
        eprintln!(
            "  ({} words {})",
            original.len().abs_diff(corrected.len()),
            if corrected.len() > original.len() { "added" } else { "removed" }
        );
    }
}
```

### Three-Way Comparison (After Retry Failure)
```rust
eprintln!("Original:    {}", original_words.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" "));
eprintln!("First fix:   {}", first_fix.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" "));
eprintln!("Retry fix:   {}", retry_fix.iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" "));
```

## Open Questions

None -- scope is well-constrained by CONTEXT.md decisions.

## Sources

### Primary (HIGH confidence)
- Codebase: `src/commands/caption.rs` lines 280-388 (fix_transcription implementation)
- Codebase: `src/commands/pipeline.rs` lines 170-171 (fix stage in pipeline)
- Codebase: `Cargo.toml` (current dependencies)
- Codebase: `src/ui.rs` (spinner patterns)
- Rust std docs: `std::io::IsTerminal` (stable since 1.70)

### Secondary (MEDIUM confidence)
- dialoguer 0.12 API (from prior v1.5 research in `.planning/research/`)
- indicatif + dialoguer coexistence patterns (from `.planning/research/PITFALLS.md`)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - dialoguer already decided, single new dependency
- Architecture: HIGH - modifying one function with clear current behavior
- Pitfalls: HIGH - well-documented spinner/prompt interaction from prior research

**Research date:** 2026-02-25
**Valid until:** 2026-03-25
