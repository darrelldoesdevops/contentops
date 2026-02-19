# contentops

## What This Is

A Rust CLI tool that automates video processing for TikTok/short-form content creation. Replaces CapCut ($100/year) by orchestrating FFmpeg and Whisper to handle silence removal, auto-captioning, and text overlays — the repetitive editing tasks that eat time between shooting and publishing.

## Core Value

Take a raw video file and remove dead air automatically — if silence removal doesn't work reliably, nothing else matters.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] CLI accepts input video file with feature flags (--remove-silence, --caption, --overlay)
- [ ] Silence detection using FFmpeg's silencedetect filter with sensible hardcoded defaults
- [ ] Silent segment removal with clean cuts and concatenation
- [ ] Output in TikTok-standard format (H.264 video, AAC audio)
- [ ] Automatic temp file cleanup after successful processing
- [ ] Fail-fast error handling — stop on FFmpeg failure, no partial output
- [ ] Modular pipeline architecture so captioning and overlays plug in as future stages

### Out of Scope

- GUI or web interface — CLI-only personal tool
- Whisper/captioning in v0.1 — future milestone
- Text overlay in v0.1 — future milestone
- Configurable silence thresholds in v0.1 — hardcode defaults, expose flags later
- Resolution/codec options — TikTok standard only for now
- Pipeline config files (YAML/TOML) — feature flags are sufficient
- Multi-user features or community polish — personal workflow tool

## Context

- Creator currently uses CapCut for these tasks, wants to eliminate the subscription
- macOS development environment, FFmpeg installed via Homebrew
- Whisper will run locally (not API) when captioning is added
- "CI/CD for video" metaphor drives the design: input → stages → output, like a build pipeline
- All video manipulation goes through FFmpeg via std::process::Command

## Constraints

- **Tech stack**: Rust with clap, serde/serde_json, thiserror or anyhow
- **Dependencies**: FFmpeg (external, must be installed), Whisper (future, local)
- **Platform**: macOS primary, cross-platform not a priority
- **Output format**: H.264/AAC (TikTok standard)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Feature flags over pipeline config | Personal tool, simplicity wins | — Pending |
| Hardcoded silence defaults for v0.1 | Ship fast, tune later | — Pending |
| Fail-fast error handling | No partial output surprises | — Pending |
| TikTok-standard output format | Primary use case, defer flexibility | — Pending |
| Auto-cleanup temp files | No --debug flag needed for personal use | — Pending |

---
*Last updated: 2026-02-19 after initialization*
