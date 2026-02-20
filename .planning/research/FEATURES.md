# Feature Landscape

**Domain:** Video processing CLI for short-form content creators (TikTok/Reels/Shorts)
**Researched:** 2026-02-19 (v1.0), updated 2026-02-20 (milestone 2: audit, doctor, pipeline, CI/CD)
**Context:** Rust CLI replacing CapCut ($100/yr) for a single creator on macOS

---

## Milestone 2 — New Features Only

This file section documents the four new feature areas for the second milestone. The existing feature landscape (cut, caption, overlay) is complete and not repeated here. Each new area is assessed independently.

---

## Feature Area 1: Codebase Audit

### What a codebase audit command does

An audit command (run as a one-off or during refactor phases) gives a human-readable snapshot of code health. It does not modify code — it reports problems and lets the developer fix them manually or with guided tool invocations.

For a Rust project at 2,400 LOC with 3 subcommands and external shell dependencies, the meaningful signals are:

| Check | Why It Matters | Tool / Method | Complexity |
|-------|---------------|---------------|------------|
| **Clippy warnings** | Rust's official lint set; catches dead code, unused imports, needless clones, pattern issues. Standard CI gate. | `cargo clippy -- -W clippy::all 2>&1` piped + parsed | Low |
| **Dead code** | 2,400 LOC is small enough that dead functions/structs are findable. `#[allow(dead_code)]` suppression sites are worth flagging. | Clippy `dead_code` lint + `#[allow(dead_code)]` grep | Low |
| **Duplicate spinner/progress code** | All three command modules copy-paste the same spinner factory. Should extract to shared utility. | AST-level: `cargo clippy --message-format=json` or textual grep | Low |
| **Unused dependencies in Cargo.toml** | `which`, `ctrlc`, `tempfile`, `humansize`, `serde/serde_json` — confirm all are used. | `cargo-machete` (crates.io, actively maintained 2025) | Low |
| **LOC breakdown** | Per-file line counts show where complexity lives. `tokei` is the standard Rust-ecosystem LOC tool. | `tokei src/` | Low |
| **TODO/FIXME comments** | Unresolved intent markers. | grep | Low |
| **Unsafe usage** | Zero expected in this codebase; any presence is a flag. | `cargo geiger` or grep | Low |

### Table Stakes for Audit

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Clippy output** | Every Rust project CI runs clippy. Expected baseline. | Low | Run with `-W clippy::all`, parse output, count warns/errors. `cargo clippy --message-format=json` gives structured output. |
| **Dead code report** | rustc's `dead_code` lint is on by default. Audit without it is incomplete. | Low | Already surfaced by clippy; just filter and display clearly. |
| **LOC summary** | Developers expect "this is a 2,401 LOC project" headcount. `tokei` is the ecosystem standard. | Low | Shell out to `tokei src/ --output json`. If `tokei` not installed, fallback to `wc -l`. |
| **Unused deps check** | Cargo.toml drift (adding libs, forgetting to remove them) is a real issue. | Low | `cargo-machete` is the current recommended tool (replaced `cargo-udeps` for simpler workflow). |
| **Actionable output** | Audit that just lists problems without "how to fix" is incomplete. | Low | Each finding links to the fix command: "Run `cargo clippy --fix` to auto-correct these". |

### Differentiators for Audit

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Fix mode** | `contentops audit --fix` runs `cargo clippy --fix` and `cargo fmt`. One command instead of remembering flags. | Low | Shell out to `cargo clippy --fix --allow-dirty --allow-staged` and `cargo fmt`. |
| **Colored summary header** | "3 errors, 7 warnings, 2 dead code" in red/yellow/green at the top before the details. | Low | owo-colors already in the project. Pattern: severity-colored counts, then details below. |
| **Exit code contract** | Exit 1 if errors exist, 0 if only warnings/clean. CI-usable. | Low | Matches Rust toolchain convention. |

