# Stack Research

**Domain:** TikTok upload metadata generation + interactive title approval + safe zone compliance (Rust CLI addition)
**Researched:** 2026-02-25
**Confidence:** HIGH (crate versions verified via docs.rs; safe zone measurements triangulated from multiple sources; TikTok API schema from official developer docs)

## New Capabilities Required

This research covers additions only. Existing stack (clap 4.5, serde/serde_json 1.0, indicatif 0.18, owo-colors 4.2, thiserror 2.0, anyhow 1.0, voice_activity_detector 0.2.1, hound 3.5, tempfile 3.25) is validated and unchanged.

---

## Recommended Stack

### Interactive Terminal Prompts

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `dialoguer` | 0.12.0 | Interactive Select, Input, Confirm prompts during pipeline | From the same `console-rs` family as `indicatif` (already in Cargo.toml). Designed to work alongside indicatif — no terminal state conflicts. Provides `Select` (arrow-key numbered list), `Input` (text editing with default), and `Confirm` (y/n) — exactly the three interaction patterns needed. 0.12.0 released October 14, 2025. |

**Why not `inquire`:** inquire 0.7.x depends on `crossterm` which adds ~100KB to binary and introduces its own terminal backend. dialoguer shares the `console` crate already pulled in transitively by indicatif. No additional terminal backend conflict risk.

**Why not `cliclack`:** cliclack's visual style (box-drawing, multiline prompts) conflicts visually with indicatif spinners mid-pipeline. dialoguer integrates cleanly between spinner finish and next spinner start.

### No New Crates Required for Metadata Generation

The sidecar metadata file (TikTok upload fields) is plain JSON. `serde` + `serde_json` (already in stack) handle it fully. No additional crate needed.

### No New Crates Required for Safe Zone Compliance

Safe zone enforcement is a constants + coordinate calculation problem. The existing ASS subtitle style in `caption.rs` hardcodes `MarginV: 480` — this needs to become a named constant. The overlay `build_title_filter` in `overlay.rs` hardcodes `y_base` via `scale()` — this stays as-is since the title overlay appears at the top (safe zone top is 130px, overlay starts at `scale(200, h)` = 200px on 1920h — already compliant). No new dependencies.

---

## Cargo.toml Addition

```toml
[dependencies]
dialoguer = "0.12"
```

One line. dialoguer transitively pulls `console` which indicatif already uses — Cargo deduplicates automatically.

---

## TikTok Metadata Sidecar: Schema

The sidecar file written next to the output video is a JSON file (e.g., `video_pipeline.tiktok.json`). Fields sourced from TikTok Content Posting API (direct post reference, verified 2026-02-25):

```json
{
  "title": "string, max 2200 UTF-16 rune characters; hashtags inline e.g. #fyp #dev",
  "privacy_level": "PUBLIC_TO_EVERYONE | MUTUAL_FOLLOW_FRIENDS | FOLLOWER_OF_CREATOR | SELF_ONLY",
  "disable_duet": false,
  "disable_stitch": false,
  "disable_comment": false,
  "brand_content_toggle": false,
  "brand_organic_toggle": false
}
```

The `title` field holds both caption text and hashtags — TikTok has no separate description field. Hashtags are embedded inline (`#example`) delimited by spaces or newlines. Character limit is 2200 UTF-16 runes (not bytes), which is generous for auto-generated content.

**Serde struct for this:**

```rust
#[derive(Serialize, Deserialize)]
pub struct TikTokMetadata {
    pub title: String,
    pub privacy_level: String,
    pub disable_duet: bool,
    pub disable_stitch: bool,
    pub disable_comment: bool,
    pub brand_content_toggle: bool,
    pub brand_organic_toggle: bool,
}
```

Uses existing `serde` + `serde_json` — no new dependencies.

---

## TikTok Safe Zone: Pixel Constants (1080x1920)

Verified across TikAdSuite official spec guide, orsonlord.com overlay guide, and multiple safe zone checkers (2025-2026 sources). Measurements are for the standard in-feed TikTok format.

