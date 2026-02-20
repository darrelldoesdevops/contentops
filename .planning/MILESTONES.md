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

