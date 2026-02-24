# Phase 18: Tuning Flags & Cleanup - Research

**Researched:** 2026-02-24
**Domain:** Rust CLI (clap flags, dead code removal, VAD doctor check)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- `--vad-threshold` default: 0.5 (Silero recommended default)
- `--min-silence-ms` default: 400ms (aggressive cutting for fast-paced content)
- Both flags apply to both `cut` and `pipeline` commands
- Invalid values (out of range) rejected with clear error message -- no clamping
- Hard remove `--breaths` flag: delete entirely from clap args in both cut and pipeline
- No migration hint, no deprecation warning for --breaths
- Full delete all commented-out DEPRECATED code, not just uncomment
- Delete `run_silencedetect()` from ffmpeg.rs entirely
- Delete `parse_silencedetect`, `silence_to_speech`, `filter_silences_by_words`, `SilenceInterval`, `total_silence_removed` from silence.rs
- Keep `total_silence_from_speeches` in silence.rs (actively used)
- Manual audit: systematically search for all silencedetect/amplitude references
- All amplitude-related constants deleted
- Add VAD health check: create VoiceActivityDetector instance to verify ONNX Runtime initializes
- Display: "VAD (Silero V5): OK" or "VAD (Silero V5): FAILED - {error}"
- Update README doctor output example to include VAD check line

### Claude's Discretion
- Whether to review/remove old silencedetect-related doctor checks (if any exist beyond FFmpeg presence)
- Valid ranges for --vad-threshold (likely 0.0-1.0) and --min-silence-ms (likely > 0)
- Exact error message wording for out-of-range values
- How to structure the VAD doctor check function

### Deferred Ideas (OUT OF SCOPE)
None
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VAD-04 | User can tune VAD sensitivity via `--vad-threshold` flag (f32, default 0.5) | Clap `#[arg]` with value_parser for range validation; vad.rs accepts threshold param |
| VAD-05 | User can tune minimum silence duration via `--min-silence-ms` flag (u32, default 400) | Post-processing merge of short gaps in vad.rs run_vad(); clap u32 arg |
| CLN-01 | `--breaths` flag removed from cut and pipeline commands | Delete field from CutArgs/PipelineArgs, remove all usages in cut.rs/pipeline.rs |
| CLN-02 | Dead amplitude-based code removed from silence.rs | Delete SilenceInterval, parse_silencedetect, silence_to_speech, filter_silences_by_words, total_silence_removed, words_to_speech_intervals |
</phase_requirements>

## Summary

Phase 18 is pure internal cleanup and flag plumbing. No new external dependencies, no architecture changes. The work breaks into three areas: (1) add two clap flags that pass through to `vad::run_vad()`, (2) delete the `--breaths` flag and all dead amplitude code, (3) add a VAD health check to doctor and update README.

**Primary recommendation:** Single wave -- all changes are in separate files with minimal overlap. Flag addition touches cli.rs + vad.rs + cut.rs + pipeline.rs. Cleanup touches silence.rs + ffmpeg.rs + cut.rs + pipeline.rs. Doctor touches doctor.rs + README.md. Split into 2 plans to keep reviews clean: Plan 1 = flags + cleanup, Plan 2 = doctor + README.

## Standard Stack

No new dependencies. All work uses existing crate APIs:

| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| clap | (existing) | CLI argument parsing | `#[arg]` attributes for new flags |
| voice_activity_detector | 0.2.1 | VAD inference | `VoiceActivityDetector::builder()` for doctor health check |
| hound | (existing) | WAV reading | Already used in vad.rs |

## Architecture Patterns

### Flag Threading Pattern

Current pattern in codebase: CLI args struct -> command `run()` function -> internal helpers. The two new flags need to reach `vad::run_vad()`. Current signature:

```rust
pub fn run_vad(wav_path: &Path, video_duration: f64) -> anyhow::Result<Vec<SpeechInterval>>
```

Needs to become:

```rust
pub fn run_vad(wav_path: &Path, video_duration: f64, threshold: f32, min_silence_ms: u32) -> anyhow::Result<Vec<SpeechInterval>>
```

Both `cut.rs` and `pipeline.rs` call `vad::run_vad()` directly, so both need the args threaded through.

### min_silence_ms Implementation

The current VAD code processes chunk-by-chunk and emits a `SpeechInterval` whenever speech->non-speech transition happens. `min_silence_ms` means: "don't split on silences shorter than N ms". This is a post-processing merge step:

```rust
// After building raw speeches, merge gaps shorter than min_silence_ms
let min_gap = min_silence_ms as f64 / 1000.0;
let mut merged = Vec::new();
let mut current = speeches.remove(0);
for next in speeches {
    if next.start - current.end < min_gap {
        current.end = next.end; // merge
    } else {
        merged.push(current);
        current = next;
    }
}
merged.push(current);
```

### Clap Range Validation

Use `value_parser` with a custom range:

```rust
#[arg(long, default_value = "0.5", value_parser = clap::value_parser!(f32).range(0.0..=1.0))]
pub vad_threshold: f32,

#[arg(long, default_value = "400", value_parser = clap::value_parser!(u32).range(1..))]
pub min_silence_ms: u32,
```

