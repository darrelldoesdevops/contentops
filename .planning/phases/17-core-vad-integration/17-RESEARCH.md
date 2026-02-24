# Phase 17: Core VAD Integration - Research

**Researched:** 2026-02-24
**Domain:** Rust audio processing / Silero VAD via ONNX Runtime
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Switchover strategy
- Full replacement: VAD completely replaces silencedetect in both cut and pipeline commands -- no fallback path
- Old amplitude-based code paths should be commented out (marked deprecated) so Phase 18 knows exactly what to remove
- The `--breaths` flag is silently ignored in this phase (sole user, no deprecation warning needed); Phase 18 removes it entirely

#### Speech detection behavior
- Aggressive cutting: tight speech boundaries, minimal padding -- maximize dead air removal for fast-paced talking-head content
- Remove all detected silence gaps, even very short ones -- no merging of close speech segments
- Videos are pure talking-head (no intro music, no SFX) -- VAD's speech/non-speech classification is sufficient
- Show summary stats after VAD processing: "Found X speech segments, removing Y seconds of silence"

#### Shared audio extraction
- Share one WAV file: extract 16kHz mono WAV once, reuse for both VAD and Whisper in pipeline
- `cut` command uses the same shared helper (one code path for audio extraction everywhere)
- Temp WAV file registered with existing TempFileRegistry -- cleaned up when command completes

#### Error handling
- Fail with clear error if ONNX Runtime fails to initialize -- no silent fallback to silencedetect
- Error with warning if VAD produces zero speech segments: "No speech detected in input" and exit with error
- Existing spinner is sufficient for VAD progress -- no VAD-specific progress indicator needed
- Doctor updates deferred to Phase 18

### Claude's Discretion
- Whether shared helper lives in ffmpeg.rs or a new module
- Whether to extract WAV as temp file or stream audio to VAD
- VAD chunk size and accumulation loop implementation details
- Exact spinner message text during VAD processing

### Deferred Ideas (OUT OF SCOPE)
- None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VAD-01 | Cut command detects speech using Silero VAD instead of FFmpeg silencedetect | `cut.rs` run() refactored: replace `run_silencedetect` + `parse_silencedetect` with `extract_16k_wav` + VAD inference loop |
| VAD-02 | Pipeline command detects speech using Silero VAD instead of amplitude-based detection | `pipeline.rs` Stage 3 refactored: WAV already extracted for Whisper (Stage 1) → reuse for VAD; replace silencedetect block |
| VAD-03 | Shared `ffmpeg::extract_16k_wav` helper extracts 16kHz mono audio for both VAD and Whisper | New function in `ffmpeg.rs`; caption.rs transcribe() calls it; cut.rs calls it; pipeline Stage 3 reuses Stage 1's WAV |
</phase_requirements>

---

## Summary

Phase 17 replaces FFmpeg `silencedetect` amplitude-based silence removal with neural VAD inference from the `voice_activity_detector 0.2.1` crate (Silero VAD V5, bundled ONNX, pinned to `ort =2.0.0-rc.10`). The crate is already in `Cargo.toml` and resolves correctly.

The primary implementation tasks are: (1) add a `ffmpeg::extract_16k_wav` helper that deduplicates the existing 3 copies of 16kHz WAV extraction scattered across `caption.rs`; (2) implement a VAD inference function that reads the WAV as i16 samples via `hound` and drives `LabelIterator` to collect speech intervals; (3) replace silencedetect calls in `cut.rs` and `pipeline.rs` with the new VAD path.

