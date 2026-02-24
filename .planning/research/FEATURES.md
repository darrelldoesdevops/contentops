# Feature Research

**Domain:** Neural voice activity detection (Silero VAD) — Rust CLI video processing tool
**Researched:** 2026-02-24
**Confidence:** HIGH (parameters), MEDIUM (tuning guidance), MEDIUM (crate selection)
**Milestone:** v1.4 — Silero VAD replacing FFmpeg silencedetect in `cut` and `pipeline`

---

## Context

This research covers only the v1.4 milestone additions. Existing features (silence removal via silencedetect, --breaths flag, audio normalization, Whisper transcription, caption burn, title overlays, pipeline) are complete and not repeated here.

**Current pipeline (to be replaced at `cut` stage):**

```
Input video
  → normalize (FFmpeg loudnorm)
  → silencedetect -af silencedetect=n=-30dB:d=0.5 (stderr parse)
  → silence_to_speech() → SpeechInterval list
  → build_concat_filter() → FFmpeg concat
  → output video
```

**Target pipeline after v1.4:**

```
Input video
  → normalize (FFmpeg loudnorm) [unchanged]
  → FFmpeg: extract 16kHz mono PCM f32le → raw samples
  → Silero VAD: samples → Vec<SpeechInterval> with (start_sec, end_sec)
  → build_concat_filter() [unchanged]
  → FFmpeg concat [unchanged]
  → output video
```

---

## How Silero VAD Works

### Model architecture

Silero VAD is a pre-trained ONNX model (~260K parameters, ~2MB for V5) that classifies short audio chunks as speech or non-speech. It processes fixed-size windows of audio samples and outputs a float probability (0.0–1.0) per window. The model was trained on 6000+ languages and is robust to background noise, mic quality variation, and accented speech — all conditions common in TikTok talking-head recordings.

### Audio requirements

- **Sample rate:** 16000 Hz only for the standard model (8000 Hz also supported, but 16kHz has better accuracy)
- **Channels:** Mono only — stereo must be downmixed before inference
- **Format:** Linear PCM, float32 samples preferred by most Rust crates
- **Window size:** Fixed at **512 samples at 16kHz** (= 32ms per chunk). Other sizes (1024, 1536) are permitted but may affect accuracy. The model was explicitly trained on 30ms, 60ms, and 100ms chunks.
- **Pre-processing required:** Video audio at 44.1kHz/48kHz stereo must be resampled to 16kHz mono before VAD. FFmpeg handles this in-process via:
  ```
  ffmpeg -i input.mp4 -ac 1 -ar 16000 -f f32le -vn pipe:1
  ```

### Output: speech segments (not silence)

Silero VAD **returns speech timestamps** — `(start_sec, end_sec)` pairs for detected speech. This is the inverse of the current FFmpeg silencedetect approach which returns silence intervals. The `silence_to_speech()` function in `src/silence.rs` becomes unnecessary; VAD output maps directly to `Vec<SpeechInterval>`.

```
Current: silencedetect → parse silence → invert to speech → concat filter
Target:  Silero VAD   → speech timestamps directly → concat filter
```

### Parameters

| Parameter | Type | Default | Range | What It Does |
|-----------|------|---------|-------|--------------|
| `threshold` | f32 | 0.5 | 0.0–1.0 | Probability cutoff: chunks above this are speech. Lower = more sensitive, captures quiet speech. Higher = more conservative, ignores marginal audio. |
| `min_speech_duration_ms` | u32 | 250 | 50–2000 | Minimum contiguous speech duration to retain. Shorter speech chunks are dropped. Prevents isolated coughs or clicks from appearing as speech. |
| `min_silence_duration_ms` | u32 | 100 | 100–2000 | Silence gap required to split two speech segments. If silence between two utterances is shorter than this, they merge into one segment. |
| `speech_pad_ms` | u32 | 30 | 0–500 | Padding added to each side of every speech segment. Preserves natural leading/trailing audio around words. |
| `max_speech_duration_s` | f32 | ∞ | — | Maximum speech segment length before forced split. Relevant for streaming; not needed for offline batch processing. |
| `window_size_samples` | u32 | 512 | 512/1024/1536 | Chunk size for inference. 512 samples @ 16kHz = 32ms per inference call. |

