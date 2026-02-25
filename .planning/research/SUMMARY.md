# Project Research Summary

**Project:** contentops v1.5 — TikTok Upload Ready
**Domain:** Rust CLI video pipeline — interactive title approval, TikTok metadata generation, safe zone compliance
**Researched:** 2026-02-25
**Confidence:** HIGH

## Executive Summary

The v1.5 milestone adds upload-readiness to an already-working 6-stage pipeline. The core work is three things: (1) moving title generation out of `overlay::run()` and surfacing it as an interactive approval gate between Stage 5 and Stage 6, (2) generating TikTok description/hashtags from the approved title and transcript via a new `metadata.rs` module after overlay completes, and (3) writing a JSON sidecar next to the output video. Safe zone compliance for existing subtitle and overlay positions is largely already satisfied — only the title overlay's left X margin (30px → 60px) and bottom Y position (1400px → 1050px) need adjustment, plus ASS right margin (40px → 120px) for subtitle horizontal clearance.

The recommended stack addition is minimal: one crate (`dialoguer 0.12`) for interactive Select and Input prompts. Everything else — serde_json for the sidecar, Claude CLI subprocess for description generation, existing ASS constants for captions — is already in place. One conflict to resolve: STACK.md proposes a full TikTok API-shaped sidecar (with `privacy_level`, `disable_duet`, etc.) while ARCHITECTURE.md proposes a minimal sidecar (`title`, `description`, `hashtags[]`). The minimal format is correct for this personal tool; the API-shaped format implies upload automation which is explicitly deferred as an anti-feature.

The critical risk is the interactive approval prompt blocking headless/CI pipelines. This must be verified first: gate all `stdin` reads behind `std::io::IsTerminal::is_terminal()`. The second risk is the indicatif spinner still ticking when the prompt renders — always `finish_and_clear()` before any prompt. The third is sidecar path collision with the existing `_captioned.json` from the `caption` command — use `_tiktok.json` suffix exclusively.

## Key Findings

### Recommended Stack

The existing stack handles everything except interactive prompts. Add `dialoguer = "0.12"` to Cargo.toml — it shares the `console` crate already pulled in by `indicatif`, avoiding a competing terminal backend. No other new dependencies: `serde_json` handles the sidecar, the Claude CLI subprocess handles description generation using the pattern already established in `overlay.rs`.

Note: FEATURES.md recommends `inquire` over `dialoguer` for richer UX defaults. STACK.md recommends `dialoguer` to avoid the `crossterm` dependency that `inquire` brings. The use case is Select + Input only — `dialoguer` is the correct choice.

**Core technologies:**
- `dialoguer` 0.12.0 — interactive Select/Input prompts; shares `console` crate with indicatif; no terminal conflict
- `serde` + `serde_json` (existing) — TikTok metadata sidecar serialization; zero new dependencies
- Claude CLI subprocess (existing) — title option generation and description generation; same `Command::new("claude")` pattern already in `overlay.rs`

### Expected Features

**Must have (v1.5 table stakes):**
- `fix_transcription` word count validation — fallback to original words on mismatch; prevents silent timing corruption
- Title options generation (3 options) — Claude prompt returning numbered list; parsed before Stage 6
- Interactive title selection — `dialoguer::Select` between Stage 5 and Stage 6; blocks overlay burn
- Custom title edit path — `dialoguer::Input` branch for freeform title entry
- TikTok description generation — Claude call from transcript; hook first line, 150–400 chars, 3–5 hashtags
- Sidecar file write — `{stem}_tiktok.json` next to output video with `title`, `description`, `hashtags[]`
- Character limit validation — truncate at last complete sentence before 4,000 char cap; log warning

**Should have (v1.x, after validation):**
- `--no-interactive` flag — skip approval prompt in headless/scripted contexts; take first generated title
- `--niche` flag — appends niche context to description Claude prompt for targeted hashtags

**Defer (v2+):**
- Auto-upload to TikTok — requires OAuth, app review, Creator Marketplace access; not viable for personal tools
- Description approval interactive prompt — lower stakes than title; user edits in TikTok's web UI before upload
- Hashtag performance tracking — requires persistent state, out of scope

### Architecture Approach

Four capabilities integrate into the existing 6-stage pipeline by inserting an approval gate at Stage 5.5 and a metadata stage at Stage 7. The key architectural move is pulling `generate_title()` out of `overlay::run()` and calling it from `finish_stages()` in `pipeline.rs`, so the approval prompt sits cleanly between caption encoding and overlay encoding. A new `src/metadata.rs` module owns description generation and sidecar writing, receiving the already-approved title as a parameter. Threading is explicitly out of scope — the 2–5 second sequential overhead from Claude title generation after caption encoding is not worth the added complexity.

