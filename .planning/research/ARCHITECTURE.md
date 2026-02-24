# Architecture Research

**Domain:** Rust CLI video post-production — Silero VAD integration replacing FFmpeg silencedetect
**Researched:** 2026-02-24
**Confidence:** HIGH (direct codebase audit; MEDIUM on silero-vad-rust API specifics due to sparse docs.rs coverage)

## Integration Question: Where VAD Replaces silencedetect

The existing call chain in both `cut.rs` and `pipeline.rs` is:

```
normalize_to_temp(input) → normalized.mp4
    ↓
ffmpeg::run_silencedetect(normalized_str, threshold, min_duration) → stderr String
    ↓
silence::parse_silencedetect(stderr, duration) → Vec<SilenceInterval>
    ↓
silence::silence_to_speech(silences, duration, padding) → Vec<SpeechInterval>
    ↓
silence::build_concat_filter(speeches) → String
    ↓
ffmpeg cut
```

VAD replaces steps 2 and 3 — the `run_silencedetect` call and `parse_silencedetect` call. The output type after those two steps is `Vec<SpeechInterval>`, which VAD produces directly. Everything from `silence_to_speech` onward is unchanged in the `cut` command.

In `pipeline.rs`, the flow adds `filter_silences_by_words` after `parse_silencedetect` (word-protection). With VAD, VAD already understands speech, so `filter_silences_by_words` is either removed or made optional.

## Audio Loading: WAV via FFmpeg, Not hound

The existing codebase already extracts 16kHz mono WAV for Whisper in `caption::transcribe()`:

```rust
let ffmpeg_args = ["-i", &input_str, "-ar", "16000", "-ac", "1", "-f", "wav", &wav_str];
```

This is the exact format Silero VAD requires: 16kHz, mono, WAV. The VAD module reuses this same extraction pattern — **no hound crate needed for reading from disk**.

The audio loading approach for VAD is:
1. FFmpeg extracts 16kHz mono WAV to a temp file (same as Whisper path)
2. `src/vad.rs` reads the WAV samples using either `hound` (if bundled in silero-vad-rust) or reads raw PCM bytes directly

The `silero-vad-rust` crate provides a `read_audio(path, sample_rate)` helper that returns `Vec<f32>`. Using it avoids a `hound` dependency in our code. Alternatively, FFmpeg can pipe raw PCM (`-f f32le -ac 1 -ar 16000`) directly to stdin, skipping the WAV file entirely — but the WAV-file approach is simpler and already established in the codebase.

## Normalization: Still Needed in cut.rs, Not for VAD

**cut.rs context:** Normalization (`normalize_to_temp`) was needed before amplitude-based silencedetect because silencedetect uses dB thresholds. Variable loudness video would produce inconsistent silence detection. With VAD, the neural model is robust to absolute amplitude levels — VAD detects speech patterns, not dB thresholds. Normalization is therefore **unnecessary for VAD itself**.

However, in `cut.rs`, the normalized file is the source for the final FFmpeg cut operation (the `input_str` fed to the concat filter). Removing normalization would change cut quality. The choices are:

1. **Keep normalize for cut quality, skip for VAD audio extraction** — extract a separate 16kHz mono WAV from the original input, run VAD on it, then cut the normalized video. This adds one extra FFmpeg pass.
2. **Remove normalize entirely** — cut from the original input, lose audio loudness normalization in the output.
3. **Keep normalize, reuse normalized file for VAD** — extract 16kHz mono WAV from the normalized file. VAD results are unchanged (VAD is amplitude-agnostic).

Option 3 is the simplest integration: normalize first (as today), then extract 16kHz WAV from the normalized file, run VAD on it, cut the normalized file. No behavior change to cut quality.

**For pipeline.rs:** Same reasoning. Keep normalization for output quality; run VAD on the 16kHz extraction from the normalized file.

## New Module: src/vad.rs

This module is the clean integration boundary. It owns all Silero VAD interaction and returns `Vec<SpeechInterval>` — the same type `silence_to_speech` returns today.

