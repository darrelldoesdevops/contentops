# Technology Stack

**Project:** contentops
**Researched:** 2026-02-19
**Overall Confidence:** HIGH

## Decision: FFmpeg Orchestration Strategy

The single most important stack decision for this project: **use `std::process::Command` directly, not an FFmpeg binding crate.**

**Why not FFmpeg FFI bindings (rust-ffmpeg, rsmpeg, ffmpeg-next)?**
These crates statically link against FFmpeg's C libraries via FFI. They require a full C toolchain alongside Rust, create compilation complexity (especially cross-platform), introduce unsafe code, and tie you to GPL licensing concerns. For a CLI that shells out to FFmpeg for discrete operations (silence detection, concatenation, encoding), FFI bindings are massive overkill.

**Why not ffmpeg-sidecar?**
ffmpeg-sidecar (v2.4.0) is well-designed and wraps the CLI binary nicely. However, contentops doesn't need its Iterator-based frame processing, automatic FFmpeg download, or progress parsing from stderr. The project runs discrete FFmpeg commands (detect silence, cut segments, concatenate) rather than streaming frames through Rust. Adding ffmpeg-sidecar would be an abstraction layer over `std::process::Command` that doesn't earn its weight for this use case.

**Why `std::process::Command`?**
- Zero additional dependencies for FFmpeg interaction
- Full control over argument construction -- critical when building complex filter graphs
- Direct access to stdout/stderr for parsing silencedetect output
- No abstraction mismatch: you're building FFmpeg command lines, so build FFmpeg command lines
- Easy to debug: log the exact command, paste it into terminal to reproduce

**Confidence: HIGH** -- This matches the project context (PROJECT.md confirms `std::process::Command`), and the FFmpeg CLI approach is well-established in production tools.

## Recommended Stack

### Core Framework

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Rust (edition 2021) | 1.75+ | Language | Type safety, zero-cost abstractions, excellent CLI ecosystem | HIGH |
| clap | 4.5.59 | CLI argument parsing | De facto standard, derive macros for zero-boilerplate CLI definition | HIGH |
| serde | 1.0.228 | Serialization framework | Required for config parsing, JSON output of Whisper results | HIGH |
| serde_json | 1.0.149 | JSON parsing | Parse FFmpeg probe output (ffprobe -print_format json), Whisper JSON output | HIGH |

### Error Handling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| anyhow | 1.0.101 | Application error handling | Personal CLI tool = application code, not a library. anyhow's context chaining (`with_context`) produces readable error messages for pipeline failures | HIGH |

**Why anyhow over thiserror?** thiserror (v2.0.18) is for library code where callers match on error variants. contentops is an application -- errors get displayed to the user, not pattern-matched by consumers. anyhow's `.context("Failed to detect silence in {}")` pattern is exactly right for a pipeline tool.

**Why not both?** For a personal tool, the added complexity of defining error enums with thiserror and wrapping them in anyhow provides no benefit. If contentops ever becomes a library, add thiserror then.

### Logging and Diagnostics

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tracing | 0.1.44 | Structured logging | Industry standard for Rust. Structured spans map naturally to pipeline stages (silence_detect, cut, concat) | HIGH |
| tracing-subscriber | 0.3.22 | Log output formatting | Pairs with tracing. EnvFilter lets you do `RUST_LOG=contentops=debug` for troubleshooting | HIGH |

**Why tracing over env_logger/log?** tracing's span model maps perfectly to a video pipeline: enter span "silence_detect", log events within it, exit. This gives you structured output like `silence_detect{input="video.mp4"}: found 12 silent segments`. env_logger is flat key-value logging without this hierarchy.

### File and Path Utilities

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tempfile | 3.25.0 | Temporary file management | Secure temp file creation with automatic cleanup on drop. Critical for intermediate FFmpeg outputs (segment files, concat lists) | HIGH |
| which | 8.0.0 | Find FFmpeg/ffprobe in PATH | Fail-fast at startup if FFmpeg isn't installed. Better error message than cryptic "No such file or directory" from Command | HIGH |

### CLI UX

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| indicatif | 0.18.4 | Progress bars | FFmpeg operations take seconds-to-minutes. A spinner or progress bar prevents "is it stuck?" anxiety. MultiProgress supports concurrent pipeline stages | MEDIUM |

**Why MEDIUM confidence on indicatif?** For v0.1 with simple silence removal, a spinner may be premature. Consider adding in a later phase when processing time increases with captioning. Could start with just tracing output and add indicatif when the UX pain is felt.

### Testing

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| assert_cmd | 2.1.2 | CLI integration testing | Test the actual binary end-to-end: give it a video, check exit code and output | HIGH |
| predicates | 3.1.4 | Test assertions | Pairs with assert_cmd for readable assertions on stdout/stderr content | HIGH |

### Future Phase: Captioning

| Technology | Version | Purpose | When | Confidence |
|------------|---------|---------|------|------------|
| whisper-rs | 0.15.1 | Local speech-to-text | Phase 2+ when captioning is added | MEDIUM |
| regex | 1.12.3 | Text parsing | Parsing FFmpeg silencedetect output, SRT/VTT timestamp parsing | HIGH |

**Why whisper-rs over shelling out to whisper.cpp CLI?** whisper-rs provides Rust bindings to whisper.cpp, giving type-safe access to model loading, transcription parameters, and segment timestamps. For captioning, you need structured access to word-level timestamps (not just text output), which the library API provides cleanly. The CLI would require parsing stdout which is fragile.