**Note on `speech_pad_ms` vs existing `SPEECH_PADDING`:** The current codebase applies `SPEECH_PADDING = 0.075s` (75ms) to speech intervals post-detection. VAD's native `speech_pad_ms = 30ms` is lower. For talking-head video, the padding should be configured to match the original 75ms behavior to avoid clipped word starts/ends.

---

## Rust Crate Landscape

Four Rust VAD crates are available. Evaluated for: model bundling, maintenance, API quality, ONNX runtime handling.

### Option A: `voice_activity_detector` (RECOMMENDED)

- **Version:** 0.2.1 (August 2025, actively maintained)
- **Downloads:** ~2,557/month; 11 releases; 28 commits
- **Model:** Silero VAD V5 (opset 16), bundled implicitly — ONNX Runtime downloaded automatically at compile time
- **API style:** Iterator/stream over raw samples; returns probability per chunk; label extensions emit `Speech`/`NonSpeech` enums with configurable threshold and padding chunks
- **Sample rates:** 8kHz or 16kHz
- **ONNX Runtime:** Downloaded from Microsoft during build (not linked at runtime); `load-dynamic` feature available for custom binary paths
- **Cross-platform:** Windows, macOS, Linux verified
- **Limitation:** Does not expose high-level `get_speech_timestamps()` — requires implementing the timestamp accumulation loop in application code
- **Why recommended:** Most actively maintained Rust Silero VAD crate as of 2025. Clean API. V5 model.

### Option B: `silero-vad-rust`

- **Model:** Silero ONNX opset 15 & 16 bundled in `src/silero_vad/data`
- **API:** `audio_forward()` for batch; `forward_chunk()` for streaming with state
- **Limitation:** Less documentation, unclear maintenance cadence
- **Why not:** Weaker maintenance signal vs voice_activity_detector

### Option C: `silero-vad-rs`

- **Model:** NOT bundled — user must download ONNX file separately
- **Why not:** Requires user setup step; violates zero-setup distribution requirement. 3 total commits, inactive.

### Option D: Roll own via `ort` crate directly

- **Model:** Download silero_vad.onnx (V5, ~2MB), `include_bytes!()` into binary
- **API:** Write inference loop against raw ONNX tensors
- **Why not:** Significantly more implementation work; no meaningful advantage over voice_activity_detector.

---

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Speech detection accuracy on quiet/breathy audio | The core value prop of VAD over silencedetect. If VAD performs worse on normal talking-head content, the milestone fails. | LOW (model is pre-trained) | Use threshold=0.5 default; verify on representative samples before shipping. |
| Zero additional user setup | Binary already requires ffmpeg + whisper-cli. Adding "download ONNX model" would break this. VAD model must ship inside the binary. | MEDIUM | ONNX runtime downloads at compile time (build.rs); model bytes embedded via include_bytes! or crate-internal. |
| Same CLI surface as before | Replacing an implementation detail should not require users to learn new flags. Cut and pipeline commands should work identically. | LOW | Remove --breaths; all other flags unchanged. |
| Dry-run still works | --dry-run shows what would be cut. VAD produces the same SpeechInterval output format, so dry-run display is unchanged. | LOW | Output is already in seconds; format unchanged. |
| Doctor still validates readiness | Doctor checks prerequisites. If ONNX runtime is bundled (compile-time download), no new runtime check needed. | LOW | No new doctor check required if ort handles runtime linkage at build time. Verify on CI. |
| Correct output for 16kHz requirement | Silero VAD requires 16kHz mono input. The cut command currently sends normalized audio (44.1kHz/48kHz) through silencedetect. VAD integration requires resampling step. | MEDIUM | Add FFmpeg pipe: extract 16kHz mono PCM before VAD. Keep normalized path for FFmpeg concat output. Two audio extraction paths: one for VAD (16kHz PCM), one for output concat (original rate normalized). |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| --breaths removal | VAD detects all non-speech including breaths without a separate flag or threshold tuning. The user no longer needs to know what -24dB means. | LOW | Simply remove CutArgs.breaths and PipelineArgs.breaths from cli.rs. One less flag to explain. |
| Configurable threshold (--vad-threshold) | Advanced users wanting to tune for noisy environments or quiet speakers can adjust the speech probability cutoff without recompiling. | LOW | Single f32 flag, default 0.5. Optional — could ship without it in v1.4 since default works for talking-head. |
| Configurable min-silence (--min-silence-ms) | Replaces the hardcoded 500ms min_duration from silencedetect, exposed explicitly. Allows tuning between aggressive (200ms) and conservative (600ms) silence removal. | LOW | u32 flag in milliseconds. Meaningful range: 200–800ms for talking-head. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| GPU acceleration / CUDA execution provider | "Faster inference" | VAD on 5-minute video runs in <500ms on CPU. GPU setup requires CUDA drivers not available on standard macOS. Adds platform complexity with no user-visible benefit for offline batch use. | Default CPU execution provider via ort. Fast enough. |
| Streaming/realtime VAD mode | "Detect as audio plays" | contentops is a batch-processing offline tool. Streaming VAD is for voice assistants. Real-time output format (per-chunk Speech/NonSpeech) is incompatible with the concat filter approach. | Batch process entire audio then apply concat filter. |
| Expose all VAD parameters as CLI flags | "Full control" | 5 new flags (threshold, min_speech_ms, min_silence_ms, speech_pad_ms, max_speech_s) clutters the interface. Most users never touch them. The current tool's value is simplicity. | Ship threshold + min_silence_ms as optional flags only. Leave others at tuned defaults. |
| Auto-tune parameters from content | "VAD that adapts to podcast vs talking-head" | Adds inference-time complexity; no reliable way to classify content type from audio alone without another model. | Good defaults for talking-head; document manual flag tuning in --help. |
| Standalone `vad` subcommand | "Inspect VAD results before cutting" | --dry-run already shows speech intervals. A separate vad subcommand duplicates the functionality of cut --dry-run with more API surface to maintain. | Keep --dry-run in cut command. |

