# Architecture Patterns

**Domain:** TikTok metadata generation and safe zone compliance — Rust CLI integration
**Researched:** 2026-02-25
**Confidence:** HIGH (direct codebase audit) / MEDIUM (TikTok safe zone pixel values from multiple third-party sources; no official pixel spec published)

---

## Integration Question Summary

Four new capabilities integrate into the existing 6-stage pipeline:

1. **Title approval** — interactive user confirmation before overlay burns
2. **Safe zone margins** — ASS subtitle positioning + overlay drawtext must stay inside TikTok UI-free area
3. **Description generation** — Claude generates TikTok description/hashtags from transcript
4. **Sidecar file** — metadata written next to output file at pipeline completion

---

## TikTok Safe Zone: Concrete Numbers

For 1080x1920 video (the pipeline's reference resolution):

| Area | Danger Zone | Safe Threshold (conservative) |
|------|-------------|-------------------------------|
| Top | ~150px (username, sound label) | 200px minimum; 250px recommended |
| Bottom | 250–370px (caption bar, engagement icons) | 420px minimum clearance |
| Left | 60px | 60px |
| Right | 120px (like/comment/share stack) | 120px |

Source confidence: MEDIUM. No official pixel spec exists. Numbers synthesized from three independent third-party guides that cluster around these values. The variance between sources (bottom ranges from 250 to 480px) reflects device-to-device UI differences. Use the conservative bound (420px from bottom, 120px from right).

---

## Component Boundaries

### Current State (post-Phase 18)

```
pipeline.rs::finish_stages()
    ├── [5] caption::burn_captions()   → captioned.mp4
    └── [6] overlay::run()
            ├── generate_title() → claude haiku call → title string
            └── build_title_filter() + ffmpeg → output.mp4
```

### New State (TikTok milestone)

```
pipeline.rs::finish_stages()
    ├── [5] caption::burn_captions()   → captioned.mp4       [MODIFIED: safe zone margins]
    ├── [5.5] APPROVAL GATE            → approved title       [NEW: interactive prompt]
    │         generate_title() moved here (before encoding, not during)
    ├── [6] overlay::run()             → output.mp4           [MODIFIED: safe zone margins]
    └── [7] metadata::generate()       → sidecar .json        [NEW: description + hashtags]
```

**Key insight:** Title approval is currently buried inside `overlay::run()` inside `generate_title()`. It must move to `finish_stages()` in `pipeline.rs`, before `overlay::run()` is called. This way:
- Caption encoding (Stage 5) runs uninterrupted
- While caption encodes, there is no opportunity to prompt — it's a good time for Claude to generate the title concurrently (but Rust's single-threaded pipeline does not currently do this)
- Prompt appears immediately after caption encoding completes, before overlay begins
- User reads/edits title, then overlay encoding starts

This is the maximum parallelism achievable without threading: caption encode → prompt → overlay encode.

---

## Where Title Approval Fits in the Pipeline

**Between Stage 5 (caption burn) and Stage 6 (overlay).**

In `pipeline.rs::finish_stages()`, the sequence becomes:

```rust
// Stage 5: burn captions (unchanged encoding)
caption::burn_captions(cut_video, words, &captioned_video, verbose, registry)?;

// Stage 5.5: generate title and get approval (NEW — moved from overlay::run)
let title = generate_title_with_approval(&caption_json, verbose)?;

// Stage 6: overlay (title already approved, no Claude call inside)
overlay::run(OverlayArgs {
    text: Some(title),  // always pass as text now, not auto
    auto: None,
    ...
}, verbose, registry)?;
```

`overlay::run()` already handles `text: Some(...)` — the pipeline just switches from passing `auto: Some(json_path)` to `text: Some(approved_title)`. No changes needed inside `overlay.rs` for the approval flow.

The `generate_title()` function in `overlay.rs` can remain for the standalone `overlay` subcommand's `--auto` path. In the pipeline, it is called at the `finish_stages` level instead.

---

## How Safe Zone Margins Affect ASS Positioning

### Current ASS Style Line (caption.rs line 204)

