# Project Research Summary

**Project:** contentops — Milestone 2 (audit tooling, doctor subcommand, pipeline subcommand, CI/CD)
**Domain:** Rust CLI for video post-production (short-form content: TikTok/Reels/Shorts)
**Researched:** 2026-02-20
**Confidence:** HIGH

## Executive Summary

contentops v1.0 is a complete, working Rust CLI (2,401 LOC, 3 subcommands) that replaced CapCut for a single macOS creator. The v1.1 milestone adds four capabilities: codebase audit tooling, a `doctor` subcommand, a `pipeline` subcommand, and GitHub Actions CI/CD. Critically, this milestone adds **zero new Cargo.toml runtime dependencies** — every new feature is built from existing crates (`which`, `anyhow`, `clap`, `owo-colors`), external tooling (`cargo-audit`, `cargo-deny`), and GitHub Actions YAML. The architecture is strictly additive: two new command modules (`doctor.rs`, `pipeline.rs`) plug into the established `Commands` enum and `run()` dispatch pattern without touching any existing command logic.

The recommended implementation order is strict and non-negotiable based on dependency analysis: audit/cleanup first, then doctor, then pipeline, then CI/CD. Audit before pipeline because existing duplication (three identical spinner factories across `cut.rs`, `caption.rs`, `overlay.rs`) would multiply into every pipeline code path if not extracted first. Doctor before pipeline because pipeline calls the same `require_ffmpeg()`/`require_whisper()` infrastructure that doctor validates — and because the missing `require_claude()` check in `overlay --auto` must be closed before wiring overlay into the pipeline. CI/CD last because its primary value is gating on `cargo clippy -D warnings` and `cargo test`, which must be green before the workflow is useful.

The key implementation risks are: behavioral regressions during the audit refactor (temp file ownership semantics and spinner `finish_*` behavior are not caught by the compiler), pipeline intermediate file management (intermediates must live in a temp directory, not the working directory, to avoid collisions and working-directory pollution), and GitHub Actions runner architecture (since 2024, `macos-latest` is ARM64 — Intel builds require explicit `macos-13` runner). All three risks have concrete prevention strategies identified in the research.

## Key Findings

### Recommended Stack

Zero new runtime dependencies for this milestone. All additions are tooling, configuration files, and CI YAML. The existing Cargo.toml already provides everything needed.

**Core technologies:**
- `which` (8.0, already in Cargo.toml): PATH lookup for doctor checks — faster than shelling to `which`, returns typed `Result<PathBuf>`
- `clap` (4.5, already in Cargo.toml): two new `Commands` variants (`Doctor`, `Pipeline`) and their `*Args` structs via derive macros
- `owo-colors` (4.2, already in Cargo.toml): colored `[ok]`/`[fail]`/`[warn]` output for doctor
- `cargo-audit` (0.22.1, external): security advisory checking against RustSec database
- `cargo-deny` (0.18.5, external): license compliance and duplicate dependency detection
- `dtolnay/rust-toolchain@stable`: CI toolchain — actively maintained replacement for unmaintained `actions-rs/toolchain`
- `Swatinem/rust-cache@v2`: CI build cache — reduces cold build (~3-4 min) to cached (~30s)
- `actions-rust-lang/audit@v1`: CI security audit — actively maintained replacement for unmaintained `actions-rs/audit-check`
- `taiki-e/upload-rust-binary-action@v1`: Release binary packaging — handles target naming, `.tar.gz` archives, SHA256 checksums

**Explicitly excluded:** tokio or any async runtime (pipeline stages are sequential, no async value), `cross` crate (Docker cross-compilation unnecessary when GitHub provides native macOS runners), `cargo-make`/`just` (Cargo built-ins are sufficient), `actions-rs/*` actions (unmaintained since 2023)

### Expected Features

