# Phase 6: Audit & Cleanup - Research

**Researched:** 2026-02-20
**Domain:** Rust codebase hygiene — clippy, error handling, dead code, refactoring
**Confidence:** HIGH (all findings from direct codebase inspection, no external sources needed)

## Summary

This is a pure refactoring phase with no new features. All findings come from direct code inspection of the live codebase. The work is narrow and well-bounded: clippy already passes with zero warnings, so the phase is entirely about deliberate cleanup that clippy doesn't catch (consistency, duplication, and justified suppressions) plus generating a written findings report before touching any code.

The codebase is small (7 source files, ~1000 lines) and well-structured. The issues to fix are all concrete and locatable. The biggest coordination risk is AUDIT-05: the findings report must be written and committed before any other changes begin.

**Primary recommendation:** Treat AUDIT-05 (findings report) as plan A, and the four remediation tasks (AUDIT-01 through AUDIT-04) as plan B that is gated on A.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AUDIT-01 | Codebase passes `cargo clippy -D warnings` with zero warnings | Clippy already passes — the requirement is to maintain this while removing the one existing `#[allow(dead_code)]` suppression in `temp.rs` |
| AUDIT-02 | All dead code removed or justified with documented reason | One confirmed suppression: `TempFileRegistry::cleanup_all()` in `src/temp.rs:25` — needs either removal or a `// Justification:` comment |
| AUDIT-03 | Duplicate spinner factories extracted to shared utility | 5 identical spinner factory implementations confirmed across 5 files — extract to `src/ui.rs` or `src/spinner.rs` |
| AUDIT-04 | Consistent AppError-based error handling across all commands | 4 bare `anyhow::bail!` and 3 bare `anyhow::anyhow!` usages found — convert to AppError variants or add new variants |
| AUDIT-05 | Written findings report delivered before any code changes | No prior findings report exists — must be the first artifact produced, before any code edits |
</phase_requirements>

---

## Current Clippy State

`cargo clippy -- -D warnings` **already passes with zero warnings and zero errors** (verified 2026-02-20). The codebase uses Rust 2024 edition.

The one existing lint suppression is:
```rust
// src/temp.rs:25
#[allow(dead_code)]
pub fn cleanup_all(&self) {
```

This suppression was added because `cleanup_all()` is never called. The Ctrl-C handler in `register_cleanup()` does the cleanup inline by holding the `Arc<Mutex<Vec<PathBuf>>>` directly. So `cleanup_all()` is genuinely dead code.

**AUDIT-01 constraint:** The suppression must be removed without adding any new `#[allow(...)]` attributes. This means either deleting `cleanup_all()` or using it somewhere (which would require adding a call site).

---

## Dead Code Inventory (AUDIT-02)

### Confirmed Dead Code

| Location | Item | Dead Because |
|----------|------|-------------|
| `src/temp.rs:26` | `TempFileRegistry::cleanup_all()` | Never called; Ctrl-C handler accesses `Arc<Mutex<...>>` directly |

### Confirmed Live Code (Not Dead)

| Location | Item | Used By |
|----------|------|---------|
| `src/commands/normalize.rs` | `normalize_to_temp()` | Called by `cut::run()` at `cut.rs:60` |
| `src/silence.rs` | All 4 public functions | Used by `cut.rs` and `tests/silence_tests.rs` |
| `src/commands/cut.rs` | `derive_output_path()` | Re-used by `overlay.rs:11` via `use crate::commands::cut::derive_output_path` |

### Decision for `cleanup_all()`

Two options:
1. **Remove it** — Ctrl-C cleanup is handled by `register_cleanup()` which holds the Arc directly. The function is not needed.
2. **Keep with justification** — If Phase 8 (Pipeline) will need it, add `// Kept for Phase 8: pipeline will call this on graceful shutdown`.

Given the prior decision "Pipeline calls run() directly, not subprocess", Phase 8 will share the same `TempFileRegistry` and won't need a standalone cleanup function either. **Recommendation: delete `cleanup_all()`.**

---

## Spinner Factory Duplication (AUDIT-03)

### Full Inventory

5 independent spinner implementations using identical Braille tick strings and cyan style:

| File | Form | Line |
|------|------|------|
| `src/commands/cut.rs` | `fn make_spinner(message: String) -> ProgressBar` | 25 |
| `src/commands/caption.rs` | `fn make_spinner(message: String) -> ProgressBar` | 264 |
| `src/commands/overlay.rs` | Inlined in `generate_title()` | 42 |
| `src/commands/normalize.rs` | Inlined in `normalize_to_temp()` | 63 |
| `src/ffmpeg.rs` | `fn run_ffmpeg_with_spinner()` (private) | 184 |

All 5 use the exact same Braille tick sequence:
```
"\u{2800}", "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{283b}",
"\u{2839}", "\u{2838}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2713}"
```
and the same style template: `"{spinner:.cyan} {msg}"` with 80ms tick rate.

### Extraction Plan

