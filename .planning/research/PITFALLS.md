# Pitfalls Research

**Domain:** Rust CLI orchestrating FFmpeg for video processing (silence removal, captioning, overlays)
**Researched:** 2026-02-20
**Confidence:** HIGH (v1.0 pitfalls) / MEDIUM (new-milestone pitfalls, grounded in codebase audit)

> **Scope note:** This file was updated for the v1.1 milestone. Pitfalls 1-13 cover the v1.0 domain
> (FFmpeg piping, silence removal, temp files). Pitfalls 14+ cover the new work: codebase audit,
> `doctor` subcommand, `pipeline` subcommand, and GitHub Actions CI/CD.

---

## Critical Pitfalls — v1.0 Domain (Existing)

### Pitfall 1: Pipe Deadlock When Capturing FFmpeg stderr

**What goes wrong:** FFmpeg writes progress and filter output (including silencedetect timestamps) to stderr. If you pipe both stdout and stderr from `std::process::Command` and read them sequentially, the process deadlocks. The child fills the OS pipe buffer (~64KB on macOS) on one stream while the parent blocks waiting on the other. Both processes freeze permanently.

**Why it happens:** Rust's `Command::output()` handles this internally by reading both streams, but if you use `Command::spawn()` with `.stdout(Stdio::piped())` and `.stderr(Stdio::piped())` and then read them one at a time (e.g., `child.stderr.read_to_string()` then `child.stdout.read_to_string()`), you hit the classic pipe deadlock. FFmpeg is especially prone because it writes heavily to stderr (progress, filter logs) while potentially writing decoded data to stdout.

**Consequences:** The CLI hangs indefinitely. No error message. Users think the tool crashed or the video is too large.

**Prevention:**
- For silencedetect (where you need stderr output after completion): use `Command::output()` which reads both streams concurrently, or redirect stdout to `Stdio::null()` since silencedetect output goes to stderr and you don't need stdout.
- For any pipe-based data flow: read stdout and stderr from separate threads, or use `Stdio::null()` / `Stdio::inherit()` for streams you don't need.
- Never sequentially read two piped streams from a spawned child.

**Detection:** The CLI hangs on any non-trivial video (pipe buffer fills within seconds). Easy to catch in basic testing but only if you test with real video files, not tiny test clips.

**Phase:** Foundation (Phase 1). This is the first code you write and the first bug you'll hit.

**Confidence:** HIGH — documented in Rust's [std::process::Stdio](https://doc.rust-lang.org/std/process/struct.Stdio.html) docs and [rust-lang issue #45572](https://github.com/rust-lang/rust/issues/45572).

---

### Pitfall 2: FFmpeg Hangs Waiting for Interactive Input

**What goes wrong:** FFmpeg prompts "Overwrite? [y/N]" on stderr and reads stdin when the output file already exists. When spawned from a Rust process with default stdin (inherited from parent or piped), FFmpeg blocks waiting for user input that never comes. The CLI hangs silently.

**Why it happens:** Forgetting the `-y` (overwrite without asking) flag. This is especially insidious because it works fine on first run (no file to overwrite) and only fails on re-runs.

**Consequences:** Silent hang. No error. The user re-runs the tool on the same file and it appears frozen.

**Prevention:**
- Always pass `-y` to every FFmpeg invocation.
- Also pass `-nostdin` to prevent FFmpeg from reading stdin for any reason.
- Combine: `Command::new("ffmpeg").args(["-y", "-nostdin", ...])`.

**Detection:** Run any FFmpeg command twice against the same output path.

**Phase:** Foundation (Phase 1). Hard-code these flags into your FFmpeg command builder from day one.

