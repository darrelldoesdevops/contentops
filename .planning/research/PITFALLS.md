# Pitfalls Research

**Domain:** Rust CLI video processing pipeline — adding interactive approval, safe zone compliance, and AI metadata
**Researched:** 2026-02-25
**Confidence:** HIGH (grounded in direct codebase audit of src/)

> **Scope note:** This file was added for the TikTok metadata/safe zone milestone.
> It covers pitfalls specific to ADDING interactive prompts, safe zone margin changes,
> multi-option Claude title generation, metadata sidecars, and transcript fix changes
> to the existing headless pipeline. Previous pitfalls (1–22) live in this file's
> predecessor from the VAD milestone.

---

## Critical Pitfalls

### Pitfall 1: Interactive Prompt Blocks Headless Pipeline

**What goes wrong:**
`pipeline.rs::run_stages()` runs entirely non-interactively — all `Stdio::null()` on stdin, all user feedback via `eprintln!` spinners. Adding a `dialoguer` or raw `stdin().read_line()` call inside `run_stages` or `finish_stages` blocks indefinitely when the tool is called from a shell script, CI, or any context where stdin is not a TTY (e.g., `contentops pipeline ... < /dev/null`).

**Why it happens:**
Interactive approval feels natural to add at the end of the Claude title generation step (overlay.rs line 99 region), right after `generate_title()` returns. But `pipeline.rs` passes through `finish_stages → overlay::run` without any TTY check. The prompt renders, gets no input, and hangs.

**How to avoid:**
Gate interactive prompts behind a TTY check before rendering. In Rust, use `atty::is(atty::Stream::Stdin)` or `std::io::IsTerminal::is_terminal(&std::io::stdin())` (stable since Rust 1.70). If not a TTY, skip approval and proceed with the first generated option. Document this behavior: `--approve` flag has no effect in non-interactive mode.

**Warning signs:**
- Any `stdin().read_line()` call not wrapped in an `is_terminal()` check
- `dialoguer::Select` or `dialoguer::Confirm` used without a TTY guard
- Integration tests hang when piping stdin from `/dev/null`

**Phase to address:**
Interactive approval phase — first thing verified before any other approval logic is wired up.

---

### Pitfall 2: Safe Zone Margin Change Silently Breaks Existing Videos

**What goes wrong:**
The ASS style in `generate_ass()` (caption.rs line 204) has `MarginV: 480` hardcoded. This is 480px from the bottom of the 1920px reference frame — about 25% up. TikTok's bottom UI overlay (like button, share, caption bar) covers roughly the bottom 20–25% (~384–480px). Changing `MarginV` to fix safe zone compliance changes the position of ALL subtitles retroactively on any video that re-runs the pipeline, silently producing different output from an identical command.

**Why it happens:**
`MarginV` is set once in the style block and applies globally. Developers increasing it to 540–600 to clear the TikTok UI don't realize that the existing value was already the "intended" position for non-TikTok use, and that increasing it shifts text higher into the video frame where it may cover the subject's face.

**How to avoid:**
Add a `--safe-zone` flag (default: off) that activates TikTok-compliant margins rather than changing the default. The flag computes safe margins from platform presets (TikTok 9:16 bottom UI = ~480px, top nav = ~120px). Keep the existing default unchanged so existing videos reproduce identically.

**Warning signs:**
- PRs that change the numeric literal on caption.rs line 204 without a flag gate
- Any change to `MarginV` in `generate_ass()` without a corresponding CLI flag
- Test that diffs ASS output before/after: any change to the default style line is a regression

**Phase to address:**
Safe zone implementation phase — define the flag interface before touching any margin numbers.

---

### Pitfall 3: ASS MarginV Is Bottom Margin, Not Top Position

**What goes wrong:**
ASS subtitle `Alignment: 2` (bottom-center) means `MarginV` is the distance from the **bottom** of the video frame. If you treat it as a top-Y coordinate and compute "I want subtitles at 1400px from top in a 1920px frame, so MarginV = 1400," you get subtitles 1400px from the bottom — completely off-screen above the video.

**Why it happens:**
The ASS spec alignment numbering (numpad layout) is non-obvious. Alignment 2 = bottom-center. The `y_base` logic in `overlay.rs::build_title_filter()` uses a different coordinate system (drawtext `y=` is top-origin), which reinforces the confusion when developers work across both caption and overlay code in the same PR.

**How to avoid:**
Annotate the `MarginV` line with a comment: `// MarginV: pixels from BOTTOM of frame (Alignment=2, bottom-center)`. When computing safe zone values, work backward: `safe_margin_v = frame_height - tiktok_ui_height` is wrong; `safe_margin_v = tiktok_ui_top_zone_px` is correct.

**Warning signs:**
- Subtitle text disappearing from the frame entirely after margin change
- Safe zone calculation that references frame height minus a UI overlay height as the MarginV value

**Phase to address:**
Safe zone implementation phase — add the comment before computing any new values.

