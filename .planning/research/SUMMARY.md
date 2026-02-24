# Project Research Summary

**Project:** contentops v1.4 — Silero VAD Integration
**Domain:** Rust CLI video post-production — neural voice activity detection replacing FFmpeg silencedetect
**Researched:** 2026-02-24
**Confidence:** HIGH

## Executive Summary

contentops v1.4 replaces the existing FFmpeg `silencedetect` amplitude-threshold approach with Silero VAD, a pre-trained neural ONNX model that detects speech rather than inferring it from dB levels. The recommended implementation uses `voice_activity_detector` 0.2.1, which bundles the Silero VAD V5 ONNX model via `include_bytes!` and pins `ort = "=2.0.0-rc.10"` — providing static binary distribution on all four CI targets (macOS ARM64, macOS Intel, Linux x86_64, Windows x86_64) with no runtime dylib dependency or user setup step. The existing `SpeechInterval` struct, `build_concat_filter()` function, and FFmpeg loudnorm pipeline require no changes; VAD is a drop-in replacement for the 3-call `run_silencedetect → parse_silencedetect → silence_to_speech` sequence.

The recommended approach: normalize the input video (unchanged for output quality), extract 16kHz mono f32le PCM from the normalized file via FFmpeg pipe, run Silero VAD inference, and feed `Vec<SpeechInterval>` directly to the existing concat filter. This eliminates the intermediate `SilenceInterval` type, removes the need for `filter_silences_by_words` (VAD inherently avoids cutting speech), and allows the `--breaths` flag to be deleted entirely. The net result is a simpler code path and better silence detection without requiring users to understand dB thresholds.

The critical risk is ONNX Runtime distribution. The `ort` crate's `download-binaries` feature downloads prebuilt ONNX Runtime during `cargo build`, which requires network access in CI. The `voice_activity_detector` crate resolves this cleanly — its model is embedded at compile time and its exact `ort` pin (`=2.0.0-rc.10`) is the only version with prebuilt support for all four target platforms including `x86_64-apple-darwin`. Do not upgrade `ort` independently. Binary size increases by approximately 2MB from the embedded model; ONNX Runtime itself remains a build-time download and is not statically linked into the binary.

## Key Findings

### Recommended Stack

The dependency addition is minimal: `voice_activity_detector = "0.2.1"` in `Cargo.toml`, with `hound = "3.5.1"` if reading WAV from disk (simplest approach). Do not add `ort` directly — `voice_activity_detector` pins `ort = "=2.0.0-rc.10"` exactly, and any independent `ort` entry at a different version will produce a Cargo resolver conflict.

**Core technologies:**
- `voice_activity_detector` 0.2.1 — Silero VAD V5 inference with model embedded via `include_bytes!`; only Rust VAD crate supporting static distribution across all four CI platforms
- `ort` 2.0.0-rc.10 (transitive, auto-pinned) — ONNX Runtime 1.22.0 bindings; rc.10 is the last version with `x86_64-apple-darwin` prebuilt binary; rc.11 dropped it
- `hound` 3.5.1 (optional) — WAV file reading for VAD input; not required if piping f32le PCM from FFmpeg stdout
- FFmpeg (existing) — 16kHz mono f32le PCM extraction via pipe; no new format requirements; existing `extract_16k_wav` pattern in `caption.rs` already does this

**Critical version constraint:** ort rc.11 and later dropped `x86_64-apple-darwin` from prebuilt binaries. Using `voice_activity_detector` 0.2.1 pins rc.10 automatically. Never attempt to independently upgrade `ort`.

### Expected Features

