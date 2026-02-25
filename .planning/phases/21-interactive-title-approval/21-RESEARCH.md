# Phase 21: Interactive Title Approval - Research

**Researched:** 2026-02-25

## Current Title Generation Flow

### overlay.rs: `generate_title()`
- Takes `transcript_path` + `verbose`, returns `Result<String>`
- Reads JSON transcript, joins words into plain text
- Sends single prompt to Claude haiku: "Generate a short, punchy title (3-8 words, max 3 lines)"
- Returns single title string
- Spinner pattern: `ui::make_spinner()` -> `finish_and_clear()` on error, `finish_with_message()` on success

### overlay.rs: `run()`
- Entry point for standalone `contentops overlay` and pipeline Stage 7
- If `args.auto` is Some(transcript_path) -> calls `generate_title()` internally
- If `args.text` is Some(text) -> uses provided text directly (no Claude call)
- Pipeline already uses the `text` path when user passes `--text` flag

### pipeline.rs: `finish_stages()`
- Constructs `OverlayArgs` at Stage 7
- If pipeline `text` is None -> sets `auto: Some(caption_json)` (auto-generate)
- If pipeline `text` is Some -> sets `text: Some(t)`, `auto: None` (skip generation)
- This means pipeline can already bypass title generation by passing text directly

## Key Integration Points

### Where to Insert Title Approval
1. **Option A: Inside overlay.rs** -- Modify `generate_title()` to return options and handle selection
2. **Option B: Extract to pipeline.rs** -- New function `approve_title()` called between Stage 6 and 7, passes approved title via `text` arg

Option B is cleaner because:
- Pipeline controls the interactive flow (consistent with Phase 20 pattern)
- Standalone overlay also needs approval (can call same function)
- Separation of concerns: generation vs approval vs rendering

### Public API Change Needed
- `generate_title()` is currently private (`fn`, not `pub fn`)
- Need: `pub fn generate_title_options()` that returns `Vec<String>` instead of single String
- Or: new `pub fn approve_title()` that handles generation + selection + returns approved String

## dialoguer Patterns from Phase 20

### Phase 20 caption.rs Pattern
```rust
// Spinner cleared BEFORE dialoguer prompt
// (no explicit spinner-clearing code needed here because caption.rs
//  doesn't use spinners during fix_transcription)

let choices = &[
    "Use originals (keep timing safe)",
    "Use fixed (accept word count change)",
    "Retry (re-run with word count constraint)",
];
let selection = dialoguer::Select::new()
    .with_prompt("How to handle mismatch?")
    .items(choices)
    .default(0)
    .interact()?;
```

### Spinner-to-Prompt Sequencing
- STATE.md blocker: "dialoguer + live indicatif spinners has known friction"
- Solution: call `spinner.finish_and_clear()` before any `dialoguer` interaction
- In overlay.rs, the spinner is created inside `generate_title()` and finished before returning
- Title options generation should finish spinner before presenting interactive prompt
- Pattern: generate options (with spinner) -> clear spinner -> show prompt

### dialoguer::Input for Custom Title
```rust
let custom: String = dialoguer::Input::new()
    .with_prompt("Enter custom title")
    .interact_text()?;
```

## Non-TTY Detection

### Existing Pattern (caption.rs)
```rust
use std::io::IsTerminal;
// ...
if !std::io::stdin().is_terminal() {
    return Err(AppError::TranscriptMismatch { ... }.into());
}
```

### For Phase 21
- Non-TTY should NOT error (unlike transcript mismatch)
- Just auto-select first option and continue
- `--no-interactive` flag acts as explicit non-TTY override

## CLI Flag Design

### PipelineArgs Addition
```rust
/// Skip interactive prompts (auto-select defaults)
#[arg(long)]
pub no_interactive: bool,
```

### OverlayArgs Addition
Same flag. Standalone `contentops overlay --auto` should also respect it.

### Effective Check
```rust
let interactive = !args.no_interactive && std::io::stdin().is_terminal();
```

## Prompt Design for 3 Options

### Current Prompt
```
Generate a short, punchy title (3-8 words, max 3 lines) for this talking head video.
The title should be a hook that grabs attention.
Split across 2-3 lines for visual impact (use newlines).
Keep each line to 2-4 words max.
Return ONLY the title text with newlines, nothing else. No quotes, no explanation.
```

### Multi-Option Prompt Strategy
- Ask for 3 options separated by a clear delimiter
- Best delimiter: `---` on its own line (unlikely to appear in titles)
- Fallback: if response doesn't split into 3, treat whole response as 1 option

### Parsing Strategy
```
response.split("---").map(|s| s.trim()).filter(|s| !s.is_empty()).collect()
```

## Files Modified

| File | Changes |
|------|---------|
| `src/cli.rs` | Add `--no-interactive` to PipelineArgs and OverlayArgs |
| `src/commands/overlay.rs` | Refactor `generate_title()` -> `generate_title_options()` returning Vec<String>; add `approve_title()` with dialoguer Select + Input |
| `src/commands/pipeline.rs` | Call `approve_title()` before Stage 7, pass result as `text` |

## Test Strategy

- Unit test: parse 3 options from delimiter-separated string
- Unit test: parse fallback when no delimiters present
- Unit test: empty response handling
- Integration: spinner + dialoguer sequencing requires manual terminal testing (not automatable in CI)

---

## RESEARCH COMPLETE
