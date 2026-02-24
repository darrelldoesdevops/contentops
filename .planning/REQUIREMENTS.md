# Requirements: contentops v1.4

**Defined:** 2026-02-24
**Core Value:** Take a raw video file and remove dead air automatically

## v1.4 Requirements

### VAD Integration

- [ ] **VAD-01**: Cut command detects speech using Silero VAD instead of FFmpeg silencedetect
- [ ] **VAD-02**: Pipeline command detects speech using Silero VAD instead of amplitude-based detection
- [ ] **VAD-03**: Shared `ffmpeg::extract_16k_wav` helper extracts 16kHz mono audio for both VAD and Whisper
- [ ] **VAD-04**: User can tune VAD sensitivity via `--vad-threshold` flag (f32, default 0.5)
- [ ] **VAD-05**: User can tune minimum silence duration via `--min-silence-ms` flag (u32, default 400)

### Build & CI

- [ ] **CI-01**: `voice_activity_detector` 0.2.1 compiles on macOS ARM64, macOS Intel, Linux x86_64, Windows x86_64
- [ ] **CI-02**: ONNX Runtime binary cached in GitHub Actions to avoid repeated downloads

### Cleanup

- [ ] **CLN-01**: `--breaths` flag removed from cut and pipeline commands
- [ ] **CLN-02**: Dead amplitude-based code removed from silence.rs (parse_silencedetect, silence_to_speech, filter_silences_by_words, SilenceInterval, total_silence_removed)

## v2 Requirements

### VAD Tuning

- **VAD-06**: Configurable VAD backend (silencedetect fallback vs Silero)
- **VAD-07**: GPU/CUDA execution provider for ONNX Runtime

## Out of Scope

| Feature | Reason |
|---------|--------|
| All 5 VAD parameters as CLI flags | Over-exposes internal knobs; threshold + min-silence covers 95% of tuning |
| Silero VAD v4 support | v5 bundled in crate; managing model versions adds complexity |
| Custom ONNX model path | Bundled model eliminates setup; power users can fork |
| load-dynamic ort linking | Breaks single-binary distribution via Homebrew |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CI-01 | Phase 16 | Pending |
| CI-02 | Phase 16 | Pending |
| VAD-03 | Phase 17 | Pending |
| VAD-01 | Phase 17 | Pending |
| VAD-02 | Phase 17 | Pending |
| VAD-04 | Phase 18 | Pending |
| VAD-05 | Phase 18 | Pending |
| CLN-01 | Phase 18 | Pending |
| CLN-02 | Phase 18 | Pending |

**Coverage:**
- v1.4 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-02-24*
*Last updated: 2026-02-24 after initial definition*
