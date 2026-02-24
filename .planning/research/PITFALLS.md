# Pitfalls Research

**Domain:** Rust CLI orchestrating FFmpeg for video processing (silence removal, captioning, overlays)
**Researched:** 2026-02-20, updated 2026-02-24 for v1.4 Silero VAD milestone
**Confidence:** HIGH (v1.0 pitfalls) / MEDIUM (new-milestone pitfalls, grounded in codebase audit)

> **Scope note:** This file was updated for the v1.1 milestone. Pitfalls 1-13 cover the v1.0 domain
> (FFmpeg piping, silence removal, temp files). Pitfalls 14+ cover the new work: codebase audit,
> `doctor` subcommand, `pipeline` subcommand, and GitHub Actions CI/CD.
>
> Pitfalls 20+ cover the v1.4 Silero VAD milestone: ort/ONNX Runtime linking, model versioning,
> cross-platform distribution, CI integration, binary size, and audio format requirements.

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

**Phase:** Foundation (Phase 1). Test this in your very first FFmpeg integration.

**Confidence:** HIGH — FFmpeg documented behavior.

---

### Pitfall 3: Silencedetect Timestamp Parsing Fails on Non-English Locales

**What goes wrong:** FFmpeg's silencedetect filter outputs timestamps as decimal numbers with `.` as the decimal separator (e.g., `silence_end: 2.34`). On systems with a locale that uses `,` as the decimal separator (common in Europe), FFmpeg may output `2,34` instead, causing float parsing to fail.

**Why it happens:** FFmpeg respects the system locale for number formatting in some output contexts. Rust's `str::parse::<f64>()` expects `.` as the decimal separator regardless of locale.

**Consequences:** Silence timestamp parsing panics or returns no timestamps, causing the entire silent segment removal to fail silently (the output video has no cuts made).

**Prevention:**
- Use `LC_ALL=C` or `LANG=C` when invoking FFmpeg from Rust if you intend to parse its output.
- In Rust: set `.env("LC_ALL", "C")` on the `Command`.
- Alternatively, replace `,` with `.` before parsing.

**Detection:** Test on a machine with a non-English locale, or temporarily `export LANG=de_DE` before running.

**Phase:** Silence Removal (Phase 2).

**Confidence:** MEDIUM — documented locale issue, verified through community reports.

---

### Pitfall 4: Float Precision in Silence Timestamp Arithmetic Causes A/V Sync Drift

**What goes wrong:** When calculating silence segment boundaries (start/end), floating-point arithmetic on ffmpeg timestamps accumulates error over many segments. After 50+ cuts in a long video, the concat filter's segment boundaries drift from the actual audio waveform, causing A/V desync.

**Why it happens:** FFmpeg timestamps are floating-point. Adding/subtracting the silence pad duration (e.g., 0.075 seconds) many times accumulates binary floating-point representation error. The concat filter is sensitive to sub-millisecond timing precision.

**Consequences:** A/V sync drift increases with video length and number of cuts. Imperceptible for short videos with few cuts; noticeable (50-100ms drift) for 10-minute videos with 100+ cuts.

**Prevention:**
- Use i64 milliseconds internally, not f64 seconds.
- Convert to float only when building the FFmpeg filter expression.
- Round pad calculations to 3 decimal places maximum.

**Detection:** Run silence removal on a 30-minute lecture with frequent cuts. Compare the audio waveform of a specific word in the input vs. output.

**Phase:** Silence Removal (Phase 2).

**Confidence:** MEDIUM — general float arithmetic issue; specific drift threshold is an estimate.

---

### Pitfall 5: Concat Filter Fails With Zero-Duration Segments

**What goes wrong:** When speech is immediately adjacent to the start or end of the file, the silence pad calculation can produce a segment with a duration of 0 seconds or negative duration. FFmpeg's concat filter rejects zero-duration segments and exits with an error.

**Why it happens:** If the first silence ends at 0.05s and your pad is 0.075s, the segment start calculates to -0.025s, which is clamped to 0. If the next silence starts at 0.0s, you get a 0-duration first segment.

**Consequences:** `ffmpeg: Error while opening encoder for output stream` or similar. The output file is not produced.

**Prevention:**
- Filter out segments where `end - start <= 0.001` before building the concat filter.
- Clamp segment start to `max(0.0, silence_end - pad)`.
- Clamp segment end to `min(total_duration, silence_start + pad)`.

**Detection:** Test with a video that starts speaking immediately (no silence at the beginning), or a video where all content is speech (no silence at all).

**Phase:** Silence Removal (Phase 2).

**Confidence:** HIGH — directly observed failure mode.

---

### Pitfall 6: Temp File Cleanup Fails on Process Kill

**What goes wrong:** When the user Ctrl+C's the process, the temp directory is not cleaned up. On subsequent runs, stale temp files accumulate and can be hundreds of MB.

**Why it happens:** Rust's `Drop` trait is not called on SIGTERM/SIGKILL. `ctrlc::set_handler` can work for SIGINT but not for kills.

**Consequences:** Disk fills up over time. Especially problematic on CI where workspace cleanup is expected.

**Prevention:**
- Use `tempfile::TempDir` which registers cleanup even on panic (but not on kill).
- Accept that kill-level signals cannot be handled cleanly; document this.
- Don't register the temp dir in any global state that would prevent GC.
- Add a cleanup message on normal exit so users know where temp files live.