**Must have (table stakes):**
- Doctor: `[ok]`/`[fail]`/`[warn]` checks for ffmpeg, ffprobe, whisper-cli, claude — flutter/npm/brew-style output is the established convention
- Doctor: per-subcommand readiness summary ("cut: ready, caption: missing whisper-cli") — flat check list without this is incomplete
- Doctor: exit 0 by default (exit 1 only with `--strict`) — running doctor on a CI machine without FFmpeg must not break CI
- Doctor: version minimum check for ffmpeg (>= 6.0) — filter syntax differs across major versions
- Pipeline: single command chaining cut → caption → overlay, with intermediates in a temp directory (not working directory)
- Pipeline: preserve temp directory on failure with printed path and recovery hint
- Pipeline: `--dry-run` showing planned stages without executing
- CI: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` + `cargo audit` on push/PR
- Release: both `aarch64-apple-darwin` (macos-latest) and `x86_64-apple-darwin` (macos-13) binaries with SHA256 checksums

**Should have (differentiators):**
- Audit `--fix` mode: wraps `cargo clippy --fix --allow-dirty` + `cargo fmt` in one command
- Pipeline: `--keep-intermediates` flag for debugging/stage reuse
- Universal macOS binary (`lipo`-merged) for single-download UX
- Colored audit summary header: "3 errors, 7 warnings" in red/yellow before details

**Defer (v2+):**
- YAML/TOML pipeline config files — shell aliases cover preset combinations; one user doesn't need reproducible pipeline definitions
- Re-entrant pipeline (resume from failed stage) — complex state tracking; manual rerun via individual commands is acceptable
- Linux/Windows builds — hard-coded `/System/Library/Fonts/Supplemental/Impact.ttf`; no current users on other platforms
- Homebrew formula — ongoing maintenance burden; GitHub Releases binary download is sufficient
- crates.io publish — this is an application binary, not a library; pre-built binaries have strictly better UX

### Architecture Approach

The v1.1 architecture is minimal and additive. Two new source files and two new GitHub Actions workflows. The only changes to existing files are: making `derive_caption_output` public in `caption.rs` (one-line visibility change, prerequisite for pipeline), adding `require_claude()` to `error.rs` (closes the missing prereq check for `overlay --auto`), and registering the new modules in `commands/mod.rs`.

**Major components:**
1. `commands/doctor.rs` (~80 LOC) — `which::which()` checks per tool, colored pass/warn/fail output via `owo-colors`, per-subcommand readiness grouping, exits 0 by default
2. `commands/pipeline.rs` (~70 LOC) — calls `cut::run()`, `caption::run()`, `overlay::run()` as direct Rust function calls (not subprocess calls), threads shared `TempFileRegistry` across all stages, uses temp directory for intermediates
3. `.github/workflows/ci.yml` — fmt + clippy + test + audit on push/PR; runs on `macos-latest` to avoid macOS-specific path failures on ubuntu runners
4. `.github/workflows/release.yml` — tag-triggered; two-runner matrix (`macos-latest` for ARM64, `macos-13` for Intel x86_64); uses `taiki-e/upload-rust-binary-action` for artifact packaging and checksums

**Key architectural constraints confirmed by source audit:**
- Pipeline must call `run()` functions directly, not shell to `contentops cut` subprocess — subprocess approach loses `TempFileRegistry`, loses typed `AppError`, requires binary on PATH during development
- Prerequisite checks stay at each command's call site — centralized pre-dispatch checking in `main.rs` causes whisper check to fire for `cut` and claude check to fire for `caption`
- `PipelineArgs` exposes only `input`, `model`, `lang`, `breaths`, `font` — re-exposing all flags from three commands creates 15+ fields; users needing fine control use individual commands

**Build order for implementation:**
1. `require_claude()` + overlay guard in `error.rs` (closes immediate gap, one-liner)
2. `pub fn derive_caption_output` in `caption.rs` (prerequisite for pipeline)
3. `DoctorArgs` + `Commands::Doctor` in `cli.rs`
4. `commands/doctor.rs`
5. `PipelineArgs` + `Commands::Pipeline` in `cli.rs`
6. `commands/pipeline.rs`
7. `.github/workflows/release.yml` (independent, can be written any time)

### Critical Pitfalls

1. **Refactoring breaks existing subcommands without compile-time warning** — extract shared spinner/temp logic one subcommand at a time; verify each produces identical output on a real video file before touching the next. Temp file ownership semantics (`.keep()` call timing, `Drop` order) are the highest-risk silent regression.

2. **Pipeline intermediate files pollute the working directory** — use a dedicated temp directory (`{stem}_pipeline_tmp_{timestamp}/`) for all intermediates; only the final output goes in the user's directory. On success: delete temp dir. On failure: preserve it and print the path with a recovery hint ("Run `contentops caption` on it to continue manually").

3. **`doctor` exits 1 when dependencies are missing, breaking CI** — doctor is a diagnostic tool, not a prerequisite enforcer. Default exit 0 always; reserve exit 1 for `--strict` flag. Design this contract before writing any code.

4. **macOS-specific paths cause CI failures on ubuntu-latest** — `overlay.rs` hardcodes `/System/Library/Fonts/Supplemental/Impact.ttf`. Any CI job on ubuntu-latest that compiles and runs the binary fails with a non-Rust error. Keep CI on `macos-latest` for all jobs that exercise the binary.

5. **`macos-latest` is ARM64 since 2024 — Intel users need explicit `macos-13`** — without a two-runner matrix, releases ship ARM-only. `macos-13` is the last Intel GitHub runner. Name artifacts with architecture suffix (`contentops-aarch64-apple-darwin`, `contentops-x86_64-apple-darwin`).

## Implications for Roadmap

Based on combined research, the dependency ordering is strict. Each phase unblocks the next.

### Phase 1: Audit and Cleanup
**Rationale:** Three identical `make_spinner` implementations across `cut.rs`, `caption.rs`, `overlay.rs` would be called by pipeline in every stage. `#[allow(dead_code)]` on `cleanup_all` in `temp.rs` is a live example of the dead code accumulation pattern that ruins audit signal. Inconsistent `anyhow::bail!` vs `AppError` returns mean some pipeline errors get `format_error()` treatment and some don't. Clean before adding complexity.
**Delivers:** Clippy-clean codebase (`-D warnings` passes with zero suppressions added), extracted shared spinner utility, consistent `AppError`-based error handling, zero `#[allow(dead_code)]` attributes without documented justification.
**Addresses:** Codebase audit feature area; closes technical debt items documented in PITFALLS.md technical debt table.
**Avoids:** Pitfall 14 (refactoring regressions) — cleanup happens before pipeline adds new call sites that depend on extracted code.

