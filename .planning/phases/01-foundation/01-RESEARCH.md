# Phase 1: Foundation - Research

**Researched:** 2026-02-19
**Domain:** Rust CLI infrastructure — clap, FFmpeg subprocess, error handling, temp files, signal handling
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### CLI structure
- Subcommand-per-function pattern (not generic `process`): `contentops cut input.mp4`, `contentops caption input.mp4`, etc.
- Verb-based subcommand names: `cut` for silence removal (Phase 2), `caption` for captioning, `overlay` for overlays
- Phase 1 implements the `cut` subcommand as the first FFmpeg-exercising command
- Bare `contentops` (no subcommand) shows help with available subcommands and one-line descriptions
- clap derive macros for argument parsing
- Default output: same directory as input, subcommand-specific suffix (`input_cut.mp4`, `input_captioned.mp4`, `input_overlay.mp4`)
- `-o` flag overrides output path
- Overwrite existing output files silently (matches FFmpeg `-y` behavior)

#### Error presentation
- Colored + structured errors (like rustc/cargo): red `error:` prefix, bold stage name, indented FFmpeg stderr
- On FFmpeg failure: show last 10-20 lines of stderr. Full log saved to file.
- FFmpeg not found: actionable error with install hint (`brew install ffmpeg`)
- Every error always identifies which pipeline stage failed (e.g., "error in stage 'audio extraction': ...")

#### FFmpeg output handling
- Spinner + status line during processing ("Processing input.mp4...") using indicatif crate
- Phase 5 upgrades spinner to real progress bar (PIPE-05)
- `--verbose` flag available from Phase 1: streams raw FFmpeg stderr in real-time
- Success message shows output path + file size: "✓ Created input_cut.mp4 (12.3 MB)"

#### Temp file behavior
- Temp files created next to input file (same directory), dot-prefixed
- Naming pattern: `.contentops_tmp_<random>.ext`
- Cleaned up after both successful and failed runs
- Signal handling for Ctrl+C cleanup: Claude's discretion on implementation approach for Phase 1

### Claude's Discretion
- Signal handling strategy for Ctrl+C cleanup (best-effort vs full SIGINT handler)
- Exact spinner style from indicatif
- Internal module organization and error type hierarchy
- Color crate choice (anyhow, miette, or manual colored output)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PIPE-01 | User can run contentops and it detects FFmpeg on PATH, failing with a clear error if missing | `which` crate v8.0.0 — `which::which("ffmpeg")` returns Err when not found; actionable error message pattern documented |
| PIPE-02 | FFmpeg subprocess wrapper handles pipe safety, exit code checking, and always passes `-y -nostdin` | `std::process::Command` with `.stdin(Stdio::null())` + `.stderr(Stdio::piped())` + `.output()` covers all requirements; `-y` and `-nostdin` are standard FFmpeg flags |
| PIPE-03 | Temporary files are automatically cleaned up after successful processing | `tempfile::Builder` with `.prefix(".contentops_tmp_")` + `.tempfile_in(dir)` provides drop-based cleanup; manual cleanup needed for SIGINT paths |
| PIPE-04 | Processing errors are reported with context (which stage failed, FFmpeg stderr output) | `thiserror` for typed error enum with stage field; `anyhow` for context propagation; `owo-colors` for rustc-style formatting |
</phase_requirements>

## Summary

Phase 1 builds the entire CLI infrastructure that every subsequent phase depends on. The Rust ecosystem has mature, well-maintained crates for each component: clap 4.5 for argument parsing, indicatif 0.18 for spinners, thiserror + anyhow for error handling, which for PATH detection, tempfile for temp file lifecycle, and ctrlc for signal handling.

The most important architectural decision is the error type hierarchy. Using `thiserror` to define a typed `AppError` enum (with a `stage` field) satisfies PIPE-04 while keeping errors structured for future pipeline stages. `anyhow::Context` wraps errors at call sites with stage names, enabling the "error in stage 'X'" pattern without manual string formatting at every level.

The main complexity is temp file cleanup under SIGINT. The `tempfile` crate's `NamedTempFile` uses Rust's drop mechanism, which does not run on unhandled SIGINT. The recommended approach is to use `ctrlc` to register a handler that performs cleanup and calls `std::process::exit`. This is simpler than a full RAII guard and sufficient for Phase 1.