**Detection:** Start a long encode and kill -9 the process. Check for leftover temp directories.

**Phase:** Foundation (Phase 1).

**Confidence:** HIGH — standard systems programming constraint.

---

### Pitfall 7: Progress Bar Interferes With stderr Logging

**What goes wrong:** When using `indicatif` progress bars while also writing to stderr (e.g., for debug output or error messages), the progress bar and text output interleave incorrectly. The progress bar overwrites error messages, or error messages are partially rendered.

**Why it happens:** indicatif uses ANSI escape codes to move the cursor and redraw the progress bar on the same terminal lines. Any other writes to stderr during this time corrupt the display.

**Consequences:** Error messages from FFmpeg or the Rust code are invisible or garbled. Users see corrupted output.

**Prevention:**
- Never write to stderr directly while a progress bar is active. Use `println_below()` or `bar.println()` to write messages that interleave correctly with the bar.
- For error output: finish the bar first, then write the error.
- Use `ProgressBar::suspend()` around any print calls.

**Detection:** Run any command that produces stderr output while a progress bar is active.

**Phase:** Overlays and Polish (Phase 5).

**Confidence:** HIGH — documented indicatif behavior.

---

### Pitfall 8: Whisper Timestamps Are Word-Level But Not Sub-Word

**What goes wrong:** Whisper's word-level timestamps (`--word-timestamps true`) give one timestamp per token, but tokens are not always whole words. Contractions like "don't" become two tokens ("don" and "'t") each with timestamps. This produces subtitles where punctuation splits incorrectly across word boundaries.

**Why it happens:** Whisper tokenizes text using a byte-pair encoding that doesn't respect English word boundaries. The `--word-timestamps` flag gives timestamps per BPE token, not per orthographic word.

**Consequences:** Caption highlighting splits at apostrophes, creating visual glitches. "don" lights up, then "'t" lights up as a separate word.

**Prevention:**
- Post-process: merge tokens that start with punctuation (apostrophe, comma, period) with the previous token.
- Specifically: if a word starts with `'`, merge it with the preceding word's span.

**Detection:** Transcribe any audio containing contractions or possessives. Look for split words in the generated ASS subtitles.

**Phase:** Caption Generation (Phase 3).

**Confidence:** HIGH — directly observed, fixed in v1.0 implementation.

---

### Pitfall 9: ASS Subtitle Rendering Misaligns With Fonts Not Present

**What goes wrong:** The ASS subtitle style specifies a font family (e.g., "Arial"). If that font is not installed on the render machine, FFmpeg silently substitutes a different font. The substitution font has different metrics (character widths, line heights), causing text overflow, incorrect positioning, or visual misalignment.

**Why it happens:** FFmpeg delegates font rendering to libass, which uses fontconfig on Linux/macOS and GDI on Windows. Missing fonts are substituted without any warning or error.

**Consequences:** Captions are cut off at edges, positioned incorrectly, or look completely different from what was tested.

**Prevention:**
- Use fonts that are universally available on all platforms (Arial on macOS/Windows; DejaVu Sans on Linux).
- Or bundle a specific font and pass it explicitly to ffmpeg via `-vf subtitles=file.ass:fontsdir=/path/to/fonts`.
- Test on all three platforms.

**Detection:** Test on a fresh Linux machine where common fonts may not be installed.

**Phase:** Caption Rendering (Phase 4).

**Confidence:** HIGH — documented libass behavior.

---

### Pitfall 10: Impact Font Not Available on Non-macOS Systems

**What goes wrong:** The Impact font used for overlay title cards is not universally installed. It's a Microsoft font present on macOS and Windows by default, but absent on most Linux distributions.

**Why it happens:** Impact is a proprietary Microsoft font. Linux distributions typically only include open-source fonts (Liberation, DejaVu, Noto) by default.

**Consequences:** On Linux, FFmpeg exits with "No such file or directory: Impact" or silently renders with a different font.

**Prevention:**
- Use `#[cfg(target_os = "linux")]` to select a different font.
- Or probe font availability at runtime and fall back.
- Implemented in v1.3: `#[cfg]` blocks select the right font constant per platform.

**Detection:** Run `contentops overlay` on a fresh Linux machine.

**Phase:** Platform-Portable Code (Phase 13). Fixed in v1.3.

**Confidence:** HIGH — directly verified in v1.3 implementation.

---

### Pitfall 11: GitHub Actions Cache Invalidates on Rust Toolchain Update

**What goes wrong:** The Rust toolchain updates frequently. If the cache key only uses `Cargo.lock`, it won't invalidate when the toolchain version changes. Cached compilation artifacts from an old toolchain can cause spurious build failures or incorrect behavior when the toolchain updates.

**Why it happens:** Cargo's artifact format is not guaranteed to be forward-compatible across toolchain versions. Using cached `.d` files and `.rlib` files from a different rustc version can produce link errors.

**Consequences:** CI fails on toolchain updates with opaque errors. The fix (clearing cache) is non-obvious.

**Prevention:**
- Include the Rust toolchain version in the cache key: `${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}-${{ steps.toolchain.outputs.rustc_hash }}`.
- Or simply accept occasional cache misses by using `restore-keys` as fallback.

**Detection:** Update the Rust toolchain version and observe whether CI builds correctly on cached runs.