```
Style: Default,Arial,58,&H00FFFFFF,...,Alignment,MarginL,MarginR,MarginV,...
Style: Default,Arial,58,&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,4,2,2,40,40,480,1
```

Decoded:
- `Alignment: 2` = bottom-center (ASS alignment: 1=BL, 2=BC, 3=BR, 4=ML, 5=MC, 6=MR, 7=TL, 8=TC, 9=TR)
- `MarginL: 40` — left margin in ASS coordinate space (PlayResX=1080)
- `MarginR: 40` — right margin
- `MarginV: 480` — vertical margin from bottom edge (because Alignment=2 is bottom)

**Analysis against safe zone:**
- `MarginV: 480` — at 1920 reference, 480px from bottom is already above the 420px danger zone. SAFE.
- `MarginL: 40, MarginR: 40` — too thin. Right side needs 120px clear. Change to `MarginL: 80, MarginR: 120`.

**Recommended change to `generate_ass()`:**

```rust
// Before:
output.push_str("Style: Default,Arial,58,...,2,40,40,480,1\n");
//                                              MarginL MarginR MarginV

// After:
output.push_str("Style: Default,Arial,58,...,2,80,120,480,1\n");
//                                               ^L  ^R   ^V stays same
```

The MarginV of 480 already clears the bottom safe zone. Only the left/right margins need adjustment.

Note: ASS `PlayResX/PlayResY` are 1080/1920 hardcoded. These are ASS coordinate space values, not pixels. When ffmpeg burns `ass=` with a different resolution video, ffmpeg scales the ASS coordinates proportionally. The margin values scale correctly.

---

## How Safe Zone Margins Affect Overlay drawtext

### Current overlay.rs positioning

```rust
let y_base: u32 = match args.position.as_str() {
    "bottom" => scale(1400, video_height),  // 1400/1920 = 72.9% from top = 27% from bottom
    "center" => scale(760, video_height),
    _ => scale(200, video_height),          // "top" = 200px from top in 1920 ref space
};

let final_x: i32 = scale_i32(30, video_height);  // 30px left margin — UNSAFE
```

**Analysis:**
- `"top"` position: y=200px. TikTok top danger zone is 150-250px. Current 200px is marginal; 250px is safer.
- `final_x = 30px`: TikTok left margin is 60px minimum. Current 30px is UNSAFE.
- `"bottom"` position: y=1400px. Bottom of overlay text lands around y=1400+(3 lines * ~170px) ≈ 1910px. That is UNSAFE — fully inside the bottom danger zone (bottom 370-420px = y > 1500px in 1920 ref). Bottom position needs to move up to ~y=1050 to clear.

**Recommended changes to `build_title_filter()` constants:**

```rust
// Safe zone aligned values (reference: 1920px height)
let final_x: i32 = scale_i32(60, video_height);      // was 30 → 60 (left safe zone)

let y_base: u32 = match args.position.as_str() {
    "bottom" => scale(1050, video_height),  // was 1400 → 1050 (clears bottom danger zone)
    "center" => scale(760, video_height),   // unchanged — already in safe zone
    _ => scale(250, video_height),          // was 200 → 250 (clears top danger zone)
};
```

The accent bar `accent_x` will adjust automatically since it is computed relative to `final_x`.

---

## Where Description Generation Happens

**After Stage 6 (overlay completes), as Stage 7.**

Description generation is a read-only Claude call that does not modify the video. It runs after the final video file is written. This placement:
- Does not block any encoding
- Has access to the final approved title (needed for description coherence)
- Has access to the full transcript from `caption_json`

In `finish_stages()`:

```rust
// Stage 7: generate TikTok metadata (NEW)
let metadata = metadata::generate(&caption_json, &approved_title, verbose)?;
let sidecar_path = output.with_extension("json");
metadata::write_sidecar(&metadata, &sidecar_path)?;
```

A new module `src/metadata.rs` owns this. It takes the transcript JSON path and approved title, shells out to `claude -p --model haiku`, returns a `TikTokMetadata` struct.

