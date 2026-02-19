# Architecture Patterns

**Domain:** Rust CLI video processing pipeline (FFmpeg orchestration)
**Researched:** 2026-02-19

## Recommended Architecture

A **sequential pipeline of discrete stages**, each wrapping one or more FFmpeg (or Whisper) subprocess invocations. Stages communicate through intermediate files on disk, not in-memory streams. The CLI layer parses args, constructs a pipeline configuration, then hands off to the pipeline executor which runs stages in order with fail-fast semantics.

```
CLI (clap) --> PipelineConfig --> PipelineExecutor
                                      |
                    +--Stage 1--+--Stage 2--+--Stage 3--+
                    | Silence   | Caption   | Overlay   |
                    | Removal   | (Whisper) | (drawtext)|
                    +-----------+-----------+-----------+
                    Each stage: input file --> FFmpeg/Whisper subprocess --> output file
```

### Why This Shape

1. **FFmpeg is the bottleneck, not Rust.** The tool orchestrates subprocesses; it does not decode video frames in Rust. Simplicity beats abstraction.
2. **Intermediate files are correct here.** Video files are large; piping raw frames between stages via stdout/stdin adds complexity (sync, buffering, error recovery) with no benefit for a personal tool processing one file at a time.
3. **Stages are independently testable.** Each stage can be run alone, debugged with the exact FFmpeg command, and swapped without touching other stages.

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **CLI Layer** (`main.rs`, `cli.rs`) | Arg parsing (clap), validation, entry point | PipelineConfig |
| **PipelineConfig** (`config.rs`) | Holds stage parameters, file paths, feature flags | Pipeline Executor |
| **Pipeline Executor** (`pipeline.rs`) | Runs stages in sequence, manages temp files, fail-fast | Individual Stages |
| **FFmpeg Runner** (`ffmpeg.rs`) | Wraps `std::process::Command` for FFmpeg invocations. Builds args, captures stderr, checks exit code. Single abstraction for all FFmpeg calls. | Stages (used by) |
| **Stage: Silence Removal** (`stages/silence.rs`) | Two-pass: (1) silencedetect parse, (2) trim+concat | FFmpeg Runner |
| **Stage: Caption Generation** (`stages/caption.rs`) | Extract audio, invoke Whisper CLI, parse SRT output | FFmpeg Runner + Whisper subprocess |
| **Stage: Text Overlay** (`stages/overlay.rs`) | Apply drawtext/subtitles filter to burn captions | FFmpeg Runner |
| **Temp File Manager** (`tempfiles.rs`) | Create/track/cleanup intermediate files using `tempfile` crate | Pipeline Executor |
| **Error Types** (`error.rs`) | Unified error enum with `thiserror` | All components |

## Data Flow

### File-Based Pipeline

```
input.mp4
    |
    v
[Stage 1: Silence Removal]
    |  Pass 1: ffmpeg -i input.mp4 -af silencedetect=n=-30dB:d=0.5 -f null -
    |           --> parse stderr for silence_start/silence_end timestamps
    |  Pass 2: ffmpeg -i input.mp4 -vf "select='...'" -af "aselect='...'" desilenced.mp4
    |
    v
desilenced.mp4 (temp)
    |
    v
[Stage 2: Caption Generation]
    |  Step A: ffmpeg -i desilenced.mp4 -ar 16000 -ac 1 -c:a pcm_s16le audio.wav
    |  Step B: whisper audio.wav --model medium --output_format srt
    |           --> produces audio.srt
    |
    v
audio.srt (temp) + desilenced.mp4 (temp)
    |
    v
[Stage 3: Text Overlay]
    |  ffmpeg -i desilenced.mp4 -vf "subtitles=audio.srt:force_style='...'" \
    |         -c:v libx264 -c:a aac output.mp4
    |
    v
output.mp4 (final)
    |
    v
[Cleanup: remove all temp files]
```

### Information Flow (Config --> Execution)

```
User CLI args
    |
    v
clap parse --> PipelineConfig {
    input: PathBuf,
    output: PathBuf,
    stages: Vec<StageConfig>,  // which stages to run, in order
    silence_threshold: f64,     // -30dB default
    silence_duration: f64,      // 0.5s default
    whisper_model: String,      // "medium" default
    caption_style: CaptionStyle,
    overwrite: bool,
}
    |
    v
PipelineExecutor::run(config) {
    let mut current_file = config.input.clone();
    for stage in &config.stages {
        let output = temp_manager.next_temp_file(".mp4");
        stage.execute(&current_file, &output, &config)?;  // fail-fast with ?
        current_file = output;
    }
    fs::rename(current_file, config.output)?;
    temp_manager.cleanup();
}
```

## Core Abstractions

### The Stage Trait

Use an **enum dispatch** rather than trait objects. With a fixed, small set of stages known at compile time, enum dispatch avoids the indirection of `dyn Trait` and plays well with feature flags.