Create `src/ui.rs` with a single public function:
```rust
// src/ui.rs
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};

pub fn make_spinner(message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "\u{2800}", "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{283b}",
                "\u{2839}", "\u{2838}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2713}",
            ]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(message.into());
    pb
}
```

Then update `main.rs` to include `mod ui;` and replace all 5 sites with `crate::ui::make_spinner(...)` or `ui::make_spinner(...)`.

**Note on `ffmpeg.rs`:** `run_ffmpeg_with_spinner` is private and wraps `run_ffmpeg`. After extraction, it can inline `ui::make_spinner()`. No API change needed.

---

## Error Handling Inconsistencies (AUDIT-04)

### Current AppError Variants

Defined in `src/error.rs`:
- `FfmpegNotFound`
- `FfmpegFailed { stage, code, stderr }`
- `StageIo { stage, source }`
- `InputNotFound(PathBuf)`
- `WhisperNotFound`
- `WhisperFailed { stage, code, stderr }`
- `ModelNotFound(PathBuf)`

### Bare anyhow Usage Inventory

**`anyhow::bail!` (4 occurrences):**

| File | Line | Message | Fix |
|------|------|---------|-----|
| `commands/cut.rs` | 135 | `"No speech detected -- entire video is silence"` | New `AppError::NoSpeechDetected` or `AppError::AllSilence` variant |
| `commands/overlay.rs` | 82 | `"claude CLI failed: {stderr}"` | New `AppError::ClaudeFailed { code, stderr }` variant |
| `commands/overlay.rs` | 90 | `"claude returned empty title"` | Same new variant or `AppError::StageIo` with context |
| `commands/overlay.rs` | 194 | `"transcription file not found: {path}"` | Reuse `AppError::InputNotFound(path)` |

**`anyhow::anyhow!` (3 occurrences):**

| File | Line | Context | Fix |
|------|------|---------|-----|
| `commands/caption.rs` | 443 | `"parsing whisper JSON: {e}"` | New `AppError::JsonParse { stage, source }` or `AppError::StageIo` |
| `commands/caption.rs` | 497 | `"serializing JSON: {e}"` | Same |
| `commands/overlay.rs` | 27 | `"parsing transcription JSON: {e}"` | Same |
| `commands/normalize.rs` | 98 | `"failed to parse loudnorm measurement..."` | New `AppError::ParseFailed { stage }` variant |

### New AppError Variants Needed

To convert all bare anyhow usage:

```rust
// Suggested additions to src/error.rs:

#[error("no speech detected in {0}: entire video is silence")]
NoSpeechDetected(PathBuf),           // or accept a String filename

#[error("error in stage '{stage}': claude CLI exited with code {code}\n{stderr}")]
ClaudeFailed { stage: String, code: i32, stderr: String },

#[error("error in stage '{stage}': failed to parse output")]
ParseFailed { stage: String },
```

**Alternative minimal approach:** Some of the `anyhow::anyhow!` usages (JSON parse errors) map cleanly to `AppError::StageIo` if we accept losing type precision — but the stage context is already present. The `ParseFailed` variant is cleaner than misusing `StageIo` (which implies `std::io::Error`).

### format_error() Impact

`format_error()` in `src/error.rs` is a match statement over AppError variants used in `main.rs`. Each new variant requires a new arm. This is a compile-time exhaustiveness check, so no variants can be silently skipped.

---

## Architecture Patterns

### Module Organization (Current)

```
src/
├── main.rs          # Entry point, dispatch to commands
├── cli.rs           # Clap structs: Cli, Commands, *Args
├── error.rs         # AppError enum + format_error()
├── ffmpeg.rs        # FFmpeg subprocess wrappers
├── silence.rs       # Silence detection logic (pub, tested)
├── temp.rs          # TempFileRegistry, make_temp_file
├── lib.rs           # Re-exports silence (for integration tests)
└── commands/
    ├── mod.rs       # pub mod cut, caption, overlay, normalize
    ├── cut.rs       # Silence removal command
    ├── caption.rs   # Whisper transcription command
    ├── overlay.rs   # Text overlay command
    └── normalize.rs # Audio normalization (used only by cut)
```

### Planned After Phase 6

```
src/
├── ui.rs            # NEW: shared spinner factory
└── ... (rest unchanged)
```

### Where to Put `ui.rs`

`ui.rs` at the crate root (peer to `ffmpeg.rs`, `error.rs`) is consistent with the existing structure. It does not belong inside `commands/` since it is used by both `commands/` and `ffmpeg.rs`.

---

## Pitfalls

### Pitfall 1: Removing `cleanup_all()` Breaks `#[allow(dead_code)]` Removal Logic
**What goes wrong:** If `cleanup_all()` is deleted, the `#[allow(dead_code)]` attribute on line 25 must also be deleted. Forgetting to remove the allow attribute itself doesn't cause a build error, but violates AUDIT-01's intent (zero suppression attributes added — existing ones must also go).
**How to avoid:** Delete both lines 25 and 26-31 together.

### Pitfall 2: `format_error()` Match is Non-Exhaustive
**What goes wrong:** Adding new AppError variants without adding match arms in `format_error()` causes a compile error.
**How to avoid:** Add match arms in `format_error()` in the same commit as new variants. The compiler will flag this.