**Must have (v1.4 table stakes):**
- VAD replaces silencedetect in `cut` command — core value proposition of milestone
- FFmpeg 16kHz PCM extraction pipe — prerequisite for VAD inference
- `--breaths` flag removed from `cut` and `pipeline` — VAD detects all non-speech including breaths; flag becomes meaningless
- Default parameters tuned for talking-head content: `min_silence_duration_ms=400`, `speech_pad_ms=75` — matches existing `SPEECH_PADDING=0.075s` and `SILENCE_MIN_DURATION=500ms` behavior
- Dead code removal in `silence.rs` (`parse_silencedetect`, `silence_to_speech`, `filter_silences_by_words`, `SilenceInterval`) — eliminates now-unreachable code
- CI passes on macOS, Linux, Windows with ONNX Runtime downloaded at build time

**Should have (v1.x, after validation):**
- `--vad-threshold` flag (f32, default 0.5) — for noisy environments or quiet speakers
- `--min-silence-ms` flag (u32, default 400) — exposes the primary tuning knob explicitly

**Defer (v2+):**
- Configurable VAD backend (silencedetect vs Silero) — only if regressions on specific content types are reported
- All five VAD parameters as CLI flags — over-exposes internal knobs; default experience is more important
- GPU/CUDA execution provider — CPU inference completes in under 500ms on 5-minute video; GPU adds platform complexity with no user-visible benefit

### Architecture Approach

The integration introduces one new module (`src/vad.rs`) and one new shared helper (`ffmpeg::extract_16k_wav`). All other modules either require minor modifications or no changes. The new `vad::detect_speech(audio_path, duration)` function replaces a 3-call sequence in both `cut.rs` and `pipeline.rs`, returning `Vec<SpeechInterval>` — the same type consumed by the unchanged `build_concat_filter()`. The existing `SpeechInterval` struct in `silence.rs` requires no modification.

**Major components:**
1. `src/vad.rs` (new) — wraps `voice_activity_detector` crate; owns all VAD inference; returns `Vec<SpeechInterval>`; configured with talking-head defaults; the sole integration boundary
2. `ffmpeg::extract_16k_wav` (new shared helper, promoted from `caption.rs`) — shared by VAD path and caption transcription; produces 16kHz mono WAV from any video file
3. `silence.rs` (pruned) — retains only `SpeechInterval`, `build_concat_filter`, and `adjust_timestamps`; removes all silencedetect-related code
4. `cut.rs` / `pipeline.rs` (modified) — replace 3-4 call sequence with single `vad::detect_speech()` call; `pipeline.rs` drops `filter_silences_by_words` entirely

**Build order for implementation:**
1. Add dependency to `Cargo.toml`, verify compilation on all platforms
2. Add `ffmpeg::extract_16k_wav` shared helper
3. Create `src/vad.rs` with `detect_speech()`, validate on a real audio file
4. Modify `cut.rs`, then `pipeline.rs`
5. Update `caption.rs` to use shared helper
6. Prune `silence.rs` dead code (only after end-to-end tests pass)
7. Final `Cargo.toml` cleanup

### Critical Pitfalls

1. **Do not upgrade `ort` independently** — `voice_activity_detector` pins `ort = "=2.0.0-rc.10"` exactly. Adding `ort` at any other version causes a Cargo resolver conflict. ort rc.11 dropped `x86_64-apple-darwin` prebuilt support, breaking macOS Intel CI. Prevention: never add `ort` directly to `Cargo.toml`.

2. **Audio format must be 16kHz mono before VAD** — the ONNX model does not reject wrong-format input; it silently returns garbage probabilities. Feeding 44.1kHz stereo audio to VAD produces all-silence or all-speech results. Prevention: always extract 16kHz mono f32le via FFmpeg before inference; assert format at the VAD boundary.

3. **Silero VAD V5 enforces exactly 512 samples per chunk** — using a v4 crate with a v5 model (or vice versa) produces silent failures: constant near-0 or near-1 probabilities. Prevention: `voice_activity_detector` 0.2.1 bundles V5 internally; no separate model file management needed.

4. **macOS universal binary + dynamic ONNX Runtime is incompatible** — Microsoft does not distribute a universal (fat) `libonnxruntime.dylib`. The `voice_activity_detector` embedded-model approach avoids this entirely since ONNX Runtime is downloaded at build time (not bundled as a dylib). Prevention: do not switch to `load-dynamic` ort feature for macOS distribution.