**Major components:**
1. `pipeline.rs::finish_stages()` — orchestrator; gains approval gate call at Stage 5.5 and metadata write at Stage 7
2. `overlay.rs::generate_title()` — unchanged code; called from `pipeline.rs` instead of inside `overlay::run()`
3. `src/metadata.rs` (NEW) — Claude description call; `TikTokMetadata` struct; sidecar write
4. `caption.rs::generate_ass()` — margin adjustment only (MarginR 40→120)
5. `overlay.rs::build_title_filter()` — position adjustment only (x 30→60, bottom y_base 1400→1050, top y_base 200→250)

**Build order:** safe zone fixes → word count guard → title approval → metadata sidecar

### Critical Pitfalls

1. **Interactive prompt blocks headless pipeline** — wrap all `stdin` reads in `std::io::IsTerminal::is_terminal(&std::io::stdin())` check (stable since Rust 1.70); skip approval and use first option if not a TTY
2. **Indicatif spinner corrupts prompt rendering** — `pb.finish_and_clear()` before any interactive prompt; test in real terminal (not test runner)
3. **Multi-option Claude response parsed as single title** — write the parser before writing the prompt; test against mocked Claude output including "Here are your options:" preamble and trailing whitespace
4. **Sidecar path collision with existing `_captioned.json`** — use `_tiktok.json` suffix; never derive from `derive_caption_output()` with "captioned" stem
5. **ASS MarginV is bottom-origin not top-origin** — `Alignment=2` means MarginV is distance from bottom of frame; annotate in code before touching any values; computing as `frame_height - tiktok_ui_height` is wrong

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Safe Zone Margin Fixes
**Rationale:** Purely mechanical constant changes with no dependencies on new features; establishes correct visual baseline before users start approving titles that get burned with wrong positioning.
**Delivers:** ASS subtitle right margin corrected (MarginR 40→120); overlay title left X corrected (30→60); overlay bottom Y corrected (1400→1050); overlay top Y corrected (200→250); named safe zone constants in code
**Addresses:** Safe zone compliance audit from FEATURES.md; title overlay text width overflow into right icon column
**Avoids:** Pitfall 3 (ASS coordinate confusion) — annotate MarginV before touching values; Pitfall 2 (silent regression) — verify default ASS style line unchanged

### Phase 2: Fix Transcription Word Count Guard
**Rationale:** Guards existing pipeline correctness before adding new complexity downstream. Low-cost, high-value; protects caption timing integrity. Must be established before any proximity work on `fix_transcription`.
**Delivers:** Word count validation after `fix_transcription`; fallback to original words on mismatch; warning log output
**Addresses:** Word count validation P1 feature from FEATURES.md
**Avoids:** Pitfall 5 (metadata changes accidentally breaking word-count guard) — explicit separation enforced

### Phase 3: Interactive Title Approval
**Rationale:** Core of the v1.5 value proposition. Depends on safe zone fixes (overlay must position correctly before user approves a title to be burned). Approval gate is the first interactive addition — TTY handling and spinner clearance must be verified before wiring more complexity.
**Delivers:** `generate_title()` called in `pipeline.rs::finish_stages()` at Stage 5.5; `dialoguer::Select` with 3 options; `dialoguer::Input` edit path; TTY guard for headless use; multi-option parse with mocked-response tests
**Uses:** `dialoguer` 0.12 (new dependency); existing Claude CLI subprocess pattern from `overlay.rs`
**Implements:** Stage 5.5 approval gate from ARCHITECTURE.md component boundary diagram
**Avoids:** Pitfall 1 (headless blocking) — TTY check is first task; Pitfall 4 (multi-option parsing) — parser tests before real Claude wiring; Pitfall 7 (spinner corruption) — `finish_and_clear()` before prompt

### Phase 4: TikTok Metadata Generation and Sidecar
**Rationale:** Requires approved title as input (depends on Phase 3). New `metadata.rs` module is cleanly isolated. Description generation is a read-only Claude call that doesn't modify the video — lowest risk change in the milestone.
**Delivers:** `src/metadata.rs` with description generation + `TikTokMetadata` struct; `{stem}_tiktok.json` sidecar next to output video with `title`, `description`, `hashtags[]`; 4,000 char validation with truncation
**Uses:** Existing `serde_json`; existing Claude CLI subprocess pattern
**Implements:** Stage 7 metadata generation from ARCHITECTURE.md data flow diagram
**Avoids:** Pitfall 6 (sidecar path collision) — `_tiktok.json` suffix established as first task; Pitfall 5 (mixing with fix_transcription) — entirely separate module and Claude call