### Anti-Features for Audit

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **In-process linting** | Reimplementing clippy checks in Rust. Massive scope. | Shell out to `cargo clippy --message-format=json` and parse the JSON output. |
| **HTML/JSON report output** | Not needed for single-developer personal tool. | Terminal output only. If machine-readable needed, `cargo clippy --message-format=json` already does this. |
| **Automated refactoring** | AST rewriting (e.g., auto-extract duplicate spinner code) is complex and risky. | Report the duplication, let developer extract manually. `--fix` handles only safe clippy-auto-fixable issues. |

---

## Feature Area 2: Doctor Subcommand

### The doctor pattern

`flutter doctor`, `brew doctor`, `npm doctor` all follow the same convention: check each prerequisite independently, report pass/warn/fail per check, summarize at the end, print install hints for failures. The user runs it once when something is broken, not in the hot path.

For contentops, the runtime prerequisites are:
- `ffmpeg` on PATH (required by: cut, caption, overlay)
- `ffprobe` on PATH (required by: cut, overlay — duration probing)
- `whisper-cli` on PATH (required by: caption)
- A whisper model file (required by: caption — path provided as flag, but doctor checks for any model at known locations)
- `claude` CLI on PATH (required by: overlay --auto)

### Table Stakes for Doctor

| Check | Why Expected | Output Pattern | Complexity |
|-------|-------------|----------------|------------|
| **ffmpeg present + version** | Primary dependency. Every check starts here. | `[ok] ffmpeg 6.1.1` / `[fail] ffmpeg not found — brew install ffmpeg` | Low |
| **ffprobe present** | Ships with ffmpeg but can be missing if user did a non-standard install. | `[ok] ffprobe 6.1.1` / `[warn] ffprobe not found — progress bars disabled` | Low |
| **whisper-cli present + version** | Required for caption. Missing = caption fails with a confusing error today. | `[ok] whisper-cli found` / `[fail] whisper-cli not found — brew install whisper-cli` | Low |
| **claude CLI present** | Required for overlay --auto. Missing = overlay fails mid-execution today. | `[ok] claude found` / `[warn] claude not found — overlay --auto unavailable` | Low |
| **whisper model discoverable** | No model = caption always fails. Check `~/.local/share/whisper/`, common brew locations. | `[ok] model found: ~/.cache/whisper/ggml-base.bin` / `[warn] no model found — download from huggingface` | Low |
| **Exit code contract** | `doctor` exits 1 if any [fail] check, 0 if only [warn] or all [ok]. | Allows `contentops doctor || exit` in scripts. | Low |

### Differentiators for Doctor

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Per-subcommand readiness** | Show which commands work: "cut: ready, caption: missing whisper-cli, overlay: ready (--auto unavailable)". Actionable vs. flat check list. | Low | Group checks by which command they gate. |
| **Version minimum enforcement** | ffmpeg <5 has different filter syntax. Warn if version is too old. | Low | Parse `ffmpeg -version` output, check major version >= 5. |
| **Run automatically on first command failure** | If `cut` fails with `ffmpeg not found`, suggest "run contentops doctor for full diagnostics". | Low | Already partially exists in error.rs hints. Unify messaging. |

### Anti-Features for Doctor

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Auto-installing missing tools** | Running `brew install` from within the tool is surprising and breaks in non-brew environments. | Print exact install command, let user run it. |
| **Network checks** | Checking HuggingFace or API reachability is slow and adds failure modes. | Doctor is local-only: check what's on disk and PATH. |
| **Config file validation** | No config files in this tool (anti-feature from v1 research). | N/A. |

### Doctor Output Convention (HIGH confidence — matches flutter/npm/brew pattern)

```
contentops doctor

Checking prerequisites...

  [ok]   ffmpeg 7.1 (/opt/homebrew/bin/ffmpeg)
  [ok]   ffprobe 7.1 (/opt/homebrew/bin/ffprobe)
  [fail] whisper-cli not found
           hint: brew install whisper-cli
  [warn] claude not found — overlay --auto unavailable
           hint: npm install -g @anthropic-ai/claude-code

Subcommand readiness:
  cut       ready
  caption   not ready (whisper-cli missing)
  overlay   ready (--auto unavailable)

1 failure, 1 warning.
Run `contentops doctor` to recheck after installing.
```

