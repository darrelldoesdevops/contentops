# Phase 6: Audit Findings Report

**Date:** 2026-02-20
**Scope:** 7 source files in `src/`, ~1000 LOC Rust
**Clippy Status:** `cargo clippy -- -D warnings` passes with zero warnings (verified)
**Rust Edition:** 2024

## Audit Summary

The codebase is clean and well-structured. Clippy passes without warnings. Issues found are consistency/duplication problems that clippy does not flag: one dead code suppression, five duplicate spinner factories, and eight bare anyhow error calls bypassing the typed AppError system.

## 1. Dead Code

| # | Location | Item | Suppression | Why Dead | Recommendation |
|---|----------|------|-------------|----------|----------------|
| 1 | `src/temp.rs:25-31` | `TempFileRegistry::cleanup_all()` | `#[allow(dead_code)]` on line 25 | Never called; Ctrl-C handler in `register_cleanup()` accesses `Arc<Mutex<Vec<PathBuf>>>` directly | **Delete** -- Pipeline (Phase 8) will share TempFileRegistry via direct `run()` calls, so this method won't be needed |

**Confirmed live code (NOT dead):**

| Location | Item | Used By |
|----------|------|---------|
| `src/commands/normalize.rs` | `normalize_to_temp()` | `cut::run()` at `cut.rs:60` |
| `src/silence.rs` | All 4 public functions | `cut.rs` and `tests/silence_tests.rs` |
| `src/commands/cut.rs` | `derive_output_path()` | `overlay.rs:10` via `use crate::commands::cut::derive_output_path` |

## 2. Spinner Duplication

5 identical spinner factory implementations using the same Braille tick strings and cyan style:

| # | File | Line | Form |
|---|------|------|------|
| 1 | `src/commands/cut.rs` | 25 | `fn make_spinner(message: String) -> ProgressBar` |
| 2 | `src/commands/caption.rs` | 264 | `fn make_spinner(message: String) -> ProgressBar` |
| 3 | `src/commands/overlay.rs` | 42 | Inlined in `generate_title()` |
| 4 | `src/commands/normalize.rs` | 63 | Inlined in `normalize_to_temp()` |
| 5 | `src/ffmpeg.rs` | 184 | Private `run_ffmpeg_with_spinner()` |

All use identical configuration:
- Tick strings: `\u{2800}` through `\u{2713}` (Braille animation + checkmark)
- Style template: `"{spinner:.cyan} {msg}"`
- Tick rate: 80ms

**Recommendation:** Extract to `src/ui.rs` with `pub fn make_spinner(message: impl Into<String>) -> ProgressBar`. Add `mod ui;` to `main.rs`. Replace all 5 sites.

## 3. Error Handling Inconsistencies

### Bare `anyhow::bail!` (4 occurrences)

| # | File | Line | Message | Recommended Fix |
|---|------|------|---------|-----------------|
| 1 | `src/commands/cut.rs` | 135 | `"No speech detected -- entire video is silence"` | New `AppError::NoSpeechDetected(PathBuf)` variant |
| 2 | `src/commands/overlay.rs` | 82 | `"claude CLI failed: {stderr}"` | New `AppError::ClaudeFailed { stage, code, stderr }` variant |
| 3 | `src/commands/overlay.rs` | 90 | `"claude returned empty title"` | Same `ClaudeFailed` variant (code: 0, stderr: message) |
| 4 | `src/commands/overlay.rs` | 194 | `"transcription file not found: {path}"` | Reuse existing `AppError::InputNotFound(path)` |

### Bare `anyhow::anyhow!` (4 occurrences)

| # | File | Line | Context | Recommended Fix |
|---|------|------|---------|-----------------|
| 1 | `src/commands/caption.rs` | 443 | `"parsing whisper JSON: {e}"` | New `AppError::ParseFailed { stage, message }` variant |
| 2 | `src/commands/caption.rs` | 497 | `"serializing JSON: {e}"` | Same `ParseFailed` variant |
| 3 | `src/commands/overlay.rs` | 27 | `"parsing transcription JSON: {e}"` | Same `ParseFailed` variant |
| 4 | `src/commands/normalize.rs` | 98 | `"failed to parse loudnorm measurement..."` | Same `ParseFailed` variant |

### New AppError Variants Needed

```
NoSpeechDetected(PathBuf)
ClaudeFailed { stage, code, stderr }
ParseFailed { stage, message }
```

Each requires a matching `format_error()` arm in `src/error.rs` following the existing colored output pattern with `owo_colors`.

## 4. Clippy Status

- `cargo clippy -- -D warnings`: **PASSES** (zero warnings, zero errors)
- Existing suppressions: **1** (`#[allow(dead_code)]` on `cleanup_all()` in `src/temp.rs:25`)
- After dead code deletion: **0** suppressions expected

## 5. Remediation Plan

| Plan | Requirement | What Changes | Files Modified |
|------|-------------|-------------|----------------|
| 06-02 | AUDIT-01, AUDIT-02, AUDIT-03 | Delete dead code + extract spinner utility | `src/temp.rs`, `src/ui.rs` (new), `src/main.rs`, `src/commands/cut.rs`, `src/commands/caption.rs`, `src/commands/overlay.rs`, `src/commands/normalize.rs`, `src/ffmpeg.rs` |
| 06-03 | AUDIT-04 | Convert bare anyhow to AppError variants | `src/error.rs`, `src/commands/cut.rs`, `src/commands/caption.rs`, `src/commands/overlay.rs`, `src/commands/normalize.rs` |

---
*Audit completed: 2026-02-20*
*No source files modified during this audit*