---

### Pitfall 4: Multi-Option Claude Response Parsed as Single Title

**What goes wrong:**
The existing `generate_title()` in `overlay.rs` prompts Claude to return a single title (lines 38–43). If the new milestone asks Claude to return 3 options for user selection, the prompt and parser must both change atomically. If the prompt changes to request a numbered list but the parser still does `String::from_utf8_lossy(&output.stdout).trim().to_string()`, option 2 and 3 text gets used verbatim as the title, including the numbering ("2. THE HOOK\n3. ANOTHER HOOK").

**Why it happens:**
The prompt and the parser are separate concerns written in different parts of the code. It's easy to update the prompt wording without updating the downstream parsing logic, especially if the code is refactored across a new function.

**How to avoid:**
When changing to multi-option generation, define a structured response format first (e.g., newline-delimited, or JSON array). Write the parser before the prompt. Test with a mocked Claude response that includes numbering, preamble text ("Here are 3 options:"), trailing explanation — all of which Claude can produce even when instructed not to. The existing codebase already strips markdown fences (caption.rs lines 326–336) — replicate that defensive parsing for multi-option responses.

**Warning signs:**
- Numbering or bullet characters appearing in the final rendered title overlay
- Empty first option when Claude returns "Here are your options:\n1. ..."
- Title truncated at first newline when options are newline-separated

**Phase to address:**
Claude multi-option generation phase — write parser tests before wiring to real Claude.

---

### Pitfall 5: Word-Count Mismatch Guard Breaks If Metadata Prompt Reuses fix_transcription

**What goes wrong:**
`fix_transcription()` (caption.rs lines 352–362) has a strict guard: if `corrected.len() != words.len()`, it silently falls back to original words. This is intentional for transcription fixing. If the metadata generation milestone reuses or extends `fix_transcription()` to also generate TikTok hashtags or descriptions inline (e.g., by appending metadata as extra fields), Claude's response will always have a different element count and the guard will silently discard all corrections.

**Why it happens:**
`fix_transcription` has a clean public API that developers may try to extend rather than duplicate. Adding a metadata field to the response object seems like a natural extension. But the word-count guard was designed for a contract where the array length is invariant.

**How to avoid:**
Keep metadata generation entirely separate from `fix_transcription`. New metadata goes into a dedicated function with its own Claude call and its own response parser. The word-count guard in `fix_transcription` is load-bearing correctness — do not modify it or add additional fields to the Word struct that get populated by Claude responses.

**Warning signs:**
- Any modification to `fix_transcription()` signature or its response parsing
- Adding fields to the `Word` struct that are populated by LLM output
- Log output showing "Warning: LLM returned N words, expected M" more frequently than before

**Phase to address:**
Metadata generation phase — explicitly note in the phase plan that `fix_transcription` is off-limits.

---

### Pitfall 6: Sidecar JSON Path Collides With Existing caption Command Output

**What goes wrong:**
`caption::run()` already writes a JSON sidecar at `{stem}_captioned.json` (caption.rs lines 622–629). If the new milestone writes a TikTok metadata sidecar using a similar path derivation, two different commands can write to the same path, silently overwriting each other. Specifically: running `contentops caption` on a file then `contentops pipeline` on the same file, or running `pipeline` twice, could produce a metadata JSON that overwrites the word-level transcript JSON or vice versa.

**Why it happens:**
`derive_caption_output()` (caption.rs line 68) is the shared path derivation function. Any new sidecar that uses the same stem + suffix pattern risks collision unless the suffix is explicitly unique. The pattern `{stem}_captioned.json` is used by both `caption` command and `pipeline`'s `finish_stages` (pipeline.rs line 284, `captioned.json` in temp dir). These happen to not collide today because pipeline uses a temp dir. A new metadata file using the same naming in the final output dir would collide with the caption command's output.

**How to avoid:**
Use a distinct suffix for metadata sidecars: `{stem}_tiktok.json` or `{stem}_metadata.json`, never `{stem}_captioned.json`. Audit all sidecar writes before adding a new one to confirm there is no suffix overlap. Add the output path to the function's docstring once established.

**Warning signs:**
- New sidecar write using `derive_caption_output(&args.input, "captioned", "json")`
- Any path computation that produces `{stem}_captioned.json` from a non-caption command
- User reports that re-running pipeline overwrites their word-level transcript

**Phase to address:**
Metadata sidecar phase — define path convention as first task.

---

### Pitfall 7: Indicatif Spinner Corrupts Interactive Prompt Rendering

**What goes wrong:**
`ui::make_spinner()` creates an `indicatif::ProgressBar` with `enable_steady_tick`. When an interactive prompt (dialoguer, raw stdin readline) is displayed while a spinner is active, the spinner's tick overwrites the prompt line at 80ms intervals. The user sees their cursor position jumping or the prompt disappearing as the spinner redraws. On some terminals, the prompt becomes unresponsive.

