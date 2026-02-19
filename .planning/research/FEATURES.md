# Feature Landscape

**Domain:** Video processing CLI for short-form content creators (TikTok/Reels/Shorts)
**Researched:** 2026-02-19
**Context:** Rust CLI replacing CapCut ($100/yr) for a single creator on macOS

## Table Stakes

Features the tool must have to be usable. Without these, the creator goes back to CapCut.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Silence removal (jump cuts)** | Core value prop; every competitor does this (TimeBolt, Gling, AutoCut, auto-editor). CapCut does it in one click. | Medium | Two-step: FFmpeg `silencedetect` to find silent segments, then segment extraction + concat. Threshold (-30dB) and min-duration (0.5s) are the key params. |
| **Configurable silence threshold** | Different mics/rooms need different dB thresholds. -30dB is a starting default but -20dB to -40dB range needed. | Low | CLI flag `--silence-threshold` with sensible default. Can defer to v0.2 per PROJECT.md but needed soon. |
| **Margin/padding on cuts** | Raw jump cuts sound jarring. Every tool (auto-editor, TimeBolt) offers margin control. Without 200-500ms padding, cuts clip the start of words. | Low | `--margin` flag (e.g., `--margin 0.3` for 300ms). Apply to both sides of each non-silent segment. Critical for natural-sounding output. |
| **TikTok-standard output** | H.264 High Profile, AAC-LC 44.1kHz, 1080x1920 9:16, 30fps, 8-15 Mbps VBR. Wrong codec = quality loss on upload. | Low | Hardcode FFmpeg output flags: `-c:v libx264 -profile:v high -level 4.2 -crf 18 -preset medium -c:a aac -ar 44100 -b:a 256k`. |
| **Auto-captioning (Whisper)** | 80%+ of TikTok videos have captions. Captions directly influence algorithmic performance and viewer retention. CapCut's #1 used feature. | High | Use `whisper-rs` (Rust bindings to whisper.cpp, v0.15.1, actively maintained). Generate word-level timestamps. Output SRT/ASS format. |
| **Burn-in captions to video** | Captions must be baked into the video frame (hard subs). TikTok's native captions are unreliable and non-customizable. | Medium | FFmpeg `subtitles` or `ass` filter to hardcode subs into video during final encode. Requires re-render. |
| **Text overlay** | Hook text, CTA, branding -- every short-form video has static text on screen. CapCut provides this. | Medium | FFmpeg `drawtext` filter. Need position control (safe zone aware), font, size, color, duration. |
| **Fail-fast error handling** | Partial output is worse than no output. If FFmpeg fails mid-pipeline, creator needs to know immediately. | Low | Already in PROJECT.md. Check exit codes, clean up temp files, report which stage failed. |
| **Temp file cleanup** | Processing generates many intermediate files. Leaving them wastes disk space. | Low | Already in PROJECT.md. Delete temp segments on success; optionally keep on `--debug`. |

## Differentiators

Features that make contentops better than "just use CapCut" beyond the $100/yr savings. Not expected, but valuable.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Pipeline composition** | Chain operations in one command: `contentops input.mp4 --remove-silence --caption --overlay "FOLLOW ME"`. No manual multi-step workflow. CapCut requires clicking through each feature separately. | Medium | Modular stage architecture: silence removal -> captioning -> overlay -> encode. Each stage reads from previous stage's output. |
| **Word-level animated captions (karaoke style)** | The dominant TikTok caption trend in 2025-2026. Words highlight as they're spoken. CapCut does this but requires manual style selection. | High | Whisper provides word-level timestamps. Generate ASS subtitle file with `\k` karaoke tags. Burn with FFmpeg `ass` filter. Predefined styles (bold white with black outline is the standard). |
| **Safe zone awareness** | Auto-position text/captions within TikTok's safe zone (960x1160px effective area). Keep text out of the top 250px (username/audio) and bottom 320px (comments/buttons). | Medium | Encode safe zone margins as constants. Caption positioning defaults to center of safe zone. Overlay text defaults to top-third of safe zone. No manual pixel math. |
| **Audio normalization (LUFS)** | Consistent loudness across videos. Viewers scroll past quiet videos. -14 LUFS is the standard for online content. | Low | FFmpeg `loudnorm` filter, two-pass for quality. Target -14 LUFS (online standard). Can run as a pipeline stage before final encode. |
| **Batch processing** | Process a folder of raw clips in one command. Content creators often shoot 5-10 clips per session. | Low | `contentops process --dir ./raw/ --remove-silence`. Iterate over MP4 files, apply same flags. Report per-file success/failure. |
| **Filler word removal** | Remove "um", "uh", "like", "you know" -- not just silence. CapCut and Descript both offer this. Requires speech recognition. | High | Depends on Whisper transcription. Identify filler words in transcript, map to timestamps, cut those segments (same as silence removal but word-targeted). 85-95% accuracy per industry tools. |
| **Preset profiles** | `--preset talking-head` applies silence removal + captions + normalization. `--preset broll` applies just normalization + overlay. Saves typing. | Low | Map preset names to flag combinations. Hardcode 2-3 presets. No config files needed for personal tool. |
| **Dry run / preview** | Show what would happen without processing. "Found 47 silent segments totaling 2m30s. Estimated output: 5m15s." | Low | Run only the detection/analysis stages, print summary, skip encoding. Fast feedback loop for tuning thresholds. |

## Anti-Features

