# Feature Research

**Domain:** TikTok upload readiness — title approval, description generation, sidecar metadata, safe zone compliance
**Researched:** 2026-02-25
**Confidence:** HIGH (TikTok constraints), HIGH (safe zone dimensions), MEDIUM (CLI UX patterns), HIGH (codebase baseline)
**Milestone:** v1.5 Upload Ready

---

## Context

This research covers only the v1.5 milestone additions. Existing features (pipeline, VAD silence removal, ASS subtitle burning, animated title overlay via Claude) are complete and not repeated here.

**Current pipeline output:**
```
Raw video
  → normalize → cut (VAD) → caption (ASS burn) → overlay (drawtext title)
  → output_{stem}_pipeline.mp4
```

**Target pipeline output after v1.5:**
```
Raw video
  → normalize → cut → transcribe → fix → caption → overlay (with approved title)
  → output_{stem}_pipeline.mp4
  → output_{stem}_pipeline.txt  (title + description sidecar)
```

---

## TikTok Platform Constraints (Research-Verified)

### Character Limits (HIGH confidence — multiple sources, 2026)

| Field | Limit | Notes |
|-------|-------|-------|
| Caption/description | 4,000 characters | Increased from 2,200 in late 2024 |
| First visible line (before "more") | ~100–125 characters | Platform renders first line as hook |
| Hashtags | 3–5 recommended | Per hashtag: informal limit ~24 chars each |
| Title overlay text | No platform limit | Burned into video; not a platform field |
| Username | 24 characters | Not relevant here |

**Practical description length:** 150–400 characters performs well for discoverability. 800–2,200 for SEO-optimized longer-form. 4,000 is the ceiling, rarely worth hitting.

### Video Specifications (HIGH confidence)