### Phase Ordering Rationale

- Safe zone fixes first: isolated constant changes with no new code paths; correct visual output before user sees approval prompts
- Word count guard second: protects an existing correctness invariant that must hold through all remaining changes; lowest complexity change
- Interactive approval third: highest integration risk (TTY, spinner, multi-option parsing); tackle in isolation with clean surface area
- Metadata sidecar last: depends on approved title from Phase 3; completes the milestone; lowest risk given clean module boundary

### Research Flags

Phases with well-documented patterns (skip research-phase):
- **Phase 1 (safe zone fixes):** Pixel constants already researched and triangulated; mechanical implementation of named constants
- **Phase 2 (word count guard):** Simple `len()` comparison and conditional fallback; `fix_transcription` call site already audited
- **Phase 4 (metadata sidecar):** Standard Claude subprocess + serde_json pattern used in two places already in codebase

Phases that benefit from a small proof-of-concept before full implementation:
- **Phase 3 (interactive approval):** `dialoguer` integration with live `indicatif` spinners has known friction (Pitfall 7); write a 20-line throwaway to verify `finish_and_clear()` + `Select::new().interact()` sequencing in an actual terminal before wiring into `pipeline.rs`

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | `dialoguer` 0.12 verified on docs.rs; version confirmed October 14, 2025; TikTok API schema from official developer docs; existing stack unchanged |
| Features | HIGH | TikTok character limits and safe zone dimensions corroborated across 4+ independent sources; codebase baseline confirmed by direct audit of src/ |
| Architecture | HIGH | Based on direct codebase audit of all relevant src/ files; pipeline stage ordering and component boundaries confirmed by reading actual source |
| Pitfalls | HIGH | All pitfalls grounded in specific file/line references from direct source audit; no speculative pitfalls |

**Overall confidence:** HIGH

### Gaps to Address

- **Sidecar format conflict:** STACK.md describes a full TikTok API-shaped sidecar (`privacy_level`, `disable_duet`, etc.) while ARCHITECTURE.md describes a minimal sidecar (`title`, `description`, `hashtags[]`). Resolution: use the minimal format. This is a personal tool for copy-paste upload, not an upload automation client. Document the decision at the start of Phase 4.
- **Safe zone pixel variance:** Bottom margin figures range from 250px to 480px across sources; no official TikTok pixel spec exists. Current `MarginV=480` is already the most conservative value in the range — keep it unchanged. ARCHITECTURE.md's recommended 420px minimum is satisfied.
- **Flag naming conflict:** FEATURES.md calls the headless flag `--no-approve`; STACK.md calls it `--no-interactive`. Align on `--no-interactive` in Phase 3 — it maps better to the `IsTerminal` check behavior and is more general if other interactive prompts are added later.

## Sources

### Primary (HIGH confidence)
- Direct codebase audit: `src/commands/caption.rs`, `overlay.rs`, `pipeline.rs`, `ui.rs` — all architectural findings
- developers.tiktok.com/doc/content-posting-api-reference-direct-post — TikTok API field schema, title 2200 UTF-16 char limit
- docs.rs/dialoguer/0.12.0 — version confirmed, Select/Input/Confirm API
- github.com/console-rs/dialoguer — indicatif companion status, October 2025 release date

### Secondary (MEDIUM confidence)
- postplanify.com/blog/social-media-safe-zones-2026-complete-guide — 900x1492 safe area, margin breakdown: top 108px, bottom 320px, left 60px, right 120px
- tikadsuite.com/blog/tiktok-ad-safe-zones — bottom 350px exclusion, top 130px, right 64px (primary)
- goldentoolhub.com/social-media-character-limits-2026 — 4,000 char caption limit (corroborated by Hootsuite)
- buffer.com/resources/tiktok-hashtags/ — 3–5 quality hashtags recommendation
- fadeevab.com/comparison-of-rust-cli-prompts/ — cliclack vs dialoguer vs inquire technical comparison

### Tertiary (LOW confidence)
- Multiple safe zone checkers (zeely.ai, kreatli.com, predis.ai) — corroborate top 130px / bottom 250–480px range; high variance across tools; used only for triangulation

---
*Research completed: 2026-02-25*
*Ready for roadmap: yes*