---

## Sidecar File: Format and Timing

**Written:** After Stage 6 completes (overlay done), at end of `finish_stages()`.

**Location:** Same directory as output file, same stem, `.json` extension.
- Example: `video_pipeline.mp4` → `video_pipeline.json`
- Uses `output.with_extension("json")` in Rust.

**Format:** JSON

```json
{
  "title": "The approved title text\nwith newlines preserved",
  "description": "Full TikTok caption text up to 2200 chars",
  "hashtags": ["hashtag1", "hashtag2", "hashtag3", "hashtag4", "hashtag5"]
}
```

Rationale for this schema:
- `title` is the string shown in overlay — useful for reference/editing
- `description` is ready-to-paste into TikTok's caption field
- `hashtags` as array (not embedded in description) allows caller to join with spaces, pick a subset, or format differently
- TikTok limits: 2200 chars total description, max 5 hashtags recommended
- No `video_path`, `duration`, or timestamps — those are derivable; keep the sidecar minimal

**Struct in Rust:**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TikTokMetadata {
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
}
```

---

## New vs Modified Components

| Component | Status | Change |
|-----------|--------|--------|
| `src/metadata.rs` | NEW | Claude call → TikTokMetadata, sidecar write |
| `src/commands/pipeline.rs::finish_stages()` | MODIFIED | Add approval gate, move generate_title call, call metadata::generate |
| `src/commands/caption.rs::generate_ass()` | MODIFIED | MarginL 40→80, MarginR 40→120 |
| `src/commands/overlay.rs::build_title_filter()` | MODIFIED | final_x 30→60, top y_base 200→250, bottom y_base 1400→1050 |
| `src/commands/overlay.rs::generate_title()` | UNCHANGED | Still used by standalone `overlay --auto` path |
| `src/cli.rs::PipelineArgs` | NO CHANGE | No new flags needed for this milestone |

---

## Data Flow: New Pipeline End Sequence

```
[Stage 4 complete] cut.mp4 + adjusted_words[] in memory

finish_stages():
    write captioned.json (transcript for overlay auto-title)   [UNCHANGED]

[Stage 5] caption::burn_captions(cut.mp4, words) → captioned.mp4   [MODIFIED: ASS margins]

[Stage 5.5] generate_title(captioned.json) → raw_title              [NEW in pipeline.rs]
            prompt user: "Title: {raw_title}\n[Enter to accept / type replacement]:"
            → approved_title: String

[Stage 6] overlay::run(captioned.mp4, text=approved_title) → output.mp4  [MODIFIED: drawtext margins]

[Stage 7] metadata::generate(captioned.json, approved_title) → TikTokMetadata  [NEW]
           write output.with_extension("json") → sidecar                         [NEW]