Note: `clap::value_parser!(f32).range()` -- clap's built-in range validation for f32 produces clear error messages like `error: invalid value '1.5' for '--vad-threshold <VAD_THRESHOLD>': 1.5 is not in 0..=1`. This satisfies the "clear error message, no clamping" decision.

### Doctor VAD Check Pattern

Existing doctor checks follow this pattern:

```rust
fn check_something() -> CheckResult {
    // Try the operation
    // Return CheckResult { name, status, detail }
}
```

VAD check should:
1. Create a `VoiceActivityDetector::builder().sample_rate(16000).chunk_size(512).build()`
2. If Ok -> Status::Ok, detail "VAD (Silero V5): OK"
3. If Err -> Status::Fail, detail "VAD (Silero V5): FAILED - {error}"

This validates ONNX Runtime initializes correctly (the ONNX model is bundled in the binary).

### Dead Code Inventory

**silence.rs -- DELETE:**
- `SilenceInterval` struct (lines 1-5)
- `parse_silencedetect()` function (lines 13-52)
- `silence_to_speech()` function (lines 54-112)
- `filter_silences_by_words()` function (lines 183-198)
- `words_to_speech_intervals()` function (lines 139-181) -- dead, not called anywhere in src/
- `total_silence_removed()` function (lines 234-243)

**silence.rs -- KEEP:**
- `SpeechInterval` struct
- `build_concat_filter()` function
- `adjust_timestamps()` function
- `total_silence_from_speeches()` function

**ffmpeg.rs -- DELETE:**
- `run_silencedetect()` function (lines 116-139, including the `#[allow(dead_code)]` and DEPRECATED comment)

**cut.rs -- DELETE:**
- All commented-out DEPRECATED blocks (lines 14-19, 51-57, 88-92, 133-134)

**pipeline.rs -- DELETE:**
- All commented-out DEPRECATED blocks (lines 14-19, 140-150, 187-190)
- `_breaths` parameter from `run_stages()` (line 97)
- `args.breaths` argument in `run_stages()` call (line 65)

**cli.rs -- DELETE:**
- `breaths` field from `CutArgs` (line 56)
- `breaths` field from `PipelineArgs` (lines 141-142)

**README.md -- UPDATE:**
- Remove `--breaths` from pipeline flags table (line 99)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Flag range validation | Custom validation logic | clap `value_parser!().range()` | Built-in, produces standard error messages |
| ONNX health check | File existence check | `VoiceActivityDetector::builder().build()` | Actually tests runtime initialization |

## Common Pitfalls

### Pitfall 1: Forgetting to thread args through pipeline's run_stages
**What goes wrong:** Pipeline command ignores --vad-threshold and --min-silence-ms
**Why it happens:** pipeline.rs has an intermediate `run_stages()` function that takes individual params
**How to avoid:** Add threshold and min_silence_ms params to run_stages signature, pass from PipelineArgs

### Pitfall 2: Not updating tests
**What goes wrong:** Tests reference deleted structs/functions and fail to compile
**How to avoid:** Search for `SilenceInterval`, `parse_silencedetect`, etc. in test files. Delete or update tests that reference removed code.

### Pitfall 3: Orphaned imports
**What goes wrong:** `use crate::silence::SilenceInterval` or `use crate::ffmpeg::run_silencedetect` left in files
**How to avoid:** `cargo build` will catch these, but audit imports proactively

## Code Examples

### CutArgs with new flags (cli.rs)

```rust
#[derive(Args)]
pub struct CutArgs {
    pub input: PathBuf,

    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value = "0.5", value_parser = clap::value_parser!(f32).range(0.0..=1.0))]
    pub vad_threshold: f32,

    #[arg(long, default_value = "400", value_parser = clap::value_parser!(u32).range(1..))]
    pub min_silence_ms: u32,
}
```

### Updated vad::run_vad signature

```rust
pub fn run_vad(wav_path: &Path, video_duration: f64, threshold: f32, min_silence_ms: u32) -> anyhow::Result<Vec<SpeechInterval>> {
    // ... existing code using threshold instead of VAD_THRESHOLD constant ...
    // ... post-processing merge for min_silence_ms ...
}
```

### Doctor VAD check

```rust
fn check_vad() -> CheckResult {
    match voice_activity_detector::VoiceActivityDetector::builder()
        .sample_rate(16000i64)
        .chunk_size(512usize)
        .build()
    {
        Ok(_) => CheckResult {
            name: "VAD (Silero V5)".to_string(),
            status: Status::Ok,
            detail: String::new(),
        },
        Err(e) => CheckResult {
            name: "VAD (Silero V5)".to_string(),
            status: Status::Fail,
            detail: format!("FAILED - {}", e),
        },
    }
}
```

## Sources

### Primary (HIGH confidence)
- Codebase audit: `src/silence.rs`, `src/vad.rs`, `src/cli.rs`, `src/commands/cut.rs`, `src/commands/pipeline.rs`, `src/commands/doctor.rs`, `src/ffmpeg.rs`
- Phase 17 CONTEXT.md and RESEARCH.md for VAD architecture decisions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies, all internal code changes
- Architecture: HIGH - patterns directly from existing codebase
- Pitfalls: HIGH - straightforward removal + addition, failure modes well-understood

**Research date:** 2026-02-24
**Valid until:** indefinite (internal codebase knowledge)