### Phase 2: Doctor Subcommand
**Rationale:** Doctor validates the `which::which()` + `require_*()` prerequisite check infrastructure before pipeline chains all three stages together. Also closes the `require_claude()` gap in `overlay --auto` — a concrete bug that should not ship into the pipeline. Doctor is low-risk (no temp files, no FFmpeg, no state) and provides immediate user value.
**Delivers:** `contentops doctor` with colored pass/warn/fail output, per-subcommand readiness summary, minimum FFmpeg version check, exit 0 by default.
**Addresses:** Doctor feature area; closes `overlay --auto` missing prerequisite check.
**Avoids:** Pitfall 15 (doctor exit code semantics) — exit contract is designed and documented before any code is written.

### Phase 3: Pipeline Subcommand
**Rationale:** Pipeline builds on cleaned-up subcommands (Phase 1) and validated prerequisite infrastructure (Phase 2). Requires `derive_caption_output` to be public (one-line change) and `require_claude()` to exist in `error.rs` (added in Phase 2). This is the highest user-value feature of the milestone — the three-command manual workflow becomes one command.
**Delivers:** `contentops pipeline input.mp4 --model ggml-base.bin` replacing three manual invocations. Intermediate files managed in temp directory; final output to working directory.
**Addresses:** Pipeline feature area; the primary workflow bottleneck identified in FEATURES.md.
**Avoids:** Pitfall 16 (file collisions) via temp directory; Pitfall 17 (partial failure ambiguity) via intermediate preservation on failure with recovery hint.

### Phase 4: GitHub Actions CI/CD
**Rationale:** CI gates on `cargo clippy -D warnings` and `cargo test`. Setting it up before the codebase is clean (Phase 1) creates a permanently red CI that must be immediately fixed. Setting it up before new commands exist (Phases 2-3) means the first green run doesn't actually validate the milestone features. Comes last, ships green on day one.
**Delivers:** Automated CI on push/PR; GitHub Release with ARM64 + Intel macOS binaries and SHA256 checksums on tag push.
**Addresses:** CI/CD feature area; enables other users to install `contentops` without building from source.
**Avoids:** Pitfall 18 (macOS paths on ubuntu CI) — CI on `macos-latest`; Pitfall 19 (no integration tests) — FFmpeg installed in CI runner; Pitfall 20 (wrong arch binary) — explicit two-runner matrix with architecture-suffixed artifact names.

### Phase Ordering Rationale

- **Audit before pipeline:** Pipeline calls existing `run()` functions — duplication in those functions directly multiplies into pipeline's error surface. Extract first, integrate second.
- **Doctor before pipeline:** `require_claude()` is added during doctor work; pipeline depends on it for `overlay --auto` stage. Also validates the `require_*()` pattern works correctly before pipeline chains all three stages.
- **CI/CD last:** CI is automation of what already works. Green from day one requires the codebase to already be clean and the commands to already be implemented.

### Research Flags

Phases with standard, well-documented patterns (no additional research needed):
- **Phase 1 (Audit):** `cargo clippy --message-format=json` is documented; Rust extraction refactoring patterns are standard.
- **Phase 2 (Doctor):** Flutter/npm/brew doctor pattern is well-established; `which` crate already used in production code.
- **Phase 4 (CI/CD):** All GitHub Actions used are officially documented and actively maintained; two-runner macOS matrix is a known pattern.