**Phase:** CI/CD (Phase 9).

**Confidence:** MEDIUM — common pattern in GitHub Actions Rust CI.

---

### Pitfall 12: Universal Binary `lipo` Step Fails if Architecture Builds Run in Parallel

**What goes wrong:** The macOS `lipo` step that combines ARM64 and Intel binaries into a universal binary fails if either architecture binary is not present (e.g., one build job failed silently, or artifacts were not downloaded correctly).

**Why it happens:** GitHub Actions matrix jobs run in parallel. If the ARM64 or Intel job fails but is configured as `continue-on-error`, the `lipo` job still runs but receives only one binary. `lipo -create` with one input just copies the file; the output appears valid but is not universal.

**Consequences:** A "universal" binary is published that is actually single-architecture. Homebrew silently installs the wrong architecture for half of users.

**Prevention:**
- Do not use `continue-on-error` on architecture build jobs.
- Add a verification step after `lipo`: run `lipo -info ./contentops-universal` and verify the output contains both `x86_64` and `arm64`.
- Fail the release job if verification fails.

**Detection:** Run `lipo -info` on any published universal binary before releasing.

**Phase:** CI/CD (Phase 9), Release (Phase 14 for multi-platform).

**Confidence:** HIGH — documented release workflow risk.

---

### Pitfall 13: Homebrew Formula SHA256 Mismatch After Re-Release

**What goes wrong:** If a GitHub Release is edited (assets deleted and re-uploaded), the SHA256 of the new asset differs from what was originally computed and stored in the Homebrew formula. Users see `sha256 mismatch` errors on `brew upgrade`.

**Why it happens:** GitHub generates new binary content even for identically-named assets. The formula's auto-update workflow captures the SHA256 at release time; re-uploaded assets have different hashes.

**Consequences:** Homebrew install fails for all users until the formula is manually corrected.

**Prevention:**
- Never delete and re-upload release assets. Create a new patch release instead.
- The release workflow should be treated as immutable once published.

**Detection:** Check `brew audit --formula contentops` after any release correction.

**Phase:** GitHub Actions Auto-Update (Phase 11).

**Confidence:** HIGH — fundamental constraint of Homebrew formula versioning.

---

## Critical Pitfalls — v1.1 Domain (Audit, Doctor, Pipeline, CI)

### Pitfall 14: Typed Error Variants Can't Be Added Without Touching All Match Arms

**What goes wrong:** When adding a new `AppError` variant, the compiler forces you to update every `match` statement that covers `AppError`. This is good for exhaustiveness, but it means adding one variant requires touching many files simultaneously.

**Why it happens:** Rust's exhaustive pattern matching is a feature, not a bug. But when the error enum grows across multiple milestones, it creates merge conflicts if multiple people work on it simultaneously.

**Consequences:** Large diffs when adding error variants. In a solo project, this is manageable. In a team, it creates merge conflicts.

**Prevention:**
- Keep `AppError` variants coarse-grained (one per subcommand/domain, not one per error case).
- Use `#[non_exhaustive]` if you expect to add variants frequently.

**Detection:** Count the number of match arms touching `AppError` before adding a new variant.

**Phase:** Audit & Cleanup (Phase 6).

**Confidence:** HIGH — standard Rust enum exhaustiveness constraint.

---

### Pitfall 15: Doctor Subcommand Checks the Wrong Binary

**What goes wrong:** `contentops doctor` checks whether `ffmpeg` is on `PATH`, but when users install FFmpeg via a non-standard path (e.g., `/opt/homebrew/bin/ffmpeg` vs `/usr/local/bin/ffmpeg`), the check passes but the actual `cut` command fails because `cut` uses a hardcoded path.

**Why it happens:** The doctor check and the actual invocation must resolve binaries the same way. If doctor uses `which ffmpeg` and cut uses `Command::new("ffmpeg")`, they agree on PATH resolution. But if cut ever uses an absolute path, they diverge.

**Consequences:** `doctor` reports green, but `cut` fails. Users are confused.

**Prevention:**
- Always use `Command::new("ffmpeg")` (PATH-relative) for both the doctor check and the actual invocations.
- Doctor should check using the same resolution mechanism as the actual commands.

**Detection:** Set `PATH` to a custom location with FFmpeg and run both `doctor` and `cut`.

**Phase:** Doctor Subcommand (Phase 7).

**Confidence:** HIGH — design constraint verified in v1.1.

---

### Pitfall 16: Pipeline Subcommand Shares TempFileRegistry Incorrectly

**What goes wrong:** If `pipeline` creates a new `TempFileRegistry` per sub-step instead of passing a shared registry, temp files from intermediate steps (e.g., the `cut` output used as `caption` input) are deleted before the next step runs.

**Why it happens:** Each subcommand's `run()` function creates its own `TempFileRegistry` with `Drop` cleanup. If `pipeline` calls `cut::run()` and `caption::run()` as separate function calls without sharing state, the temp files from `cut` are deleted when `cut::run()` returns.

**Consequences:** Caption fails with "file not found" on the intermediate cut video.

**Prevention:**
- Pass a shared `TempFileRegistry` into each subcommand's `run()` function.
- Or structure pipeline to use named intermediate files that live until pipeline completion.

**Detection:** Run `contentops pipeline` and verify the intermediate files exist during the caption step.