```rust
#[derive(Debug)]
pub enum Stage {
    #[cfg(feature = "silence")]
    SilenceRemoval(SilenceConfig),
    #[cfg(feature = "caption")]
    CaptionGeneration(CaptionConfig),
    #[cfg(feature = "overlay")]
    TextOverlay(OverlayConfig),
}

impl Stage {
    pub fn execute(
        &self,
        input: &Path,
        output: &Path,
        ffmpeg: &FfmpegRunner,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "silence")]
            Stage::SilenceRemoval(cfg) => silence::execute(input, output, cfg, ffmpeg),
            #[cfg(feature = "caption")]
            Stage::CaptionGeneration(cfg) => caption::execute(input, output, cfg, ffmpeg),
            #[cfg(feature = "overlay")]
            Stage::TextOverlay(cfg) => overlay::execute(input, output, cfg, ffmpeg),
        }
    }
}
```

### The FFmpeg Runner

Centralized subprocess wrapper. Every FFmpeg call goes through this, ensuring consistent error handling, logging, and potential progress tracking.

```rust
pub struct FfmpegRunner {
    ffmpeg_path: PathBuf,  // located at startup via `which ffmpeg`
}

impl FfmpegRunner {
    /// Run an FFmpeg command. Returns Ok(()) on success, Err with stderr on failure.
    pub fn run(&self, args: &[&str]) -> Result<()> { ... }

    /// Run FFmpeg and capture stdout (for probing).
    pub fn run_capture(&self, args: &[&str]) -> Result<String> { ... }

    /// Run FFmpeg and capture stderr for parsing (silencedetect output).
    pub fn run_parse_stderr(&self, args: &[&str]) -> Result<String> { ... }
}
```

### Error Handling