---

## Feature Area 3: Pipeline Subcommand

### What pipeline means for contentops

The current workflow requires three manual commands:
```
contentops cut input.mp4 -o input_cut.mp4
contentops caption input_cut.mp4 --model ggml-base.bin --burn
contentops overlay input_cut_captioned.mp4 --auto input_cut_captioned.json
```

The pipeline command chains these into one invocation, passing intermediate outputs between stages automatically.

### Design decision: subcommand vs. flags on root

Two approaches exist in the wild:

1. **Root-level flags**: `contentops input.mp4 --cut --caption --overlay --auto`
2. **Explicit pipeline subcommand**: `contentops pipeline input.mp4 --cut --caption --overlay --auto`

The subcommand approach wins for this project because:
- Doesn't conflict with existing single-command invocations (`cut`, `caption`, `overlay` remain as-is)
- Explicit — no ambiguity about what will run
- Clap's existing subcommand pattern already in place
- Allows pipeline-specific flags (e.g., `--keep-intermediates`) without cluttering root

### Table Stakes for Pipeline

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Sequential stage execution** | cut -> caption -> overlay in order, passing output of each as input to next. | Medium | Each stage uses its existing `run()` function. Pipeline manages temp file threading. |
| **Atomic failure** | If caption fails, don't produce a partially-overlaid output. Stop and report which stage failed with the error. | Low | Existing error types already surface stage name. Pipeline propagates `Result`, prints "failed in stage: caption". |
| **Automatic intermediate filenames** | User provides input only. Pipeline derives: `input.mp4` -> `input_cut.mp4` -> `input_cut_captioned.mp4` -> `input_cut_captioned_overlay.mp4`. | Low | Use existing `derive_output_path()` from cut.rs. Chain: each stage output becomes next stage input. |
| **Stage selection flags** | `--cut`, `--caption`, `--burn`, `--overlay`, `--auto` flags select which stages run. Not all stages required. | Low | Boolean flags on PipelineArgs struct. At least one stage required (clap `required_unless_present_any`). |
| **Progress visibility** | Each stage shows its own spinner/progress bar as today. Pipeline adds a "Stage N/M: ..." header. | Low | Print stage header before calling each stage's `run()`. |
| **Intermediate file cleanup** | Temp files from each stage cleaned up on success. On failure, keep the last successful output so the user can inspect. | Low | TempFileRegistry already handles this. On error: don't clean the last intermediate. |
| **Final output control** | `--output` flag specifies where the final result lands. | Low | Pass through to final stage's output path. |

### Differentiators for Pipeline

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Model flag hoisted to pipeline level** | `--model` flag on pipeline applies to caption stage. User doesn't need to know which stage owns it. | Low | PipelineArgs holds `model: PathBuf`, passes to CaptionArgs. |
| **Dry-run mode** | `--dry-run` shows which stages will run and estimated outputs without executing. | Low | Print stage plan: "Will run: cut -> caption (burn) -> overlay (auto)". No FFmpeg calls. |
| **Keep-intermediates flag** | `--keep-intermediates` preserves `_cut.mp4`, `_captioned.mp4` alongside final. Useful for debugging or reuse. | Low | Pass `registry.skip_cleanup()` flag or simply don't delete registered paths. |

### Anti-Features for Pipeline

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **YAML/TOML pipeline config** | Config files add a second interface. One user doesn't need reproducible pipeline definitions. | Flags are sufficient. Shell alias covers preset combos. |
| **Parallel stage execution** | Cut -> caption -> overlay are inherently sequential (each depends on previous output). Parallelism doesn't apply. | Sequential execution. |
| **Re-entrant pipeline (resume from failed stage)** | Complex state tracking. Failure is fast in FFmpeg. | Just rerun. With intermediates kept, rerun from caption stage manually using `contentops caption input_cut.mp4`. |
| **Pipeline config discovery** | Auto-detecting pipeline from file naming patterns. | Explicit flags only. |