**Phase:** Pipeline Subcommand (Phase 8).

**Confidence:** HIGH — directly verified in v1.1 implementation.

---

### Pitfall 17: cargo-audit Blocks CI on Every New Advisory

**What goes wrong:** `cargo audit` exits non-zero when any advisory is published for any dependency, including transitive ones. A new advisory for a dep you don't control (e.g., a patch to `libc` or `ring`) breaks CI immediately.

**Why it happens:** cargo-audit treats all advisories as errors by default. Security advisories are published continuously.

**Consequences:** CI breaks on an unrelated dependency update. The fix is often "bump a dep" but cargo update might not immediately pull in the fix if the fix isn't yet released.

**Prevention:**
- Use `cargo audit --ignore RUSTSEC-XXXX-XXXX` for advisories you've triaged and accepted.
- Or configure `.cargo/audit.toml` with `[advisories] ignore = [...]`.
- Consider whether cargo-audit should be a hard gate vs. a warning-only check.

**Detection:** Watch for CI failures not caused by your code changes. Check `cargo audit` output for advisory IDs.

**Phase:** CI/CD (Phase 9).

**Confidence:** HIGH — common cargo-audit operational experience.

---

### Pitfall 18: Release Workflow Triggers on Any Tag, Not Just Version Tags

**What goes wrong:** If the `release.yml` GitHub Actions workflow triggers on `on: push: tags: ['*']`, pushing any tag (e.g., a date-based tag or a test tag) triggers a full release build and potentially publishes artifacts.

**Why it happens:** Wildcards in tag filters match everything. It's easy to accidentally push a non-version tag.

**Consequences:** Spurious releases appear in GitHub Releases. Homebrew auto-update triggers on every tag push.

**Prevention:**
- Use semantic version tags only: `tags: ['v[0-9]+.[0-9]+.[0-9]+']`.
- Protect release tags with branch protection rules.

**Detection:** Push a non-semver tag and observe whether the release workflow triggers.

**Phase:** CI/CD (Phase 9).

**Confidence:** HIGH — documented GitHub Actions tag filter behavior.

---

### Pitfall 19: Cross-Platform CI Matrix Masks Platform-Specific Failures

**What goes wrong:** When a CI matrix job fails on one platform (e.g., Windows), the overall CI check may appear to pass if the matrix is configured with `fail-fast: false` and the successful jobs are what the PR branch protection checks.

**Why it happens:** GitHub PR status checks can be configured to require only the overall matrix check (pass if any job passes) rather than requiring all matrix entries to pass.

**Consequences:** Windows-specific bugs ship undetected. Users file bugs; the Windows CI was never actually passing.

**Prevention:**
- Configure branch protection to require each matrix job individually, or use `fail-fast: true`.
- Alternatively, add a synthetic "all-pass" job that depends on all matrix jobs with `needs: [job-matrix-ids]`.

**Detection:** Deliberately break Windows-only code and verify CI fails on PRs.

**Phase:** CI/CD (Phase 9).

**Confidence:** MEDIUM — GitHub Actions matrix behavior depends on configuration.

---

## Critical Pitfalls — v1.4 Domain (Silero VAD / ONNX Runtime)

### Pitfall 20: ort `download` Strategy Fails in Sandboxed CI Environments

**What goes wrong:** The `ort` crate's default `download` linking strategy downloads ONNX Runtime prebuilt binaries during `cargo build` via the build script (`build.rs`). This network call fails in sandboxed or network-restricted CI environments (restricted GitHub Actions runners, Nix builds, Docker with `--network=none`), producing a build error with no useful message about the root cause.

**Why it happens:** Build scripts run during compilation and are permitted to access the network by default, but some CI configurations block external HTTP from build scripts. The crate downloads from Microsoft CDN URLs; any proxy or network restriction breaks this silently.

**Consequences:** CI fails on every runner with an opaque network or I/O error from within the build script. Local builds work fine, CI does not.

**How to avoid:**
- Do not rely on the `download` strategy in CI. Instead, pre-download ONNX Runtime binaries as a CI step before `cargo build`, then use the `system` strategy with `ORT_LIB_LOCATION` set.
- Or use the `load-dynamic` feature with a pre-staged dylib, setting `ORT_DYLIB_PATH`.
- For GitHub Actions, add a download step per platform before the cargo build step:

```yaml
- name: Download ONNX Runtime
  run: |
    ORT_VERSION="1.20.1"
    # Linux x86_64 example:
    curl -L "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-linux-x64-${ORT_VERSION}.tgz" -o ort.tgz
    tar xzf ort.tgz
    echo "ORT_DYLIB_PATH=$(pwd)/onnxruntime-linux-x64-${ORT_VERSION}/lib/libonnxruntime.so" >> $GITHUB_ENV
```

**Warning signs:** Build log contains "download" or "fetch" errors from within `build.rs`. The error message does not mention ONNX Runtime by name — it shows as a generic download failure.

**Phase to address:** CI/CD update phase (first phase of v1.4).