**Primary recommendation:** Use `LabelIterator` (`.label(&mut vad, threshold, padding_chunks)`) rather than a manual predict loop -- it handles the padding state machine internally and yields `LabeledAudio` variants that trivially map to speech/non-speech intervals. Track chunk index to recover timestamps.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `voice_activity_detector` | 0.2.1 | Silero VAD V5 ONNX inference | Already in Cargo.toml; bundles model; handles ORT session lifecycle |
| `ort` | =2.0.0-rc.10 | ONNX Runtime backend | Pinned by voice_activity_detector; MUST NOT upgrade independently |
| `ort-sys` | =2.0.0-rc.10 | Native ORT bindings | Same pin reason |
| `hound` | 3.x | WAV file reading (i16 samples) | De-facto Rust WAV library; dev-dep in voice_activity_detector itself |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `indicatif` (existing) | 0.18 | Spinner during VAD processing | Already in Cargo.toml; use `ui::make_spinner` |
| `tempfile` (existing) | 3.25 | Temp WAV file management | Already used; WAV temp registered with TempFileRegistry |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `hound` for WAV reading | `byteorder` + manual parse | byteorder is already in tree (via ort) but hand-rolling WAV header parse is brittle |
| `LabelIterator` | Manual predict loop | Predict loop requires hand-coding the padding state machine; LabelIterator already implements it correctly |
| WAV temp file | Pipe raw PCM to VAD | Temp file allows reuse for Whisper in pipeline; piping would complicate the shared-helper design |

**Installation:**
```toml
# Cargo.toml -- add hound
hound = "3.5"
```

---

## Architecture Patterns

### Recommended Project Structure

No new modules needed. Changes are confined to:

```
src/
├── ffmpeg.rs          # add extract_16k_wav() helper
├── commands/
│   ├── cut.rs         # replace silencedetect with VAD
│   ├── pipeline.rs    # replace silencedetect with VAD; reuse WAV from Stage 1
│   └── caption.rs     # call ffmpeg::extract_16k_wav() instead of inline extraction
```