### Pipeline Invocation Examples

```bash
# Full pipeline: cut silence, burn captions, add auto overlay
contentops pipeline input.mp4 --cut --caption --burn --overlay --auto \
  --model ~/.cache/whisper/ggml-base.bin

# Cut and caption only (no overlay)
contentops pipeline input.mp4 --cut --caption --model ~/models/ggml-base.bin

# Caption and overlay only (video already cut)
contentops pipeline input.mp4 --caption --burn --overlay --auto \
  --model ~/.cache/whisper/ggml-base.bin

# Keep intermediates for debugging
contentops pipeline input.mp4 --cut --caption --burn --keep-intermediates \
  --model ~/models/ggml-base.bin
```

---

## Feature Area 4: GitHub Releases CI/CD

### What users expect from a Rust CLI GitHub Release

The de facto standard (HIGH confidence — taiki-e/upload-rust-binary-action, houseabsolute/actions-rust-release are the dominant approaches as of 2025-2026):

| Artifact | Why Expected | Notes |
|----------|-------------|-------|
| **macOS arm64 binary** | Apple Silicon is the primary target (M1/M2/M3 Macs). | `aarch64-apple-darwin` target triple. |
| **macOS x86_64 binary** | Intel Mac compatibility. Older machines, Rosetta users. | `x86_64-apple-darwin` target triple. |
| **macOS universal binary** | Single binary for both architectures via `lipo`. Best user experience. | `universal-apple-darwin` via `taiki-e/upload-rust-binary-action`. |
| **.tar.gz archive** | Unix convention. Binary + README inside. `contentops-aarch64-apple-darwin.tar.gz`. | Naming: `$bin-$target.tar.gz`. |
| **SHA256 checksum file** | Integrity verification. Alongside each .tar.gz. | `contentops-aarch64-apple-darwin.tar.gz.sha256`. |
| **GitHub Release notes** | What changed. Linked to tag. | Minimal: tag name + 3-5 bullet changelog. |

### Table Stakes for CI/CD

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Tag-triggered workflow** | Standard Rust release pattern: push `v*` tag, workflow fires. | Low | `on: push: tags: ["v*"]` in workflow YAML. |
| **macOS arm64 + x86_64 builds** | Primary user platform. Must ship both for compatibility. | Low | Matrix with `aarch64-apple-darwin` and `x86_64-apple-darwin` on `macos-latest`. |
| **Compressed archive (.tar.gz)** | Raw binary upload is not standard. Archive allows bundling README, LICENSE. | Low | `taiki-e/upload-rust-binary-action` handles this automatically. |
| **SHA256 checksum** | Security expectation for downloadable binaries. | Low | `checksum: sha256` option on upload action. |
| **`cargo test` gate** | Release should only proceed if tests pass. | Low | `cargo test` job that release job depends on. |
| **Version in binary** | `contentops --version` should output the release tag version. | Low | `clap` version from `Cargo.toml`; CI bumps Cargo.toml version or reads from tag. |

### Differentiators for CI/CD

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Universal macOS binary** | One download works on all Macs. `taiki-e/upload-rust-binary-action` supports `universal-apple-darwin` target natively. | Low | Add `universal-apple-darwin` to matrix. The action builds both arches and uses `lipo` to combine. |
| **Automated Cargo.toml version bump** | Tag `v1.2.0` automatically sets `version = "1.2.0"` in Cargo.toml. | Low | `cargo-set-version` or sed in workflow before build. |
| **`cargo clippy` CI gate** | Clippy warnings block release. Enforces code quality. | Low | Add `cargo clippy -- -D warnings` step before build. |