**Why it happens:**
The existing spinner pattern in `fix_transcription()` and `generate_title()` is "start spinner, do work, finish spinner." Interactive approval adds a third state: "start spinner, generate options, finish spinner, then show prompt." If the spinner is not fully finished (not `finish_and_clear()`) before the prompt renders, the steady tick is still running.

**How to avoid:**
Always call `pb.finish_and_clear()` (not `finish_with_message()`) on any spinner before displaying an interactive prompt. Treat spinner completion and prompt display as a required sequence, not concurrent operations. Test by inserting an artificial sleep after `finish_and_clear()` and verifying the terminal is clean before the prompt appears.

**Warning signs:**
- Prompt text flickering or disappearing immediately after appearing
- User input echoed on wrong terminal line
- Works fine in `--verbose` mode (no spinner) but breaks in normal mode

**Phase to address:**
Interactive approval phase — tested before merging any prompt code.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcode safe zone margins as constants | Ship faster | Every new platform (YouTube Shorts, Instagram Reels) requires code change + rebuild | Never — use a `--platform` flag with presets |
| Single Claude call for both fix + metadata | One fewer subprocess | Breaks word-count guard, forces complex response parsing | Never — keep them separate |
| Skip TTY check on interactive prompts | Simpler code | CI/scripted use hangs indefinitely | Never |
| Reuse `_captioned.json` suffix for metadata | Familiar naming | Silent overwrite of word-level transcript | Never |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Claude CLI multi-option | Asking for "a numbered list" — Claude adds preamble "Here are 3 options:" | Ask for bare newline-separated values; strip any non-option lines defensively |
| Claude CLI multi-option | Parsing on `\n` split, getting empty strings from trailing newlines | `split('\n').filter(|s| !s.trim().is_empty())` |
| ASS MarginV | Computing as distance from top (drawtext `y=` style) | MarginV for Alignment=2 is distance from bottom of frame |
| dialoguer in pipeline | Using `dialoguer::Select` without TTY check | Wrap in `std::io::IsTerminal::is_terminal(&std::io::stdin())` guard |
| Indicatif + dialoguer | Showing prompt while spinner is ticking | `pb.finish_and_clear()` before any prompt |

---

## "Looks Done But Isn't" Checklist

- [ ] **Interactive approval:** Verify the flow works when `stdin` is `/dev/null` (pipe test: `contentops pipeline ... < /dev/null`)
- [ ] **Safe zone:** Verify that running without `--safe-zone` produces byte-identical ASS output to the current codebase (regression test on the style line)
- [ ] **Multi-option Claude:** Verify parser handles Claude prepending "Here are your options:" or numbering responses — test with a mocked stdout
- [ ] **Metadata sidecar:** Verify running `contentops caption` and `contentops pipeline` on the same file does not produce a filename collision
- [ ] **Spinner + prompt:** Verify in a real terminal (not test runner) that prompt is visible and accepts input without spinner interference
- [ ] **word-count guard:** Verify `fix_transcription` still logs "Warning: LLM returned N words" for mismatches after any metadata changes nearby

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Interactive prompt blocks headless pipeline | Interactive approval — first task | `contentops pipeline ... < /dev/null` completes without hanging |
| Safe zone change breaks existing output | Safe zone implementation — flag interface first | `--no-safe-zone` (or absence of `--safe-zone`) produces byte-identical ASS style line |
| ASS MarginV coordinate confusion | Safe zone implementation — annotate before changing values | Manual review of computed safe margin values against TikTok layout spec |
| Multi-option Claude response parsing | Claude multi-option generation — write parser first | Unit test with mock Claude outputs including preamble and trailing whitespace |
| Word-count guard broken by metadata changes | Metadata generation — keep separate from fix_transcription | `fix_transcription` unit tests still pass; word-count mismatch still warns correctly |
| Sidecar path collision | Metadata sidecar — define path convention first | Running both `caption` and `pipeline` on same file produces distinct JSON files |
| Spinner corrupts interactive prompt | Interactive approval — test in real terminal | Manual test: observe prompt renders cleanly after spinner clears |

---

## Sources

- Direct codebase audit: `src/commands/caption.rs`, `src/commands/overlay.rs`, `src/commands/pipeline.rs`, `src/ui.rs`
- ASS subtitle spec — Alignment numpad layout, MarginV semantics: [libass documentation](https://github.com/libass/libass)
- Rust `IsTerminal` trait: stable since 1.70.0 — [std::io::IsTerminal](https://doc.rust-lang.org/std/io/trait.IsTerminal.html)
- Indicatif + dialoguer interaction: known issue in indicatif issue tracker (concurrent steady tick and stdin read)
- TikTok safe zone: community-reported bottom UI covers ~384–480px of 1920px frame (MEDIUM confidence — no official TikTok spec published)

---
*Pitfalls research for: TikTok metadata/safe zone milestone additions to contentops*
*Researched: 2026-02-25*
