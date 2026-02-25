# contentops

## What This Is

A Rust CLI tool that replaces CapCut for TikTok/short-form video post-production. Orchestrates FFmpeg, Whisper, and Silero VAD to handle neural silence removal, auto-captioning with word-by-word highlighting, animated title overlays, and a one-command pipeline -- the repetitive editing tasks between shooting and publishing. Runs on macOS, Linux, and Windows. Installable via Homebrew (`brew install darrelldoesdevops/tap/contentops`) with auto-updating formula, or direct binary download.

## Core Value

Take a raw video file and remove dead air automatically -- if silence removal doesn't work reliably, nothing else matters.

## Requirements

### Validated

- Silence removal with concat filter, breath detection, and automatic loudness normalization -- v1.0
- CapCut-style word-by-word caption highlighting burned into video -- v1.0
- Animated title overlays with auto-generation via Claude CLI -- v1.0
- Progress bars with real-time percentage tracking -- v1.0
- TikTok-standard H.264/AAC output at CRF 14 -- v1.0
- Codebase audit with findings report, dead code removal, spinner consolidation, consistent error handling -- v1.1
- Doctor subcommand with colored prerequisite checks and per-subcommand readiness -- v1.1
- Pipeline subcommand chaining cut, caption, and overlay in one command -- v1.1
- GitHub Actions CI with fmt, clippy, test, and cargo-audit gates -- v1.1
- Tag-triggered releases with ARM64, Intel, and universal macOS binaries -- v1.1
- Homebrew tap with architecture-conditional formula and cross-repo auto-update on release -- v1.2
- Comprehensive README with hero example, flag reference tables, and troubleshooting -- v1.2
- Platform-conditional font paths, cross-platform null muxer, OS-aware error hints -- v1.3
- Linux and Windows binaries in GitHub Releases with three-platform CI -- v1.3
- Cross-platform README with Linux/Windows install paths and prerequisites -- v1.3
- Silero VAD V5 neural speech detection replacing amplitude-based silencedetect -- v1.4
- Shared 16kHz WAV extraction for VAD and Whisper with normalize-first pipeline -- v1.4
- --vad-threshold and --min-silence-ms tuning flags with 100ms speech padding -- v1.4
- VAD doctor health check and dead amplitude code cleanup -- v1.4
- Interactive transcript fix mismatch handling with dialoguer prompts and non-TTY hard fail -- v1.5
- Interactive title approval with multi-option Claude generation, dialoguer selection, custom edit, and --no-interactive flag -- v1.5
- TikTok metadata sidecar generation with Claude description and hashtags -- v1.5

### Active

## Current Milestone: v1.5 Upload Ready

**Goal:** Pipeline outputs everything needed to upload a TikTok -- approved title overlay, auto-generated description, sidecar file with copy-paste metadata.

**Target features:**
- Interactive title approval: Claude generates 2-3 title options from transcript, user picks/edits inline during pipeline processing
- TikTok description auto-generation from transcript via Claude
- Sidecar metadata file (.txt) next to output video with title + description
- Transcript fix prompt hardening to enforce exact word count

### Out of Scope

- GUI or web interface -- CLI-only personal tool
- Cloud/API transcription -- local Whisper only for privacy and cost
- Resolution/aspect ratio conversion -- TikTok standard only
- Pipeline config files (YAML/TOML) -- subcommands are sufficient
- Batch processing -- single file at a time
- Homebrew formula in homebrew-core -- review overhead, personal tap is sufficient
- crates.io publish -- application binary, not a library
- Auto-installing missing tools -- surprising behavior, print hints instead
- All 5 VAD parameters as CLI flags -- threshold + min-silence covers 95% of tuning
- Custom ONNX model path -- bundled model eliminates setup
- GPU/CUDA execution provider for ONNX Runtime -- CPU inference fast enough for single files

## Context

Shipped v1.4 with ~3,273 LOC Rust.
Tech stack: Rust (clap, serde, indicatif, owo-colors, thiserror, voice_activity_detector, hound, dialoguer), FFmpeg, whisper-cli.
Five subcommands: `cut` (VAD silence removal + normalize), `caption` (generate + burn), `overlay` (title cards), `doctor` (prerequisite checks + VAD health), `pipeline` (one-command workflow).
Platforms: macOS (ARM64 + Intel + universal), Linux (x86_64), Windows (x86_64).
Distribution: Homebrew tap (`darrelldoesdevops/tap/contentops`) with auto-updating formula, plus direct download one-liners for all platforms.
CI/CD via GitHub Actions: fmt, clippy, test, audit on 4 platforms; five architecture binaries on tag; cross-repo workflow_dispatch updates tap formula on release.

