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


## v1.2 Distribution & Docs (Shipped: 2026-02-21)

**Phases completed:** 3 phases (10-12), 3 plans
**Timeline:** 2 days (2026-02-20 -> 2026-02-21)
**Codebase:** 785 lines of Rust, 2 non-planning files changed (+163 insertions)

**Key accomplishments:**
- Homebrew tap with architecture-conditional formula distributing ARM64 and Intel binaries via `brew install darrelldoesdevops/tap/contentops`
- Cross-repo GitHub Actions automation: version tag push auto-updates tap formula version and SHA256 via workflow_dispatch
- TAP_UPDATE_TOKEN classic PAT for cross-repo dispatch authentication
- 150-line README with pipeline hero example, prerequisites, dual install paths, flag reference tables for all 5 subcommands, and error-to-fix troubleshooting

---


## v1.3 Cross-Platform (Shipped: 2026-02-23)

**Phases completed:** 3 phases (13-15), 3 plans, 5 tasks
**Timeline:** 2 days (2026-02-21 → 2026-02-23)

**Key accomplishments:**
- Platform-conditional font paths (#[cfg] for macOS/Windows, runtime probe with DejaVu fallback on Linux)
- Cross-platform null muxer (`-f null -`) replacing macOS-only `/dev/null`
- Platform-aware error hints (brew on macOS, apt on Linux, choco on Windows)
- Linux and Windows build jobs in release.yml with SHA256 checksums
- Three-platform CI matrix (macOS, Linux, Windows)
- README with Linux curl and Windows PowerShell install one-liners, three-column prerequisites table

---


## v1.4 Silero VAD (Shipped: 2026-02-25)

**Phases completed:** 3 phases (16-18), 5 plans
**Timeline:** 2 days (2026-02-24 → 2026-02-25)
**Codebase:** 3,273 lines of Rust, 34 files changed (+2,668/-664)

**Key accomplishments:**
- Silero VAD V5 neural network replacing FFmpeg silencedetect for speech/silence detection across all commands
- 4-platform CI matrix (macOS ARM64/Intel, Linux x86_64, Windows x86_64) with ORT binary caching
- Shared `extract_16k_wav` helper reused by VAD, Whisper transcription, and pipeline
- `--vad-threshold` and `--min-silence-ms` tuning flags on cut and pipeline commands
- VAD doctor health check verifying ONNX Runtime initialization
- Removed --breaths flag and all dead amplitude-based silencedetect code
- Normalize-first pipeline ensuring single audio timeline for VAD/Whisper/concat
- 100ms speech padding preventing VAD boundary clipping into speech

**Post-plan iterations:**
- Fixed subtitle overlap from non-monotonic timestamps at speech interval boundaries
- Fixed cross-group ASS event overlap in caption generation
- Fixed breath audio bleed by restructuring pipeline to normalize before WAV extraction
- Added speech padding (100ms) to prevent VAD boundary clipping
- Tuned defaults from 0.5/400 to 0.5/300 based on real-world testing

**Git range:** `feat(16-01)` → `feat(18-02)`

---