**Primary recommendation:** Use `thiserror` + `anyhow` for errors, `which` for FFmpeg detection, `std::process::Command` for subprocess (no extra crate needed), `tempfile::Builder` for temp files, `ctrlc` for SIGINT cleanup, `indicatif` for spinner, `owo-colors` for output formatting.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5.59 | CLI argument parsing, subcommands | De facto standard; powers ripgrep, bat, fd; derive API is idiomatic |
| indicatif | 0.18.4 | Spinner + progress bars | Most downloaded Rust progress library; thread-safe; integrates with owo-colors |
| thiserror | 2.0.18 | Typed error definitions | dtolnay-maintained; zero overhead; idiomatic for structured errors in apps |
| anyhow | 1.0.101 | Error propagation + context | Pairs with thiserror; `with_context()` enables stage-labelled errors |
| which | 8.0.0 | Executable PATH detection | Exact Unix `which` semantics; cross-platform; returns typed errors |
| tempfile | 3.25.0 | Temp file creation/cleanup | Drop-based cleanup; `Builder` supports custom prefix/dir; 456M+ downloads |
| owo-colors | 4.2.3 | Terminal color output | Zero-allocation; recommended by rust-cli-recommendations; drop-in for `colored` |
| ctrlc | 3.5.2 | Ctrl+C / SIGINT handling | Cross-platform; simple Arc<AtomicBool> pattern; `termination` feature adds SIGTERM/SIGHUP |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| humansize | latest | Human-readable file sizes ("12.3 MB") | For success message "✓ Created ... (12.3 MB)"; alternatively use bytesize |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| thiserror + anyhow | miette | miette gives rustc-style diagnostic output with source spans — valuable for user-facing errors, but heavier; the user's manual colored output approach with owo-colors achieves the same rustc-style look with less overhead |
| owo-colors | colored | colored is older, has global state issues; owo-colors is actively maintained and zero-allocation |
| ctrlc | signal-hook | signal-hook is more comprehensive but ctrlc is sufficient for Phase 1; can migrate if needed |
| std::process::Command | subprocess crate | subprocess adds deadlock-free communication for bidirectional pipes; not needed since we only capture stderr/stdout after completion via `.output()` |

**Installation:**
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.18"
thiserror = "2.0"
anyhow = "1.0"
which = "8.0"
tempfile = "3.25"
owo-colors = "4.2"
ctrlc = { version = "3.5", features = ["termination"] }
humansize = "2.1"
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── main.rs          # Entry point: parse CLI, dispatch to command handlers
├── cli.rs           # clap Parser/Subcommand definitions
├── commands/
│   ├── mod.rs       # pub use for command handlers
│   └── cut.rs       # `cut` subcommand implementation (Phase 1 stub)
├── ffmpeg.rs        # FFmpeg subprocess wrapper (PIPE-02)
├── error.rs         # AppError enum + Display formatting (PIPE-04)
└── temp.rs          # Temp file lifecycle management (PIPE-03)
```

### Pattern 1: clap Subcommand-Per-Function with Derive

The `Commands` enum maps 1:1 to subcommands. Each variant holds its own `Args` struct. The main `Cli` struct uses `Option<Commands>` so bare invocation shows help.

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "contentops", about = "Video processing pipeline")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Remove silence from video
    Cut(CutArgs),
}

#[derive(Args)]
struct CutArgs {
    /// Input video file
    input: PathBuf,

    /// Output path (default: input_cut.mp4)
    #[arg(short = 'o')]
    output: Option<PathBuf>,
}
```

When `command` is `None`, print help and exit with `cli.print_help()` + `std::process::exit(0)`.

### Pattern 2: Typed Error Enum with Stage Context

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("ffmpeg not found on PATH\n  hint: brew install ffmpeg")]
    FfmpegNotFound,

    #[error("stage '{stage}' failed: ffmpeg exited with {code}\n{stderr}")]
    FfmpegFailed {
        stage: String,
        code: i32,
        stderr: String,
    },

    #[error("stage '{stage}' failed: {source}")]
    IoError {
        stage: String,
        #[source]
        source: std::io::Error,
    },
}
```

Callers wrap with `anyhow::Context` for additional context:
```rust
// Source: https://docs.rs/anyhow/latest/anyhow/
use anyhow::Context;

ffmpeg_run(&args)
    .with_context(|| format!("in stage '{}'", stage_name))?;
```

### Pattern 3: FFmpeg Subprocess Wrapper

```rust
// src/ffmpeg.rs
use std::process::{Command, Stdio};

pub struct FfmpegOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: Vec<u8>,
}