| Spec | Value |
|------|-------|
| Resolution | 1080 x 1920 px |
| Aspect ratio | 9:16 |
| Format | H.264/AAC (already the tool's output) |
| Max upload (web) | 500 MB |

### Safe Zone Dimensions for 1080x1920 (MEDIUM-HIGH confidence — postplanify.com + multiple corroborating sources)

The "safe zone" is the region guaranteed to be unobstructed by TikTok's UI chrome (profile picture, follow button, like/comment/share icons, caption bar, sound label).

| Edge | Unsafe Margin | Notes |
|------|--------------|-------|
| Top | 108–200 px | Profile picture + follow button + sound name |
| Bottom | 320 px (standard) | Caption bar + engagement row |
| Bottom | 370–450 px (ads/conservative) | Includes CTA button; use for subtitle placement |
| Left | 60 px | Edge margin only |
| Right | 120 px | Like/comment/share/bookmark icon column |

**Resulting safe content area:** 900 x 1,492 px centered in 1080 x 1920 frame.

**Subtitle-specific placement:**
- Current code: `MarginV=480` in ASS style (480px from bottom of 1920px frame)
- 480px is already inside the safe zone (320px bottom margin + margin for safety)
- No change needed for subtitle vertical position — current 480px MarginV is conservative and correct

**Title overlay current position:**
- "top" position: `y_base = scale(200, height)` = ~200px from top
- Top safe zone starts at 108px; 200px is inside safe zone — correct
- "bottom" position: `y_base = scale(1400, height)` = ~1400px from top = 520px from bottom
- 520px from bottom > 320px bottom margin — correct

**Safe zone compliance verdict:** Both current subtitle and overlay positions already respect safe zones. No pixel changes required. The v1.5 work is documenting this and ensuring any new title positions introduced stay within safe zone.

---

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Title approval before burning | If Claude generates the title and it's wrong, user has no recourse without re-running. Approval gate is expected for any AI-generated content. | MEDIUM | Interactive prompt mid-pipeline: show 2-3 options, pick number or type custom |
| Sidecar metadata file | The point of "upload ready" is having copy-paste content. Without a sidecar file, user must manually copy the description from terminal output. | LOW | Plain text file next to output video: `{stem}_pipeline.txt` with title + description sections |
| Description in sidecar | Title-only sidecar is incomplete. Users need caption text to paste into TikTok's description field. | MEDIUM | Claude generates from transcript; auto-included in sidecar |
| Character-limit-aware description | Description exceeding 4,000 chars is silently truncated by TikTok, breaking hashtag formatting. Must validate against limit before writing. | LOW | Simple length check on Claude output; re-prompt or truncate if needed |
| Transcript fix word count enforcement | Current `fix_transcription` prompt allows Claude to add/remove words, which corrupts ASS timing. The constraint "do NOT add or remove entries" is not enforced — Claude sometimes violates it silently. | MEDIUM | Validate output word count against input; retry or skip fix if mismatch |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Multiple title options (2-3) | Single Claude-generated option forces accept/reject binary. 2-3 options let user pick best without needing to describe what they want in a follow-up. | MEDIUM | Call Claude once with prompt asking for 3 options separated by `---`; parse into Vec<String> |
| Edit-in-place option | After seeing options, user types a custom title without quitting the pipeline. No re-run required. | LOW | Add "e) enter custom title" to Select prompt; branch on selection |
| Transcript-derived description hooks | Description generated from actual transcript text is more accurate than generic AI content. Hook structure (first line = value proposition) improves engagement. | MEDIUM | Prompt instructs Claude to write first line as hook, 3-5 hashtags at end, ~150-300 chars total for mobile readability |
| Sidecar as copy-paste file | Plain `.txt` format over JSON. User opens file, selects all, pastes. No parsing required. | LOW | Format: title on line 1, blank line, description block. No JSON overhead. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-upload to TikTok | "Fully automated pipeline" | TikTok's API requires Creator Marketplace access, OAuth flows, and app review. Personal-use tools are not eligible for direct upload API access. Adds OAuth complexity with no realistic path to working. | Sidecar file with copy-paste content. Manual upload is a 30-second task. |
| Per-account hashtag memory | "Remember what hashtags work for my account" | Requires persistent config/database, account metadata, performance tracking integration. Out of scope for a personal CLI tool. | Include niche hashtags in the Claude description prompt via a `--niche` flag as a future addition |
| Blocking description approval | "Let me approve the description too" | Adds second interactive pause to pipeline. Description is lower-stakes than title (not burned into video). If wrong, user edits in TikTok's UI before uploading — much easier than re-burning video. | Write description to sidecar; user reviews and edits in any text editor before uploading |
| Safe zone preview/overlay image | "Show me what the safe zone looks like" | Requires generating a frame grab, compositing a safe zone guide, displaying image in terminal (iTerm2 only) or writing to disk. Scope explosion for low-value output. | Document safe zone dimensions in README and `--help`. Positions are pre-validated in code. |
| Multiple description variants | "Generate 3 description options like titles" | Description is easy to edit in TikTok's web UI. Title is burned into video. Asymmetric stakes justify single-option for description. | Single Claude-generated description to sidecar |
| JSON sidecar format | "Machine-readable for automation" | This is a personal CLI tool, not an API. User reads the output directly. JSON adds parsing friction for copy-paste workflow. | Plain `.txt` with labeled sections |

---

## Feature Dependencies

```
[transcript text available]
    └──required-by──> [title options generation]
                          └──required-by──> [title approval prompt]
                                                └──required-by──> [overlay burn]

    └──required-by──> [description generation]
                          └──required-by──> [sidecar file write]

[title approval result]
    └──feeds──> [overlay drawtext filter] (already exists)
    └──feeds──> [sidecar file title line]

[description text]
    └──feeds──> [sidecar file description section]

[word count validation]
    └──guards──> [fix_transcription output]
    └──required-before──> [caption burn] (timing depends on word count integrity)

[sidecar file write]
    └──requires──> [output video path known] (sidecar named after output video)
```

### Dependency Notes

- **Title approval must happen before overlay burn:** The title text is the input to `build_title_filter()`. Approval must complete before Stage 6 (overlay) in the pipeline.
- **Description generation can happen in parallel with caption burn:** Transcript is available after Stage 3 (fix). Description generation is I/O bound (Claude CLI call); it does not block caption burn.
- **Word count validation guards caption timing:** If fix_transcription returns a different word count, adjusted timestamps will be wrong. Validation must happen immediately after fix, before build_concat_filter.
- **Sidecar write is the last step:** Written after overlay completes, alongside the final output video. Named `{stem}.txt` matching the output video name.

---

## MVP Definition

### Launch With (v1.5)

- [ ] `fix_transcription` word count validation — compare input vs output `words.len()`; if mismatch, log warning and fall back to original words without retrying
- [ ] Title options generation — Claude prompt asking for 3 options in format `1. <title>\n2. <title>\n3. <title>` (deterministic parse); called before Stage 6 (overlay)
- [ ] Interactive title selection — `inquire::Select` with options [1, 2, 3, "Enter custom"] displayed after pipeline stage 5 completes; blocks pipeline until user selects
- [ ] Custom title edit path — if user selects "Enter custom", `inquire::Text` prompt accepts freeform title; same downstream path as selection
- [ ] TikTok description generation — single Claude call with transcript; prompt instructs: first line is hook, 150–300 chars total, 3–5 hashtags at end, conversational tone
- [ ] Sidecar metadata file — plain `.txt` written next to output video: title (line 1), blank line, description block; named `{output_stem}.txt`
- [ ] Character limit validation — if Claude returns description >4,000 chars, truncate at last complete sentence before limit and append ellipsis; log warning

### Add After Validation (v1.x)

- [ ] `--no-approve` flag (skip interactive title approval, use first generated option) — add when pipeline is used in batch/scripted contexts
- [ ] `--niche` flag for description generation (e.g., `--niche "software engineering"`) — appended to Claude prompt for niche-specific hashtags
- [ ] Re-try fix_transcription with explicit word count in prompt — add if word count mismatch occurs frequently in testing

### Future Consideration (v2+)

- [ ] Description approval interactive prompt — only if users report copy-paste mistakes from sidecar
- [ ] Hashtag performance tracking via config file — requires persistent state, out of scope for personal tool

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Word count validation in fix_transcription | HIGH — prevents timing corruption silently | LOW (len check, fallback) | P1 |
| Title options generation (3 options) | HIGH — core of "upload ready" value | MEDIUM (Claude prompt + parse) | P1 |
| Interactive title selection via inquire | HIGH — can't skip, blocks overlay burn | MEDIUM (inquire crate) | P1 |
| Custom title edit path | HIGH — always needed if options miss | LOW (inquire::Text branch) | P1 |
| Description generation from transcript | HIGH — completes the sidecar | MEDIUM (Claude prompt) | P1 |
| Sidecar .txt file write | HIGH — the deliverable of the milestone | LOW (fs::write, formatting) | P1 |
| Character limit validation (4,000 char cap) | MEDIUM — rarely hit for short videos | LOW (len check + truncate) | P1 |
| `--no-approve` flag | MEDIUM — useful for scripting | LOW | P2 |
| `--niche` flag for hashtags | MEDIUM — improves hashtag relevance | LOW | P2 |
| Safe zone compliance audit of current positions | LOW — current positions already compliant | LOW (verification only) | P1 (verification, no code change) |

**Priority key:**
- P1: Required for v1.5 milestone to ship
- P2: Add after v1.5 validates well
- P3: Future consideration

---

## TikTok Description Claude Prompt Guidance

Based on research into TikTok SEO and content best practices (HIGH confidence for character limits, MEDIUM for hook structure):

**Target description structure:**
```
[Hook line: 1 sentence, value proposition or curiosity gap, <100 chars]

[2-3 sentences expanding on the topic from transcript]

#hashtag1 #hashtag2 #hashtag3 #hashtagniche1 #hashtagniche2
```

**Prompt constraints to encode:**
- Total 150-400 characters (mobile-readable, not padded to 4,000 limit)
- First line functions as preview before "more" button (~100 char display cutoff)
- 3-5 hashtags only (quality > quantity; "adding too many appears spammy")
- Hashtags at end of description (not interspersed)
- Conversational language, not corporate marketing tone
- No emojis in description unless user content style suggests it

---

## Interactive Approval UX: Crate Selection

**Recommended: `inquire`** (HIGH confidence — direct comparison research)

Reasons:
- `Select` prompt type exactly matches the pick-one-of-three use case
- `Text` prompt type exactly matches the custom title edit case
- Both prompts can be composed in sequence without library switching
- More feature-rich than `dialoguer` (autocomplete, help text, page size for long option lists)
- More actively maintained than `cliclack` for this use case
- Already has `RenderConfig` for consistent styling with the tool's existing owo-colors aesthetic

**Alternative `dialoguer`:** Also viable; slightly simpler API but fewer features. Not recommended because `inquire` covers both Select and Text input natively with better UX defaults.

**UX pattern for approval:**
```
Stage 5/6 complete.

? Choose a title for this video
> 1. WHAT I LEARNED BUILDING
     A RUST CLI IN A WEEK
  2. THE REALITY OF SHIPPING
     PRODUCTION RUST CODE
  3. HOW I REPLACED CAPCUT
     WITH A TERMINAL COMMAND
  e. Enter custom title

[User presses 1, arrow keys, or e]
```

**Timing in pipeline:** Approval occurs after Stage 5 (caption burn) completes — transcript has been processed, all expensive computation done. User has time to read while waiting for caption burn progress bar. Overlay burn (Stage 6) then uses approved title.

---

## Safe Zone Compliance: Current Status

| Element | Current Position | Safe Zone Requirement | Compliant? |
|---------|-----------------|----------------------|------------|
| ASS subtitles | MarginV=480 (480px from bottom) | Bottom safe zone: 320px | YES — 480 > 320 |
| Title overlay "top" | y=~200px from top | Top safe zone: ~108-150px | YES — 200 > 150 |
| Title overlay "bottom" | y=~1400px from top (520px from bottom) | Bottom safe zone: 320px | YES — 520 > 320 |
| Title overlay "center" | y=~760px from top | No constraint at center | YES |
| Title text width | x=~30px from left, no right constraint | Right unsafe: 120px from right | VERIFY — text wraps right at video edge; long titles may overlap icon column |

**Action items:**
- Subtitle position: no change needed
- Title overlay: add right-margin clamp to `build_title_filter` to prevent text overflow into right 120px icon zone. Current code sets `final_x = scale(30, height)` (left edge) but does not cap max text width. Long titles on narrow aspect ratios could overflow.
- No new `--safe-zone` flag needed; compliance is enforced in code, not user-configurable.

---

## Sources

- [postplanify.com — Social Media Safe Zones 2026 Complete Guide](https://postplanify.com/blog/social-media-safe-zones-2026-complete-guide) — 900x1492 safe area, margin breakdown: top 108px, bottom 320px, left 60px, right 120px (MEDIUM-HIGH confidence — detailed measurements, 2026)
- [zeely.ai — TikTok safe zones 2026 guide](https://zeely.ai/blog/tiktok-safe-zones/) — bottom margin 250-300px, right icon column 120px, top 150-200px (MEDIUM confidence — corroborates postplanify)
- [goldentoolhub.com — Social Media Character Limits 2026](https://goldentoolhub.com/social-media-character-limits-2026/) — 4,000 char caption limit (MEDIUM confidence, corroborated by hootsuite)
- [Hootsuite blog — TikTok video descriptions up to 2,200 characters](https://blog.hootsuite.com/social-media-updates/tiktok/tiktoks-video-descriptions-can-now-be-up-to-2200-characters-long/) — historical context on limit expansion from 2,200 (HIGH confidence — official news coverage)
- [conturata.com — Writing TikTok SEO Descriptions With ChatGPT](https://conturata.com/ai/tiktok-seo) — description structure: 800-2,200 chars, 5 hashtags (3 broad + 2 niche), conversational tone (MEDIUM confidence — practitioner guide)
- [buffer.com — Top TikTok Hashtags + How to Use Them](https://buffer.com/resources/tiktok-hashtags/) — 3-5 quality hashtags recommended, quality > quantity (MEDIUM confidence — well-known social media tool vendor)
- [fadeevab.com — Comparison of Rust CLI Prompts](https://fadeevab.com/comparison-of-rust-cli-prompts/) — cliclack vs dialoguer vs inquire vs promptly comparison (MEDIUM confidence — technical comparison, single author)
- [inquire GitHub — mikaelmello/inquire](https://github.com/mikaelmello/inquire) — Select and Text prompt types, RenderConfig (HIGH confidence — official repo)
- Codebase inspection — `src/commands/caption.rs` line 204: MarginV=480; `src/commands/overlay.rs` line 160-163: position y values; `src/commands/pipeline.rs`: pipeline stage ordering (HIGH confidence — direct code read)

---

*Feature research for: TikTok upload readiness (contentops v1.5)*
*Researched: 2026-02-25*
