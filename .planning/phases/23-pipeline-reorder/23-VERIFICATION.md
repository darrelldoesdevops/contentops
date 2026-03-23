---
phase: 23-pipeline-reorder
verified: 2026-03-22T19:35:00Z
status: human_needed
score: 3/4 must-haves verified (4th requires real video playback)
human_verification:
  - test: "Run pipeline on a real talking-head video and watch the output"
    expected: "Caption word highlights track speech with no drift — words light up exactly when spoken, not ahead or behind"
    why_human: "Caption sync correctness depends on Whisper timestamp accuracy on the cut video, which requires playback to assess. Code structure is correct but the subjective quality of highlight timing cannot be verified programmatically."
---

# Phase 23: Pipeline Reorder Verification Report

**Phase Goal:** The pipeline runs cut before transcription so Whisper timestamps align with the final video and captions highlight correctly
**Verified:** 2026-03-22T19:35:00Z
**Status:** human_needed (all automated checks passed; real-video caption sync needs human confirmation)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Pipeline runs cut before transcription so Whisper timestamps match the cut video | VERIFIED | `Stage 3/7: cut` at line 300; `Stage 4/7: transcribe` at line 412 — cut block (lines 299-409) executes before transcribe call (line 413) |
| 2 | No call to `adjust_timestamps` exists in the pipeline execution path | VERIFIED | `grep adjust_timestamps pipeline.rs` → no matches; `adjusted_words` and `word_data` also absent |
| 3 | Caption word boundaries align with speech timing because Whisper runs on already-cut audio | UNCERTAIN | Code structure is correct — `caption::transcribe(&cut_output, ..., None)` at line 413 passes cut video with `None` for wav_path so Whisper self-extracts from final timeline. Actual alignment quality requires human playback test |
| 4 | Dry-run output shows correct stage order: scale -> normalize -> cut -> transcribe -> fix -> caption -> overlay | VERIFIED | Lines 163-172: `3. cut`, `4. transcribe`, `5. fix`, `6. caption`, `7. overlay` in correct order |

**Score:** 3/4 automated truths verified (truth 3 needs human confirmation)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/pipeline.rs` | Reordered pipeline with cut-before-transcribe flow | VERIFIED | File exists, 530 lines, substantive implementation with full stage logic |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `run_stages()` cut block | `caption::transcribe()` call | cut runs first, then transcribe receives cut video | VERIFIED | Cut block at lines 299-409 completes before `caption::transcribe(&cut_output, model, "en", verbose, registry, None)` at line 413 |
| `run_stages()` `words` result | `finish_stages()` words argument | words passed directly, no `adjust_timestamps` transformation | VERIFIED | `finish_stages(temp_dir, &cut_output, &words, ...)` at line 424-434 — `&words` not `&adjusted_words` |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| PIPE-01 | 23-01-PLAN.md | Pipeline runs cut before transcription so Whisper timestamps match cut video timeline | SATISFIED | `Stage 3/7: cut` (line 300) precedes `Stage 4/7: transcribe` (line 412); `caption::transcribe(&cut_output, ...)` at line 413 |
| PIPE-02 | 23-01-PLAN.md | `adjust_timestamps` logic removed from pipeline | SATISFIED | No occurrences of `adjust_timestamps`, `adjusted_words`, or `word_data` anywhere in `pipeline.rs` |
| PIPE-03 | 23-01-PLAN.md | Caption highlight tracks spoken words accurately without boundary drift or timestamp clamping artifacts | NEEDS HUMAN | Code path is correct (Whisper runs on cut video, no post-hoc adjustment), but subjective caption sync quality requires real-video playback |

**Orphaned requirements check:** REQUIREMENTS.md maps PIPE-01, PIPE-02, PIPE-03 to Phase 23 — all accounted for by 23-01-PLAN.md. CLN-01 is mapped to Phase 24, not orphaned here.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

No TODOs, FIXMEs, placeholders, empty returns, or stub patterns detected in `pipeline.rs`.

### Human Verification Required

#### 1. Caption Sync on Real Video

**Test:** Run `contentops pipeline -i <talking-head-video.mp4> -m <whisper-model-path>` on a real video

**Expected:**
- Stage output prints: Stage 1/7: scale, Stage 2/7: normalize, Stage 3/7: cut, Stage 4/7: transcribe, Stage 5/7: fix, Stage 6/7: caption, Stage 7/7: overlay
- Output video caption word highlights track speech — words illuminate when spoken, no leading or lagging drift
- No visible timestamp clamping artifacts (words jumping or grouping unnaturally)

**Why human:** Caption alignment quality is a subjective visual judgment that depends on Whisper's timestamp output on the specific cut video. Code structure guarantees Whisper runs on the cut timeline, but the resulting perceptual quality of the highlights requires watching the output video.

**Dry-run verification (can do this now):** Run `contentops pipeline --dry-run -i <any-file> -m <any-path>` and confirm lines 3 and 4 read `3. cut` and `4. transcribe`.

### Gaps Summary

No gaps. All automated acceptance criteria from the plan pass:

- `Stage 3/7: cut` present at line 300
- `Stage 4/7: transcribe` present at line 412
- `Stage 5/7: fix` present at line 421
- `caption::transcribe(&cut_output, model, "en", verbose, registry, None)` at line 413 — first arg is `&cut_output`, last arg is `None`
- `adjust_timestamps` not present in `pipeline.rs`
- `adjusted_words` not present in `pipeline.rs`
- `word_data` not present in `pipeline.rs`
- `finish_stages(... &words ...)` at line 427 — uses `&words` not `&adjusted_words`
- Dry-run prints `3. cut` (line 165) and `4. transcribe` (line 167)
- `cargo fmt --check` passes
- `cargo clippy -- -D warnings` passes
- `cargo test` passes (3 tests)

The only outstanding item is human confirmation that real video captions sync correctly (PIPE-03), which was flagged as a blocking human-verify checkpoint in the original plan.

---

_Verified: 2026-03-22T19:35:00Z_
_Verifier: Claude (gsd-verifier)_