```rust
// src/vad.rs

use crate::silence::SpeechInterval;

pub fn detect_speech(
    audio_path: &str,   // path to 16kHz mono WAV
    duration: f64,      // total duration in seconds (for building final interval)
) -> anyhow::Result<Vec<SpeechInterval>> {
    // 1. load_silero_vad() — bundles ONNX, no external download needed
    // 2. read_audio(audio_path, 16_000) → Vec<f32>
    // 3. configure VadParameters { return_seconds: true, ... }
    // 4. get_speech_timestamps(&audio, model, params) → Vec<{start, end}>
    // 5. map to Vec<SpeechInterval>
}
```

This signature is the exact replacement for:
```rust
// replaced call sequence (cut.rs / pipeline.rs):
let stderr = ffmpeg::run_silencedetect(&normalized_str, threshold, min_duration)?;
let silences = silence::parse_silencedetect(&stderr, video_duration);
let speeches = silence::silence_to_speech(&silences, video_duration, SPEECH_PADDING);
```

Becomes:
```rust
// new call:
let speeches = vad::detect_speech(&wav_16k_path, video_duration)?;
```

## New FFmpeg Helper: extract_16k_wav

A new function in `ffmpeg.rs` extracts 16kHz mono WAV to a temp path. This is identical to what `caption::transcribe()` does internally — it should be lifted to a shared helper:

```rust
// src/ffmpeg.rs (new function)
pub fn extract_16k_wav(input: &str, output: &str) -> Result<(), std::io::Error> {
    let args = ["-i", input, "-ar", "16000", "-ac", "1", "-f", "wav", output];
    let result = run_ffmpeg(&args)?;
    if !result.success {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "audio extraction failed",
        ));
    }
    Ok(())
}
```

This function already exists implicitly in `caption.rs` — promoting it to `ffmpeg.rs` serves both the caption and VAD paths.

## Existing silence.rs Functions: Keep/Change/Remove

| Function | Status | Reason |
|----------|--------|--------|
| `parse_silencedetect` | **REMOVE** | Dead code once VAD is primary path. Keep only if fallback mode retained. |
| `silence_to_speech` | **REMOVE** | VAD returns speech intervals directly; this conversion is no longer needed. |
| `build_concat_filter` | **KEEP** | Unchanged — takes `Vec<SpeechInterval>`, builds the FFmpeg concat filter string. |
| `adjust_timestamps` | **KEEP** | Used in pipeline.rs to shift word timestamps after cutting. Unchanged. |
| `filter_silences_by_words` | **REMOVE** | Takes `Vec<SilenceInterval>` as input, which no longer exists in the VAD path. VAD inherently avoids cutting speech. |
| `words_to_speech_intervals` | **KEEP IF USED** | Check if referenced anywhere — provides alternative path for word-based cutting. |
| `total_silence_removed` | **REMOVE or ADAPT** | Takes `Vec<SilenceInterval>`. For reporting, compute removed time from speech intervals instead: `duration - speeches.iter().map(|s| s.end - s.start).sum::<f64>()`. |
| `SilenceInterval` struct | **REMOVE** | No longer produced by any code path. |
| `SpeechInterval` struct | **KEEP** | The shared currency between VAD output and build_concat_filter/adjust_timestamps. |

## Updated Source Tree

```
src/
├── main.rs                  NO CHANGE
├── cli.rs                   NO CHANGE (unless adding --vad flag)
├── commands/
│   ├── mod.rs               NO CHANGE
│   ├── cut.rs               MODIFIED — replace normalize→silencedetect→parse→speech with normalize→extract_wav→vad::detect_speech
│   ├── caption.rs           MODIFIED — promote audio extraction to ffmpeg::extract_16k_wav
│   ├── pipeline.rs          MODIFIED — same as cut.rs changes; remove filter_silences_by_words call
│   ├── normalize.rs         NO CHANGE (kept for output quality, not for VAD)
│   ├── doctor.rs            NO CHANGE (or add ort/onnx runtime check if needed)
│   └── overlay.rs           NO CHANGE
├── vad.rs                   NEW — silero-vad-rust wrapper returning Vec<SpeechInterval>
├── ffmpeg.rs                MODIFIED — add extract_16k_wav() as shared helper
├── silence.rs               MODIFIED — remove SilenceInterval, parse_silencedetect, silence_to_speech, filter_silences_by_words, total_silence_removed; keep build_concat_filter, adjust_timestamps, SpeechInterval
├── temp.rs                  NO CHANGE
├── error.rs                 NO CHANGE (or add VadError variant)
└── ui.rs                    NO CHANGE

Cargo.toml                   MODIFIED — add silero-vad-rust (and ndarray if needed)
```