---

## Parameter Defaults for Talking-Head Content

Talking-head content (solo speaker, low background noise, occasional pauses, no music) is the primary use case. Recommended defaults derived from:
- Silero upstream defaults
- LiveKit plugin defaults (min_silence=550ms for conversational speech)
- faster-whisper defaults (speech_pad=400ms for Whisper compatibility)
- Existing contentops behavior (SPEECH_PADDING=75ms, SILENCE_MIN_DURATION=500ms)

| Parameter | Upstream Default | Recommended for Talking-Head | Rationale |
|-----------|-----------------|-------------------------------|-----------|
| `threshold` | 0.5 | **0.5** | Works well for clean speech; no adjustment needed for talking-head |
| `min_speech_duration_ms` | 250 | **250** | Filters isolated breath sounds, clicks without affecting normal speech |
| `min_silence_duration_ms` | 100 | **400–500** | Matches existing 0.5s SILENCE_MIN_DURATION; prevents over-cutting between sentences |
| `speech_pad_ms` | 30 | **75** | Matches existing SPEECH_PADDING=0.075s; preserves natural word boundaries |
| `window_size_samples` | 512 | **512** | 32ms @ 16kHz; standard, well-tested chunk size |

**Tuning guide for other content types (for documentation):**

- **Podcast (multiple speakers, cross-talk):** Lower `min_silence_duration_ms` to 200ms; speakers overlap and short gaps are not true silence.
- **Noisy background (outdoor, ambient):** Raise `threshold` to 0.65–0.7; reduces false speech on rustling/wind.
- **Quiet/soft speaker:** Lower `threshold` to 0.35–0.45; captures soft speech that would otherwise be classified as non-speech.
- **Music beds:** VAD does not separate speech from music. Music is not silence and will not be removed. This is a known limitation — contentops is not designed for music-heavy content.

---

## Output Format: Speech Timestamps in Seconds

Silero VAD produces speech segments in seconds. This maps directly to the existing `SpeechInterval` struct:

```rust
pub struct SpeechInterval {
    pub start: f64,  // seconds
    pub end: f64,    // seconds
}
```

The `build_concat_filter()` function in `src/silence.rs` already consumes `Vec<SpeechInterval>` and produces the FFmpeg filter string. No changes needed there.

**Data flow replacement:**

```
Before:
  parse_silencedetect(stderr) → Vec<SilenceInterval>
  silence_to_speech(silences) → Vec<SpeechInterval>

After:
  vad_detect(pcm_samples)    → Vec<SpeechInterval>   (direct)
```