### Anti-Features for CI/CD

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Linux builds** | macOS-primary tool. `ffmpeg` linking and `whisper-cli` are brew-installed; Linux cross-compilation adds significant complexity for zero current users. | macOS only. Add Linux later if users appear. |
| **Windows builds** | Tool uses macOS-specific font paths (`/System/Library/Fonts/Supplemental/Impact.ttf`). Windows is not a supported target. | macOS only. |
| **Homebrew formula** | Homebrew tap maintenance is ongoing work. Requires formula update PRs on every release. | Direct binary download via GitHub Releases is sufficient for personal/small-team tool. |
| **Docker image** | This tool requires FFmpeg and whisper-cli as native binaries. Containerizing video processing adds volume-mount complexity with no benefit. | Native binary only. |
| **crates.io publish** | This is an application binary, not a library. Publishing to crates.io is for `cargo install` use cases; GitHub Releases with pre-built binaries is strictly better UX for CLI tools. | GitHub Releases only. |

### Workflow Structure (MEDIUM confidence — based on taiki-e/upload-rust-binary-action docs + ahmedjama.com pattern)

```yaml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test
      - run: cargo clippy -- -D warnings

  build-release:
    needs: test
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: universal-apple-darwin
            os: macos-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: contentops
          target: ${{ matrix.target }}
          checksum: sha256
          token: ${{ secrets.GITHUB_TOKEN }}
```

Artifact naming output: `contentops-aarch64-apple-darwin.tar.gz` + `.sha256` file per target.

---

## Feature Dependencies (Milestone 2)

```
[existing: cut, caption, overlay]
      |
      v
Doctor (no deps — reads PATH and disk)
      |
      v (validates prerequisites for...)
Audit (deps: cargo toolchain — always present if project builds)
      |
Pipeline (deps: cut + caption + overlay run() functions must accept args cleanly)
      |
      v
CI/CD (deps: project builds; tests pass; Cargo.toml has correct metadata)
```

Key ordering rationale:
- **Doctor first**: Confirms prerequisites work before building anything complex. Also surfaces the need to clean up error messaging (feeds audit context).
- **Audit second**: Cleanup pass before adding pipeline complexity. Dead code and duplicate spinners should be fixed before the pipeline command multiplies them.
- **Pipeline third**: Builds on clean, validated codebase. Requires cut/caption/overlay `run()` functions to accept `PipelineArgs`-derived inputs cleanly — audit may reveal refactoring needed first.
- **CI/CD last**: Automation of what already works. Gate on tests + clippy passing means audit cleanup must precede green CI.

---

## Prioritization Matrix (Milestone 2 features only)

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Doctor subcommand | HIGH — saves "why is caption broken?" debugging | LOW | P1 |
| Pipeline subcommand | HIGH — the main workflow bottleneck | MEDIUM | P1 |
| CI/CD / GitHub Releases | HIGH — makes tool installable by others | LOW | P1 |
| Codebase audit | MEDIUM — code health / maintenance | LOW | P2 |
| Audit --fix mode | LOW — convenience | LOW | P3 |
| Universal macOS binary | MEDIUM — best install UX | LOW | P2 |

---

## Sources

- [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action) — artifact naming, checksum, universal-apple-darwin support (HIGH confidence, official GitHub Action docs)
- [ahmedjama.com cross-platform Rust CI/CD 2025](https://ahmedjama.com/blog/2025/12/cross-platform-rust-pipeline-github-actions/) — workflow structure, target matrix (MEDIUM confidence)
- [houseabsolute/actions-rust-release](https://github.com/houseabsolute/actions-rust-release) — alternative release action (MEDIUM confidence)
- [flutter doctor UX pattern](https://www.dhiwise.com/post/flutter-doctor-command-a-vital-tool-for-developers) — pass/warn/fail output conventions (HIGH confidence)
- [npm doctor](https://docs.npmjs.com/cli/v7/commands/npm-doctor/) — check categorization pattern (HIGH confidence)
- [cargo-machete](https://crates.io/crates/cargo-machete) — unused dependency detection (MEDIUM confidence — crates.io listing, actively maintained)
- [cargo clippy --message-format=json](https://doc.rust-lang.org/clippy/usage.html) — structured lint output for audit parsing (HIGH confidence)
- Existing codebase audit: `src/commands/cut.rs`, `src/commands/caption.rs`, `src/commands/overlay.rs` — duplicate spinner code confirmed, error.rs `require_*` functions confirm existing prerequisite check pattern
