# Milestones

## v1.0 MVP (Shipped: 2026-02-20)

**Phases completed:** 5 phases, 8 plans
**Timeline:** 2 days (2026-02-19 → 2026-02-20)
**Codebase:** 2,401 lines of Rust, 49 files

**Key accomplishments:**
- CLI skeleton with FFmpeg detection, typed errors, and temp file lifecycle
- Silence removal with concat filter, speech padding, breath detection, and automatic audio normalization
- Whisper-powered caption generation with word-level timestamps
- CapCut-style word-by-word caption highlighting with ASS subtitles burned into video
- Animated text overlays with auto-title generation via Claude CLI
- Progress bars with real-time percentage tracking across all commands

**Post-plan iterations:**
- Rewrote captions from karaoke fill to CapCut-style word highlighting (blue active word)
- Upgraded encoding to CRF 14 / preset slow across all commands
- Added punctuation stripping, contraction merging, and --split-on-word for cleaner captions
- Redesigned overlay to match CapCut aesthetic (white boxes, orange accents, Impact font, slide animation)
- Folded normalize into cut pipeline (removed standalone subcommand)

---


## v1.1 Polish & Pipeline (Shipped: 2026-02-20)

**Phases completed:** 4 phases (6-9), 12 commits
**Timeline:** 1 day (2026-02-20)
**Codebase:** 2,873 lines of Rust, 198 files changed (+933/-99)

**Key accomplishments:**
- Audited codebase to zero clippy warnings, removed dead code, extracted 5 duplicate spinners to shared ui.rs
- Converted all 8 bare anyhow calls to typed AppError variants with consistent colored error output
- Added `contentops doctor` with colored prerequisite checks and per-subcommand readiness summary
- Added `contentops pipeline` chaining cut, caption, and overlay in one command with temp dir management
- GitHub Actions CI gating fmt, clippy, test, and cargo-audit on every push/PR
- Tag-triggered releases shipping ARM64, Intel, and universal macOS binaries with SHA256 checksums

---