A new `src/vad.rs` module is an option (Claude's Discretion), but since the VAD logic is ~50 lines and already depends on `ffmpeg.rs` for the WAV path, colocating in `ffmpeg.rs` or a minimal `vad.rs` both work.

### Pattern 1: extract_16k_wav Helper

**What:** Single function that wraps the 16kHz mono WAV extraction that is currently duplicated in `caption.rs:transcribe()` (line 392), `caption.rs:run()` (line 596), and will be needed by `cut.rs` and `pipeline.rs`.

**Signature:**
```rust
// Source: analysis of existing caption.rs + ffmpeg.rs patterns
pub fn extract_16k_wav(
    input: &str,
    dest: &Path,
    verbose: bool,
) -> Result<(), std::io::Error>
```

**Implementation:**
```rust
pub fn extract_16k_wav(input: &str, dest: &Path, verbose: bool) -> Result<(), std::io::Error> {
    let dest_str = dest.to_string_lossy();
    let args = ["-i", input, "-ar", "16000", "-ac", "1", "-f", "wav", &dest_str];
    let output = if verbose {
        run_ffmpeg_verbose(&args)?
    } else {
        run_ffmpeg(&args)?
    };
    if !output.success {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("ffmpeg WAV extraction failed with code {:?}", output.exit_code),
        ));
    }
    Ok(())
}
```

### Pattern 2: VAD Inference Loop

**What:** Read WAV samples, run `LabelIterator`, collect speech intervals with timestamps.

**Key facts from source inspection:**
- `voice_activity_detector::VoiceActivityDetector::builder().sample_rate(16000i64).chunk_size(512usize).build()` — sample_rate takes `i64`, chunk_size takes `usize`; for 16kHz, chunk_size MUST be 512 (enforced by Silero V5 model; other values return `VadConfigError`)
- `builder().build()` returns `Result<VoiceActivityDetector, voice_activity_detector::Error>` (single error variant: `VadConfigError { sample_rate, chunk_size }`)
- `predict()` is `&mut self` -- requires mutable VAD; `LabelIterator` borrows it mutably for its lifetime
- `IteratorExt::label(self, vad, threshold, padding_chunks)` yields `LabeledAudio<T>` (Speech or NonSpeech)
- Chunk duration at 16kHz, 512 samples: **32ms per chunk**
- `LabeledAudio::is_speech()` → bool; variants are `Speech(Vec<T>)` and `NonSpeech(Vec<T>)`

**Chunk-to-timestamp mapping:**
```
chunk_index * 512 / 16000.0  → start_seconds
(chunk_index + 1) * 512 / 16000.0  → end_seconds
```

**Implementation pattern (Claude's Discretion: threshold and padding_chunks values):**
```rust
// Source: voice_activity_detector/src/vad.rs + iterator/label.rs source
use voice_activity_detector::{IteratorExt, LabeledAudio, VoiceActivityDetector};
use crate::silence::SpeechInterval;

pub fn run_vad(wav_path: &Path) -> anyhow::Result<Vec<SpeechInterval>> {
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|e| anyhow::anyhow!("WAV open failed: {}", e))?;

    // Verify format assertion (16kHz mono, from extract_16k_wav)
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16000, "VAD requires 16kHz audio");
    assert_eq!(spec.channels, 1, "VAD requires mono audio");

    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(16000i64)
        .chunk_size(512usize)
        .build()
        .map_err(|e| anyhow::anyhow!("ORT init failed: {}", e))?;

    // Collect all samples first (file is temp; memory is ~3MB per minute at 16kHz i16)
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let chunk_secs = 512.0f64 / 16000.0;  // 0.032s per chunk
    let mut speeches: Vec<SpeechInterval> = Vec::new();
    let mut chunk_idx: usize = 0;
    let mut in_speech = false;
    let mut speech_start = 0.0f64;

    // Option A: Use LabelIterator (recommended -- handles padding state machine)
    let threshold = 0.5f32;   // Claude's Discretion
    let padding_chunks = 0usize; // aggressive cutting = 0 padding (per user decision)

    for label in samples.into_iter().label(&mut vad, threshold, padding_chunks) {
        let start = chunk_idx as f64 * chunk_secs;
        let end = (chunk_idx + 1) as f64 * chunk_secs;
        match label {
            LabeledAudio::Speech(_) => {
                if !in_speech {
                    speech_start = start;
                    in_speech = true;
                }
            }
            LabeledAudio::NonSpeech(_) => {
                if in_speech {
                    speeches.push(SpeechInterval { start: speech_start, end: start });
                    in_speech = false;
                }
            }
        }
        chunk_idx += 1;
    }
    // Flush trailing speech
    if in_speech {
        speeches.push(SpeechInterval {
            start: speech_start,
            end: chunk_idx as f64 * chunk_secs,
        });
    }

    Ok(speeches)
}
```

**Note on `padding_chunks`:** The user decision is "no merging of close speech segments" and "tight speech boundaries". Set `padding_chunks = 0` for zero pre/post padding. If testing shows over-cutting (clipping word starts/ends), a value of 1-3 chunks (32-96ms) can be tuned -- this is in Claude's Discretion.

### Pattern 3: Replacing silencedetect in cut.rs

Current code path (to be commented out and marked `// DEPRECATED: Phase 18 removes`):
```rust
// DEPRECATED: Phase 18 removes
// let (threshold, min_duration) = if args.breaths { ... };
// let stderr = ffmpeg::run_silencedetect(...);
// let silences = silence::parse_silencedetect(&stderr, video_duration);
```

New code path:
1. `ffmpeg::extract_16k_wav(&input_str, &wav_path, verbose)?`
2. `let speeches = vad::run_vad(&wav_path)?` (or inline in ffmpeg.rs)
3. Use `speeches` directly (already `Vec<SpeechInterval>`) -- skip `silence_to_speech`
4. `silence::build_concat_filter(&speeches)` -- reuse existing function unchanged

The `args.breaths` field: silently ignore (no branch, no warning). `--breaths` flag still compiles but has no effect. The `SPEECH_PADDING` constant: no longer applied (VAD handles boundaries); `silence_to_speech` not called.

### Pattern 4: Replacing silencedetect in pipeline.rs

Pipeline Stage 3 reuses the WAV that was already extracted in Stage 1 (Whisper transcription):

**Stage 1 (Whisper):** Caption's `transcribe()` calls `extract_16k_wav` → produces WAV → Whisper reads it → keep WAV alive (don't delete after transcription)

**Stage 3 (VAD):** Reuse the same WAV path from Stage 1 → run VAD → get `SpeechInterval` list

This requires the Whisper stage to return or pass through the WAV path. In current `pipeline.rs`, `caption::transcribe()` is responsible for creating and cleaning up its own WAV. We need to either:
- **(Option A)** Extract WAV before calling `transcribe()`, pass WAV path in, and `transcribe()` uses it instead of creating its own. Requires modifying `transcribe()` signature.
- **(Option B)** Keep `transcribe()` signature unchanged; do a second WAV extraction for VAD. Simpler but wastes disk I/O.

Given the user decision ("share one WAV file"), Option A is required. The `transcribe()` signature needs a `wav_path: Option<&Path>` parameter or similar. Claude's Discretion on exact approach.

### Anti-Patterns to Avoid

- **Calling `silence_to_speech` on VAD output:** VAD `run_vad` returns `Vec<SpeechInterval>` directly (not silences). Calling `silence_to_speech` would invert and re-invert, applying unwanted padding.
- **Using i8/u8 samples from hound:** The WAV extracted by ffmpeg (no `-c:a pcm_f32le`) is standard 16-bit PCM (s16le). Use `reader.samples::<i16>()`.
- **Upgrading ort independently:** `voice_activity_detector` pins `ort =2.0.0-rc.10` with `=` (exact). Any `cargo update` of ort will break compilation.
- **Building VAD outside `LazyLock`:** The crate already uses `LazyLock<Arc<Mutex<Session>>>` for the default session -- constructing multiple `VoiceActivityDetector` instances is fine (they share the session via `Arc`), but there's no need to cache the detector across calls.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VAD state machine with padding | Manual chunk accumulation with padding buffer | `LabelIterator` via `.label()` | `label.rs` already implements flush states (Idle/FlushStartPadding/Active/FlushEndPadding) with a VecDeque buffer correctly |
| WAV file reading | Manual RIFF header parse + sample decode | `hound` WavReader | WAV format has many sub-formats; hound handles PCM variants correctly |
| ORT session management | Custom session builder with model bytes | `VoiceActivityDetector::builder().build()` | Crate uses `LazyLock<Arc<Mutex<Session>>>` with the bundled model bytes via `include_bytes!` |

**Key insight:** The `predict()` method on `VoiceActivityDetector` does NOT take a chunk index or timestamp -- timestamp reconstruction is the caller's responsibility using `chunk_index * chunk_duration`.

---

## Common Pitfalls

### Pitfall 1: Wrong chunk_size for 16kHz
**What goes wrong:** `VoiceActivityDetector::builder().sample_rate(16000).chunk_size(256).build()` returns `Err(VadConfigError)` because Silero V5 enforces 512 samples at 16kHz.
**Why it happens:** The model has fixed input dimensions (256 for 8kHz, 512 for 16kHz) -- other values produce a config error.
**How to avoid:** Always use `chunk_size(512usize)` with `sample_rate(16000i64)`.
**Warning signs:** Runtime `VadConfigError { sample_rate: 16000, chunk_size: X }`.

### Pitfall 2: sample_rate type
**What goes wrong:** `sample_rate(16000u32)` -- the builder setter takes `Into<i64>`, not `Into<u32>`.
**How to avoid:** Use `sample_rate(16000i64)` explicitly.

### Pitfall 3: Timestamp drift from partial final chunk
**What goes wrong:** If audio length is not a multiple of 512 samples, the last chunk is padded with zeros by `predict()`. The chunk_idx counter counts emitted chunks, but the padded final chunk's "end" timestamp may overshoot actual audio duration.
**How to avoid:** Clamp final speech interval end to `video_duration` (obtained via `ffmpeg::probe_duration_strict`).

### Pitfall 4: Reuse WAV path in pipeline -- lifetime issues
**What goes wrong:** If `caption::transcribe()` deletes the WAV file at function exit (via `NamedTempFile` drop), the pipeline can't reuse it for VAD.
**How to avoid:** Either (a) keep WAV as a `PathBuf` + `TempFileRegistry` entry (don't use `NamedTempFile` which auto-deletes on drop), or (b) extract a second WAV for VAD. The current codebase uses `make_temp_file()` + `registry.register()` pattern (see `cut.rs:138`) which gives explicit control -- match this pattern.

### Pitfall 5: Zero speech segments
**What goes wrong:** VAD returns empty `Vec<SpeechInterval>` → `build_concat_filter` returns empty string → FFmpeg gets invalid filter.
**How to avoid:** Check `if speeches.is_empty()` before building filter and return `AppError::NoSpeechDetected` (already defined in `error.rs:62`).

### Pitfall 6: ORT binary download at runtime
**What goes wrong:** On first run in a new environment, `ort` downloads prebuilt ONNX Runtime libraries (~50MB) from Microsoft CDN. This can cause slow first run or failure in air-gapped CI.
**Why this is not a problem:** The CI workflows already set `ORT_CACHE_DIR: ~/.ort-cache` and cache that directory between runs (Phase 16 implementation). The crate bundles the model ONNX bytes (1.8MB via `include_bytes!`) -- only the runtime binary needs download.

---

## Code Examples

Verified patterns from source code (voice_activity_detector source + existing codebase):

### Initialize VAD
```rust
// Source: voice_activity_detector/src/vad.rs:36-38, 116-128
use voice_activity_detector::VoiceActivityDetector;

let mut vad = VoiceActivityDetector::builder()
    .sample_rate(16000i64)   // i64, not u32
    .chunk_size(512usize)    // MUST be 512 for 16kHz (Silero V5 constraint)
    .build()
    .map_err(|e| anyhow::anyhow!("ONNX Runtime failed to initialize: {}", e))?;
```

### Run LabelIterator to get speech/non-speech labels
```rust
// Source: voice_activity_detector README.md + iterator/label.rs
use voice_activity_detector::{IteratorExt, LabeledAudio};

// samples: Vec<i16> from hound WavReader
let labels = samples.into_iter().label(&mut vad, 0.5f32, 0usize);
// Each item: LabeledAudio::Speech(Vec<i16>) | LabeledAudio::NonSpeech(Vec<i16>)
```

### Read WAV as i16 samples with hound
```rust
// Source: hound documentation + voice_activity_detector dev-dependencies (uses hound 3.5.1)
let mut reader = hound::WavReader::open(wav_path)?;
let samples: Vec<i16> = reader
    .samples::<i16>()
    .filter_map(|s| s.ok())
    .collect();
```

### Extract 16kHz mono WAV (shared helper pattern)
```rust
// Source: caption.rs:392-408 (existing pattern, extract to ffmpeg.rs)
// ffmpeg -i input -ar 16000 -ac 1 -f wav output.wav
let args = ["-i", input_str, "-ar", "16000", "-ac", "1", "-f", "wav", wav_str];
// Note: no -c:a flag = default pcm_s16le for WAV format
```

### Existing SpeechInterval type (reuse as-is)
```rust
// Source: silence.rs:7-11
pub struct SpeechInterval {
    pub start: f64,
    pub end: f64,
}
// build_concat_filter() takes &[SpeechInterval] -- use directly from VAD output
```

### Existing TempFileRegistry pattern for WAV
```rust
// Source: cut.rs:136-139 (existing pattern to follow)
let temp_file = make_temp_file(parent_dir, ".wav")?;
let wav_path = temp_file.path().to_path_buf();
registry.register(wav_path.clone());
// keep temp_file alive until end of scope (don't drop it or WAV disappears)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Silero V4 (0.1.x) | Silero V5 bundled in 0.2.1 | 2025-08-04 | Only 512/256 chunk sizes supported (V4 allowed more); verify chunk_size |
| Manual ORT session construction | `LazyLock<Arc<Mutex<Session>>>` in crate | 0.2.0 | Session shared across VoiceActivityDetector instances; no double-init |
| `get_speech_timestamps()` (Python Silero API) | Not exposed in Rust crate | N/A | Must implement timestamp recovery via chunk index tracking (as documented) |

**Deprecated/outdated:**
- `run_silencedetect()` in `ffmpeg.rs`: To be commented out (marked deprecated) in Phase 17; removed in Phase 18
- `parse_silencedetect()` in `silence.rs`: Same treatment
- `silence_to_speech()`: No longer called for VAD path (VAD returns SpeechInterval directly, not SilenceInterval)
- `filter_silences_by_words()`: Only used in pipeline hybrid cut path (silencedetect + word protection); VAD path replaces this whole subsystem
- `total_silence_removed()`: Needs a new version that works on `Vec<SpeechInterval>` directly (compute `video_duration - sum(speech durations)`)

---

## Open Questions

1. **VAD threshold value**
   - What we know: `0.5f32` is the README example; Silero V5 documentation suggests 0.5 as default
   - What's unclear: Whether 0.5 causes over-detection or under-detection on typical talking-head content with normalized audio
   - Recommendation: Start with `0.5`; this is Claude's Discretion; can add a `const VAD_THRESHOLD: f32 = 0.5` in the relevant module for easy tuning

2. **Where does `run_vad` live?**
   - Options: (a) `src/vad.rs` new module, (b) inline in `ffmpeg.rs`, (c) `src/commands/cut.rs` only
   - Recommendation: `src/vad.rs` new module -- clean separation, both cut and pipeline import it; `ffmpeg.rs` stays pure FFmpeg I/O
   - This is Claude's Discretion

3. **transcribe() signature change for WAV sharing**
   - Current: `transcribe(input, model, lang, verbose, registry) -> Vec<Word>` creates+deletes its own WAV
   - Required: Keep WAV alive for pipeline's VAD stage
   - Recommendation: Add `wav_out: Option<&Path>` parameter -- if `Some(path)`, write WAV there and don't delete it; if `None`, use existing temp behavior. This touches `caption.rs` but keeps backward compatibility for the standalone `caption` command.

---

## Sources

### Primary (HIGH confidence)
- `voice_activity_detector` source at `/Users/darrelltang/.cargo/registry/src/.../voice_activity_detector-0.2.1/src/` -- vad.rs, label.rs, predict.rs, sample.rs, lib.rs (direct source inspection)
- `Cargo.toml.orig` -- confirmed exact `ort =2.0.0-rc.10` and `ort-sys =2.0.0-rc.10` pins
- `CHANGELOG.md` -- confirmed 0.2.1 released 2025-08-04; updated to Silero V5 and ORT rc.10
- `README.md` (bundled) -- confirmed builder API, LabelIterator usage, mono-only constraint
- Existing codebase (`cut.rs`, `pipeline.rs`, `caption.rs`, `ffmpeg.rs`, `silence.rs`) -- direct source read
- Generated cargo docs (`target/doc/voice_activity_detector/`) -- confirmed struct/trait signatures

### Secondary (MEDIUM confidence)
- docs.rs `voice_activity_detector/latest` WebFetch -- confirmed module index, LabeledAudio variants, IteratorExt methods
- `hound` docs.rs WebFetch -- confirmed WavReader.samples::<i16>() API

### Tertiary (LOW confidence)
- WebSearch results -- used only for confirmation, all critical facts verified via source

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- direct source inspection of Cargo.toml.orig + crate source
- Architecture: HIGH -- existing codebase patterns are clear; VAD API fully inspected from source
- Pitfalls: HIGH -- derived from actual source code behavior (VadConfigError enum, chunk_size validation in vad.rs:118-127)

**Research date:** 2026-02-24
**Valid until:** 2026-09-24 (stable; only risk is ORT rc.10 being superseded, but the pin prevents auto-upgrade)