## Data Flow: Before and After

### Before (current)

```
cut input.mp4
    ↓
normalize_to_temp(input) → normalized.mp4   [loudnorm 2-pass, keeps video stream]
    ↓
ffmpeg::run_silencedetect(normalized)       [runs FFmpeg silencedetect filter, returns stderr]
    ↓
silence::parse_silencedetect(stderr)        [regex parse → Vec<SilenceInterval>]
    ↓
silence::silence_to_speech(silences)        [invert silences → Vec<SpeechInterval>]
    ↓
silence::build_concat_filter(speeches)      [build FFmpeg filter_complex string]
    ↓
ffmpeg cut normalized.mp4 → output.mp4
```

### After (with VAD)

```
cut input.mp4
    ↓
normalize_to_temp(input) → normalized.mp4   [unchanged — needed for output quality]
    ↓
ffmpeg::extract_16k_wav(normalized) → temp.wav  [NEW: 16kHz mono WAV extraction]
    ↓
vad::detect_speech(temp.wav, duration)      [NEW: Silero VAD → Vec<SpeechInterval> directly]
    ↓
silence::build_concat_filter(speeches)      [UNCHANGED]
    ↓
ffmpeg cut normalized.mp4 → output.mp4      [UNCHANGED]
```

The intermediate `Vec<SilenceInterval>` type is eliminated. VAD speaks `Vec<SpeechInterval>` natively.

### pipeline.rs Specific Change

Current pipeline.rs after silencedetect has an extra step:

```
filter_silences_by_words(silences, word_times) → safe_silences
silence_to_speech(safe_silences, ...) → speeches
```

With VAD, word-protection (`filter_silences_by_words`) is dropped. VAD's neural detection already avoids cutting speech regions. Pipeline becomes:

```
vad::detect_speech(temp.wav, duration) → speeches
adjust_timestamps(word_data, &speeches) → adjusted_words   [unchanged]
```

## crate: silero-vad-rust

**Confidence: MEDIUM** — The `silero-vad-rust` crate (distinct from `silero-vad-rs`) is the one referenced in the project context. Key verified properties:

| Property | Value | Confidence |
|----------|-------|------------|
| ONNX model bundled | Yes — opset 15 & 16 in `src/silero_vad/data` | MEDIUM (search result claim, not direct docs verification) |
| External download needed | No | MEDIUM |
| Audio format | `Vec<f32>` at 16kHz or 8kHz | HIGH |
| `get_speech_timestamps` exists | Yes | HIGH |
| `VadParameters.return_seconds` | Yes — timestamps in seconds when true | MEDIUM |
| `load_silero_vad()` | Yes | HIGH |
| Import path | `silero_vad_rust::silero_vad::utils_vad` | MEDIUM |

The `silero-vad-rs` crate (different crate) requires a separate ONNX download and uses `ndarray::Array1<f32>` + `VADIterator`. The project context references `get_speech_timestamps()` and `SpeechInterval` matching `silero-vad-rust`, not `silero-vad-rs`.

**Flag for implementation phase:** Verify the exact import paths and struct field names against `docs.rs/silero-vad-rust` before coding. The API surface is confirmed at the function level but field names need verification.

## Component Boundaries

| Boundary | Interface | Notes |
|----------|-----------|-------|
| `vad.rs` → `silence.rs` | Returns `Vec<SpeechInterval>` | SpeechInterval stays in silence.rs |
| `vad.rs` → `ffmpeg.rs` | Calls `extract_16k_wav` | Or inlines it — either works |
| `cut.rs` → `vad.rs` | Calls `vad::detect_speech(wav_path, duration)` | Replaces 3-function call sequence |
| `pipeline.rs` → `vad.rs` | Same as cut.rs | Remove filter_silences_by_words call |
| `caption.rs` → `ffmpeg.rs` | Use shared `extract_16k_wav` | Reduces duplication |

## Build Order for Implementation