The `parse_silencedetect`, `silence_to_speech`, `filter_silences_by_words`, `words_to_speech_intervals` functions in `src/silence.rs` become dead code after VAD integration. They can be deleted or preserved if the breaths-via-transcription path is kept.

---

## Integration Architecture

### Audio path for VAD (new, parallel to existing normalize path)

```
Input video
  │
  ├──[existing]──> FFmpeg loudnorm → normalized temp .mp4 → concat filter output
  │
  └──[new]──> FFmpeg pipe:
                -i normalized.mp4
                -ac 1 -ar 16000 -f f32le -vn pipe:1
              → Vec<f32> samples in Rust
              → voice_activity_detector crate
              → Vec<SpeechInterval>
              → build_concat_filter()
```

### Audio pre-processing via FFmpeg pipe

Use FFmpeg to extract 16kHz mono f32le audio from the normalized video to stdout, captured in Rust as raw bytes → cast to `Vec<f32>`:

```rust
// FFmpeg command
// ffmpeg -i {normalized_path} -ac 1 -ar 16000 -f f32le -vn pipe:1
let output = Command::new("ffmpeg")
    .args(["-i", &normalized_path, "-ac", "1", "-ar", "16000", "-f", "f32le", "-vn", "pipe:1"])
    .output()?;

let bytes = output.stdout;
// bytes.len() must be divisible by 4 (f32 = 4 bytes)
let samples: Vec<f32> = bytes
    .chunks_exact(4)
    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
    .collect();
```

### ONNX runtime at build time

The `voice_activity_detector` crate downloads ONNX Runtime from Microsoft's servers during `cargo build`. This happens once, is cached by Cargo, and is included in the CI pipeline without extra configuration. No runtime dynamic library dependency. No user-visible setup step.

**Cross-compilation concern (MEDIUM confidence):** Building for Linux/Windows on macOS CI may require ONNX Runtime binaries for those platforms to be available at link time. Verify in CI before shipping. The `load-dynamic` feature may simplify cross-compilation by deferring library loading.

---

## Feature Dependencies

```
[FFmpeg loudnorm normalize step]
    └──produces──> normalized temp .mp4
                       └──feeds──> [FFmpeg 16kHz PCM extraction]
                                       └──produces──> Vec<f32> samples
                                                          └──feeds──> [Silero VAD inference]
                                                                          └──produces──> Vec<SpeechInterval>
                                                                                             └──feeds──> [build_concat_filter()]
                                                                                                             └──feeds──> [FFmpeg concat filter output]

[--breaths flag removal] ──requires──> [VAD replaces silencedetect]
[Doctor --no-new-check]  ──requires──> [ONNX runtime bundled at build time]
```

### Dependency Notes

- **VAD requires 16kHz mono:** Must add FFmpeg extraction step before VAD inference. The normalized file is already available from the existing loudnorm step, so no new temp file is needed for the concat output — only the PCM bytes are extracted for VAD, then discarded.
- **--breaths removal requires VAD first:** Cannot remove --breaths until VAD is the detection backend, since --breaths is the only way to remove breaths with silencedetect.
- **SpeechInterval struct is already correct:** `src/silence.rs` defines `SpeechInterval { start: f64, end: f64 }` which matches VAD output format exactly. No struct changes needed.
- **build_concat_filter() is format-agnostic:** Consumes `Vec<SpeechInterval>` regardless of how they were generated. No changes needed.

---

## MVP Definition

### Launch With (v1.4)

- [ ] `voice_activity_detector` crate added to Cargo.toml — no separate ONNX model download required
- [ ] FFmpeg pipe command extracts 16kHz mono f32le PCM from normalized video
- [ ] VAD inference on f32 samples produces `Vec<SpeechInterval>` in seconds
- [ ] `cut` command uses VAD output instead of silencedetect + silence_to_speech
- [ ] `pipeline` command uses VAD output (via cut::run) with same change
- [ ] `--breaths` flag removed from `cut` and `pipeline` in cli.rs
- [ ] Dead code from silence.rs removed (parse_silencedetect, silence_to_speech, filter_silences_by_words)
- [ ] Default parameters tuned for talking-head: min_silence_duration_ms=400, speech_pad_ms=75
- [ ] `--dry-run` continues to show speech intervals with same format
- [ ] Three-platform CI (macOS, Linux, Windows) passes with ONNX runtime downloaded at build time