### Pitfall 3: `normalize` is in `commands/mod.rs` but Not in CLI
**What goes wrong:** `normalize` is listed as `pub mod normalize` in `commands/mod.rs` but is not a CLI subcommand (no `Commands::Normalize` variant). Someone might misidentify it as dead code.
**It is not dead:** `normalize_to_temp()` is called from `cut.rs:60`. The module is an internal implementation helper, not an unexposed command. No change needed.

### Pitfall 4: Spinner in `ffmpeg.rs` Has a Different Scope
**What goes wrong:** `run_ffmpeg_with_spinner` in `ffmpeg.rs` is a private fallback inside `run_ffmpeg_with_progress`. Its spinner is not created via `make_spinner()` because `ffmpeg.rs` didn't want to depend on anything. After extraction to `ui.rs`, both `commands/` and `ffmpeg.rs` can depend on `crate::ui`.
**How to avoid:** Add `mod ui` to `main.rs` (not `lib.rs`) so it's available to all binary code. Update `ffmpeg.rs` to use `crate::ui::make_spinner`.

### Pitfall 5: `anyhow::anyhow!` for JSON Errors Loses Error Type
**What goes wrong:** `serde_json::Error` is not `std::io::Error`, so it cannot go into `AppError::StageIo`. The `anyhow::anyhow!` wrapper discards the type. Conversion to AppError requires either a new variant or accepting that JSON errors show as generic messages.
**How to avoid:** Use `AppError::ParseFailed { stage }` — caller has the parse error as context via `?`'s chain. Alternatively, add `source: serde_json::Error` to a new variant (requires adding serde_json to AppError's direct dependencies, which is fine since it's already a crate dep).

---

## Code Examples

### Verified: Spinner extraction pattern

Before (cut.rs, caption.rs — identical functions):
```rust
fn make_spinner(message: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "\u{2800}", "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{283b}",
                "\u{2839}", "\u{2838}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2713}",
            ]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(message);
    pb
}
```

After (src/ui.rs):
```rust
pub fn make_spinner(message: impl Into<String>) -> ProgressBar {
    // ... (same body)
    pb.set_message(message.into()); // into() handles String or &str
    pb
}
```

Call sites change from `make_spinner("text".to_string())` to `crate::ui::make_spinner("text")`.

### Verified: AppError conversion for `anyhow::bail!` in cut.rs

Before:
```rust
// src/commands/cut.rs:135
anyhow::bail!("No speech detected -- entire video is silence");
```

After (with new variant):
```rust
return Err(AppError::NoSpeechDetected(args.input.clone()).into());
```

Or simpler if we want to avoid a new variant:
```rust
return Err(AppError::StageIo {
    stage: "speech-detection".to_string(),
    source: std::io::Error::new(std::io::ErrorKind::InvalidData, "entire video is silence"),
}.into());
```

The first form is preferred since it gives a clean user-facing message via `format_error()`.

---

## Open Questions

1. **What justification comment for `cleanup_all()` if kept?**
   - What we know: Pipeline (Phase 8) will call `run()` directly and share `TempFileRegistry`, so it won't need a standalone cleanup either.
   - What's unclear: Whether future phases might add a `cleanup_all()` call.
   - Recommendation: Delete it. Reintroduce in Phase 8 if it turns out to be needed.

2. **Is `AppError::ClaudeFailed` worth adding for two error sites?**
   - Both are in `overlay.rs::generate_title()`, a private function.
   - Both could use a generic `AppError::ExternalTool { tool: "claude", ... }` pattern that Phase 7 (Doctor) would likely expand.
   - Recommendation: Add `ClaudeFailed` now. Phase 7 will need `require_claude()` anyway, so the infrastructure is worthwhile.

3. **Should `format_error()` format the new variants with colored output?**
   - Existing variants all use `owo_colors` for `"error:".red().bold()`.
   - New variants must be consistent.
   - Recommendation: Yes, same pattern. No aesthetic decisions needed.

---

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection — all 7 source files and 1 integration test file read in full
- `cargo clippy -- -D warnings` run output (zero warnings, confirmed 2026-02-20)
- `RUSTFLAGS="-D warnings" cargo build` confirmed zero warnings

### Not Consulted (Not Needed)
- Context7, WebSearch — this phase is internal refactoring, no external libraries or APIs involved
- The standard stack (indicatif, thiserror, anyhow) is already chosen; no new dependencies required

---

## Metadata

**Confidence breakdown:**
- Dead code inventory: HIGH — confirmed by clippy, `#[allow]` attribute, and call-site search
- Spinner duplication: HIGH — all 5 instances read in full, identical tick strings confirmed
- Error handling gaps: HIGH — all `anyhow::bail!` and `anyhow::anyhow!` occurrences enumerated
- Refactoring approach: HIGH — `src/ui.rs` extraction is standard Rust module pattern

**Research date:** 2026-02-20
**Valid until:** Indefinite — codebase is static until Phase 6 work begins