pub fn run_ffmpeg(args: &[&str]) -> Result<FfmpegOutput, std::io::Error> {
    let output = Command::new("ffmpeg")
        .args(args)
        .arg("-y")          // overwrite output
        .arg("-nostdin")    // never read from stdin
        .stdin(Stdio::null())
        .stdout(Stdio::null())  // FFmpeg progress goes to stderr
        .stderr(Stdio::piped())
        .output()?;

    Ok(FfmpegOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stderr: output.stderr,
    })
}
```

For `--verbose` mode, use `.stderr(Stdio::inherit())` instead of piped — raw stderr streams to terminal in real-time.

### Pattern 4: Spinner with indicatif

```rust
// Source: https://docs.rs/indicatif/latest/indicatif/
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn start_spinner(message: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏","✓"]),
    );
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message(message.to_string());
    bar
}

// On success:
bar.finish_with_message(format!("✓ Created {} ({})", output_path, size_str));
// On failure:
bar.finish_and_clear();
```

### Pattern 5: Temp File with Custom Prefix and Directory

```rust
// Source: https://docs.rs/tempfile/latest/tempfile/struct.Builder.html
use tempfile::Builder;

pub fn make_temp_file(dir: &Path, ext: &str) -> Result<tempfile::NamedTempFile> {
    Builder::new()
        .prefix(".contentops_tmp_")
        .suffix(ext)
        .tempfile_in(dir)
        .with_context(|| format!("creating temp file in {}", dir.display()))
}
```

`NamedTempFile` deletes on drop. For SIGINT, register ctrlc handler before creating temp files (see Pattern 6).

### Pattern 6: Ctrl+C Cleanup with ctrlc

**Recommendation: best-effort approach for Phase 1** — register a handler that removes known temp paths and exits. No complex RAII guard needed.

```rust
// Source: https://docs.rs/ctrlc/latest/ctrlc/
use std::sync::{Arc, Mutex};
use ctrlc;

pub fn register_cleanup(temp_paths: Arc<Mutex<Vec<PathBuf>>>) {
    ctrlc::set_handler(move || {
        let paths = temp_paths.lock().unwrap();
        for path in paths.iter() {
            let _ = std::fs::remove_file(path);
        }
        std::process::exit(1);
    }).expect("Error setting Ctrl-C handler");
}
```

Register once in `main()` before any command handler runs. Command handlers add temp file paths to the shared `Vec` on creation.

### Pattern 7: FFmpeg Detection (PIPE-01)

```rust
// Source: https://docs.rs/which/latest/which/
use which::which;

