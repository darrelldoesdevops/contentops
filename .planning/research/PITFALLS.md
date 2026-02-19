# Domain Pitfalls

**Domain:** Rust CLI orchestrating FFmpeg for video processing (silence removal, captioning, overlays)
**Researched:** 2026-02-19

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or broken output.

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

**Confidence:** HIGH -- documented in Rust's [std::process::Stdio](https://doc.rust-lang.org/std/process/struct.Stdio.html) docs and [rust-lang issue #45572](https://github.com/rust-lang/rust/issues/45572).

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

**Confidence:** HIGH -- well-documented FFmpeg behavior, confirmed by [ffmpeg-python issue #452](https://github.com/kkroening/ffmpeg-python/issues/452).

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

**Confidence:** HIGH -- confirmed by [FFmpeg concat documentation](https://trac.ffmpeg.org/wiki/Concatenate), [Remsi's approach](https://github.com/bambax/Remsi), and multiple FFmpeg mailing list threads.

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

**Confidence:** HIGH -- verified in [silencedetect source](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_silencedetect.c) and [FFmpeg filter docs](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html).

---

### Pitfall 5: Temp File Leaks on Crash, Signal, or FFmpeg Failure

**What goes wrong:** Intermediate video segments, temp concat files, or half-written output files accumulate on disk when the process exits unexpectedly. Rust's `Drop` trait runs on normal scope exit and panics, but NOT on `SIGINT` (Ctrl+C), `SIGTERM`, or `std::process::abort()`.

**Why it happens:** Video files are large (hundreds of MB). A multi-segment silence removal might create N intermediate files. If FFmpeg fails mid-pipeline or the user hits Ctrl+C, destructors for `tempfile::NamedTempFile` or `tempfile::TempDir` may not run.

**Consequences:** Gigabytes of orphaned temp files in `/tmp` or the working directory. For a personal tool this is annoying; for repeated use it fills disk.

**Prevention:**
- Use `tempfile::TempDir` for all intermediate files -- at least Drop handles the normal/panic cases.
- Register a SIGINT handler via the `ctrlc` crate that sets an `AtomicBool` flag, then check it between pipeline stages and clean up.
- Use a dedicated temp directory with a predictable name pattern (e.g., `contentops-XXXX`) so a cleanup command can find orphans.
- Place temp files in the same directory as the output (not system `/tmp`) so the user knows where to look.
- As a last resort, add a `--clean` flag that removes any `contentops-*` temp directories.

**Detection:** Kill the process mid-run with Ctrl+C and check for leftover files.

**Phase:** Foundation (Phase 1). Bake this into the temp file strategy before writing any pipeline code.

**Confidence:** HIGH -- documented in [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/) and [Rust CLI signal handling guide](https://rust-cli.github.io/book/in-depth/signals.html).

---

## Moderate Pitfalls

### Pitfall 6: iPhone/Mobile Video Rotation Metadata

**What goes wrong:** iPhones and Android phones store video in landscape orientation with a rotation metadata flag (0/90/180/270). When you process the video through FFmpeg filters (select, setpts, etc.), the rotation metadata may be stripped or mishandled, producing output that plays sideways or upside-down.

**Why it happens:** FFmpeg's behavior around auto-rotation has changed across versions. Newer FFmpeg (5.0+) auto-rotates by default when using filters, but the `-noautorotate` flag or certain filter chains can suppress this. The metadata-vs-pixels distinction is a persistent source of confusion.

**Prevention:**
- Let FFmpeg auto-rotate (default behavior in modern FFmpeg). Don't add `-noautorotate`.
- After processing, verify output plays correctly on both desktop and mobile.
- If you ever use `-c:v copy`, rotation metadata may be preserved but pixels won't be rotated -- this is fine for stream copy but wrong for filter-based processing.
- For the select/aselect approach, auto-rotation should work. Test with a portrait iPhone video.

**Detection:** Process a portrait-mode iPhone recording. If the output plays sideways in QuickTime or on TikTok, rotation handling is broken.

**Phase:** Foundation (Phase 1) or output encoding. Test with real iPhone footage early.

**Confidence:** MEDIUM -- FFmpeg auto-rotation behavior varies by version and filter chain. Needs testing with actual device footage.

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

**Confidence:** MEDIUM -- well-documented VFR problem, but the select/aselect approach may mitigate it naturally. Needs testing.

---

### Pitfall 8: Parsing silencedetect Output With Regex Instead of Structure

**What goes wrong:** Silencedetect output format is not a stable API. Parsing it with brittle regex patterns (e.g., hardcoded field positions, split-by-space assumptions) breaks across FFmpeg versions or with unexpected log output interleaved.

**Why it happens:** silencedetect writes to stderr, which also contains FFmpeg's startup banners, codec info, progress output, and warnings. A naive `line.contains("silence_start")` approach works until FFmpeg prints a warning line that happens to contain those words, or until the output format changes between versions.

**Prevention:**
- Use robust regex with anchored patterns: `r"\[silencedetect .+\] silence_start: (-?\d+\.?\d*)"` and similar for `silence_end` and `silence_duration`.
- Parse `silence_duration` as a cross-check: verify `silence_end - silence_start ~= silence_duration`.
- Consider using FFmpeg metadata output (`-f null -`) with `ametadata=print` for structured output instead of log parsing. However, for v0.1, regex on stderr is the pragmatic approach -- just make it robust.
- Pin a minimum FFmpeg version in your documentation/checks.

**Detection:** Run silencedetect on various video files and diff the raw stderr output. Look for unexpected lines between silence events.

**Phase:** Silence removal (Phase 1).

**Confidence:** HIGH -- the [FFmpeg-devel mailing list](https://ffmpeg.org/pipermail/ffmpeg-devel/2014-May/157790.html) has discussed output format changes for silencedetect.

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

**Confidence:** HIGH -- standard H.264 compatibility requirement, confirmed by [TikTok format guides](https://snaptiksave.online/tiktok-video-formats-explained/).

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

**Confidence:** HIGH -- documented in [Rust std::process docs](https://doc.rust-lang.org/std/process/struct.ExitStatus.html).

---

## Minor Pitfalls

### Pitfall 11: Silence Threshold Defaults That Don't Match Content Type

**What goes wrong:** The default silence threshold (-60dB) works for studio-quality audio but not for smartphone recordings with background noise. Videos recorded in a room with AC, street noise, or fans have a noise floor above -60dB, meaning silencedetect never fires or fires incorrectly.

**Prevention:**
- Start with `-60dB` and `2s` minimum duration as defaults.
- Plan to expose `--silence-threshold` and `--silence-duration` flags in a later phase.
- Test with actual phone-recorded content, not synthetic test files.

**Phase:** Phase 2 (configuration). Hardcoded defaults for v0.1, expose tuning later.

**Confidence:** MEDIUM -- threshold sensitivity varies wildly by recording environment.

---

### Pitfall 12: Select Filter Expression Length Limits

**What goes wrong:** For videos with many short silences (e.g., a fast-talking creator), the select filter expression `between(t,S1,E1)+between(t,S2,E2)+...` can become extremely long. FFmpeg has practical limits on filter graph expression complexity, and very long expressions may cause parsing errors or performance degradation.

**Prevention:**
- For v0.1, this is unlikely to be a problem (most TikTok source videos are under 10 minutes with <50 silence segments).
- If it becomes an issue, fall back to the segment-and-concat approach for videos with >100 segments.
- Monitor the generated filter string length and warn if it exceeds ~10,000 characters.

**Phase:** Phase 2 or later. Not a v0.1 concern for short-form video.

**Confidence:** LOW -- theoretical limit, no confirmed breakage found for realistic use cases.

---

### Pitfall 13: FFmpeg Version Incompatibilities

**What goes wrong:** Different FFmpeg versions (Homebrew installs different versions over time) have different filter availability, default behaviors, and output formats. A tool that works on FFmpeg 6.x may break on 7.x due to changed auto-rotation defaults, deprecated options, or new filter behaviors.

**Prevention:**
- Check FFmpeg version at startup: `ffmpeg -version`.
- Set a minimum version requirement (FFmpeg 6.0+ is reasonable for 2026).
- Print a clear error if the version is too old rather than failing with cryptic FFmpeg errors.

**Phase:** Foundation (Phase 1). Add a version check to the startup validation.

**Confidence:** MEDIUM -- FFmpeg is generally backward-compatible for basic features, but edge cases exist.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Foundation / FFmpeg runner | Pipe deadlock (#1), interactive hang (#2), exit code (#10) | Build FFmpeg runner abstraction with -y, -nostdin, proper pipe handling, exit code checks |
| Silence detection | Trailing silence (#4), stderr parsing (#8), threshold (#11) | Robust parser with edge case handling, fallback boundaries |
| Silence removal | A/V sync drift (#3), VFR input (#7) | Use select/aselect approach, test with real iPhone footage |
| Output encoding | Pixel format (#9), rotation (#6) | Hard-code yuv420p, let FFmpeg auto-rotate, test portrait video |
| Temp file management | Leak on crash (#5) | TempDir + ctrlc handler + predictable naming |
| Captioning (future) | Whisper integration, subtitle burn-in timing | Separate research needed when phase begins |
| Overlays (future) | Text rendering, positioning, font availability | Separate research needed when phase begins |

---

## Sources

- [Rust std::process::Stdio docs -- pipe deadlock warning](https://doc.rust-lang.org/std/process/struct.Stdio.html)
- [Rust issue #45572 -- Command hangs if piped stdout buffer fills](https://github.com/rust-lang/rust/issues/45572)
- [Rust issue #73126 -- Command output() error handling hazards](https://github.com/rust-lang/rust/issues/73126)
- [FFmpeg silencedetect filter docs (7.1)](https://ayosec.github.io/ffmpeg-filters-docs/7.1/Filters/Audio/silencedetect.html)
- [FFmpeg silencedetect source code](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_silencedetect.c)
- [FFmpeg Concatenate wiki](https://trac.ffmpeg.org/wiki/Concatenate)
- [Remsi -- silence removal approach](https://github.com/bambax/Remsi)
- [ffmpeg-python split_silence.py -- edge case handling](https://github.com/kkroening/ffmpeg-python/blob/master/examples/split_silence.py)
- [FFmpeg-devel -- silencedetect timestamp precision](https://www.mail-archive.com/ffmpeg-devel@ffmpeg.org/msg160779.html)
- [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/)
- [Rust CLI book -- signal handling](https://rust-cli.github.io/book/in-depth/signals.html)
- [ctrlc crate](https://docs.rs/ctrlc)
- [TikTok video format guide](https://snaptiksave.online/tiktok-video-formats-explained/)
- [FFmpeg-devel -- silencedetect format change](https://ffmpeg.org/pipermail/ffmpeg-devel/2014-May/157790.html)
- [FFmpeg rotation and iPhone video handling](https://thornelabs.net/posts/correct-smartphone-video-orientation-and-how-to-rotate-ios-and-android-videos-with-ffmpeg/)