Use `thiserror` for the error enum (structured, matchable errors) and `anyhow` at the binary boundary (`main.rs`) for ergonomic error display. This is the standard Rust pattern for application-level CLIs.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ContentOpsError {
    #[error("FFmpeg failed (exit {exit_code}): {stderr}")]
    FfmpegFailed { exit_code: i32, stderr: String },

    #[error("Whisper failed: {0}")]
    WhisperFailed(String),

    #[error("Failed to parse silence detection output: {0}")]
    SilenceParseError(String),

    #[error("Input file not found: {0}")]
    InputNotFound(PathBuf),

    #[error("FFmpeg not found on PATH")]
    FfmpegNotFound,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

## Feature Flags Design

Use Cargo features to gate stages so the binary can be built with only the stages needed.

```toml
[features]
default = ["silence", "caption", "overlay"]
silence = []
caption = []  # requires whisper CLI on PATH at runtime, not compile time
overlay = []
```

Feature flags control **compilation** of stage modules. Runtime dependency checks (is `ffmpeg` on PATH? is `whisper` on PATH?) happen at startup before pipeline execution.

## Patterns to Follow

### Pattern 1: Dependency Validation at Startup

Check that external tools exist before running any stages.

```rust
fn validate_dependencies(config: &PipelineConfig) -> Result<()> {
    FfmpegRunner::locate()?;  // errors if ffmpeg not on PATH
    if config.has_stage(StageKind::Caption) {
        which::which("whisper")
            .map_err(|_| ContentOpsError::WhisperNotFound)?;
    }
    Ok(())
}
```

### Pattern 2: Structured Stderr Parsing

FFmpeg outputs diagnostic info to stderr. Parse it with regex for silence timestamps rather than fragile string splitting.

```rust
// Parse lines like: [silencedetect @ 0x...] silence_end: 3.504 | silence_duration: 1.204
static SILENCE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"silence_end:\s*([\d.]+)\s*\|\s*silence_duration:\s*([\d.]+)").unwrap()
});
```

### Pattern 3: Temp File Lifecycle Tied to Pipeline

Use `tempfile::TempDir` for the entire pipeline run. All intermediate files go in one temp directory. On success or failure, the directory drops and cleans up.

```rust
let work_dir = tempfile::tempdir()?;
// All intermediate files created inside work_dir
// Automatic cleanup on drop (success or error)
```

### Pattern 4: Dry-Run Mode

Print the FFmpeg commands that would be executed without running them. Critical for debugging and trust.

```rust
if config.dry_run {
    println!("Would run: ffmpeg {}", args.join(" "));
    return Ok(());
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Trait Objects for a Fixed Stage Set

**What:** `Vec<Box<dyn Stage>>` with dynamic dispatch for 3 known stages.
**Why bad:** Adds complexity (object safety constraints, lifetime headaches) with no benefit. You know all stages at compile time.
**Instead:** Enum dispatch with `#[cfg(feature)]` gating.

### Anti-Pattern 2: In-Memory Frame Piping Between Stages

**What:** Piping raw video frames between stages via stdin/stdout.
**Why bad:** Massive memory overhead for raw frames. Synchronization complexity. No benefit for sequential file-to-file processing.
**Instead:** Intermediate files in a temp directory. FFmpeg handles I/O efficiently.

### Anti-Pattern 3: FFmpeg Bindings (ffmpeg-next, rsmpeg)

**What:** Using Rust FFI bindings to libavcodec/libavformat.
**Why bad:** Massive compilation complexity (requires system FFmpeg dev libraries). Brittle across FFmpeg versions. The CLI tool does not need frame-level access -- it orchestrates complete FFmpeg commands.
**Instead:** `std::process::Command` wrapping the `ffmpeg` binary.

### Anti-Pattern 4: Async Runtime for Sequential Processing

**What:** Pulling in tokio for subprocess orchestration.
**Why bad:** This is a sequential pipeline processing one file. Async adds cognitive overhead, binary size, and compile time with zero benefit.
**Instead:** Synchronous `std::process::Command` with blocking execution.

### Anti-Pattern 5: Global Mutable State for Stage Configuration

**What:** Static config or global variables.
**Why bad:** Testing nightmare, hidden coupling.
**Instead:** Pass `&PipelineConfig` (or stage-specific config) through function parameters.

## Project Structure

```
contentops/
  Cargo.toml
  src/
    main.rs              # Entry point: parse args, validate deps, run pipeline
    cli.rs               # Clap derive structs
    config.rs            # PipelineConfig, StageConfig types
    error.rs             # ContentOpsError enum (thiserror)
    pipeline.rs          # PipelineExecutor: stage sequencing, temp file management
    ffmpeg.rs            # FfmpegRunner: subprocess wrapper
    stages/
      mod.rs             # Stage enum, feature-gated re-exports
      silence.rs         # Silence detection + removal logic
      caption.rs         # Audio extraction + Whisper invocation + SRT parsing
      overlay.rs         # drawtext/subtitles filter application
```

## Suggested Build Order

Build order follows data flow dependencies. Each phase produces a working (if incomplete) CLI.

| Order | Component | Depends On | Rationale |
|-------|-----------|------------|-----------|
| 1 | `error.rs` + `ffmpeg.rs` | Nothing | Foundation. Everything calls FFmpeg. Build the runner first. |
| 2 | `cli.rs` + `config.rs` | Nothing | Define the interface. Can iterate on UX independently. |
| 3 | `pipeline.rs` + temp file management | error, ffmpeg, config | The orchestrator that ties stages together. Build with a no-op stage first. |
| 4 | `stages/silence.rs` | ffmpeg, pipeline | First real stage. Most complex (two-pass). Proves the pipeline pattern works. |
| 5 | `stages/caption.rs` | ffmpeg, pipeline | Second stage. Introduces Whisper as external dependency. |
| 6 | `stages/overlay.rs` | ffmpeg, pipeline, caption output | Final stage. Depends on SRT files from caption stage. |

**Key dependency insight:** Silence removal is independent of captioning. Overlay depends on captioning output (SRT file). Build silence first because it exercises the full pipeline pattern (multi-pass FFmpeg) without needing Whisper installed.

## Future Architecture Considerations

### FFmpeg 8.0 Native Whisper Filter

FFmpeg 8.0 (released August 2025) includes a native `whisper` audio filter that can generate subtitles directly within an FFmpeg pipeline. This could eventually collapse the Caption + Overlay stages into a single FFmpeg invocation. However:

- FFmpeg 8.0 may not be widely available on macOS via Homebrew yet
- The filter requires whisper.cpp models to be downloaded separately
- For a personal macOS tool, the current two-step approach (external Whisper CLI + FFmpeg overlay) is more portable and debuggable

**Recommendation:** Build with the subprocess approach now. Add an FFmpeg 8.0 native path as a future optimization behind a feature flag.

### Adding New Stages

The architecture supports new stages by:
1. Adding a new file in `stages/`
2. Adding a variant to the `Stage` enum
3. Adding a feature flag in `Cargo.toml`
4. Adding CLI args for the new stage in `cli.rs`

No changes to `pipeline.rs` needed beyond stage ordering.

## Sources

- [ffmpeg-sidecar crate](https://github.com/nathanbabcock/ffmpeg-sidecar) - Builder pattern reference for FFmpeg CLI wrapping
- [FFmpeg silencedetect filter docs](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html) - Silence detection parameters and metadata output
- [Remsi silence removal](https://github.com/bambax/Remsi) - Two-pass silence removal architecture (detect then select/aselect)
- [remove-silence](https://github.com/onsubmit/remove-silence) - Four-step silence removal pipeline (probe, detect, segment, concat)
- [Rust error handling with thiserror/anyhow](https://www.shakacode.com/blog/thiserror-anyhow-or-how-i-handle-errors-in-rust-apps/) - Library vs application error pattern
- [std::process::Command error handling](https://users.rust-lang.org/t/best-error-handing-practices-when-using-std-command/42259) - Wrapper function patterns
- [Rust async pipeline pattern](https://github.com/alexpusch/rust-magic-patterns/blob/master/async-pipeline-pattern/Readme.md) - Pipeline stage architecture (used as counter-example: async not needed here)
- [tempfile crate](https://docs.rs/tempfile/) - Temp directory lifecycle management
- [FFmpeg 8.0 Whisper integration](https://gigazine.net/gsc_news/en/20250825-ffmpeg-8-0-huffman) - Native transcription filter (future consideration)
- [Clap CLI structure](https://kbknapp.dev/cli-structure-01/) - Subcommand and modular CLI design patterns
- [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html) - Feature flag conditional compilation
