# Project Research Summary

**Project:** contentops
**Domain:** Rust CLI video processing pipeline (FFmpeg orchestration)
**Researched:** 2026-02-19
**Confidence:** HIGH

## Executive Summary

contentops is an FFmpeg orchestration CLI -- not a video editor, not a media framework. The tool shells out to FFmpeg via `std::process::Command` for discrete operations (detect silence, cut segments, encode output) and chains them as a sequential pipeline with intermediate files on disk. This is the established pattern used by auto-editor, Remsi, and similar tools. The Rust ecosystem has excellent support for this: clap for CLI parsing, tempfile for intermediate file lifecycle, and tracing for structured logging across pipeline stages. No async runtime, no FFmpeg FFI bindings, no abstraction libraries needed.

The recommended approach is a phased build starting with the silence removal pipeline, which is both the core value proposition and the most technically challenging stage (two-pass FFmpeg with stderr parsing, A/V sync preservation, edge case handling). Silence removal is fully independent of captioning and overlays, so it can ship as a working tool immediately. Captioning (Whisper integration) and text overlays are additive stages that slot into the same pipeline architecture later.

The primary risks are concentrated in Phase 1: pipe deadlock when capturing FFmpeg stderr, audio/video sync drift from incorrect cutting strategy (must use select/aselect filters, not segment-and-concat), silencedetect parser edge cases (trailing silence, leading silence), and temp file leaks on Ctrl+C. All of these are well-documented with known solutions. The critical architectural decision -- using the select/aselect filter approach instead of segment extraction and concatenation -- prevents the sync drift problem entirely and should be locked in from the start.

## Key Findings

### Recommended Stack

Minimal dependency footprint. Every crate earns its place. No async runtime, no FFmpeg bindings, no abstraction layers.

**Core technologies:**
- **Rust + clap 4.5**: CLI parsing with derive macros. De facto standard, zero boilerplate.
- **serde + serde_json**: Parse ffprobe JSON output and future Whisper results.
- **anyhow**: Application-level error handling with `.context()` chains. Not a library, no need for thiserror enums.
- **tracing + tracing-subscriber**: Structured logging with span model that maps to pipeline stages.
- **tempfile**: Secure temp directory lifecycle. Auto-cleanup on drop covers normal/panic cases.
- **which**: Fail-fast dependency validation at startup (is FFmpeg installed?).
- **regex**: Parse FFmpeg silencedetect stderr output.

**Deferred to later phases:** whisper-rs (captioning), indicatif (progress bars).

**Explicitly excluded:** tokio/async, ffmpeg-sidecar, rust-ffmpeg/ffmpeg-next, colored/owo-colors, toml config, rayon.

### Expected Features

**Must have (table stakes -- without these, go back to CapCut):**
- Silence detection + removal with jump cuts
- Margin/padding on cuts (200-500ms to avoid clipping words)
- TikTok-standard output encoding (H.264 High, AAC, 1080x1920, yuv420p)
- Fail-fast error handling + temp file cleanup

**Should have (differentiators that justify building this):**
- Auto-captioning via Whisper with word-level timestamps
- Burn-in captions (hard subs baked into video)
- Karaoke-style animated captions (dominant TikTok trend)
- Audio normalization (-14 LUFS)
- Pipeline composition (chain stages in one command)
- Safe zone-aware positioning

**Defer indefinitely:**
- GUI, cloud processing, plugin system, scene detection
- Filler word removal (high complexity, marginal gain over silence removal)
- Batch processing (shell loop covers this: `for f in *.mp4; do contentops "$f"; done`)

### Architecture Approach

Sequential pipeline of discrete stages communicating through intermediate files on disk. Each stage wraps one or more FFmpeg subprocess invocations. The pipeline executor runs stages in order with fail-fast semantics. Enum dispatch (not trait objects) for the fixed set of stages, gated by Cargo feature flags.

**Major components:**
1. **CLI Layer** (cli.rs) -- clap parsing, validation, entry point
2. **PipelineConfig** (config.rs) -- stage parameters, file paths, flags
3. **Pipeline Executor** (pipeline.rs) -- stage sequencing, temp directory lifecycle, fail-fast
4. **FFmpeg Runner** (ffmpeg.rs) -- centralized subprocess wrapper with `-y`, `-nostdin`, exit code checking, stderr capture
5. **Stage: Silence Removal** (stages/silence.rs) -- two-pass: silencedetect parse, then select/aselect filter
6. **Stage: Caption Generation** (stages/caption.rs) -- future: audio extraction + Whisper + SRT
7. **Stage: Text Overlay** (stages/overlay.rs) -- future: drawtext/subtitles filter