| Order | Task | Depends On | Why This Order |
|-------|------|------------|----------------|
| 1 | Add `silero-vad-rust` to `Cargo.toml`, verify it compiles | Nothing | Fail fast on dependency issues |
| 2 | Add `ffmpeg::extract_16k_wav` | Step 1 (confirms dep compiles) | Shared helper needed by both vad.rs and caption.rs |
| 3 | Create `src/vad.rs` with `detect_speech()` returning `Vec<SpeechInterval>` | Steps 1–2 | Core integration — verify VAD produces correct intervals on a real file |
| 4 | Modify `cut.rs` — replace silencedetect call sequence with vad::detect_speech | Step 3 | Simplest integration point, no word-protection complexity |
| 5 | Modify `pipeline.rs` — same replacement, drop filter_silences_by_words | Step 4 confirmed | More complex; word-timestamp adjustment must still work |
| 6 | Update `caption.rs` — use shared extract_16k_wav | Step 2 | Cleanup; not functionally blocking |
| 7 | Prune `silence.rs` — remove dead functions and SilenceInterval | Steps 4–5 confirmed working | Do not prune until end-to-end tests pass |
| 8 | Update `Cargo.toml` — remove any newly dead transitive deps | Step 7 | Final cleanup |

## Anti-Patterns

### Anti-Pattern 1: Running VAD on the Original Video File

**What:** Feeding the MP4 directly to `silero-vad-rust`'s `read_audio` instead of extracting WAV first.

**Why wrong:** `read_audio` in silero-vad-rust expects a WAV file. Even if it accepted other formats, it would need to decode the video to get audio — FFmpeg already does this optimally.

**Do this instead:** Always extract 16kHz mono WAV with FFmpeg first, then feed the WAV path to VAD.

### Anti-Pattern 2: Running VAD Before Normalization

**What:** Skipping `normalize_to_temp` and running VAD on the raw input, then cutting the raw input.

**Why wrong:** VAD results are the same either way (VAD is amplitude-agnostic), but the cut output loses loudness normalization. The normalize step is for output quality, not for VAD.

**Do this instead:** Normalize first, extract WAV from normalized file, run VAD, cut the normalized file.

### Anti-Pattern 3: Keeping filter_silences_by_words in the VAD Path

**What:** Continuing to use `filter_silences_by_words` after VAD replaces silencedetect.

**Why wrong:** `filter_silences_by_words` takes `Vec<SilenceInterval>` — a type that no longer exists in the VAD path. VAD returns speech intervals directly; it inherently does not cut speech.

**Do this instead:** Remove the filter call. VAD's neural detection handles speech protection natively.

### Anti-Pattern 4: Adding a `--vad` Flag Instead of Replacing

**What:** Implementing VAD as an opt-in flag (`contentops cut --vad`) while keeping silencedetect as default.

**Why wrong:** Two code paths, two sets of parameters to tune, two behaviors to document and test. The whole point of VAD is that it's better — make it the only path.

**Do this instead:** Replace silencedetect entirely. If a fallback is needed, that is a separate research decision requiring explicit justification.

## Integration Points

### External Libraries (New)

| Library | Integration Pattern | Notes |
|---------|---------------------|-------|
| `silero-vad-rust` | `vad.rs` calls `load_silero_vad()` + `get_speech_timestamps()` | ONNX bundled — no runtime download |
| `ort` (transitive) | Pulled in by silero-vad-rust for ONNX runtime | May require `ORT_DYLIB_PATH` on some platforms — verify |

### Internal Boundaries

| Boundary | Before | After |
|----------|--------|-------|
| `cut.rs` → speech detection | 3 calls: run_silencedetect → parse_silencedetect → silence_to_speech | 1 call: vad::detect_speech |
| `pipeline.rs` → speech detection | 4 calls: same 3 + filter_silences_by_words | 1 call: vad::detect_speech |
| `silence.rs` surface | 7 public functions + 2 structs | 3 public functions + 1 struct |
| `ffmpeg.rs` surface | No WAV extraction helper | +1 extract_16k_wav |

## Sources

- Direct codebase audit: all source files in `/Users/darrelltang/darrelldoesdevops/contentops/src/`
- silero-vad-rust crate: https://crates.io/crates/silero-vad-rust
- silero-vad-rs docs.rs: https://docs.rs/silero-vad-rs/latest/silero_vad_rs/
- VADIterator API: https://docs.rs/silero-vad-rs/latest/silero_vad_rs/vad/struct.VADIterator.html
- hound crate: https://crates.io/crates/hound
- Silero VAD original: https://github.com/snakers4/silero-vad

---
*Architecture research for: Silero VAD integration into contentops Rust CLI*
*Researched: 2026-02-24*