| Zone | Blocked By | Pixel Measurement | Safe Content Boundary |
|------|-----------|-------------------|----------------------|
| Top | Username, search bar | 130px from top | Content starts at Y=130 |
| Bottom (standard) | Caption bar, audio disc, like/comment/share stack bottom | 350px from bottom | Content ends at Y=1570 (1920-350) |
| Bottom (shopping ads) | CTA button added | 450px from bottom | Content ends at Y=1470 |
| Right | Like, comment, share, follow icons | 64px from right (primary) / 120px conservative | Content ends at X=960-1016 |
| Left | No UI elements | 0px (36px aesthetic margin recommended) | Content starts at X=36 |

**Safe content rectangle for critical content (faces, subtitles, hooks):**
- X: 36–960 (924px wide)
- Y: 130–1570 (1440px tall)

### ASS Subtitle Positioning

Current code writes `MarginV: 480` in the style definition (`caption.rs` line 204). This positions subtitle bottom edge 480px from the bottom of the 1920px frame — i.e., subtitles sit above Y=1440. This is inside the safe zone (350px bottom exclusion → bottom safe boundary is Y=1570; subtitle sits 90px above that). **Current value is already compliant — no change needed.**

If subtitles are moved for aesthetic reasons, the minimum safe `MarginV` is `350` (bottom safe zone). Keep at `480` or above for comfortable clearance.

**Named constant to introduce:**

```rust
// In caption.rs or a new tiktok_safe.rs module
pub const TIKTOK_MARGIN_V: u32 = 480;       // Bottom margin for subtitles (px at 1920h)
pub const TIKTOK_TOP_EXCLUSION: u32 = 130;   // Top UI zone height (px)
pub const TIKTOK_BOTTOM_EXCLUSION: u32 = 350; // Bottom UI zone height (px)
pub const TIKTOK_RIGHT_EXCLUSION: u32 = 120;  // Right icon zone width (px, conservative)
```

### Overlay Title Positioning

Current `overlay.rs` uses `scale(200, video_height)` for the `top` position preset, which yields 200px Y offset on 1920h video — above 130px exclusion zone. Overlay appears at Y=200, well inside safe zone. **No change needed for compliance.**

Bottom preset uses `scale(1400, video_height)` = 1400px from top = 520px from bottom of 1920h frame. This is above the 350px exclusion zone boundary (Y=1570). **Compliant.**

Center preset uses `scale(760, video_height)`. **Compliant.**

---

## Claude CLI Prompt Pattern: Multiple Title Options

The existing `generate_title()` in `overlay.rs` returns one title. To support interactive selection, the prompt must request N numbered options in a parseable format.

**Recommended prompt pattern:**

```
Generate 5 short, punchy titles (3-8 words each) for this talking head video.
Each title should be a hook that grabs attention.
Split longer titles across 2-3 lines using actual newlines within the title.
Keep each line to 2-4 words max.

Return ONLY a numbered list in this exact format, nothing else:
1. First title line one
First title line two
2. Second title
3. Third title
...

Transcript: {transcript}
```

**Parse strategy:** Split output on lines matching `^\d+\.` to extract each option. Lines following a numbered line (until the next numbered line or EOF) are continuation lines of the same title (joined with `\n`).

**Shell invocation pattern (same as existing):**

```rust
Command::new("claude")
    .arg("-p")
    .arg(&prompt)
    .arg("--model")
    .arg("haiku")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    ...
```

**Interactive selection using dialoguer:**

```rust
use dialoguer::Select;

let options: Vec<String> = parse_title_options(&raw_output); // returns Vec<String>
let display: Vec<String> = options.iter()
    .map(|t| t.replace('\n', " / "))  // flatten multiline for display
    .collect();

let selection = Select::new()
    .with_prompt("Select a title (arrow keys, Enter to confirm)")
    .items(&display)
    .default(0)
    .interact()?;

let chosen = &options[selection];
```

`interact()` returns `usize` (zero-based index). The selected title (with embedded newlines preserved) is passed unchanged to `build_title_filter()` which already handles multi-line titles via `text.split('\n')`.

**Edit option:** After selection, offer `Input::new().with_prompt("Edit title (or Enter to keep)").with_initial_text(chosen).interact_text()?` to allow manual refinement before burning.

---

## Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `dialoguer` | 0.12.0 | `Select` for numbered title choice, `Input` for optional edit, `Confirm` for y/n approval gates | All interactive prompt needs in this milestone |