### Critical Pitfalls

1. **Pipe deadlock on stderr capture** -- Use `Command::output()` (reads both streams) or redirect stdout to null for silencedetect. Never sequentially read two piped streams.
2. **A/V sync drift after silence removal** -- Use select/aselect filter with `setpts=N/FRAME_RATE/TB` and `asetpts=N/SR/TB` to rebuild timestamps. Do NOT use segment-and-concat with `-c copy`.
3. **FFmpeg hangs on overwrite prompt** -- Always pass `-y -nostdin` to every FFmpeg invocation. Build this into the FFmpeg runner from day one.
4. **Silencedetect trailing/leading silence** -- Handle unmatched `silence_start` (end of file) and leading `silence_end` (silence from t=0). Use video duration as fallback boundary.
5. **Temp file leaks on Ctrl+C** -- `tempfile::TempDir` handles normal/panic cases. Add ctrlc handler for SIGINT. Use predictable naming for manual cleanup.

## Implications for Roadmap

### Phase 1: Foundation + FFmpeg Runner

**Rationale:** Every pipeline stage depends on the FFmpeg subprocess wrapper. Getting pipe handling, exit codes, and `-y -nostdin` flags right is the foundation. This phase also sets up CLI parsing and the pipeline executor skeleton.

**Delivers:** Working CLI skeleton that validates FFmpeg is installed, parses args, and can execute FFmpeg commands reliably.

**Addresses:** Project scaffolding, dependency validation, error handling foundation.

