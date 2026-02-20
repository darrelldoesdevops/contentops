# contentops

## What This Is

A Rust CLI tool that replaces CapCut for TikTok/short-form video post-production. Orchestrates FFmpeg and Whisper to handle silence removal, auto-captioning with word-by-word highlighting, and animated title overlays -- the repetitive editing tasks between shooting and publishing.

## Core Value

Take a raw video file and remove dead air automatically -- if silence removal doesn't work reliably, nothing else matters.

## Requirements

### Validated

- Silence removal with concat filter, breath detection, and automatic loudness normalization -- v1.0
- CapCut-style word-by-word caption highlighting burned into video -- v1.0
- Animated title overlays with auto-generation via Claude CLI -- v1.0
- Progress bars with real-time percentage tracking -- v1.0
- TikTok-standard H.264/AAC output at CRF 14 -- v1.0

### Active

- [ ] Codebase audit (idiomatic Rust, dead code, security, best practices) with report
- [ ] Doctor subcommand + auto-prerequisite checks (FFmpeg, whisper-cli, Claude CLI)
- [ ] Pipeline subcommand chaining cut → caption → overlay
- [ ] GitHub Releases CI/CD with pre-built binaries

### Out of Scope

- GUI or web interface -- CLI-only personal tool
- Cloud/API transcription -- local Whisper only for privacy and cost
- Resolution/aspect ratio conversion -- TikTok standard only
- Cross-platform testing -- macOS primary
- Configurable silence thresholds -- hardcoded defaults work well for spoken content
- Pipeline config files (YAML/TOML) -- subcommands are sufficient
- Batch processing -- single file at a time

## Context

## Current Milestone: v1.1 Polish & Pipeline

**Goal:** Harden the codebase, add a one-command pipeline, and ship installable binaries.

**Target features:**
- Codebase audit with findings report, then fixes
- Doctor subcommand + auto-prerequisite checks
- Pipeline subcommand (cut → caption → overlay)
- GitHub Actions CI/CD publishing pre-built binaries

## Context

Shipped v1.0 with 2,401 LOC Rust.
Tech stack: Rust (clap, serde, indicatif, owo-colors), FFmpeg, whisper-cli.
Three subcommands: `cut` (silence removal + normalize), `caption` (generate + burn), `overlay` (title cards).
Heavily iterated on caption styling and overlay animation post-plan.

## Constraints

- **Tech stack**: Rust with clap, serde, anyhow, indicatif
- **Dependencies**: FFmpeg (external), whisper-cli (external), claude CLI (optional, for auto-titles)
- **Platform**: macOS primary
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

---
*Last updated: 2026-02-20 after v1.1 milestone start*
