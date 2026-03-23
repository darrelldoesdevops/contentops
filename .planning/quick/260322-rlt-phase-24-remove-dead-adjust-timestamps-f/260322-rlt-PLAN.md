---
phase: 24-dead-code-removal
plan: 01
type: execute
wave: 1
depends_on: []
files_modified: [src/silence.rs]
autonomous: true
requirements: [CLN-01]

must_haves:
  truths:
    - "No adjust_timestamps function exists in the codebase"
    - "No monotonicity clamping logic exists in silence.rs"
    - "cargo clippy passes with zero warnings — no dead_code lint needed"
  artifacts:
    - path: "src/silence.rs"
      provides: "SpeechInterval, build_concat_filter, total_silence_from_speeches only"
      contains: "pub fn build_concat_filter"
  key_links: []
---

<objective>
Remove the dead `adjust_timestamps` function and its monotonicity clamping logic from `src/silence.rs`.

Purpose: Phase 23 reordered the pipeline so Whisper runs on the cut video directly. The `adjust_timestamps` call was removed from `pipeline.rs`, leaving the function definition as dead code. This completes the cleanup (CLN-01).
Output: `silence.rs` contains only `SpeechInterval`, `build_concat_filter`, and `total_silence_from_speeches`.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@src/silence.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove adjust_timestamps function from silence.rs</name>
  <files>src/silence.rs</files>
  <action>
    Delete the entire `adjust_timestamps` function (lines 32-78 of src/silence.rs). This includes:
    - The function signature: `pub fn adjust_timestamps(word_times: &[(f64, f64, String)], speeches: &[SpeechInterval]) -> Vec<(f64, f64, String)>`
    - The cumulative gap calculation loop
    - The word-to-speech-interval mapping loop
    - The monotonicity enforcement block (the "Enforce monotonicity" comment and all clamping logic below it)

    After deletion, `silence.rs` should contain exactly three public items:
    1. `pub struct SpeechInterval` (with start/end fields)
    2. `pub fn build_concat_filter`
    3. `pub fn total_silence_from_speeches`

    No other changes needed. No callers remain (verified: grep of src/ returns only the definition site). No tests reference this function (tests/silence_tests.rs only tests build_concat_filter).

    Per CLN-01: Dead adjust_timestamps function and monotonicity clamping code removed from silence.rs.
  </action>
  <verify>
    <automated>cd /Users/darrelltang/darrelldoesdevops/contentops && cargo fmt && cargo clippy -- -D warnings && cargo test 2>&1 | tail -5</automated>
  </verify>
  <done>silence.rs contains only SpeechInterval, build_concat_filter, and total_silence_from_speeches. `grep adjust_timestamps src/silence.rs` returns no results. cargo clippy passes with zero warnings. cargo test passes.</done>
</task>

</tasks>

<verification>
- `grep -r "adjust_timestamps" src/` returns no results
- `grep "monotonicity" src/silence.rs` returns no results
- `cargo clippy -- -D warnings` exits 0
- `cargo test` exits 0
- `grep "^pub" src/silence.rs` returns exactly: struct SpeechInterval, fn build_concat_filter, fn total_silence_from_speeches
</verification>

<success_criteria>
The adjust_timestamps function and all monotonicity clamping logic are removed from silence.rs. The codebase compiles, passes clippy with no warnings, and all tests pass. CLN-01 is satisfied.
</success_criteria>

<output>
After completion, create `.planning/quick/260322-rlt-phase-24-remove-dead-adjust-timestamps-f/260322-rlt-SUMMARY.md`
</output>