**Confidence:** HIGH — documented in [Bundling ONNX Runtime in Rust with Nix, Docker and GitHub Actions](https://blog.stark.pub/posts/bundling-onnxruntime-rust-nix/) and [ort linking docs](https://ort.pyke.io/setup/linking).

---

### Pitfall 21: Windows DLL Version Conflict with System32 onnxruntime.dll

**What goes wrong:** Windows 11 ships with `onnxruntime.dll` in `C:\Windows\System32`. When the binary is executed, Windows' DLL search order finds the system copy first. The system copy is typically version 1.10.x; your binary was built against 1.20.x. The version mismatch causes a runtime assertion: "The given version [N] is not supported."

**Why it happens:** Windows DLL search order: executable directory → system directories. If the correct version of `onnxruntime.dll` is not in the same directory as the binary, and the system has an older version, the wrong one loads.

**Consequences:** The binary crashes at startup on Windows systems where ONNX Runtime is pre-installed (common on Windows 11 with WSL2 or ML tools).

**How to avoid:**
- Use the `load-dynamic` feature of `ort` instead of compile-time dynamic linking. With `load-dynamic`, `ort` uses `dlopen()`/`LoadLibrary()` and you control the exact path.
- For distribution, bundle `onnxruntime.dll` alongside the binary in the same directory.
- Or statically link ONNX Runtime to avoid DLL conflicts entirely (increases binary size by ~20-30MB).
- The `copy-dylibs` feature helps during development (`cargo run`) but does not solve the deployment problem.

**Warning signs:** Works in CI (where no system ONNX Runtime exists) but crashes on end-user Windows machines. Error message mentions "version" and "not supported."

**Phase to address:** Binary distribution phase (same phase as cross-platform release build).

**Confidence:** HIGH — documented in [ort linking docs](https://ort.pyke.io/setup/linking) and [Microsoft ONNX Runtime issue #11799](https://github.com/microsoft/onnxruntime/issues/11799).

---

### Pitfall 22: macOS Universal Binary Cannot Use Dynamic ONNX Runtime

**What goes wrong:** The current release workflow creates a universal macOS binary by running `lipo` to combine ARM64 and Intel binaries. If the binary dynamically links to `libonnxruntime.dylib`, the universal binary requires a universal `libonnxruntime.dylib` at runtime — which Microsoft does not distribute. They provide separate ARM64 and x86_64 dylibs.

**Why it happens:** A universal binary that dynamically loads a fat dylib requires the dylib to also be universal (contain both slices). Microsoft's prebuilt ONNX Runtime releases for macOS are single-architecture. Creating a universal dylib via `lipo` of two separate prebuilt dylibs is possible but adds significant CI complexity.

**Consequences:** The universal binary works on ARM64 Macs (because the ARM64 dylib is found), fails on Intel Macs (dylib architecture mismatch), or fails on both if the dylib is not distributed alongside the binary.

**How to avoid:**
- Use static linking for macOS: build ONNX Runtime as a static library for each architecture, link statically into each architecture binary, then `lipo` the resulting static-linked binaries together. This is the only clean path to a truly universal, self-contained macOS binary.
- Alternatively, abandon the universal binary and distribute separate ARM64 and Intel binaries (the Homebrew formula already supports this via `on_arm`/`on_intel` DSL — use it).
- Do not attempt to `lipo` two single-arch dynamic binaries that depend on different dylibs.

**Warning signs:** `lipo -info ./contentops-universal` shows both architectures, but the binary crashes on Intel Macs with a dylib load error. `otool -L ./contentops-universal` shows `libonnxruntime.dylib` as a dependency.

**Phase to address:** The macOS build phase of v1.4. Decision needed before implementation: static link or drop universal binary.

**Confidence:** HIGH — derived from [ONNX Runtime universal binary issue #12052](https://github.com/microsoft/onnxruntime/issues/12052) and macOS dynamic linking fundamentals. MEDIUM on exact behavior of lipo'd binaries with mismatched dylibs.

---

### Pitfall 23: Silero VAD Model Version Mismatch (v4 vs v5 Chunk Size)

**What goes wrong:** Silero VAD v4 and v5 ONNX models are not interchangeable. V5 enforces a strict chunk size of exactly 512 samples at 16kHz (or 256 at 8kHz). V4 was more flexible. If code written for v4 (or a Rust crate targeting v4) is used with a v5 model file, inference fails silently or returns garbage probabilities.

**Why it happens:** The ONNX model's input tensor shape is version-specific. V5's model expects a fixed `[1, 512]` input. Providing a different size causes the ONNX Runtime to reject the input or pad/truncate in ways the model wasn't designed for.

**Consequences:** VAD detects either all-silence or all-speech across the entire audio. The output video is either empty (all cut) or uncut (no silence removed).

**How to avoid:**
- Pin the model version explicitly. If using `silero-vad-rs` or `silero-vad-rust`, check which model version the crate was built for and use only that model file.
- For v5: chunk size must be exactly 512 samples at 16kHz. Never pass partial chunks — pad with zeros to exactly 512 samples.
- Name model files explicitly: `silero_vad_v5.onnx` not `silero_vad.onnx` to avoid ambiguity.
- Check the Rust crate's `Cargo.toml` or README for which model version it supports before downloading any model file.

**Warning signs:** VAD returns constant probability values (near 0.0 or near 1.0) across all chunks regardless of audio content. Input tensor shape errors in ONNX Runtime error output.

**Phase to address:** Implementation phase (Silero VAD integration).

**Confidence:** HIGH — documented v5 breaking changes in [Silero VAD v5 discussion #471](https://github.com/snakers4/silero-vad/discussions/471) and [version history wiki](https://github.com/snakers4/silero-vad/wiki/Version-history-and-Available-Models).

---

### Pitfall 24: Audio Format Mismatch — Wrong Sample Rate or Channel Count

**What goes wrong:** Silero VAD requires 16kHz mono f32 audio (normalized to [-1.0, 1.0]). Video files typically have 44.1kHz or 48kHz stereo audio in AAC or Opus format. Feeding raw audio bytes from the source video directly to VAD produces garbage predictions.

**Why it happens:** Audio format conversion is often treated as a detail rather than a prerequisite. The ONNX model does not error — it accepts any f32 array of the right chunk size. It simply produces wrong speech probabilities when the sample rate is wrong.

**Consequences:** VAD detects no speech in a video containing clear speech, or detects speech in pure silence. The silence removal either cuts everything or cuts nothing.

**How to avoid:**
- Always decode to 16kHz mono f32 PCM before VAD inference. Use FFmpeg to extract audio:
  ```bash
  ffmpeg -i input.mp4 -ac 1 -ar 16000 -f f32le output.raw
  ```
- Verify the format before inference: assert `sample_rate == 16000` and `channels == 1`.
- Do not rely on the ONNX model to reject wrong-format input — it won't.

**Warning signs:** VAD probability for all chunks is very low (< 0.1) even for clearly voiced audio. Or very high (> 0.9) for silent audio.

**Phase to address:** Implementation phase (audio decoding pipeline).

**Confidence:** HIGH — confirmed by [Silero VAD FAQ](https://github.com/snakers4/silero-vad/wiki/FAQ) and official model requirements.

---

### Pitfall 25: Silero VAD State Not Reset Between Invocations

**What goes wrong:** Silero VAD is a stateful RNN — it carries hidden state from chunk to chunk within a single audio stream. When processing multiple videos sequentially (e.g., in a batch), if the model's internal state is not reset between files, the model's perception of "is speech happening?" bleeds across file boundaries. The first few seconds of each subsequent file are mispredicted based on the previous file's end state.

**Why it happens:** The `VADIterator` and the underlying model session maintain LSTM hidden state. Developers initialize the model once and reuse it across files for performance, forgetting to call `reset_states()`.

**Consequences:** The second video processed in a session shows incorrect VAD at the start. This is subtle because it only manifests when processing multiple videos in one process lifecycle. Single-video use (the current `contentops` design) is unaffected.

**How to avoid:**
- Even though contentops currently processes one video per invocation, the model initialization and reset pattern should be correct from the start.
- Call `model.reset_states()` before processing each file.
- Or re-initialize the model per file — slightly slower but avoids the state management concern.

**Warning signs:** VAD predictions for the first few seconds of a file differ between single-run and sequential-run benchmarks.

**Phase to address:** Implementation phase (Silero VAD integration).

**Confidence:** HIGH — confirmed in [Silero VAD streaming documentation](https://github.com/snakers4/silero-vad/discussions/572) and silero-vad-rs docs.

---

### Pitfall 26: Binary Size Bloat From ort Default Features

**What goes wrong:** Adding `ort` to `Cargo.toml` with default features and static linking increases the binary size from ~5MB to 30-50MB depending on linking strategy and ONNX Runtime version. Users who previously installed a 5MB binary via Homebrew would need to download a 35MB binary.

**Why it happens:** ONNX Runtime is a large C++ library (~25MB static). Default ort features pull in training ops, model zoo fetchers, and RTTI support. Static linking embeds all of this in the Rust binary.

**Consequences:** Homebrew tap download time increases 7x. Direct download install time increases 7x. CI artifact upload time increases.

**How to avoid:**
- Explicitly disable default features: `ort = { version = "2", default-features = false, features = ["load-dynamic"] }`.
- Use the `load-dynamic` feature to keep ONNX Runtime as a separate dylib — the Rust binary itself stays small (~5-8MB), but the dylib must be distributed alongside it.
- If static linking is chosen, use `minimal-build` feature on ort to strip unused ONNX Runtime components.
- The Silero VAD model file itself is ~1.8MB (v5) — use `include_bytes!` for the model (acceptable size), but do not statically link ONNX Runtime.

**Warning signs:** Release binary grows from ~5MB to >20MB after adding `ort`. `ls -lh target/release/contentops` shows unexpected size.

**Phase to address:** Implementation phase (ort integration setup).

**Confidence:** HIGH — confirmed by ort documentation on `minimal-build` and `default-features`. Binary size estimates are MEDIUM confidence (estimates based on typical ONNX Runtime static link sizes).

---

### Pitfall 27: ort `load-dynamic` Requires dylib Present at Runtime — Distribution Problem

**What goes wrong:** Using `load-dynamic` keeps the Rust binary small but requires `libonnxruntime.so`/`libonnxruntime.dylib`/`onnxruntime.dll` to be present on the user's system at runtime. If it's not found, the binary crashes with a cryptic `dlopen` error, not a helpful "install ONNX Runtime" message.

**Why it happens:** `load-dynamic` defers the library load to runtime. If `ORT_DYLIB_PATH` is not set, `ort` searches in standard library paths, which will not contain ONNX Runtime on most systems.

**Consequences:** End users get an unhelpful error about a missing `.so`/`.dylib`. The tool appeared to install correctly (the binary downloaded fine) but fails on first run.

**How to avoid:**
- If using `load-dynamic`, distribute the dylib alongside the binary in the release archive.
- The release archive structure should be:
  ```
  contentops-linux-x86_64.tar.gz/
    contentops          # the binary
    lib/
      libonnxruntime.so  # the dylib
  ```
- The binary finds the dylib via `ort::init_from("./lib/libonnxruntime.so").commit()` called at startup.
- Update the Homebrew formula to include the dylib as a resource, or use the `keg_only` ONNX Runtime formula if one exists.
- Update `contentops doctor` to check for ONNX Runtime availability and print a helpful error if absent.

**Warning signs:** The binary works for developers (who have `ORT_DYLIB_PATH` set) but fails for users. Error message is `cannot open shared object file: No such file or directory` or `image not found`.

**Phase to address:** Binary distribution phase and doctor subcommand update.

**Confidence:** HIGH — standard dynamic library distribution problem. Specific distribution structure is a recommendation, not verified in contentops CI.

---

### Pitfall 28: Linux musl Static Build Incompatible With ONNX Runtime

**What goes wrong:** If you attempt to target `x86_64-unknown-linux-musl` (fully static Linux binary) while linking ONNX Runtime, the build fails. ONNX Runtime's C++ runtime and its dependencies (openmp, etc.) are not compatible with musl libc.

**Why it happens:** The current contentops Linux builds target glibc (`x86_64-unknown-linux-gnu`). ONNX Runtime is built against glibc. Musl libc has different symbol names and ABI. ONNX Runtime does not provide musl builds.

**Consequences:** If anyone attempts to port contentops to Alpine Linux or another musl-based system, the build fails at link time with undefined symbol errors.

**How to avoid:**
- Keep the Linux target as `x86_64-unknown-linux-gnu` (already the case).
- Do not attempt musl builds for contentops once ONNX Runtime is added.
- Document this limitation if Alpine/musl support is ever requested.

**Warning signs:** "undefined reference to `__stack_chk_fail`" or similar musl/glibc ABI mismatch errors during link.

**Phase to address:** CI build configuration phase.

**Confidence:** HIGH — ONNX Runtime musl incompatibility is a known constraint.

---

### Pitfall 29: ONNX Runtime Version Pinning Drift

**What goes wrong:** The `ort` crate pins to a specific ONNX Runtime version. The bundled model and the crate's ONNX opset compatibility are linked. If you update the `ort` crate version, it may require a different ONNX Runtime version than what's pre-downloaded in CI. If you update the model file, it may use ONNX opsets not supported by the current `ort` version.

**Why it happens:** ONNX Runtime has a strict C ABI version contract. The `ort` crate's version number tracks the ONNX Runtime version it was built for. Mismatches produce runtime panics or "unsupported opset" errors.

**Consequences:** CI builds pass (they download the correct ONNX Runtime version), but if the downloaded artifact version and the crate's expected version diverge, runtime failures occur only during actual inference.

**How to avoid:**
- Pin all three versions together: `ort` crate version, ONNX Runtime dylib version, and model ONNX opset version.
- Keep a comment in `Cargo.toml` documenting the expected ONNX Runtime version:
  ```toml
  # ort 2.0.0-rc.9 requires ONNX Runtime 1.20.x
  ort = { version = "=2.0.0-rc.9", ... }
  ```
- Pin the ONNX Runtime download URL to an exact version in CI, not `latest`.
- When upgrading, update all three simultaneously.

**Warning signs:** Runtime panic in `ort::init()` with a version assertion message. Or "unsupported opset" from ONNX Runtime when loading the model.

**Phase to address:** Implementation phase (ort integration setup).

**Confidence:** HIGH — standard version pinning constraint for native library bindings.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Dynamic ONNX Runtime linking | Smaller binary, faster CI | Dylib must be distributed alongside binary; Homebrew formula complexity | Only if binary size is a hard constraint |
| Static ONNX Runtime linking | Single self-contained binary | ~25MB binary size increase | Acceptable for most CLI tools |
| Bundle `silero_vad.onnx` via `include_bytes!` | No external model file | ~1.8MB binary size increase (acceptable); compile time increases | Always acceptable for small models |
| Skip model version verification at startup | Simpler code | Cryptic failures if wrong model is used | Never; always verify model version |
| Hardcode chunk size = 512 | Simple code | Breaks silently if model version changes to different chunk requirement | Only if model version is pinned |
| Use `download` strategy in CI | Zero CI configuration | Fails in sandboxed runners, slow builds | Only in permissive CI (no sandbox) |
| Single universal macOS binary | Simpler Homebrew formula | Cannot dynamically link ONNX Runtime; forces static link | Acceptable if static linking is chosen |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| ort + GitHub Actions | Using default `download` strategy, expecting network access in build script | Pre-download ONNX Runtime before `cargo build`, use `system` or `load-dynamic` strategy |
| Silero VAD ONNX model | Using `silero_vad.onnx` (v4) with v5 chunk sizes | Match model file version to crate expectations; use explicitly versioned model files |
| Audio decoding for VAD | Passing compressed audio bytes or wrong-rate PCM to VAD | Always FFmpeg-decode to 16kHz mono f32le PCM before inference |
| Homebrew + dylib distribution | Formula installs binary but not the required dylib | Bundle dylib in the release archive; formula installs both |
| Windows DLL search | Relying on PATH resolution for `onnxruntime.dll` | Place DLL next to binary, or use `load-dynamic` with explicit path |
| Universal macOS binary + ONNX | `lipo`-ing dynamic-linked binaries from two architectures | Static link each architecture binary, then `lipo` the static-linked results |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Reinitializing ONNX model per audio chunk | Processing is 10-100x slower than expected | Initialize once, reuse session across all chunks | Immediately on first use |
| Full audio in memory for VAD | OOM on large video files | Stream audio in chunk-sized increments from FFmpeg stdout | Files > 1GB or long recordings |
| Synchronous FFmpeg decode + VAD inference | No pipelining; wall time = decode time + inference time | Decode audio via FFmpeg, buffer chunks, run inference | Long videos (>30 minutes) |

---

## "Looks Done But Isn't" Checklist

- [ ] **ort linking:** Binary runs in CI but ships without ONNX Runtime dylib — verify `contentops --version` works on a clean machine with no dev tools installed.
- [ ] **Model bundling:** Model file is included in `include_bytes!` but wrong version — verify with a known audio clip that VAD produces non-trivial predictions (not all 0.0 or all 1.0).
- [ ] **Audio preprocessing:** VAD integration compiles and runs, but input audio was not resampled — verify with `ffprobe` that the audio fed to VAD is 16kHz mono.
- [ ] **Windows distribution:** Binary builds and tests pass in CI, but `onnxruntime.dll` is not in the release zip — verify the release artifact structure includes the dylib.
- [ ] **doctor subcommand:** Doctor does not check for ONNX Runtime — verify `contentops doctor` reports meaningful status about ONNX Runtime availability.
- [ ] **Universal binary:** `lipo -info` shows both architectures, but Intel slice crashes at VAD inference — test on actual Intel Mac hardware, not just in CI.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Wrong ONNX Runtime version in release | MEDIUM | Patch release with correct dylib bundled; re-trigger Homebrew auto-update |
| Universal binary incompatible with dynamic ONNX | HIGH | Switch to static linking (rebuilds all architecture binaries) or drop universal binary |
| VAD model version mismatch in production | LOW | Replace model file, no code changes needed if API is stable |
| CI fails due to ort download strategy | LOW | Add pre-download step to CI YAML, switch to `system` or `load-dynamic` |
| Binary size 10x larger than expected | MEDIUM | Switch from static to `load-dynamic`, restructure release archive |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| ort download strategy fails in CI | v1.4 CI update (first phase) | CI passes on all three platforms without network calls during build |
| Windows DLL version conflict | v1.4 release/distribution phase | Install binary on fresh Windows 11 VM; run `contentops cut` |
| macOS universal binary + dynamic ONNX | v1.4 macOS build design (pre-implementation decision) | `lipo -info` on built binary; test on Intel Mac |
| Silero VAD model version mismatch | v1.4 implementation phase | Run VAD on known clip; verify speech probability range is non-trivial |
| Audio format mismatch (wrong sample rate) | v1.4 implementation phase | Assert `sample_rate == 16000` in code; test with 48kHz input video |
| VAD state not reset | v1.4 implementation phase | Process two files sequentially; verify predictions are independent |
| Binary size bloat from ort | v1.4 implementation phase | Check `ls -lh target/release/contentops` before and after adding ort |
| load-dynamic dylib missing at runtime | v1.4 distribution phase | Install via Homebrew on clean machine; verify binary runs |
| Linux musl incompatibility | v1.4 CI config phase | Do not add musl target; document in CI comments |
| ONNX Runtime version pinning drift | v1.4 implementation phase | Pin versions in `Cargo.toml` comments; update all three together |

---

## Sources

- [ort linking documentation](https://ort.pyke.io/setup/linking) — MEDIUM confidence (official ort docs)
- [Bundling ONNX Runtime in Rust with Nix, Docker and GitHub Actions](https://blog.stark.pub/posts/bundling-onnxruntime-rust-nix/) — MEDIUM confidence (verified external source)
- [Silero VAD v5 release discussion #471](https://github.com/snakers4/silero-vad/discussions/471) — HIGH confidence (official maintainer announcement)
- [Silero VAD version history wiki](https://github.com/snakers4/silero-vad/wiki/Version-history-and-Available-Models) — HIGH confidence (official project documentation)
- [Silero VAD FAQ](https://github.com/snakers4/silero-vad/wiki/FAQ) — HIGH confidence (official project documentation)
- [ONNX Runtime universal binary issue #12052](https://github.com/microsoft/onnxruntime/issues/12052) — MEDIUM confidence (GitHub issue, resolved workaround documented)
- [Microsoft ONNX Runtime issue #11799 — Windows DLL conflict](https://github.com/microsoft/onnxruntime/issues/11799) — HIGH confidence (confirmed bug with documented workaround)
- [silero-vad-rs docs.rs](https://docs.rs/silero-vad-rs/latest/silero_vad_rs/) — MEDIUM confidence (crate documentation, version-specific)
- [ort crate GitHub (pykeio/ort)](https://github.com/pykeio/ort) — HIGH confidence (official source; current version v2.0.0-rc.11 as of 2026-01-07)

---

*Pitfalls research for: Rust CLI (contentops) — Silero VAD / ONNX Runtime integration (v1.4)*
*Researched: 2026-02-24*