finish_stages() returns Ok(())
```

---

## Title Approval: Making It Non-Blocking During Encoding

The only opportunity to run title generation concurrently with encoding is during Stage 5 (caption burn). This requires threading or async. The current pipeline is single-threaded and synchronous.

**Recommendation: do not introduce threading for this milestone.** The simpler approach:

```
Stage 5 encodes → Stage 5 completes → Claude generates title → user approves → Stage 6 encodes
```

The wait for user approval typically exceeds the difference between "title generated during encode" and "title generated after encode." For a 3-minute video, caption encoding takes ~60-90 seconds; title generation takes ~2-5 seconds. The sequential overhead is ~5 seconds — not worth threading complexity.

If future milestones add threading, the `generate_title()` call could be spawned as a `std::thread::spawn` before the `burn_captions()` call, with a `JoinHandle` resolved after encoding completes. But this is out of scope.

---

## Approval UX Pattern

Use `stdin` readline, print to `stderr` (consistent with all other pipeline output):

```rust
// Prompt on stderr; read from stdin
eprint!("Title: {}\n[Enter] accept  [type replacement + Enter] override: ", raw_title);
let mut input = String::new();
std::io::stdin().read_line(&mut input)?;
let approved = input.trim();
if approved.is_empty() {
    raw_title  // accept as-is
} else {
    approved.to_string()  // use replacement
}
```

This pattern:
- Works in terminal environments (no TTY detection needed for initial milestone)
- Matches existing pipeline UX (all output to stderr)
- Does not require a new dependency (no `dialoguer` or `inquire` needed)

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Moving generate_title Into metadata.rs

**What:** Combining title generation and description generation in one module.
**Why wrong:** `generate_title()` is called by both `overlay.rs` (standalone `--auto` path) and `pipeline.rs` (for approval). Putting it in `metadata.rs` creates an import dependency from `overlay.rs` to `metadata.rs`, which is awkward. Title generation belongs in `overlay.rs`; description generation belongs in `metadata.rs`.
**Do this instead:** Keep `generate_title()` in `overlay.rs`. Call it from `pipeline.rs` directly before `overlay::run()`. `metadata::generate()` receives the already-approved title as a parameter.

### Anti-Pattern 2: Embedding Hashtags in the Description String

**What:** Generating description as one blob: `"Here's why...\n\n#tiktok #content #creator"`.
**Why wrong:** Makes it impossible to pick a subset of hashtags, reformat, or apply per-platform hashtag limits without string parsing.
**Do this instead:** Separate `hashtags: Vec<String>` field in the sidecar. The description field contains only prose text. Caller concatenates as needed.

### Anti-Pattern 3: Writing Sidecar to Temp Dir

**What:** Writing the `.json` sidecar into `temp_dir` alongside `captioned.mp4` and `cut.mp4`.
**Why wrong:** `temp_dir` is cleaned up on error (or retained for debugging). The sidecar is a user-facing output that should survive next to the final video, not be buried in temp.
**Do this instead:** `output.with_extension("json")` — sibling to the final output file.

### Anti-Pattern 4: Hardcoding Safe Zone Pixel Values in ASS

**What:** Using absolute pixel values (e.g., `MarginV: 480`) without documenting they assume `PlayResY: 1920`.
**Why wrong:** If someone changes `PlayResX/PlayResY`, the absolute margins break.
**Do this instead:** Keep the current approach (ASS coordinate space with fixed PlayRes), but document that `PlayResY: 1920` is the reference. The values are in ASS coordinate space, not screen pixels. FFmpeg scales them when burning onto videos of other resolutions. This is already correct behavior — just document it.

---

## Build Order for This Milestone

| Order | Task | Depends On | Notes |
|-------|------|------------|-------|
| 1 | Fix `generate_ass()` margins (MarginL/MarginR) | Nothing | 2-line change; verify with test video that captions clear right edge |
| 2 | Fix `build_title_filter()` x and y_base values | Nothing | Independent of ASS change; verify top/bottom position visually |
| 3 | Extract title generation call from `finish_stages()` + add approval prompt | Steps 1-2 proven | Refactor only — no new deps; test with `--text` bypass to confirm overlay still works |
| 4 | Create `src/metadata.rs` with `generate()` + `write_sidecar()` | Step 3 (needs approved_title param) | New Claude call; test with real transcript |
| 5 | Wire `metadata::generate()` into `finish_stages()` | Steps 3-4 | Final integration; verify sidecar location and content |
| 6 | Add `metadata` module to `src/commands/mod.rs` or `src/lib.rs` | Step 4 | Module registration |

---

## Sources

- Direct codebase audit: `/Users/darrelltang/darrelldoesdevops/contentops/src/`
- TikTok safe zone third-party consensus: https://zeely.ai/blog/tiktok-safe-zones/ (MEDIUM)
- TikTok safe zone checker tool context: https://postplanify.com/tools/tiktok-safe-zone-checker (MEDIUM)
- TikTok description character limits: https://tlinky.com/tiktok-ad-copy-character-limit/ (MEDIUM)
- ASS subtitle format specification: https://aegi.moe/docs/ASS_Tags.htm (alignment/margin semantics)
- Existing pipeline architecture: phases 17-18 context files, direct source audit

---

*Architecture research for: TikTok metadata generation and safe zone compliance integration*
*Researched: 2026-02-25*