No other new libraries required.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `dialoguer` 0.12.0 | `inquire` 0.7.x | Pulls in `crossterm` (separate terminal backend from `console`); heavier dependency; overkill for Select + Input + Confirm |
| `dialoguer` 0.12.0 | Raw `stdin` line reading | No arrow-key navigation, no default highlighting, poor UX for option selection |
| `dialoguer` 0.12.0 | `cliclack` 0.3.x | Visual style (box-drawing chars, multiline prompt blocks) conflicts aesthetically with indicatif spinner output |
| JSON sidecar file | TOML sidecar | TikTok API uses JSON natively; serde_json already in stack; no TOML benefits here |
| Inline hashtags in `title` field | Separate `hashtags` array | TikTok's API has no separate hashtags field — they're embedded in `title` text per official API spec |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `console` crate directly for prompts | dialoguer wraps console; writing raw escape codes for selection menus duplicates what dialoguer provides | `dialoguer` Select/Input |
| `crossterm` | Already avoided by using console-rs family; adding crossterm creates two competing terminal backends | `dialoguer` which uses `console` |
| `readline` / `rustyline` | GPL-adjacent license (rustyline is MIT but heavy); designed for persistent REPL history, not pipeline one-shot prompts | `dialoguer` Input |
| `MarginV` below 350 in ASS style | Subtitles would overlap TikTok bottom UI zone (caption bar, audio disc, engagement buttons) | Keep `MarginV` >= 350; current value 480 is correct |
| Separate `hashtags` JSON field in sidecar | TikTok API does not have a separate hashtags field — only `title` | Embed hashtags inline in `title` string |

---

## Integration Points with Existing Code

| Existing Location | Change Needed | Details |
|-------------------|--------------|---------|
| `overlay.rs::generate_title()` | Modify prompt, add parse + Select loop | Change from single-title prompt to numbered list; add dialoguer Select after generation |
| `overlay.rs::run()` | Add conditional interactive flow | When `--auto` used: generate → select → optionally edit → burn |
| `pipeline.rs::finish_stages()` | Pass chosen title through | `finish_stages()` already accepts `text: Option<&str>`; no signature change needed |
| `caption.rs::generate_ass()` | Add named constants | Replace hardcoded `480` with `TIKTOK_MARGIN_V` constant |
| `pipeline.rs::finish_stages()` | Write tiktok sidecar | After overlay completes, serialize `TikTokMetadata` to `{stem}.tiktok.json` next to final output |
| `cli.rs::PipelineArgs` | Add `--no-interactive` flag | For CI/non-TTY use; skips Select, takes first generated title |

---

## Version Compatibility

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `dialoguer` | 0.12.0 | `indicatif` 0.18 | Same console-rs family; both use `console` crate; no conflict |
| `dialoguer` | 0.12.0 | Rust edition 2024 | Verified: docs.rs build passes on nightly 1.91.0 (2025-08-22) |
| `dialoguer` | 0.12.0 | `clap` 4.5 | No interaction; dialoguer runs after CLI args parsed |

---

## Sources

- docs.rs/dialoguer/0.12.0 — version confirmed, Select/Input/Confirm/FuzzySelect/Editor/Sort feature list — HIGH confidence
- github.com/console-rs/dialoguer — version 0.12.0 released October 14, 2025; indicatif listed as companion library — HIGH confidence
- developers.tiktok.com/doc/content-posting-api-reference-direct-post — `title` field (2200 UTF-16 chars), `privacy_level` enum, no separate hashtags field — HIGH confidence
- tikadsuite.com/blog/tiktok-ad-safe-zones — bottom 350px exclusion, top 130px, right 64px (primary) — HIGH confidence
- orsonlord.com/articles/free-safe-zone-overlays-for-reels-tiktok-and-shorts — 840x1280px safe content area, 480px bottom, 120px sides — MEDIUM confidence (triangulated)
- Multiple safe zone checkers (zeely.ai, kreatli.com, predis.ai) — consistent 130px top / 350px bottom measurements — MEDIUM confidence

---
*Stack research for: TikTok upload metadata generation, interactive title approval, safe zone compliance in contentops Rust CLI*
*Researched: 2026-02-25*