pub fn require_ffmpeg() -> Result<PathBuf, AppError> {
    which("ffmpeg").map_err(|_| AppError::FfmpegNotFound)
}
```

Call at the start of each command handler, before any processing begins.

### Anti-Patterns to Avoid
- **Passing `-y` as a user arg:** Always inject `-y` and `-nostdin` programmatically in the wrapper — never rely on callers to remember these flags. Missing `-nostdin` causes FFmpeg to hang waiting for input in non-interactive contexts.
- **`.output()` vs `.spawn()` confusion:** Use `.output()` when you need to capture stderr after completion. Use `.spawn()` only when you need real-time streaming (verbose mode via `Stdio::inherit()` is simpler and avoids this).
- **NamedTempFile cleanup on SIGINT:** `NamedTempFile`'s Drop impl does not run on unhandled SIGINT. Without ctrlc handler, Ctrl+C leaves temp files on disk. This is a correctness bug for PIPE-03.
- **Printing raw stderr on failure:** Dump only last 10-20 lines to terminal; save full stderr to log file. Raw FFmpeg stderr is very verbose and obscures the actionable error.
- **Global color state:** Don't use `colored` crate's global `control::set_override()` in library code. Use `owo-colors` with explicit stream checking.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| PATH executable detection | `std::env::var("PATH").split(':')` iteration | `which` crate | Cross-platform (Windows PATH separator), handles symlinks, permission checks |
| Spinner animation | Manual print-and-clear loop | `indicatif` | Thread-safe, handles terminal resize, plays well with stderr/stdout |
| Temp file naming with randomness | `format!(".contentops_tmp_{}", rand::random::<u64>())` | `tempfile::Builder` | Atomic creation (no TOCTOU race), OS-managed cleanup fallback, tested edge cases |
| Error context chaining | `format!("stage {}: {}", stage, err)` at every call site | `anyhow::with_context()` | Preserves error chain, consistent formatting, composable |
| Human file sizes | `if bytes > 1_000_000 { format!("{:.1} MB", ...) }` | `humansize` crate | Handles all edge cases, configurable conventions (SI vs binary) |

**Key insight:** Each of these looks simple but has non-obvious edge cases. The crate ecosystem has solved them — use it.

## Common Pitfalls

### Pitfall 1: FFmpeg Hangs Waiting for stdin
**What goes wrong:** FFmpeg reads from stdin to answer interactive prompts (e.g., "overwrite output? y/n"). When run as a subprocess without `-nostdin`, it blocks indefinitely.
**Why it happens:** FFmpeg's default behavior is to inherit the parent's stdin. In a CLI context where the user isn't watching for that prompt, the process hangs.
**How to avoid:** Always pass `-nostdin` and `.stdin(Stdio::null())` in the subprocess wrapper. The `-y` flag handles the overwrite confirmation.
**Warning signs:** `Command::output()` never returns; process appears to hang indefinitely.

### Pitfall 2: NamedTempFile Left Behind on Ctrl+C
**What goes wrong:** User presses Ctrl+C mid-processing. The `NamedTempFile` destructor doesn't run. Temp files accumulate in the input directory.
**Why it happens:** Rust's drop mechanism doesn't run on unhandled signals. The tempfile crate documents this explicitly.
**How to avoid:** Register `ctrlc` handler in `main()` before processing begins. Handler iterates known temp paths and removes them before calling `std::process::exit(1)`.
**Warning signs:** `.contentops_tmp_*` files in video directories after interrupted runs.

### Pitfall 3: Deadlock Capturing Large FFmpeg Output
**What goes wrong:** FFmpeg generates a large amount of stderr output. The OS pipe buffer fills up. FFmpeg blocks trying to write stderr. The parent is blocked waiting for FFmpeg to finish. Deadlock.
**Why it happens:** `std::process::Command::output()` reads both stdout and stderr after the child exits. If the pipe buffer fills before exit, neither side makes progress.
**How to avoid:** For Phase 1, this is unlikely because we're capturing stderr only and FFmpeg's stderr (progress lines) is bounded. However, use `.stderr(Stdio::piped())` with `.output()` only (not `.spawn()` + manual reading). The `.output()` method on the standard library handles this correctly via platform-specific mechanisms.
**Warning signs:** Process hangs on very long FFmpeg operations. Upgrade to `subprocess` crate or thread-based reading if encountered.

### Pitfall 4: Subcommand Help Not Shown for Bare Invocation
**What goes wrong:** `contentops` with no arguments exits with an error instead of showing help.
**Why it happens:** If `command` field is `Commands` (not `Option<Commands>`), clap treats a missing subcommand as an error.
**How to avoid:** Make the field `Option<Commands>`, then in main check for `None` and print help.
**Warning signs:** `contentops` prints "error: the following required arguments were not provided" instead of help text.

### Pitfall 5: Output Path Derivation Edge Cases
**What goes wrong:** Input path is `/foo/bar.mp4`. Naively replacing extension gives `/foo/bar_cut.mp4`, but if input is `/foo/bar.tar.gz`, you get `/foo/bar.tar_cut.gz`.
**Why it happens:** `Path::file_stem()` returns everything up to the last `.`, not a "meaningful" extension.
**How to avoid:** Use `path.file_stem()` to get stem, append `_cut`, add `.mp4` explicitly (output is always mp4 regardless of input container). Pattern: `parent / format!("{}_cut.mp4", stem)`.
**Warning signs:** Unexpected output filenames with double extensions.

## Code Examples

Verified patterns from official sources:

### FFmpeg Detection and Error Formatting
```rust
// Source: https://docs.rs/which/latest/which/ + owo-colors
use which::which;
use owo_colors::OwoColorize;

fn check_ffmpeg() -> anyhow::Result<()> {
    which("ffmpeg").map_err(|_| {
        anyhow::anyhow!(
            "{}: ffmpeg not found on PATH\n  {}: brew install ffmpeg",
            "error".red().bold(),
            "hint".bold()
        )
    })?;
    Ok(())
}
```

### Full Command Dispatch Pattern
```rust
// src/main.rs
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup Ctrl+C handler early
    let temp_registry: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(vec![]));
    register_cleanup(Arc::clone(&temp_registry));

    match cli.command {
        None => {
            Cli::command().print_help()?;
            std::process::exit(0);
        }
        Some(Commands::Cut(args)) => {
            commands::cut::run(args, cli.verbose, temp_registry)?;
        }
    }
    Ok(())
}
```

### Deriving Output Path from Input
```rust
pub fn derive_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    parent.join(format!("{}_{}.mp4", stem, suffix))
}
// derive_output_path("video.mp4", "cut") => "video_cut.mp4"
```

### Indicatif Spinner Lifecycle
```rust
// Source: https://docs.rs/indicatif/latest/indicatif/
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