**Avoids:** Pipe deadlock (#1), interactive hang (#2), exit code mishandling (#10).

### Phase 2: Silence Removal

**Rationale:** Core value proposition. Technically the most complex stage (two-pass FFmpeg, stderr parsing, edge cases). Must be correct before anything else matters. Independent of all other stages.

**Delivers:** `contentops input.mp4 --remove-silence` produces a jump-cut video with silence removed. Usable tool from this point forward.

**Addresses:** Silence detection, silence removal, margin/padding, TikTok output encoding (yuv420p, H.264 High, AAC).

**Avoids:** A/V sync drift (#3), trailing silence edge case (#4), VFR input (#7), pixel format (#9), rotation metadata (#6).

### Phase 3: Auto-Captioning

**Rationale:** Second-highest value feature. Depends on Whisper integration (whisper-rs or whisper CLI). Produces SRT/ASS files with word-level timestamps. Does NOT burn captions into video yet -- that's a separate stage.

**Delivers:** `contentops input.mp4 --caption` generates subtitle files from speech. Audio extraction + Whisper transcription pipeline.

**Uses:** whisper-rs (or whisper CLI subprocess), serde_json for structured output.

**Implements:** Caption Generation stage (stages/caption.rs).

### Phase 4: Caption Burn-in + Style

**Rationale:** Captions must be baked into the video frame (hard subs) for TikTok. This phase takes SRT/ASS output from Phase 3 and renders it into the video. Includes karaoke-style animated captions and safe zone positioning.

**Delivers:** `contentops input.mp4 --remove-silence --caption` produces final video with styled, burned-in captions.

**Addresses:** Burn-in captions, karaoke animation (ASS \k tags), safe zone awareness, caption styling.

### Phase 5: Overlays + Polish

**Rationale:** Text overlays, audio normalization, preset profiles, dry-run mode. These are additive features that improve the workflow but aren't required for core functionality.

**Delivers:** `contentops input.mp4 --remove-silence --caption --overlay "FOLLOW ME" --normalize` -- full pipeline in one command.

**Addresses:** Text overlay (drawtext), audio normalization (-14 LUFS), preset profiles, dry-run mode.

### Phase Ordering Rationale

- **Foundation before features:** FFmpeg runner abstractions prevent the top 3 pitfalls. Build once, use everywhere.
- **Silence removal is independent:** No dependency on Whisper, captioning, or overlays. Ships as a useful tool immediately.
- **Caption generation before burn-in:** SRT/ASS file generation is a prerequisite for burn-in rendering. Separating them allows testing transcription quality independently.
- **Overlays last:** Lowest priority, most dependent on other stages being stable.

### Research Flags

**Phases needing deeper research during planning:**
- **Phase 3 (Captioning):** whisper-rs API stability uncertain (v0.15.1, Sept 2025). Need to verify compatibility with current whisper.cpp. Alternative: shell out to whisper CLI. FFmpeg 8.0 native whisper filter is a potential future simplification.
- **Phase 4 (Caption Burn-in):** ASS karaoke tag syntax (\k tags), FFmpeg ass filter behavior with custom fonts, safe zone coordinate math. Sparse Rust-specific documentation.

**Phases with standard patterns (skip research):**
- **Phase 1 (Foundation):** Well-documented Rust patterns for subprocess management, clap CLI setup.
- **Phase 2 (Silence Removal):** Multiple reference implementations exist (Remsi, auto-editor, ffmpeg-python examples). Select/aselect approach is well-documented.
- **Phase 5 (Overlays):** FFmpeg drawtext and loudnorm filters are thoroughly documented.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crates verified on docs.rs with current versions. Standard Rust CLI stack. |
| Features | HIGH | Feature landscape mapped against competitors (CapCut, TimeBolt, auto-editor). Clear table stakes vs. differentiators. |
| Architecture | HIGH | Sequential pipeline with intermediate files is the established pattern for FFmpeg orchestration tools. Multiple reference implementations confirm approach. |
| Pitfalls | HIGH | All critical pitfalls verified against official documentation, source code, or multiple community sources. Prevention strategies are concrete and tested. |

**Overall confidence:** HIGH

### Gaps to Address

- **Silence threshold defaults:** STACK.md suggests -30dB, PITFALLS.md suggests -60dB. Actual optimal default depends on recording environment (phone mic in a room). Start with -30dB (FEATURES.md recommendation aligned with common tools), expose as a flag early, and tune from real usage.
- **whisper-rs stability:** v0.15.1 wraps actively-changing whisper.cpp. Verify compatibility at Phase 3 implementation time. Have fallback plan to shell out to whisper CLI.
- **VFR input handling:** iPhone recordings are VFR. The select/aselect approach with `setpts=N/FRAME_RATE/TB` should handle this, but needs testing with real device footage. Not a blocking gap -- test during Phase 2.
- **FFmpeg version matrix:** Tool targets macOS Homebrew FFmpeg. Need to verify which version Homebrew currently ships and set minimum version requirement accordingly.

## Sources

### Primary (HIGH confidence)
- [FFmpeg silencedetect filter docs](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html)
- [FFmpeg silencedetect source](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_silencedetect.c)
- [Rust std::process::Stdio docs](https://doc.rust-lang.org/std/process/struct.Stdio.html) -- pipe deadlock warning
- [Remsi silence removal](https://github.com/bambax/Remsi) -- select/aselect architecture reference
- [FFmpeg Concatenate wiki](https://trac.ffmpeg.org/wiki/Concatenate)
- All crate versions verified on docs.rs (2026-02-19): clap 4.5.59, serde 1.0.228, anyhow 1.0.101, tracing 0.1.44, tempfile 3.25.0, which 8.0.0, regex 1.12.3

### Secondary (MEDIUM confidence)
- [auto-editor](https://github.com/WyattBlue/auto-editor) -- Python CLI, feature landscape reference
- [ffmpeg-python split_silence.py](https://github.com/kkroening/ffmpeg-python/blob/master/examples/split_silence.py) -- edge case handling
- [TikTok Video Specs](https://postfa.st/sizes/tiktok/video) -- H.264 High, 1080x1920
- [TikTok Safe Zones](https://zeely.ai/blog/tiktok-safe-zones/) -- 960x1160px effective area
- [whisper-rs 0.15.1](https://docs.rs/crate/whisper-rs/latest) -- Rust Whisper bindings

### Tertiary (LOW confidence)
- [FFmpeg 8.0 native Whisper filter](https://gigazine.net/gsc_news/en/20250825-ffmpeg-8-0-huffman) -- future optimization, availability uncertain
- Select filter expression length limits -- theoretical, no confirmed breakage for short-form video

---
*Research completed: 2026-02-19*
*Ready for roadmap: yes*