5. **ort `download-binaries` requires network access during `cargo build`** — sandboxed CI environments block this with an opaque error. Prevention: add the ort binary cache directory (`~/.cache/ort` on Linux/macOS) to the GitHub Actions cache key; subsequent CI runs use the cached binary.

## Implications for Roadmap

The implementation is logically a single focused milestone. Dependencies within it are sequential (compilation must succeed before VAD can be tested; `vad.rs` must be validated before command integration; command integration must be confirmed before dead code can be pruned). The suggested roadmap phases reflect these dependencies.

### Phase 1: Dependency and Build Verification

**Rationale:** Fail-fast. The ONNX Runtime download at build time is the highest-risk step. Verifying that `voice_activity_detector` 0.2.1 compiles on all three CI platforms before writing any application code prevents wasted integration effort.

**Delivers:** Confirmed `cargo build` success on macOS ARM64, macOS Intel, Linux x86_64, and Windows x86_64 with ONNX Runtime cached in CI.

**Addresses:** Table stakes — zero additional user setup; CI passes before feature work begins.

**Avoids:** Pitfall 20 (sandboxed CI download failure), Pitfall 29 (ort version pinning drift from independent upgrade attempts)

### Phase 2: Core VAD Integration

**Rationale:** The `extract_16k_wav` helper and `src/vad.rs` module are the integration core. Completing these before modifying commands allows VAD logic to be validated in isolation on a real audio file.

**Delivers:** `ffmpeg::extract_16k_wav` shared helper; `vad::detect_speech()` returning correct `Vec<SpeechInterval>` on test audio with talking-head defaults (threshold=0.5, min_silence=400ms, pad=75ms).

**Implements:** `src/vad.rs` module; `ffmpeg.rs` shared helper extraction

**Avoids:** Pitfall 24 (audio format mismatch), Pitfall 23 (model version mismatch — avoided by using the bundled crate), Pitfall 25 (VAD state not reset between invocations)

### Phase 3: Command Integration and Dead Code Removal

**Rationale:** With `vad::detect_speech()` validated, `cut.rs` and `pipeline.rs` changes are mechanical substitutions. Dead code removal belongs in the same phase — it can only be safely done after the new path is confirmed working end-to-end.

**Delivers:** `cut` and `pipeline` commands using VAD; `--breaths` flag removed; `silence.rs` pruned to 3 functions + 1 struct; `--dry-run` continues to work; complete feature parity with existing behavior on real content.

**Implements:** All P1 features from FEATURES.md MVP definition

**Avoids:** Anti-pattern of keeping `filter_silences_by_words` in the VAD path (it accepts `Vec<SilenceInterval>`, a type that no longer exists); premature dead code removal before end-to-end confirmation

### Phase Ordering Rationale

- Dependency verification first: an ONNX Runtime download failure on macOS Intel CI would block everything; discovering this on day 1 vs day 3 matters
- `vad.rs` before command integration: can be tested in isolation without touching working commands
- Dead code removal last: pruning `silence.rs` before the new path is confirmed risks breaking the working silencedetect path
- Optional flags (`--vad-threshold`, `--min-silence-ms`) are P2 — add after default behavior is validated on real content, not in the initial roadmap phases

### Research Flags

Phases with well-documented patterns (skip additional research):
- **Phase 1 (dependency setup):** `voice_activity_detector` API verified against crates.io and GitHub source; ort prebuilt platform support verified from `dist.txt`; CI cache key changes are standard GitHub Actions Cargo patterns.
- **Phase 3 (command integration):** `cut.rs` and `pipeline.rs` call sites are exactly specified in ARCHITECTURE.md; changes are mechanical substitutions with confirmed before/after shapes.