### Add After Validation (v1.x)

- [ ] `--vad-threshold` flag (f32, default 0.5) — add once default proves insufficient for edge cases
- [ ] `--min-silence-ms` flag (u32, default 400) — expose when users request tuning
- [ ] Doctor check for ONNX runtime if bundling approach changes

### Future Consideration (v2+)

- [ ] Configurable VAD backend (silencedetect vs Silero) — only if regression in specific content types reported
- [ ] All five VAD parameters as flags — only if multiple power users request it

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| VAD replaces silencedetect in `cut` | HIGH — core value prop of milestone | MEDIUM | P1 |
| FFmpeg 16kHz PCM extraction pipe | HIGH — VAD prerequisite | LOW | P1 |
| `--breaths` flag removal | HIGH — simplifies interface | LOW (delete flag) | P1 |
| Default parameter tuning (min_silence=400ms, pad=75ms) | HIGH — accuracy requires good defaults | LOW (constants) | P1 |
| Dead code removal (silence.rs unused fns) | LOW — hygiene | LOW | P1 |
| CI passes on all 3 platforms with ONNX build | HIGH — blocks release | MEDIUM | P1 |
| `--vad-threshold` CLI flag | MEDIUM — power user control | LOW | P2 |
| `--min-silence-ms` CLI flag | MEDIUM — tuning control | LOW | P2 |
| Doctor ONNX runtime check | LOW — ONNX bundled, no runtime dep | LOW | P3 |

**Priority key:**
- P1: Must have for milestone to ship
- P2: Should have, add when core is working
- P3: Nice to have, future consideration

---

## Sources

- [Silero VAD GitHub — snakers4/silero-vad](https://github.com/snakers4/silero-vad) — upstream model, parameter documentation (HIGH confidence — official repo)
- [Silero VAD Version History Wiki](https://github.com/snakers4/silero-vad/wiki/Version-history-and-Available-Models) — V5 model details, ONNX opset 16, ~260K parameters (HIGH confidence — official wiki)
- [silero-vad-rs VADIterator docs](https://docs.rs/silero-vad-rs/latest/silero_vad_rs/vad/struct.VADIterator.html) — parameter types, output format `SpeechTimestamps` with start/end in seconds (MEDIUM confidence — Rust docs)
- [voice_activity_detector crate — lib.rs](https://lib.rs/crates/voice_activity_detector) — version 0.2.1, August 2025, Silero V5, cross-platform, maintenance status (HIGH confidence — package registry)
- [voice_activity_detector GitHub — nkeenan38](https://github.com/nkeenan38/voice_activity_detector) — API details, ONNX Runtime download behavior, load-dynamic feature (MEDIUM confidence — official repo)
- [voice_activity_detector — docs.rs](https://docs.rs/voice_activity_detector/latest/voice_activity_detector/) — VoiceActivityDetector struct, builder pattern, predict() return type f32 (HIGH confidence — official docs)
- [ort crate — Linking documentation](https://ort.pyke.io/setup/linking) — static vs dynamic linking, compile-time download strategy, load-dynamic feature (HIGH confidence — official docs)
- [LiveKit Silero VAD plugin — livekit/agents vad.py](https://github.com/livekit/agents/blob/main/livekit-plugins/livekit-plugins-silero/livekit/plugins/silero/vad.py) — production defaults: min_silence=550ms, speech_pad=500ms, threshold=0.5 (MEDIUM confidence — well-maintained production codebase)
- [faster-whisper VAD parameter discussion](https://github.com/guillaumekln/faster-whisper/issues/477) — why faster-whisper uses speech_pad=400ms vs silero default 30ms; Whisper training context (MEDIUM confidence — maintainer responses in issue)
- [Silero VAD parameter tuning discussion #562](https://github.com/snakers4/silero-vad/discussions/562) — maintainer recommendation: tune min_silence_duration_ms and speech_pad_ms as primary knobs (MEDIUM confidence — official maintainer response)
- [Bundling ONNX Runtime in Rust blog](https://blog.stark.pub/posts/bundling-onnxruntime-rust-nix/) — compile-time download approach, tradeoffs for CI/cross-platform distribution (MEDIUM confidence — detailed technical post, single source)

---

*Feature research for: Silero VAD integration (contentops v1.4)*
*Researched: 2026-02-24*