Things to deliberately NOT build. Scope traps that waste time without serving the core use case.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **GUI / web interface** | Personal CLI tool for one creator. GUI adds massive scope (framework, state management, preview rendering). | Stay CLI-only. Good flags + presets provide sufficient UX. |
| **Multi-format output** | Instagram Reels, YouTube Shorts have slightly different specs. Premature generalization. | Hardcode TikTok (1080x1920 H.264/AAC). Other formats are trivial flag changes later if ever needed. |
| **Video effects / transitions** | Fade-ins, zoom effects, transitions between clips. This is NLE territory, not automation territory. | If effects are needed, use a real editor. contentops is for the repetitive processing, not creative editing. |
| **AI content generation** | Auto-generating hooks, scripts, or video from prompts. Hype-driven, not the problem being solved. | contentops processes existing footage. The creator provides the content. |
| **Cloud processing / API** | Server deployment, upload/download, user auth. Massive infra scope for one user. | Local-only. FFmpeg and Whisper run on the Mac. |
| **Plugin system** | Extensibility architecture for third-party stages. Over-engineering for a personal tool. | Hardcode stages. Add new ones by writing Rust code. The "plugin" is a PR. |
| **Real-time preview** | Live playback of edits before committing. Requires media player integration. | Use dry-run for analysis. Open output in QuickTime/VLC to verify. |
| **Scene detection / smart clips** | Auto-splitting long videos into short clips by scene. Interesting but different problem. | The creator already shoots in short clips. This solves a problem that doesn't exist here. |
| **Config files (YAML/TOML)** | Pipeline configuration files add parsing complexity and a second interface to maintain. | Feature flags are sufficient. Presets cover common combos. One user doesn't need config-as-code. |
| **Resolution/aspect ratio conversion** | Cropping landscape to portrait, scaling, etc. | Creator shoots in 9:16 already. If needed, FFmpeg one-liner outside the tool. |

## Feature Dependencies

```
Silence Detection ─────────────> Silence Removal ──┐
                                                    ├──> Pipeline Composition ──> Final Encode
Whisper Transcription ──┬──> SRT/ASS Generation ──┤
                        │                          │
                        └──> Filler Word Detection ─┘
                                                    │
Text Overlay (drawtext) ───────────────────────────┘
                                                    │
Audio Normalization (loudnorm) ────────────────────┘

Whisper Transcription ──> Word-Level Timestamps ──> Karaoke ASS Generation ──> Burn-in Captions
                                                                               (requires ASS filter)

Safe Zone Constants ──> Caption Positioning
                    ──> Overlay Positioning

Silence Removal ──> Dry Run (reuses detection, skips encoding)
Pipeline Composition ──> Preset Profiles (maps names to flag combos)
Pipeline Composition ──> Batch Processing (loops pipeline over files)
```

Key dependency chain: **Silence removal is independent of everything else and should ship first.** Captioning requires Whisper integration which is the highest-complexity addition. Filler word removal requires captioning to already work.

## MVP Recommendation

**Phase 1 -- Ship immediately (silence removal):**
1. Silence detection + removal with jump cuts
2. Margin/padding on cuts (--margin flag)
3. TikTok-standard output encoding
4. Fail-fast error handling + temp cleanup

**Phase 2 -- High-value add (captioning):**
1. Whisper transcription via whisper-rs
2. SRT generation with word-level timestamps
3. Burn-in captions (basic centered style)
4. Audio normalization (-14 LUFS)

**Phase 3 -- Polish (style + workflow):**
1. Karaoke-style animated captions (ASS with \k tags)
2. Safe zone-aware positioning
3. Text overlay (drawtext)
4. Preset profiles

**Defer indefinitely:**
- Filler word removal: High complexity, requires robust NLP on top of Whisper. The 85-95% accuracy means manual review is still needed. Silence removal handles the biggest pauses already.
- Batch processing: One-liner shell loop (`for f in *.mp4; do contentops "$f" --remove-silence; done`) covers this until native support matters.
- Configurable silence threshold: Expose as a flag in Phase 1, but hardcode a good default (-30dB, 0.5s min duration). Tuning comes from usage.

## Sources

- [auto-editor (WyattBlue/auto-editor)](https://github.com/WyattBlue/auto-editor) - Python CLI, silence removal + motion detection
- [whisper-rs](https://crates.io/crates/whisper-rs) - Rust bindings to whisper.cpp, v0.15.1 (2025-09-10)
- [TikTok Video Specs 2026](https://postfa.st/sizes/tiktok/video) - H.264 High, 1080x1920, 8-15 Mbps, 30fps
- [TikTok Safe Zones](https://zeely.ai/blog/tiktok-safe-zones/) - 960x1160px effective area
- [TikTok Caption Best Practices 2026](https://www.opus.pro/blog/tiktok-caption-subtitle-best-practices)
- [FFmpeg silencedetect](https://ffmpeg.org/ffmpeg-filters.html) - noise threshold + duration params
- [FFmpeg loudnorm](https://wiki.tnonline.net/w/Blog/Audio_normalization_with_FFmpeg) - EBU R128, -14 LUFS for online
- [ASS Karaoke Tags](https://github.com/ggml-org/whisper.cpp/issues/884) - \k tags for word-level highlighting
- [FFmpeg drawtext](https://ottverse.com/ffmpeg-drawtext-filter-dynamic-overlays-timecode-scrolling-text-credits/) - text overlay filter
- [Silence removal padding best practices](https://donaldfeury.xyz/remove-the-silent-parts-of-a-video-using-ffmpeg-and-python/) - 200-500ms margin