let pb = ProgressBar::new_spinner();
pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap()
);
pb.enable_steady_tick(Duration::from_millis(80));
pb.set_message(format!("Processing {}...", input_name));

// ... run ffmpeg ...

match result {
    Ok(_) => pb.finish_with_message(
        format!("✓ Created {} ({})", output_path.display(), size_str)
    ),
    Err(_) => pb.finish_and_clear(),
}
```

### Last N Lines of FFmpeg Stderr
```rust
pub fn last_n_lines(stderr: &[u8], n: usize) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `structopt` crate for CLI | `clap` v4 with derive (absorbed structopt) | clap 3.0 (2022) | structopt deprecated; use clap directly |
| `failure` crate for errors | `thiserror` + `anyhow` | 2019-2020 | failure unmaintained; thiserror/anyhow are the standard |
| `colored` crate for output | `owo-colors` | 2021+ | colored has global state issues; owo-colors zero-alloc |
| Manual temp file management | `tempfile` crate | Stable for years | OS-level cleanup fallback is critical |
| `anyhow` v1.x | `anyhow` 1.0.x (stable, no v2 yet) | N/A | anyhow is v1; search results mentioning "v2" were inaccurate — latest is 1.0.101 |

**Deprecated/outdated:**
- `structopt`: Merged into clap v3+, use `clap` derive directly
- `failure`: Replaced by thiserror + anyhow; unmaintained
- `error-chain`: Replaced by thiserror + anyhow; unmaintained
- `colored`: Superseded by `owo-colors` for new projects

## Open Questions

1. **File size display for success message**
   - What we know: humansize and bytesize both work; success message needs "12.3 MB" format
   - What's unclear: Whether to use SI (MB) or binary (MiB) — user expects MB for video files
   - Recommendation: Use `humansize` with `DECIMAL` formatting (matches macOS Finder "12.3 MB" convention)

2. **Verbose mode: stderr streaming during FFmpeg run**
   - What we know: `.stderr(Stdio::inherit())` passes FFmpeg stderr directly to terminal; spinner and inherited stderr conflict on same fd
   - What's unclear: Whether indicatif spinner needs to be disabled entirely in verbose mode
   - Recommendation: In `--verbose` mode, skip the spinner entirely and print `"Running: ffmpeg <args>"` then let FFmpeg stderr stream naturally

3. **ctrlc handler and multi-threaded spinner**
   - What we know: indicatif spinner runs on a background thread via `enable_steady_tick`; ctrlc handler runs in a dedicated signal thread
   - What's unclear: Whether calling `pb.finish_and_clear()` inside ctrlc handler before `std::process::exit(1)` is safe
   - Recommendation: Don't touch the spinner in the ctrlc handler; `std::process::exit(1)` will terminate all threads including the spinner thread cleanly

## Sources

### Primary (HIGH confidence)
- https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html — subcommand derive patterns
- https://docs.rs/indicatif/latest/indicatif/ — spinner API, version 0.18.4
- https://docs.rs/anyhow/latest/anyhow/ — context trait, macros, version 1.0.101
- https://docs.rs/which/latest/which/ — which() API, version 8.0.0
- https://docs.rs/tempfile/latest/tempfile/struct.Builder.html — Builder API, version 3.25.0
- https://docs.rs/ctrlc/latest/ctrlc/ — set_handler API, version 3.5.2
- https://doc.rust-lang.org/std/process/struct.Command.html — subprocess patterns
- https://crates.io/api/v1/crates/* — version verification for all crates (Feb 2026)

### Secondary (MEDIUM confidence)
- https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html — owo-colors recommendation (verified: owo-colors v4.2.3 on crates.io)
- https://rust-cli.github.io/book/in-depth/signals.html — ctrlc + Arc<AtomicBool> pattern (verified against ctrlc docs)
- https://github.com/dtolnay/thiserror — thiserror v2.0.18 (cross-verified with crates.io)

### Tertiary (LOW confidence)
- None — all critical claims verified with official docs or crates.io

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified against crates.io API (Feb 2026)
- Architecture: HIGH — patterns verified against official docs; structure follows established Rust CLI conventions
- Pitfalls: HIGH (SIGINT/NamedTempFile, FFmpeg stdin) — documented in official crate docs; MEDIUM (deadlock) — common knowledge cross-verified with multiple sources

**Research date:** 2026-02-19
**Valid until:** 2026-03-19 (stable ecosystem; indicatif and clap iterate fast but APIs are backward-compatible within major versions)