Phases needing validation during implementation:
- **Phase 3 (Pipeline):** The JSON path wiring — `derive_caption_output(input, "captioned", "json")` must produce exactly the path that `caption::run()` writes, which is then passed to `overlay::run()` as `args.auto`. This derivation chain is the highest-risk integration point and must be verified with a real video file before declaring the phase complete.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Zero new dependencies; all tooling verified against official docs and crates.io as of 2026-02-20. One caveat: `macos-13` runner name may change — verify at CI implementation time. |
| Features | HIGH | Doctor pattern from flutter/npm/brew is well-established. Pipeline design is straightforward function composition. CI/CD patterns verified against taiki-e action documentation. |
| Architecture | HIGH | Based on direct source code audit of all 12 source files (2,401 LOC). Integration points are explicit and verifiable. Findings are from actual source, not inference. |
| Pitfalls | HIGH (existing v1.0 pitfalls) / MEDIUM (new v1.1 pitfalls) | v1.0 pitfalls verified against official Rust and FFmpeg documentation. v1.1 pitfalls grounded in codebase audit; behavioral regression risk is real but prevention is well-understood. |

**Overall confidence:** HIGH

### Gaps to Address

- **`macos-13` runner longevity:** GitHub may retire the Intel runner or rename it. Verify availability at CI implementation time; document which runner is used and why.
- **JSON path derivation chain:** `derive_caption_output` path must exactly match what `caption::run()` writes to disk. Cannot be verified without a real run. Test this before finalizing `pipeline.rs`.
- **cargo-deny vs cargo-audit overlap:** Both check security advisories. If both run in CI, advisory hits are double-reported. Decide at implementation time whether to run cargo-deny only (superset) or both (explicit separation of concerns). Either is defensible.
- **`doctor --strict` exit code use case:** Research recommends exit 0 by default, but the scripted use case `contentops doctor && contentops pipeline ...` requires exit 1 on failure to be useful. Validate the `--strict` flag design against actual usage before locking the CLI API.

## Sources

### Primary (HIGH confidence)
- [Clippy Configuration](https://doc.rust-lang.org/clippy/configuration.html) — clippy.toml options, pedantic group, `-D warnings` CI usage
- [Clippy Usage/CI](https://doc.rust-lang.org/clippy/usage.html) — `--message-format=json`, `-A clippy::module_name_repetitions` rationale
- [cargo-audit 0.22.1](https://docs.rs/crate/cargo-audit/latest) — advisory check invocation, `--deny warnings` flag
- [which 8.0.0](https://docs.rs/which/latest/which/) — PATH lookup API used in doctor and error.rs
- [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action) — binary upload, checksum, target triple naming
- [taiki-e/create-gh-release-action](https://github.com/taiki-e/create-gh-release-action) — release creation from CHANGELOG.md
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain) — CI toolchain action, `components: clippy,rustfmt`
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) — Cargo registry + target caching
- [actions-rust-lang/audit](https://github.com/actions-rust-lang/audit) — maintained audit action with GitHub Issues integration
- [Rust std::process::Stdio](https://doc.rust-lang.org/std/process/struct.Stdio.html) — pipe deadlock documentation (v1.0 pitfall sourcing)
- Direct codebase audit: all 12 source files in contentops/src/, 2026-02-20 (HIGH — actual source, not inference)

### Secondary (MEDIUM confidence)
- [cargo-deny 0.18.5](https://docs.rs/crate/cargo-deny/latest) — license compliance and duplicate dependency detection
- [ahmedjama.com cross-platform Rust CI/CD 2025](https://ahmedjama.com/blog/2025/12/cross-platform-rust-pipeline-github-actions/) — workflow matrix structure reference
- [GitHub Actions runner images changelog](https://github.com/actions/runner-images) — `macos-latest` ARM64 change documentation
- [cargo-machete](https://crates.io/crates/cargo-machete) — unused dependency detection, actively maintained 2025
- [flutter doctor UX pattern](https://www.dhiwise.com/post/flutter-doctor-command-a-vital-tool-for-developers) — pass/warn/fail output conventions
- [npm doctor](https://docs.npmjs.com/cli/v7/commands/npm-doctor/) — check categorization pattern

### Tertiary (LOW confidence)
- Select filter expression length limits — theoretical; no confirmed breakage at realistic TikTok video lengths

---
*Research completed: 2026-02-20*
*Ready for roadmap: yes*
