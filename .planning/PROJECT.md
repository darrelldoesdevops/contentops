# contentops

## What This Is

A Rust CLI tool that replaces CapCut for TikTok/short-form video post-production. Orchestrates FFmpeg and Whisper to handle silence removal, auto-captioning with word-by-word highlighting, animated title overlays, and a one-command pipeline -- the repetitive editing tasks between shooting and publishing. Installable via Homebrew (`brew install darrelldoesdevops/tap/contentops`) with auto-updating formula on each release.

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

### Active

- Linux and Windows binary support (v1.3)
- Cross-platform CI/release matrix
- Cross-platform docs and install paths

### Recently Validated

- Platform-conditional font paths (macOS/Windows const, Linux runtime probe) -- Phase 13
- Cross-platform null muxer (`-f null -` instead of `/dev/null`) -- Phase 13
- Platform-aware error hints (brew/apt/choco) -- Phase 13

### Out of Scope

- GUI or web interface -- CLI-only personal tool
- Cloud/API transcription -- local Whisper only for privacy and cost
- Resolution/aspect ratio conversion -- TikTok standard only
- Configurable silence thresholds -- hardcoded defaults work well for spoken content
- Pipeline config files (YAML/TOML) -- subcommands are sufficient
- Batch processing -- single file at a time
- Homebrew formula in homebrew-core -- review overhead, personal tap is sufficient
- crates.io publish -- application binary, not a library
- Auto-installing missing tools -- surprising behavior, print hints instead

## Context

Shipped v1.2 with 785 LOC Rust.
Tech stack: Rust (clap, serde, indicatif, owo-colors, thiserror), FFmpeg, whisper-cli.
Five subcommands: `cut` (silence removal + normalize), `caption` (generate + burn), `overlay` (title cards), `doctor` (prerequisite checks), `pipeline` (one-command workflow).
Distribution: Homebrew tap (`darrelldoesdevops/tap/contentops`) with auto-updating formula, plus direct download curl one-liners.
CI/CD via GitHub Actions: fmt, clippy, test, audit on push; ARM64 + Intel + universal binaries on tag; cross-repo workflow_dispatch updates tap formula on release.

## Constraints

- **Tech stack**: Rust with clap, serde, anyhow, indicatif, owo-colors
- **Dependencies**: FFmpeg (external), whisper-cli (external), claude CLI (optional, for auto-titles)
- **Platform**: macOS primary, Linux/Windows portable (Phase 13+)
- **Output format**: H.264/AAC, CRF 14, preset slow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Subcommands over feature flags | Cleaner UX, each command is self-contained | Good |
| Hardcoded silence defaults (-30dB, 0.5s, 75ms pad) | Ship fast, tune later; defaults work well | Good |
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
| Delete cleanup_all() | Pipeline shares TempFileRegistry directly, dead code removed | Good |
| Pipeline calls run() directly | Preserves TempFileRegistry and typed errors, no subprocess overhead | Good |
| Doctor exits 0 by default | Diagnostic tool, not prerequisite enforcer; --strict for exit 1 | Good |
| macos-latest for x86_64 cross-compile | macos-13 deprecated; cross-compile from ARM runner works | Good |
| on_arm/on_intel DSL + Hardware::CPU in install | Two scopes: DSL for top-level url/sha256, Hardware::CPU for runtime binary rename | Good |
| Sentinel comments for sed patching | Inline `# === AUTO-UPDATE: FIELD ===` enables simple sed-based formula updates | Good |
| asset.digest for SHA256 | No binary download needed; gh api returns digest directly | Good |
| Cross-repo workflow_dispatch | Classic PAT with repo+workflow scopes triggers tap update from release.yml | Good |
| README from live --help output | Flag tables match CLI exactly; prevents documentation drift | Good |
| #[cfg(target_os)] for font constants | Compile-time branching avoids runtime overhead on macOS/Windows; Linux probes at runtime | Good |
| `-f null -` over `/dev/null` | FFmpeg cross-platform null muxer, no cfg needed | Good |
| whisper non-macOS hint links to source | No canonical apt/choco package for whisper-cli | Good |

---
*Last updated: 2026-02-21 after Phase 13*