## Constraints

- **Tech stack**: Rust with clap, serde, anyhow, indicatif, owo-colors, voice_activity_detector
- **Dependencies**: FFmpeg (external), whisper-cli (external), claude CLI (optional, for auto-titles)
- **Platform**: macOS, Linux, Windows
- **Output format**: H.264/AAC, CRF 14, preset slow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Subcommands over feature flags | Cleaner UX, each command is self-contained | Good |
| Concat filter over select/aselect | Avoids A/V sync drift from frame-based selection | Good |
| whisper-cli over whisper-rs | Shell out for stability, avoid native binding complexity | Good |
| ASS subtitles over SRT | Required for word-by-word styling and karaoke effects | Good |
| Word highlighting over karaoke fill | CapCut-style looks modern, kf fill looked dated | Good |
| CRF 14 + preset slow | Near-lossless quality, worth the encode time for short videos | Good |
| Normalize folded into cut | Automatic normalization on every cut, no separate step needed | Good |
| Claude CLI for auto-titles | Already installed, no API key management needed | Good |
| Impact font + slide animation | Matches existing CapCut title card aesthetic | Good |
| Shared spinner in src/ui.rs | Eliminates 5 duplicate factories, single place to change spinner style | Good |
| Typed AppError for all errors | Consistent colored output, compiler-enforced exhaustiveness | Good |
| Pipeline calls run() directly | Preserves TempFileRegistry and typed errors, no subprocess overhead | Good |
| Doctor exits 0 by default | Diagnostic tool, not prerequisite enforcer; --strict for exit 1 | Good |
| macos-latest for x86_64 cross-compile | macos-13 deprecated; cross-compile from ARM runner works | Good |
| Cross-repo workflow_dispatch | Classic PAT with repo+workflow scopes triggers tap update from release.yml | Good |
| README from live --help output | Flag tables match CLI exactly; prevents documentation drift | Good |
| #[cfg(target_os)] for font constants | Compile-time branching avoids runtime overhead on macOS/Windows; Linux probes at runtime | Good |
| `-f null -` over `/dev/null` | FFmpeg cross-platform null muxer, no cfg needed | Good |
| ORT_CACHE_DIR: ~/.ort-cache for CI | Normalizes ONNX Runtime cache path across all platforms | Good |
| 4-platform CI matrix with cross-compile | macOS Intel via cross-compile on ARM64 runner; tests skip cross target | Good |
| Silero VAD over silencedetect | Neural network trained for speech/non-speech; amplitude thresholds can't distinguish breaths from quiet speech | Good |
| Bundle ONNX model in binary | Zero setup for users; 1.8MB size acceptable for accurate VAD | Good |
| Remove --breaths flag | VAD inherently detects all non-speech; flag adds complexity without value | Good |
| Normalize-first pipeline | Ensures single audio timeline for VAD/Whisper/concat; prevents breath audio bleed | Good |
| 100ms speech padding | VAD chunk boundaries lag actual speech onset; padding prevents clipping | Good |
| Default 0.5/300ms tuning | Balanced threshold with 300ms min-silence; tuned on real talking-head video | Good |
| dialoguer for interactive prompts | Shares console crate with indicatif; no terminal conflict vs inquire | Good |
| Non-TTY hard fail on mismatch | CI/scripts must never silently corrupt; fail loud, not silent | Good |
| Multi-option title with --- delimiter | Claude generates 3 options separated by ---; fallback to single if parsing yields <2 | Good |
| Pipeline-controlled title approval | Pipeline calls approve_title() before overlay, passes result via text arg; clean separation of generation vs rendering | Good |
| Non-critical metadata generation | Description failure warns, never fails pipeline; video is already produced | Good |
| Single Claude call for description+hashtags | JSON response with both fields; reduces API calls and latency | Good |

---
*Last updated: 2026-02-25 after Phase 22*