**Why MEDIUM confidence on whisper-rs?** v0.15.1 was released September 2025. The crate wraps whisper.cpp which is actively developed -- API breakage between whisper.cpp versions is a known issue. Verify compatibility with current whisper.cpp at implementation time.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| FFmpeg interaction | `std::process::Command` | ffmpeg-sidecar 2.4.0 | Unnecessary abstraction for discrete command execution; adds dependency for no gain in this use case |
| FFmpeg interaction | `std::process::Command` | rust-ffmpeg / ffmpeg-next | FFI complexity, C toolchain required, GPL concerns, overkill for CLI orchestration |
| Error handling | anyhow | thiserror | Application code, not library code. No callers matching on error variants |
| Error handling | anyhow | eyre + color-eyre | color-eyre provides pretty error reports, but adds complexity. anyhow is simpler and sufficient for personal tool |
| Logging | tracing | log + env_logger | tracing's span model maps better to pipeline stages; env_logger is flat |
| Config format | Feature flags (clap) | TOML config file | v0.1 scope -- feature flags are sufficient. TOML config deferred per PROJECT.md |
| Console colors | None (v0.1) | owo-colors | Not needed until UX polish phase. tracing-subscriber handles log coloring |
| Whisper | whisper-rs | Shell out to whisper CLI | Need structured word-level timestamps for captioning; CLI parsing is fragile |

## Cargo.toml Dependencies

```toml
[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
regex = "1.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tempfile = "3.25"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
which = "8.0"

# Future: captioning phase
# whisper-rs = "0.15"
# indicatif = "0.18"

[dev-dependencies]
assert_cmd = "2.1"
predicates = "3.1"
```

**Note on version pinning:** Use semver ranges (e.g., `"1.0"` not `"1.0.101"`) in Cargo.toml. Cargo.lock pins exact versions. This is standard Rust practice -- you get patch updates automatically while Cargo.lock ensures reproducible builds.

## What NOT to Install

| Technology | Why Not |
|------------|---------|
| tokio / async-std | No async needed. FFmpeg commands are sequential pipeline stages. async adds complexity with zero benefit for `Command::output().await` vs `Command::output()` |
| ffmpeg-sidecar | Abstraction over Command that doesn't match this project's discrete-command pattern |
| colored / owo-colors | tracing-subscriber already colors log output. Defer explicit coloring to UX polish phase |
| toml (crate) | No config files in v0.1. Feature flags via clap are sufficient per PROJECT.md |
| rayon | Parallelism not needed for single-video pipeline. FFmpeg itself is multi-threaded |

## Build and Development

```bash
# Prerequisites
brew install ffmpeg          # macOS -- includes ffprobe
rustup update stable         # Ensure recent Rust toolchain

# Project setup
cargo init contentops
cd contentops

# Verify FFmpeg is available
ffmpeg -version
ffprobe -version

# Development workflow
cargo build                  # Debug build
cargo test                   # Run tests (needs sample video fixtures)
cargo run -- --help          # Test CLI
RUST_LOG=contentops=debug cargo run -- input.mp4 --remove-silence  # Debug logging
```

## Sources

- [clap 4.5.59](https://docs.rs/crate/clap/latest) -- docs.rs, verified 2026-02-19
- [serde 1.0.228](https://docs.rs/crate/serde/latest) -- docs.rs, verified 2026-02-19
- [serde_json 1.0.149](https://docs.rs/crate/serde_json/latest) -- docs.rs, verified 2026-02-19
- [anyhow 1.0.101](https://docs.rs/crate/anyhow/latest) -- docs.rs, verified 2026-02-19
- [thiserror 2.0.18](https://docs.rs/crate/thiserror/latest) -- docs.rs, verified 2026-02-19
- [tracing 0.1.44](https://docs.rs/crate/tracing/latest) -- docs.rs, verified 2026-02-19
- [tracing-subscriber 0.3.22](https://docs.rs/crate/tracing-subscriber/latest) -- docs.rs, verified 2026-02-19
- [tempfile 3.25.0](https://docs.rs/crate/tempfile/latest) -- docs.rs, verified 2026-02-19
- [which 8.0.0](https://docs.rs/crate/which/latest) -- docs.rs, verified 2026-02-19
- [indicatif 0.18.4](https://docs.rs/crate/indicatif/latest) -- docs.rs, verified 2026-02-19
- [whisper-rs 0.15.1](https://docs.rs/crate/whisper-rs/latest) -- docs.rs, verified 2026-02-19
- [regex 1.12.3](https://docs.rs/crate/regex/latest) -- docs.rs, verified 2026-02-19
- [assert_cmd 2.1.2](https://docs.rs/crate/assert_cmd/latest) -- docs.rs, verified 2026-02-19
- [predicates 3.1.4](https://docs.rs/crate/predicates/latest) -- docs.rs, verified 2026-02-19
- [ffmpeg-sidecar 2.4.0](https://github.com/nathanbabcock/ffmpeg-sidecar) -- GitHub, verified 2026-02-19
- [ffmpeg-sidecar vs rust-ffmpeg discussion](https://github.com/nathanbabcock/ffmpeg-sidecar/issues/34) -- GitHub issue
- [Rain's Rust CLI recommendations on colors](https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html)