**Confidence:** HIGH — well-documented FFmpeg behavior, confirmed by [ffmpeg-python issue #452](https://github.com/kkroening/ffmpeg-python/issues/452).

---

### Pitfall 3: Audio/Video Sync Drift After Silence Removal

**What goes wrong:** After removing silent segments and concatenating the remaining clips, audio gradually drifts out of sync with video. The drift accumulates with each cut, so a video with many silent segments ends up with noticeable lip-sync issues by the end.

**Why it happens:** Two root causes:
1. **Keyframe-based cutting with `-c copy`:** Video stream can only be cut at keyframes (every 2-5 seconds), but audio is cut at the exact timestamp. This creates a mismatch at every cut point.
2. **Timestamp rounding accumulation:** Each segment's PTS (Presentation Time Stamps) carries small rounding errors that compound across concatenation.

**Consequences:** Output video has audio that gradually leads or lags the video. For TikTok content with speech, this is immediately noticeable and makes the video unusable.

**Prevention:**
- Use the **select/aselect filter approach** (like Remsi) instead of cutting into segments and concatenating. This applies identical time-based selection to both audio and video in a single FFmpeg pass:
  ```
  -vf "select='between(t,S1,E1)+between(t,S2,E2)+...',setpts=N/FRAME_RATE/TB"
  -af "aselect='between(t,S1,E1)+between(t,S2,E2)+...',asetpts=N/SR/TB"
  ```
- `setpts=N/FRAME_RATE/TB` and `asetpts=N/SR/TB` rebuild timestamps from scratch, eliminating drift.
- If you must use segment-based cutting, always re-encode (no `-c copy`) for frame-accurate cuts.

**Detection:** Process a 5+ minute video with 10+ silence cuts. Compare a spoken word at the 30-second mark and the 4-minute mark against the visual lip movement.

**Phase:** Silence removal (Phase 1). This is the core feature and getting sync wrong makes the tool worthless.

**Confidence:** HIGH — confirmed by [FFmpeg concat documentation](https://trac.ffmpeg.org/wiki/Concatenate), [Remsi's approach](https://github.com/bambax/Remsi), and multiple FFmpeg mailing list threads.

---

### Pitfall 4: Silencedetect Trailing Silence Edge Case

**What goes wrong:** When a video ends with silence, FFmpeg's silencedetect emits `silence_start` but never emits the corresponding `silence_end`. Your parser builds a list of silence intervals where the last entry has a start but no end. Depending on how you handle this, you either crash (index out of bounds), include the trailing silence in output, or silently produce a truncated video.

**Why it happens:** silencedetect sets `silence_end` metadata on the "first frame after the silence." If the file ends during silence, there is no frame after the silence, so `silence_end` is never emitted.

**Consequences:** Parser crash, trailing silence in output, or incorrect segment boundaries.

**Prevention:**
- After parsing silencedetect output, check if you have an unmatched `silence_start`. If so, treat the video duration as the implicit `silence_end`.
- Get total duration from FFmpeg's output (it prints `Duration: HH:MM:SS.ms` early in stderr) and use it as the fallback end boundary.
- Similarly handle leading silence: if the first event is `silence_end` (silence started at t=0), treat 0.0 as the implicit `silence_start`.
- Reference implementation: [ffmpeg-python split_silence.py](https://github.com/kkroening/ffmpeg-python/blob/master/examples/split_silence.py) handles both cases.

**Detection:** Test with a video that has silence at the end (common in real recordings when the creator pauses before stopping).

**Phase:** Silence removal (Phase 1).

**Confidence:** HIGH — verified in [silencedetect source](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_silencedetect.c) and [FFmpeg filter docs](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html).

---

### Pitfall 5: Temp File Leaks on Crash, Signal, or FFmpeg Failure

**What goes wrong:** Intermediate video segments, temp concat files, or half-written output files accumulate on disk when the process exits unexpectedly. Rust's `Drop` trait runs on normal scope exit and panics, but NOT on `SIGINT` (Ctrl+C), `SIGTERM`, or `std::process::abort()`.

**Why it happens:** Video files are large (hundreds of MB). A multi-segment silence removal might create N intermediate files. If FFmpeg fails mid-pipeline or the user hits Ctrl+C, destructors for `tempfile::NamedTempFile` or `tempfile::TempDir` may not run.

**Consequences:** Gigabytes of orphaned temp files in `/tmp` or the working directory. For a personal tool this is annoying; for repeated use it fills disk.

**Prevention:**
- Use `tempfile::TempDir` for all intermediate files — at least Drop handles the normal/panic cases.
- Register a SIGINT handler via the `ctrlc` crate that sets an `AtomicBool` flag, then check it between pipeline stages and clean up.
- Use a dedicated temp directory with a predictable name pattern (e.g., `contentops-XXXX`) so a cleanup command can find orphans.
- Place temp files in the same directory as the output (not system `/tmp`) so the user knows where to look.
- As a last resort, add a `--clean` flag that removes any `contentops-*` temp directories.

**Detection:** Kill the process mid-run with Ctrl+C and check for leftover files.

**Phase:** Foundation (Phase 1). Bake this into the temp file strategy before writing any pipeline code.

**Confidence:** HIGH — documented in [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/) and [Rust CLI signal handling guide](https://rust-cli.github.io/book/in-depth/signals.html).

---

## Moderate Pitfalls — v1.0 Domain (Existing)

### Pitfall 6: iPhone/Mobile Video Rotation Metadata

**What goes wrong:** iPhones and Android phones store video in landscape orientation with a rotation metadata flag (0/90/180/270). When you process the video through FFmpeg filters (select, setpts, etc.), the rotation metadata may be stripped or mishandled, producing output that plays sideways or upside-down.

**Why it happens:** FFmpeg's behavior around auto-rotation has changed across versions. Newer FFmpeg (5.0+) auto-rotates by default when using filters, but the `-noautorotate` flag or certain filter chains can suppress this. The metadata-vs-pixels distinction is a persistent source of confusion.

**Prevention:**
- Let FFmpeg auto-rotate (default behavior in modern FFmpeg). Don't add `-noautorotate`.
- After processing, verify output plays correctly on both desktop and mobile.
- If you ever use `-c:v copy`, rotation metadata may be preserved but pixels won't be rotated — this is fine for stream copy but wrong for filter-based processing.
- For the select/aselect approach, auto-rotation should work. Test with a portrait iPhone video.

**Detection:** Process a portrait-mode iPhone recording. If the output plays sideways in QuickTime or on TikTok, rotation handling is broken.

**Phase:** Foundation (Phase 1) or output encoding. Test with real iPhone footage early.

**Confidence:** MEDIUM — FFmpeg auto-rotation behavior varies by version and filter chain. Needs testing with actual device footage.

---

### Pitfall 7: Variable Frame Rate (VFR) Input Videos

**What goes wrong:** iPhone and screen recordings commonly use Variable Frame Rate (VFR). When FFmpeg processes VFR input through filters and outputs to a constant frame rate (required for reliable playback), frames are duplicated or dropped. This can cause audio sync issues, stuttering, or unnatural motion.

**Why it happens:** VFR videos have inconsistent frame durations. The `fps` filter or output frame rate settings force constant timing, but the mapping from variable to constant isn't lossless. Silent segment boundaries calculated from timestamps may not align with actual frame boundaries.

**Prevention:**
- Detect VFR input early: run `ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate,avg_frame_rate` and compare the two values. If they differ significantly, the input is VFR.
- Use `-vsync cfr` explicitly to make the behavior deterministic.
- For the select filter approach, `setpts=N/FRAME_RATE/TB` handles this naturally by rebuilding timestamps, but the output FRAME_RATE must be chosen correctly (use the avg_frame_rate from probe).
- Consider converting VFR to CFR as a preprocessing step before silence detection.

**Detection:** Process an iPhone screen recording or FaceTime capture. Check for audio drift and stuttered frames.

**Phase:** Silence removal (Phase 1). iPhone recordings are the most common input for a TikTok workflow tool.

**Confidence:** MEDIUM — well-documented VFR problem, but the select/aselect approach may mitigate it naturally. Needs testing.

---

### Pitfall 8: Parsing silencedetect Output With Regex Instead of Structure

**What goes wrong:** Silencedetect output format is not a stable API. Parsing it with brittle regex patterns (e.g., hardcoded field positions, split-by-space assumptions) breaks across FFmpeg versions or with unexpected log output interleaved.

**Why it happens:** silencedetect writes to stderr, which also contains FFmpeg's startup banners, codec info, progress output, and warnings. A naive `line.contains("silence_start")` approach works until FFmpeg prints a warning line that happens to contain those words, or until the output format changes between versions.

**Prevention:**
- Use robust regex with anchored patterns: `r"\[silencedetect .+\] silence_start: (-?\d+\.?\d*)"` and similar for `silence_end` and `silence_duration`.
- Parse `silence_duration` as a cross-check: verify `silence_end - silence_start ~= silence_duration`.
- Consider using FFmpeg metadata output (`-f null -`) with `ametadata=print` for structured output instead of log parsing. However, for v0.1, regex on stderr is the pragmatic approach — just make it robust.
- Pin a minimum FFmpeg version in your documentation/checks.

**Detection:** Run silencedetect on various video files and diff the raw stderr output. Look for unexpected lines between silence events.

**Phase:** Silence removal (Phase 1).

**Confidence:** HIGH — the [FFmpeg-devel mailing list](https://ffmpeg.org/pipermail/ffmpeg-devel/2014-May/157790.html) has discussed output format changes for silencedetect.

---

### Pitfall 9: Pixel Format Compatibility for TikTok Output

**What goes wrong:** FFmpeg defaults to the input pixel format or an encoder-determined format. Some inputs use `yuv444p`, `yuvj420p`, or `nv12`. Without explicitly specifying `-pix_fmt yuv420p`, the output may use a pixel format that TikTok's player can't decode properly, causing washed-out colors, green artifacts, or upload rejection.

**Why it happens:** H.264 supports many pixel formats, but mobile devices and social platforms expect `yuv420p` (8-bit 4:2:0). Professional cameras and screen captures may produce 10-bit or 4:4:4 content.

**Prevention:**
- Always specify `-pix_fmt yuv420p` in the encoding command.
- Combine with: `-c:v libx264 -preset medium -crf 23 -c:a aac -b:a 192k -ar 48000 -pix_fmt yuv420p`.
- This is a hard-coded output setting, not user-configurable.

**Detection:** Upload output to TikTok from a video originally recorded on a Mac screen capture or professional camera.

**Phase:** Output encoding (Phase 1). Hard-code the TikTok output profile from the start.

**Confidence:** HIGH — standard H.264 compatibility requirement, confirmed by [TikTok format guides](https://snaptiksave.online/tiktok-video-formats-explained/).

---

### Pitfall 10: Not Checking FFmpeg Exit Codes Properly

**What goes wrong:** FFmpeg exits with code 0 on success and non-zero on failure, but `Command::status()` and `Command::output()` return `Ok(...)` even when the exit code is non-zero. Your code thinks FFmpeg succeeded when it actually failed. You proceed to the next pipeline stage with a missing or corrupt intermediate file.

**Why it happens:** Rust's `Command` API considers a process that was launched and completed as `Ok`, regardless of its exit code. You must explicitly check `.status().success()` or `.output().status.success()`. This is [a known confusion point](https://github.com/rust-lang/rust/issues/73126).

**Prevention:**
- Create a helper function that runs FFmpeg and returns `Result<Output, Error>` where non-zero exit becomes an `Err` containing stderr output.
- Log the full FFmpeg command (with args) on failure for debugging.
- Include the exit code and last ~20 lines of stderr in the error message.

**Detection:** Feed FFmpeg a corrupt file or invalid arguments and verify your tool reports the error.

**Phase:** Foundation (Phase 1). Build the FFmpeg runner abstraction before anything else.

**Confidence:** HIGH — documented in [Rust std::process docs](https://doc.rust-lang.org/std/process/struct.ExitStatus.html).

---

## Minor Pitfalls — v1.0 Domain (Existing)

### Pitfall 11: Silence Threshold Defaults That Don't Match Content Type

**What goes wrong:** The default silence threshold (-60dB) works for studio-quality audio but not for smartphone recordings with background noise. Videos recorded in a room with AC, street noise, or fans have a noise floor above -60dB, meaning silencedetect never fires or fires incorrectly.

**Prevention:**
- Start with `-60dB` and `2s` minimum duration as defaults.
- Plan to expose `--silence-threshold` and `--silence-duration` flags in a later phase.
- Test with actual phone-recorded content, not synthetic test files.

**Phase:** Phase 2 (configuration). Hardcoded defaults for v0.1, expose tuning later.

**Confidence:** MEDIUM — threshold sensitivity varies wildly by recording environment.

---

### Pitfall 12: Select Filter Expression Length Limits

**What goes wrong:** For videos with many short silences (e.g., a fast-talking creator), the select filter expression `between(t,S1,E1)+between(t,S2,E2)+...` can become extremely long. FFmpeg has practical limits on filter graph expression complexity, and very long expressions may cause parsing errors or performance degradation.

**Prevention:**
- For v0.1, this is unlikely to be a problem (most TikTok source videos are under 10 minutes with <50 silence segments).
- If it becomes an issue, fall back to the segment-and-concat approach for videos with >100 segments.
- Monitor the generated filter string length and warn if it exceeds ~10,000 characters.

**Phase:** Phase 2 or later. Not a v0.1 concern for short-form video.

**Confidence:** LOW — theoretical limit, no confirmed breakage found for realistic use cases.

---

### Pitfall 13: FFmpeg Version Incompatibilities

**What goes wrong:** Different FFmpeg versions (Homebrew installs different versions over time) have different filter availability, default behaviors, and output formats. A tool that works on FFmpeg 6.x may break on 7.x due to changed auto-rotation defaults, deprecated options, or new filter behaviors.

**Prevention:**
- Check FFmpeg version at startup: `ffmpeg -version`.
- Set a minimum version requirement (FFmpeg 6.0+ is reasonable for 2026).
- Print a clear error if the version is too old rather than failing with cryptic FFmpeg errors.

**Phase:** Foundation (Phase 1). Add a version check to the startup validation.

**Confidence:** MEDIUM — FFmpeg is generally backward-compatible for basic features, but edge cases exist.

---

## Critical Pitfalls — v1.1 New Milestone

### Pitfall 14: Refactoring Working Code Breaks Existing Subcommands

**What goes wrong:** The audit/cleanup phase extracts shared logic (spinner construction, output path derivation, temp file patterns) into common modules. The refactor compiles cleanly but changes subtle behavior: a spinner that was `finish_and_clear()` is now `finish_with_message()`, a temp file that had `.keep()` called no longer does, or an error message changes format. Existing subcommands break in ways that are not caught at compile time.

**Why it happens:** The codebase has significant duplication (three identical `make_spinner` implementations in `cut.rs`, `caption.rs`, `overlay.rs`; identical error match arms; similar output path derivation). Extracting these requires changing call sites. In Rust, the borrow checker enforces correctness of ownership transfers but cannot catch behavioral regressions — a function signature change from `&TempFileRegistry` to an owned `TempFileRegistry` compiles but changes cleanup semantics.

**Specific risk in this codebase:**
- `normalize.rs:132` calls `temp_file.keep()` to persist the temp file across the function. If extracted into a shared helper that doesn't call `.keep()`, the `NamedTempFile` drops and deletes the file before the caller uses it.
- `cut.rs` manually removes normalized temp files at the end (`std::fs::remove_file(&normalized_path)`). If normalization is refactored to return an RAII guard instead of a raw `PathBuf`, this manual cleanup becomes a double-free (harmless in this case but indicative of logic confusion).

**Prevention:**
- Refactor one subcommand at a time. Verify each still works before touching the next.
- Write integration tests (even simple `assert!(output_path.exists())` smoke tests) before refactoring, so regressions are caught immediately.
- Extract only identical code first; don't combine slightly-different variants until the behavior difference is understood.
- Keep a git diff of each extraction. Review that temp file ownership semantics are preserved exactly.

**Warning signs:**
- Refactored build compiles but `cargo clippy` emits warnings about unused results or shadowed variables.
- A temp file cleanup at the call site becomes a no-op (the file was already cleaned up by the extracted helper's Drop).
- Integration test produces output file of 0 bytes or wrong duration.

**Phase:** Audit/Cleanup Phase. Do this first before adding new features.

**Confidence:** HIGH — grounded in direct codebase audit of three near-identical spinner + temp file patterns across `cut.rs`, `caption.rs`, `overlay.rs`.

---

### Pitfall 15: `doctor` Subcommand Reports Wrong Version or Missing Dep on CI

**What goes wrong:** The `doctor` command checks for `ffmpeg`, `whisper-cli`, and `claude` using `which::which()` (already used in `error.rs`). It reports them as present or absent and optionally their versions by parsing `ffmpeg -version` output. On CI, these binaries are absent — the `doctor` command exits non-zero, and if CI runs `contentops doctor` as part of a smoke test, the job fails even though the build succeeded.

**Why it happens:** `doctor` is a diagnostic tool for user environments, not for CI. Running it in CI without external deps installed is always going to fail. Additionally, parsing version strings from `ffmpeg -version` is brittle — the format is `ffmpeg version 7.1 Copyright...` but Homebrew builds include extra git hash suffixes that break naive `semver` parsing.

**Prevention:**
- Do not run `contentops doctor` in CI. CI should test compilation and unit tests only; external binary availability is not a CI concern.
- For version parsing: use a regex to extract just the numeric part (`r"ffmpeg version (\d+\.\d+)"`) rather than passing the full output to a semver parser.
- Make `doctor` exit 0 even when deps are missing — it should report status, not enforce prerequisites. Use colored output (already have `owo-colors`) to show green/red status without a non-zero exit code. Reserve exit 1 for the case where the user explicitly asks for a strict check (`--strict` flag).
- Add a `--json` output mode so scripts can machine-read the results.

**Warning signs:**
- `contentops doctor` exits 1 on a fresh CI runner (no FFmpeg installed).
- Version check output changes between FFmpeg patch releases and breaks the parser.

**Phase:** Doctor Subcommand Phase. Design exit code semantics before implementing.

**Confidence:** HIGH — directly observed in `error.rs` `require_ffmpeg()` pattern; version string parsing is a well-known fragility in CLI tools.

---

### Pitfall 16: Pipeline Intermediate Files Named After Input, Collide With Each Other

**What goes wrong:** The `pipeline` subcommand chains `cut -> caption -> overlay`. Each stage needs an intermediate output file. If stages derive filenames using the existing `derive_output_path()` convention (`{stem}_cut.mp4`, `{stem}_cut_captioned.mp4`, etc.), names become confusing and pollute the working directory. Worse: if the user runs pipeline on `video.mp4` and already has a `video_cut.mp4` present, the pipeline silently overwrites it (FFmpeg's `-y` flag).

**Why it happens:** The current filename derivation in `cut.rs:derive_output_path()` appends a suffix to the input stem. Pipeline chaining creates a suffix chain: `video_cut_captioned_overlay.mp4`. Additionally, intermediate files (the `cut` output that feeds `caption`) are permanent artifacts unless explicitly cleaned up. Users are left with every intermediate file after pipeline runs.

**Prevention:**
- For pipeline mode: use a temporary directory for ALL intermediate files. Only the final output is written to the user's working directory.
- Name the temp directory `{stem}_pipeline_tmp_{timestamp}/` so it's clearly identifiable and easily cleaned.
- On success: delete the temp directory. On failure: leave it (for debugging) but print its path.
- The final output path should be `{stem}_final.mp4` or user-specified via `-o`, not a chained suffix.
- Guard against overwriting an existing `-o` output: check existence before starting the pipeline (not just when FFmpeg runs), so the user gets an early error rather than discovering the file was overwritten after a 10-minute run.

**Warning signs:**
- After a pipeline run, the working directory contains `video_cut.mp4`, `video_cut_captioned.mp4`, and `video_cut_captioned_overlay.mp4` in addition to the final output.
- Running pipeline twice on the same input silently overwrites intermediate files from the first run.

**Phase:** Pipeline Subcommand Phase.

**Confidence:** HIGH — direct analysis of `derive_output_path()` in `cut.rs` and the stage chaining pattern.

---

### Pitfall 17: Pipeline Partial Failure Leaves User in Ambiguous State

**What goes wrong:** Pipeline runs three FFmpeg stages (cut, caption, overlay) plus whisper-cli and claude-cli. If `caption` fails after `cut` succeeds, the user has a `*_cut.mp4` temp file but no final output. If pipeline cleans up the temp directory on failure, the user loses the intermediate cut. If it doesn't clean up, re-running the pipeline re-does the cut even though it succeeded.

**Why it happens:** Multi-stage pipelines fail in the middle in practice — whisper-cli runs out of memory on long videos, claude-cli has rate limits, FFmpeg fails on a specific input codec. There's no established resume mechanism in the current architecture.

**Prevention:**
- On failure: preserve the temp directory and print exactly which stage failed and what intermediate files exist. Give the user the path so they can manually continue.
- Print a clear message: "Caption stage failed. Intermediate cut is at: /path/to/video_pipeline_tmp/cut.mp4. Run `contentops caption` on it to continue manually."
- Do not silently re-run successful stages on retry (this is v1.1 scope — full resume is v2.0).
- Consider a `--keep-intermediates` flag as a power user escape hatch.

**Warning signs:**
- User reports that pipeline fails at caption but they can't find the cut file.
- Pipeline re-runs take as long as the first run even when the first stage succeeded.

**Phase:** Pipeline Subcommand Phase. Error handling design must come before implementation.

**Confidence:** HIGH — observable failure mode from existing `cut.rs` and `caption.rs` error handling patterns. `run_ffmpeg_with_progress` and whisper-cli have no retry or resume logic.

---

### Pitfall 18: GitHub Actions CI Fails on macOS-Specific Code Paths

**What goes wrong:** The codebase has hard-coded macOS paths. `overlay.rs:107` sets `DEFAULT_FONT` to `/System/Library/Fonts/Supplemental/Impact.ttf`. If CI runs on `ubuntu-latest`, this path doesn't exist, and any test that exercises the overlay code fails with a file-not-found error from FFmpeg's drawtext filter — not a clear Rust error.

**Similarly:** `normalize.rs:87` uses `/dev/null` as FFmpeg's null output. This works on macOS and Linux but fails on Windows. Since this tool is macOS-only, `/dev/null` is fine — but if CI runs on `windows-latest` for any reason, it silently breaks.

**Prevention:**
- Use `ubuntu-latest` runners for compile + unit tests (fast, cheap). Use `macos-latest` runners ONLY for integration tests that require the macOS font path or macOS-specific behavior.
- Add a `#[cfg(target_os = "macos")]` guard on the `DEFAULT_FONT` constant, or better: make the font path resolution return `None` on non-macOS systems so the code path is skipped in tests.
- For the null device: `if cfg!(windows) { "NUL" } else { "/dev/null" }` — but since the tool is macOS-only, document this limitation rather than patching it.
- In CI workflow: split into two jobs: `test` (ubuntu-latest, compile + unit tests) and `release` (macos-latest, build binary + integration tests).

**Warning signs:**
- CI `ubuntu-latest` job fails with `No such file or directory: /System/Library/Fonts/Supplemental/Impact.ttf`.
- CI job passes because overlay tests are never run — the font path issue is latent.

**Phase:** CI/CD Phase.

**Confidence:** HIGH — direct code audit of `overlay.rs:107` and `normalize.rs:87`.

---

### Pitfall 19: CI Has No Way to Test Commands That Require External Binaries

**What goes wrong:** `cut`, `caption`, and `overlay` all shell out to `ffmpeg`, `whisper-cli`, or `claude`. On CI without these installed, `cargo test` passes (unit tests compile and run) but any integration test that actually invokes a subcommand fails immediately with `AppError::FfmpegNotFound`. This gives a false sense of test coverage — CI is green but nothing that runs the actual binary is tested.

**Why it happens:** The current architecture tightly couples business logic to `Command::new("ffmpeg")` invocations inside each command module. There is no abstraction layer that can be swapped for a test double. The existing unit tests (`tests/silence_tests.rs`) only test pure Rust parsing logic — they don't test the FFmpeg invocation path.

**Prevention:**
- For CI: install FFmpeg via `brew install ffmpeg` (macOS runner) or `apt-get install ffmpeg` (ubuntu runner). This is fast (prebuilt packages) and enables real integration tests.
- For whisper-cli: do NOT install on CI. It requires a model file (hundreds of MB) and GPU. Mock the whisper invocation in integration tests by creating a test fixture with a pre-generated JSON output file and a `--mock-whisper` flag, OR skip caption integration tests in CI with `#[cfg_attr(not(feature = "integration"), ignore)]`.
- For claude-cli: same approach — skip or mock. Claude API calls in CI would require secrets and incur costs.
- Structure tests in two tiers: `unit` (always runs, no external deps) and `integration` (runs only when `INTEGRATION_TESTS=1` or in CI with FFmpeg installed).

**Warning signs:**
- `cargo test` passes on CI but the binary produces wrong output when run manually.
- CI test suite has 0 tests that exercise the `run()` function of any command module.

**Phase:** CI/CD Phase. Define the test strategy before writing CI YAML.

**Confidence:** HIGH — direct observation of `tests/silence_tests.rs` which only tests `silence.rs` parsing.

---

### Pitfall 20: GitHub Release Workflow Uploads Wrong Binary or Wrong Architecture

**What goes wrong:** The release workflow runs `cargo build --release` on `macos-latest` and uploads the binary. But `macos-latest` on GitHub Actions is now `macos-14` (ARM64/Apple Silicon) as of early 2024. Users on Intel Macs download an ARM binary that their Rosetta 2 can run, but native Intel users may prefer a native binary. More critically: if the workflow doesn't specify the target triple explicitly, the uploaded artifact name may not indicate architecture, causing confusion.

**Additionally:** The Cargo.lock file must be committed and the workflow must use `--locked` to ensure reproducible builds. Without `--locked`, the CI build may use different dependency versions than the developer's local build.

**Prevention:**
- Use a build matrix with both `macos-latest` (ARM64) and `macos-13` (last Intel runner) to produce two binaries.
- Name artifacts explicitly: `contentops-macos-aarch64` and `contentops-macos-x86_64`.
- Always use `cargo build --release --locked` in CI.
- Use the [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action) which handles target naming, checksums, and archive creation automatically.
- Commit `Cargo.lock` to the repository (it is already present in this project per the file listing).

**Warning signs:**
- Release artifacts are named `contentops` without architecture suffix.
- `cargo build --release` on CI produces a different binary than local `cargo build --release` due to dependency version drift.

**Phase:** CI/CD Phase.

**Confidence:** MEDIUM — `macos-latest` runner architecture change is documented in GitHub's changelog; binary naming is a common oversight observed across Rust CLI project releases.

---

### Pitfall 21: `#[allow(dead_code)]` Accumulation During Audit Creates False Positives

**What goes wrong:** The audit phase runs `cargo clippy` and `rustc` dead_code warnings to find unused code. The codebase already has `#[allow(dead_code)]` on `cleanup_all` in `temp.rs`. When extracting shared helpers, new functions are created that are only used in some subcommands, causing clippy to flag them as dead code. Developers add `#[allow(dead_code)]` to suppress the warning rather than investigating — masking real dead code.

**Why it happens:** `cleanup_all` is currently dead (`#[allow(dead_code)]` on line 26 of `temp.rs`). It was written for future use or was orphaned during refactoring. If this pattern is followed during the audit, legitimate dead code accumulates behind allow attributes.

**Prevention:**
- During the audit: enumerate ALL `#[allow(dead_code)]` attributes and verify each is intentional. Remove `cleanup_all` if it has no callers and no planned use in this milestone.
- For newly extracted shared functions: if a function is only used internally within one module, keep it private (`fn` not `pub fn`) so dead code detection works correctly.
- Add `#![deny(dead_code)]` at the crate level for the duration of the audit to force investigation of every warning (remove it after cleanup is complete).
- Do not add `#[allow(dead_code)]` during the audit phase without a code comment explaining why.

**Warning signs:**
- `grep -r "allow(dead_code)"` count increases after the audit.
- Functions marked `pub` that have no external callers.

**Phase:** Audit/Cleanup Phase.

**Confidence:** HIGH — direct code observation: `temp.rs:25` already has `#[allow(dead_code)]` on `cleanup_all`.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Duplicate `make_spinner` in each command module | Avoids upfront abstraction | 3 places to update when spinner style changes | Never — extract now in audit phase |
| Hard-coded `/System/Library/Fonts/Supplemental/Impact.ttf` | Works on dev machine | CI failures, breaks for users without font | Acceptable for macOS-only tool; add `--font` fallback path |
| Manual registry cleanup at end of each `run()` | Explicit, easy to follow | Mismatched register/remove pairs if error paths are added | Replace with RAII wrapper in audit phase |
| `#[allow(dead_code)]` on `cleanup_all` | Silences warning | Masks real dead code, confuses future auditors | Remove the function or document the planned use case |
| `which::which()` for dep checks with no version validation | Simple presence check | Silently accepts wrong versions of FFmpeg | Add version check in `doctor` command; keep simple check for fast-path commands |
| `anyhow::bail!` mixed with `AppError` returns | Flexible error handling | Inconsistent error formatting (some get `format_error()` treatment, some don't) | Standardize on `AppError` for all user-visible errors in audit phase |

---

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| FFmpeg via `Command` | Forgetting `-y` and `-nostdin` on new invocations added during pipeline work | Centralize all FFmpeg invocation through `ffmpeg.rs` helpers that add these flags unconditionally |
| whisper-cli JSON output | Assuming JSON file is at `{wav_path}.json` — whisper-cli writes to the same directory as the WAV with `.json` appended to the full filename | Current code in `caption.rs:434` already handles this correctly; preserve this assumption when refactoring |
| claude-cli in pipeline | Pipeline runs claude-cli for `--auto` overlay; in pipeline mode the JSON transcript is an intermediate file from the caption stage, not a user-provided path | The pipeline must wire the caption JSON output path explicitly to the overlay `--auto` input — do not rely on derived filenames |
| GitHub Actions secrets | `GITHUB_TOKEN` is auto-provided for release uploads; no additional secrets needed for binary releases | Only add secrets if publishing to Homebrew tap; keep the initial CI simple |
| ffprobe version parsing | `ffprobe -version` output format is identical to `ffmpeg -version` — strip the same way | Reuse the same version extraction regex for both |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-running full pipeline on retry | User re-runs pipeline after caption failure; cut stage re-runs (10+ min for long video) | Preserve intermediates on failure with clear path message | Immediately on any multi-stage failure |
| Loudnorm two-pass inside pipeline | Cut already normalizes audio (two FFmpeg passes). Pipeline cut→caption feeds normalized video to caption, which is fine. But overlay also re-encodes. Three full re-encodes per pipeline run. | Accept the re-encoding cost for v1.1; optimize to stream copy where possible in v2.0 | Never a correctness problem, but 30-60 min for a 60-min source video |
| Blocking on claude-cli in pipeline | Claude API call for title generation blocks the pipeline; no timeout | Add a `--timeout` flag to the claude invocation, or use `Command::spawn()` + `wait_timeout` | When Claude API is slow or rate-limited |
| Large temp files in same directory as input | User's video directory fills up with multi-GB temp files during pipeline | Use `std::env::temp_dir()` for truly temporary files, or a dedicated `{input_dir}/.contentops_tmp/` subdirectory | On SSDs with limited space (common on MacBooks) |

---

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing user-provided text directly to FFmpeg `drawtext` filter without escaping | FFmpeg filter injection — a title like `foo':fontfile='/etc/passwd` could leak file contents into error output | Current `escape_drawtext()` in `overlay.rs:100` handles `\\`, `'`, `:`, `;` — preserve this during refactoring, add tests |
| Passing user-provided file paths to `Command::new()` without validation | Path traversal in error logs written to `.contentops_error.log` | Paths are validated for existence before use (`args.input.exists()`); keep this pattern in pipeline |
| Storing claude API key in environment and passing to subprocess | Claude CLI reads `CLAUDE_API_KEY` from env; a compromised subprocess could read it | This is inherent to the claude-cli design; note in documentation |
| Shell injection via file paths with special characters | Spaces and special chars in filenames passed to FFmpeg as string args | All FFmpeg args are passed as `Vec<&str>` to `Command`, not via shell — no injection risk |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `doctor` exits 1 when deps missing | Scripted setup fails on `contentops doctor && contentops pipeline ...` | Exit 0 always; only exit 1 with `--strict` flag |
| Pipeline produces no output and cleans up intermediates | User has no idea where 10 minutes of processing went | Always print each stage's output path; preserve temp dir on failure |
| `doctor` doesn't check whisper model availability | User runs pipeline, waits through cut stage, then fails at caption with "model not found" | `doctor` should accept `--model` path and validate the model file exists and is a valid GGML file |
| No `--dry-run` on pipeline | User can't preview what pipeline will do before committing 30 minutes of processing | Pipeline should support `--dry-run` that chains `cut --dry-run` output and skips actual encoding |
| Progress bars from multiple stages interleave badly in CI logs | CI logs are unreadable with spinner escape codes | Detect non-TTY (`atty` crate or `std::io::IsTerminal`) and suppress progress bars; current code doesn't do this |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **`doctor` subcommand:** Often missing version validation — verify it checks minimum FFmpeg version, not just presence.
- [ ] **Pipeline subcommand:** Often missing intermediate file cleanup on success — verify the temp directory is deleted after a successful run.
- [ ] **Pipeline error handling:** Often missing actionable failure messages — verify each stage failure prints the intermediate file path.
- [ ] **CI/CD workflow:** Often missing `--locked` flag — verify `cargo build --release --locked` is used, not `cargo build --release`.
- [ ] **Audit/cleanup:** Often declared done when `cargo build` passes — verify `cargo clippy -- -D warnings` also passes with no suppressions added.
- [ ] **Refactored subcommands:** Often assumed working because they compile — verify each subcommand produces identical output before and after extraction (test with the same input file, diff the output).
- [ ] **GitHub Release:** Often missing SHA256 checksums — verify the release workflow attaches `.sha256` files alongside binaries.
- [ ] **`doctor` non-TTY output:** Often only tested interactively — verify `contentops doctor` output is readable in `| cat` (no broken escape codes).

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Refactoring broke a subcommand | MEDIUM | `git revert` the extraction commit; re-extract one step at a time with tests at each step |
| Pipeline left multi-GB temp files | LOW | Delete `.contentops_tmp_*` files in the video directory; add `contentops cleanup` subcommand for users |
| CI/CD uploads wrong-arch binary | LOW | Delete the release, re-trigger the workflow with corrected matrix; add arch to artifact name |
| `doctor` exits 1 in CI and blocks release | LOW | Add `|| true` to the CI step, or remove `contentops doctor` from CI entirely |
| Dead code masking by `#[allow(dead_code)]` | LOW | Run `grep -r "allow(dead_code)" src/`; audit each; remove function or document intent |
| Pipeline stage fails, no intermediate files | HIGH | No recovery without re-running from scratch; prevention (preserve on failure) is the only mitigation |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Refactoring breaks subcommands (#14) | Audit/Cleanup Phase | Run each subcommand with a real video file before and after extraction; output files match |
| `doctor` wrong exit code semantics (#15) | Doctor Phase — design before implementing | `contentops doctor` exits 0 on a machine without FFmpeg |
| Pipeline intermediate file collisions (#16) | Pipeline Phase — design temp dir strategy first | After pipeline run, only final output is in the working directory |
| Pipeline partial failure ambiguity (#17) | Pipeline Phase — error handling design | Intentionally kill pipeline at caption stage; verify cut intermediate is preserved and path is printed |
| macOS-specific paths break CI (#18) | CI/CD Phase | CI `ubuntu-latest` job passes (compile + unit tests only, no overlay integration tests) |
| No testable integration path (#19) | CI/CD Phase | CI installs FFmpeg and at least one integration test runs `contentops cut` end-to-end |
| Wrong binary architecture in release (#20) | CI/CD Phase | Release artifacts are named with architecture suffix; both ARM and Intel binaries uploaded |
| `#[allow(dead_code)]` accumulation (#21) | Audit/Cleanup Phase | `grep -r "allow(dead_code)" src/` shows 0 results after audit |

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Foundation / FFmpeg runner | Pipe deadlock (#1), interactive hang (#2), exit code (#10) | Build FFmpeg runner abstraction with -y, -nostdin, proper pipe handling, exit code checks |
| Silence detection | Trailing silence (#4), stderr parsing (#8), threshold (#11) | Robust parser with edge case handling, fallback boundaries |
| Silence removal | A/V sync drift (#3), VFR input (#7) | Use select/aselect approach, test with real iPhone footage |
| Output encoding | Pixel format (#9), rotation (#6) | Hard-code yuv420p, let FFmpeg auto-rotate, test portrait video |
| Temp file management | Leak on crash (#5) | TempDir + ctrlc handler + predictable naming |
| Audit/Cleanup | Behavioral regressions (#14), dead code masking (#21) | Extract one module at a time; run each subcommand against real video after each extraction |
| Doctor subcommand | Wrong exit code semantics (#15), brittle version parsing | Design exit code contract first; use regex for version extraction |
| Pipeline subcommand | File collisions (#16), partial failure (#17), performance (#table) | Temp dir for intermediates; preserve on failure; clear stage failure messages |
| CI/CD | macOS-specific paths (#18), no integration tests (#19), wrong arch (#20) | Two-job CI: ubuntu compile/unit + macos integration; matrix release build |

---

## Sources

- [Rust std::process::Stdio docs — pipe deadlock warning](https://doc.rust-lang.org/std/process/struct.Stdio.html)
- [Rust issue #45572 — Command hangs if piped stdout buffer fills](https://github.com/rust-lang/rust/issues/45572)
- [Rust issue #73126 — Command output() error handling hazards](https://github.com/rust-lang/rust/issues/73126)
- [FFmpeg silencedetect filter docs (7.1)](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html)
- [FFmpeg silencedetect source code](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_silencedetect.c)
- [FFmpeg Concatenate wiki](https://trac.ffmpeg.org/wiki/Concatenate)
- [Remsi — silence removal approach](https://github.com/bambax/Remsi)
- [ffmpeg-python split_silence.py — edge case handling](https://github.com/kkroening/ffmpeg-python/blob/master/examples/split_silence.py)
- [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/)
- [Rust CLI book — signal handling](https://rust-cli.github.io/book/in-depth/signals.html)
- [ctrlc crate](https://docs.rs/ctrlc)
- [TikTok video format guide](https://snaptiksave.online/tiktok-video-formats-explained/)
- [taiki-e/upload-rust-binary-action — GitHub Actions Rust release](https://github.com/taiki-e/upload-rust-binary-action)
- [GitHub Actions — macos-latest is now ARM64](https://github.com/actions/runner-images)
- [Setting up effective CI/CD for Rust projects (Shuttle, Jan 2025)](https://www.shuttle.dev/blog/2025/01/23/setup-rust-ci-cd)
- [Cross-platform Rust CI/CD pipeline with GitHub Actions (2025)](https://ahmedjama.com/blog/2025/12/cross-platform-rust-pipeline-github-actions/)
- [Homebrew Releaser — automate formula updates from GitHub Actions](https://github.com/marketplace/actions/homebrew-releaser)
- [Rust extract method refactoring — ownership challenges (OOPSLA 2023)](https://ilyasergey.net/assets/pdf/papers/rem-oopsla23.pdf)
- [Clippy lints reference](https://rust-lang.github.io/rust-clippy/master/index.html)
- [clap global args issue — positional requirements and breaking changes](https://github.com/clap-rs/clap/issues/1386)
- Direct codebase audit of contentops v1.0 source (2026-02-20)

---
*Pitfalls research for: Rust video processing CLI — v1.1 milestone (audit, doctor, pipeline, CI/CD)*
*Researched: 2026-02-20*