Phases needing verification before coding:
- **Phase 2 (vad.rs implementation):** `voice_activity_detector` does not expose a high-level `get_speech_timestamps()`. The application must implement a chunk-probability-threshold-accumulation loop. Verify the exact iterator type, chunk labeling API, and state reset mechanism from `docs.rs/voice_activity_detector` before writing `detect_speech()`.
- **Phase 1 (CI cache for Windows):** Verify that the ort binary cache path on Windows (`%LOCALAPPDATA%\pyke\ort`) is captured by the standard GitHub Actions Cargo cache action before assuming it works. Test on a cold Windows runner.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crate versions verified against crates.io API and GitHub source; ort platform support verified from `dist.txt` at exact tag `v2.0.0-rc.10` |
| Features | HIGH (core), MEDIUM (parameter tuning) | Core feature set is unambiguous; defaults derived from multiple production systems (LiveKit, faster-whisper, upstream) but not empirically benchmarked on talking-head TikTok content |
| Architecture | HIGH (structure), MEDIUM (VAD iterator API) | Direct codebase audit confirms call sites and existing types; `voice_activity_detector` high-level timestamp accumulation API needs `docs.rs` verification before implementation |
| Pitfalls | HIGH | v1.0 pitfalls are battle-tested; ONNX/ort pitfalls are grounded in official docs and well-documented community issues |

**Overall confidence:** HIGH

### Gaps to Address

- **`voice_activity_detector` iterator API for timestamp accumulation:** The crate does not expose `get_speech_timestamps()`. The application must implement the accumulation loop (chunk → probability → threshold → speech/non-speech → merge adjacent segments → `Vec<SpeechInterval>`). Verify exact iterator type and labeling from `docs.rs/voice_activity_detector` as a pre-coding step for Phase 2.

- **Windows CI ort cache path:** The ort cache location on Windows (`%LOCALAPPDATA%\pyke\ort`) is documented but should be verified against actual GitHub Actions runner behavior before relying on the standard Cargo cache action.

- **Talking-head parameter validation:** Recommended defaults (min_silence=400ms, speech_pad=75ms) match existing contentops behavior but have not been benchmarked against representative TikTok-format video. Validate on 3-5 real files before the release tag.

## Sources

### Primary (HIGH confidence)
- crates.io API `/api/v1/crates/voice_activity_detector/0.2.1/dependencies` — exact ort pin `=2.0.0-rc.10`, ndarray 0.16.1 confirmed
- GitHub `nkeenan38/voice_activity_detector` `src/vad.rs` — `include_bytes!("silero_vad.onnx")` and `commit_from_memory(MODEL)` confirmed
- GitHub `pykeio/ort` `ort-sys/dist.txt` at tag `v2.0.0-rc.10` — all four target triples confirmed including `x86_64-apple-darwin`
- GitHub `snakers4/silero-vad` — upstream model, V5 parameter documentation, chunk size requirements
- Direct codebase audit: all source files in `contentops/src/` — call sites, existing types, module boundaries confirmed
- ort linking docs `ort.pyke.io/setup/linking` — static vs dynamic linking strategy, `download-binaries` behavior

### Secondary (MEDIUM confidence)
- GitHub `livekit/agents` `livekit-plugins-silero/vad.py` — production defaults: min_silence=550ms, speech_pad=500ms, threshold=0.5
- GitHub `guillaumekln/faster-whisper` issue #477 — speech_pad=400ms rationale for Whisper compatibility
- Silero VAD discussion #562 — maintainer: min_silence_duration_ms and speech_pad_ms are primary tuning knobs
- Blog `blog.stark.pub` — bundling ONNX Runtime in Rust for CI/cross-platform distribution, confirmed sandboxed CI failure mode

### Tertiary (needs implementation validation)
- `docs.rs/voice_activity_detector` — high-level accumulation API; verify iterator type and chunk labeling before implementing `vad.rs`
- Windows CI ort cache path `%LOCALAPPDATA%\pyke\ort` — verify against actual GitHub Actions runner before relying on standard cache action

---
*Research completed: 2026-02-24*
*Ready for roadmap: yes*
