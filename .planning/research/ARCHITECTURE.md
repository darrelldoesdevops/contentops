# Architecture Research

**Domain:** Rust CLI video post-production — v1.1 integration audit + v1.2 Homebrew tap / README
**Researched:** 2026-02-20
**Confidence:** HIGH (direct codebase audit + verified Homebrew/GitHub Actions sources)

## Current Architecture (v1.0 — as built)

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                   main.rs (39 lines)                        │
│  Cli::parse() → TempFileRegistry::new() → match Commands    │
├──────────────┬───────────────────────────────────────────────┤
│  cli.rs      │  Commands enum, *Args structs (clap derive)  │
│              │  Cut | Caption | Overlay                      │
├──────────────┴───────────────────────────────────────────────┤
│  commands/   — Self-contained run() functions                │
│  cut.rs      — normalize → silence-detect → concat filter   │
│  caption.rs  — audio extract → whisper-cli → SRT/JSON/ASS   │
│  overlay.rs  — (claude auto-title) → drawtext filter        │
│  normalize.rs — loudnorm 2-pass (util, used only by cut)    │
├─────────────────────────────────────────────────────────────┤
│  Shared infrastructure                                       │
│  ffmpeg.rs   — FFmpeg/ffprobe wrappers, progress bars       │
│  silence.rs  — silence parsing, speech segment math         │
│  temp.rs     — TempFileRegistry, make_temp_file, Ctrl-C     │
│  error.rs    — AppError enum, require_ffmpeg/whisper        │
├─────────────────────────────────────────────────────────────┤
│  External tools (shelled via std::process::Command)         │
│  ffmpeg / ffprobe     whisper-cli     claude (optional)     │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities (actual, post-audit)

| Component | Responsibility | LOC | Notes |
|-----------|----------------|-----|-------|
| `main.rs` | Parse CLI, dispatch, top-level error handler | 39 | Thin; all logic in commands |
| `cli.rs` | Clap types: `Cli`, `Commands`, all `*Args` structs | ~109 | Single source of truth for CLI shape |
| `commands/cut.rs` | Silence removal: normalize → detect → concat | ~231 | Owns `pub fn derive_output_path` (reused by overlay) |
| `commands/caption.rs` | Audio extract → whisper → SRT/JSON/ASS generate | ~622 | Most complex; `derive_caption_output` is private |
| `commands/overlay.rs` | Build drawtext filter, optional Claude auto-title | ~296 | Imports `derive_output_path` from cut |
| `commands/normalize.rs` | Two-pass EBU R128 loudnorm to temp file | ~192 | Called only by cut; returns PathBuf |
| `ffmpeg.rs` | `run_ffmpeg*`, `probe_duration*`, `run_silencedetect` | ~202 | All FFmpeg subprocess wrappers |
| `silence.rs` | Parse silencedetect stderr, compute speech segments | ~? | Pure math, no I/O |
| `temp.rs` | `TempFileRegistry`, `make_temp_file`, Ctrl-C handler | 56 | Registry is `Arc<Mutex<Vec<PathBuf>>>` |
| `error.rs` | `AppError` thiserror enum, `require_ffmpeg/whisper`, `format_error` | 134 | Prerequisite checks live here |

## v1.1 New Components

### Source Tree Changes

```
src/
├── main.rs                  MODIFIED — +2 match arms (Doctor, Pipeline)
├── cli.rs                   MODIFIED — +2 Commands variants + DoctorArgs + PipelineArgs
├── commands/
│   ├── mod.rs               MODIFIED — pub mod doctor; pub mod pipeline;
│   ├── cut.rs               NO CHANGE
│   ├── caption.rs           MODIFIED — make derive_caption_output pub
│   ├── overlay.rs           MODIFIED — add require_claude() call when args.auto.is_some()
│   ├── normalize.rs         NO CHANGE
│   ├── doctor.rs            NEW — ~80 lines
│   └── pipeline.rs          NEW — ~70 lines
├── error.rs                 MODIFIED — +ClaudeNotFound variant, +require_claude()
├── ffmpeg.rs                NO CHANGE
├── silence.rs               NO CHANGE
└── temp.rs                  NO CHANGE

.github/
└── workflows/
    └── release.yml          NEW — build matrix + GitHub Release upload
```

## Integration Pattern: Doctor Subcommand

### Where It Hooks In

`error.rs` already has `require_ffmpeg()` and `require_whisper()` using `which::which()`. Doctor is an additive user-facing diagnostic wrapper around the same mechanic — it does not replace the inline guards in each command's `run()`.

### Dispatch Addition (main.rs)

```rust
Some(Commands::Doctor(args)) => commands::doctor::run(args, cli.verbose),
```

Doctor takes no `registry` argument — it creates no temp files.

### DoctorArgs (cli.rs)

```rust
#[derive(Args)]
pub struct DoctorArgs {}  // no fields — contentops doctor takes no arguments
```

### Implementation Pattern (doctor.rs)

```rust
pub fn run(_args: DoctorArgs, _verbose: bool) -> anyhow::Result<()> {
    let mut all_ok = true;

    all_ok &= check_required("ffmpeg",      "brew install ffmpeg");
    all_ok &= check_required("ffprobe",     "brew install ffmpeg");
    all_ok &= check_required("whisper-cli", "brew install whisper-cli");
    check_optional("claude", "brew install claude");

    if all_ok { Ok(()) } else { std::process::exit(1) }
}

fn check_required(tool: &str, hint: &str) -> bool {
    match which::which(tool) {
        Ok(path) => { eprintln!("  ok  {}  ({})", tool, path.display()); true }
        Err(_)   => { eprintln!("  MISSING {}  hint: {}", tool, hint); false }
    }
}
```

Version reporting: call `Command::new(tool).arg("--version").output()` and print the first line of stdout/stderr. ffmpeg prints version to stderr; whisper-cli prints to stdout — handle both.

### Auto-Prerequisite Checks in Normal Commands

**Current state:** `require_ffmpeg()` and `require_whisper()` already run at the top of each command's `run()`. This IS the auto-prerequisite check pattern.

**Gap to close:** Claude CLI is not checked. `overlay --auto` shells out to `claude` but fails with a generic `StageIo` error if the binary is missing.

**Fix:** Add to `error.rs`:

```rust
#[error("claude not found on PATH\n  hint: brew install claude")]
ClaudeNotFound,

pub fn require_claude() -> Result<PathBuf, AppError> {
    which::which("claude").map_err(|_| AppError::ClaudeNotFound)
}
```

Add to `overlay::run()`:

```rust
if args.auto.is_some() {
    require_claude()?;
}
```

**Do not add a centralized pre-dispatch hook in main.rs.** Each command knows its own dependencies. Checking whisper before `cut` is misleading; checking claude before `caption` is wrong. Keep checks at the call site.

## Integration Pattern: Pipeline Subcommand

### Data Flow

```
contentops pipeline input.mp4 --model ggml-base.bin
    ↓
pipeline::run()
    ├── require_ffmpeg() + require_whisper() upfront
    │
    ├── Step 1: cut::run(CutArgs { input, output: Some(cut_path), ... })
    │       produces: input_cut.mp4
    │
    ├── Step 2: caption::run(CaptionArgs { input: cut_path, model, lang, burn: false, ... })
    │       produces: input_cut_captioned.srt
    │                 input_cut_captioned.json  ← needed by overlay --auto
    │
    └── Step 3: overlay::run(OverlayArgs { input: cut_path, auto: Some(json_path), ... })
            produces: input_cut_overlay.mp4

Final outputs:
  input_cut_overlay.mp4       — ready to post
  input_cut_captioned.srt     — sidecar for external editing
  input_cut_captioned.json    — word-level timestamps (intermediate, keep for reference)
```

Note: Step 3 takes `cut_path` (silence-removed) as input, not the caption step's video output. This is correct — captions are a sidecar. The overlay goes on the clean cut, title auto-generated from the JSON transcript.

### How Pipeline Reuses Existing Logic

Pipeline calls the existing `run()` functions directly as Rust function calls. No subprocess shelling. Shared `TempFileRegistry` spans all three steps.

```rust
// src/commands/pipeline.rs
use crate::cli::{CaptionArgs, CutArgs, OverlayArgs, PipelineArgs};
use crate::commands::{caption, cut, overlay};
use crate::error::{require_ffmpeg, require_whisper};
use crate::temp::TempFileRegistry;

pub fn run(args: PipelineArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    require_ffmpeg()?;
    require_whisper()?;

    let cut_path = cut::derive_output_path(&args.input, "cut");
    let json_path = caption::derive_caption_output(&args.input_cut(), "captioned", "json");
    //                                              ^^^^ see path derivation note below

    cut::run(CutArgs {
        input: args.input.clone(),
        output: Some(cut_path.clone()),
        dry_run: false,
        breaths: args.breaths,
    }, verbose, registry)?;

    caption::run(CaptionArgs {
        input: cut_path.clone(),
        output: None,
        model: args.model.clone(),
        lang: args.lang.clone(),
        burn: false,
    }, verbose, registry)?;

    overlay::run(OverlayArgs {
        input: cut_path.clone(),
        text: None,
        auto: Some(json_path),
        output: None,
        font: args.font.clone(),
        font_size: 44,
        color: "black".to_string(),
        position: "top".to_string(),
        start: 0.3,
        duration: 3.5,
    }, verbose, registry)?;

    Ok(())
}
```

### Path Derivation Dependency (Required Change)

`derive_output_path` in `cut.rs` is already `pub`. Pipeline can call it directly.

`derive_caption_output` in `caption.rs` is currently `fn` (private). Pipeline needs to derive the JSON path that caption will produce. **Required change:** make it `pub fn derive_caption_output` in caption.rs.

The JSON path is: given `input_cut.mp4`, caption produces `input_cut_captioned.json` (suffix="captioned", ext="json"). Pipeline needs this path to pass as `args.auto` to overlay.

### PipelineArgs (cli.rs addition)

```rust
#[derive(Args)]
pub struct PipelineArgs {
    /// Input video file
    pub input: PathBuf,

    /// Path to whisper model file
    #[arg(long)]
    pub model: PathBuf,

    /// Language code for transcription
    #[arg(long, default_value = "en")]
    pub lang: String,

    /// Also detect and remove breaths (forwarded to cut)
    #[arg(long)]
    pub breaths: bool,

    /// Path to .ttf font file (forwarded to overlay)
    #[arg(long)]
    pub font: Option<PathBuf>,
}
```

Overlay positioning, timing, font size, and color are hardcoded in pipeline to the same defaults as the individual command. Users who need control use individual commands.

### OverlayArgs Construction

`OverlayArgs` does not derive `Default` (clap required-field validation conflicts with Default for `text`). Pipeline constructs it with all fields explicit — this is fine since pipeline has a fixed, opinionated configuration.

## Integration Pattern: GitHub Actions CI/CD

### Scope

New file only: `.github/workflows/release.yml`. Zero src/ changes.

### Trigger

```yaml
on:
  push:
    tags:
      - 'v*'
```

### Build Matrix

Two targets cover macOS deployment:

| Target | Runner | Notes |
|--------|--------|-------|
| `aarch64-apple-darwin` | `macos-latest` | Apple Silicon (M-series), default since ~2024 |
| `x86_64-apple-darwin` | `macos-13` | Intel; macos-13 is last Intel runner in GHA |

Cross-compilation (building arm64 on x86 or vice versa) is possible on macOS with `rustup target add` but same-arch builds are simpler and faster. Use the two-runner matrix.

### Workflow Structure

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-13
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}
      - run: mv target/${{ matrix.target }}/release/contentops contentops-${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: contentops-${{ matrix.target }}
          path: contentops-${{ matrix.target }}

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: contentops-*
          merge-multiple: true
      - uses: softprops/action-gh-release@v2
        with:
          files: contentops-*
```

### Caching

`Swatinem/rust-cache@v2` caches the Cargo registry and target directory. On a ~2,400 LOC project: cold build ~3-4 min, cached build ~30s. Include it.

### Binary Naming Convention

`contentops-aarch64-apple-darwin` and `contentops-x86_64-apple-darwin` — users download the right one for their machine. No universal binary (lipo) needed; the two files are sufficient.

---

## v1.2 New Components: Homebrew Tap + README

### Two-Repository System Overview

```
┌──────────────────────────────────────────┐
│  darrelltang/contentops  (existing repo) │
│                                          │
│  .github/workflows/release.yml           │
│    ↓  on: push tags v*                   │
│    ↓  jobs: build-arm64, build-x86_64    │
│    ↓  job: release → softprops/release   │
│    ↓                                     │
│    ↓  NEW: job: update-tap               │
│       gh workflow run bump-formula.yml   │
│       --repo darrelltang/homebrew-contentops
│       --field version=$TAG               │
└───────────────────┬──────────────────────┘
                    │  cross-repo workflow_dispatch
                    │  (PAT_TOKEN secret required)
                    ▼
┌──────────────────────────────────────────┐
│  darrelltang/homebrew-contentops  (NEW)  │
│                                          │
│  Formula/                                │
│    contentops.rb       ← updated here    │
│  .github/workflows/                      │
│    bump-formula.yml    ← triggered above │
│  .github/scripts/                        │
│    update-formula      ← bash script     │
└──────────────────────────────────────────┘
```

### New Repository: `darrelltang/homebrew-contentops`

**Naming:** The `homebrew-` prefix is required for the short `brew tap darrelltang/contentops` syntax. Without it, users must use `brew tap darrelltang/homebrew-contentops`.

**Required structure:**

```
homebrew-contentops/
├── Formula/
│   └── contentops.rb          # Homebrew formula
├── .github/
│   ├── workflows/
│   │   └── bump-formula.yml   # workflow_dispatch target
│   └── scripts/
│       └── update-formula     # bash: download + sha256 + sed
└── README.md                  # optional but expected
```

### Formula File Structure (`Formula/contentops.rb`)

The formula distributes architecture-specific pre-built binaries using `on_arm`/`on_intel` blocks with separate URLs and SHA256 values per arch:

```ruby
class Contentops < Formula
  desc "Video post-production CLI: silence removal, captions, overlays"
  homepage "https://github.com/darrelltang/contentops"
  version "0.1.0"

  on_arm do
    url "https://github.com/darrelltang/contentops/releases/download/v0.1.0/contentops-aarch64-apple-darwin"
    sha256 "PLACEHOLDER_ARM64_SHA256"
  end

  on_intel do
    url "https://github.com/darrelltang/contentops/releases/download/v0.1.0/contentops-x86_64-apple-darwin"
    sha256 "PLACEHOLDER_X86_SHA256"
  end

  def install
    bin.install "contentops-aarch64-apple-darwin" => "contentops" if Hardware::CPU.arm?
    bin.install "contentops-x86_64-apple-darwin" => "contentops" if Hardware::CPU.intel?
  end

  test do
    system "#{bin}/contentops", "--version"
  end
end
```

**Note:** Homebrew bottles (the standard binary package format) require Homebrew infrastructure to build and sign. For a personal tap with pre-built Rust binaries, the `on_arm`/`on_intel` approach with direct release URLs is the correct pattern — bottles are for Homebrew/core, not personal taps.

### Update Script (`.github/scripts/update-formula`)

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION="${VERSION:?VERSION not set}"
REPO="darrelltang/contentops"

ARM_URL="https://github.com/${REPO}/releases/download/v${VERSION}/contentops-aarch64-apple-darwin"
X86_URL="https://github.com/${REPO}/releases/download/v${VERSION}/contentops-x86_64-apple-darwin"

ARM_SHA=$(curl -fsSL "$ARM_URL" | shasum -a 256 | awk '{print $1}')
X86_SHA=$(curl -fsSL "$X86_URL" | shasum -a 256 | awk '{print $1}')

sed -i '' "s|releases/download/v.*/contentops-aarch64|releases/download/v${VERSION}/contentops-aarch64|g" Formula/contentops.rb
sed -i '' "s|releases/download/v.*/contentops-x86_64|releases/download/v${VERSION}/contentops-x86_64|g" Formula/contentops.rb
sed -i '' "s/version \".*\"/version \"${VERSION}\"/" Formula/contentops.rb
# Replace SHA lines — order matters: arm first, intel second
# (use line-specific replacement if sed -i '' pattern conflicts)
```

**Note on SHA replacement:** `sed` replacing two SHA256 values in the same file is fragile when both look identical in pattern. A more robust approach: use Python's `re.sub` or write the formula file from a template rather than patching in place.

### Tap Workflow (`.github/workflows/bump-formula.yml`)

```yaml
name: Bump Formula

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Version (without v prefix, e.g. 0.2.0)"
        required: true

permissions:
  contents: write

jobs:
  bump:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Update formula
        env:
          VERSION: ${{ github.event.inputs.version }}
        run: bash .github/scripts/update-formula

      - name: Commit and push
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/contentops.rb
          git commit -m "contentops ${{ github.event.inputs.version }}"
          git push
```

### Modified: `release.yml` — New `update-tap` Job

The existing `release` job in `contentops` repo completes successfully, then `update-tap` fires:

```yaml
  update-tap:
    name: Update Homebrew Tap
    needs: release          # runs after GitHub Release is published
    runs-on: ubuntu-latest
    steps:
      - name: Trigger tap formula update
        env:
          GH_TOKEN: ${{ secrets.TAP_GITHUB_TOKEN }}
        run: |
          VERSION="${GITHUB_REF_NAME#v}"   # strip leading 'v' from tag
          gh workflow run bump-formula.yml \
            --repo darrelltang/homebrew-contentops \
            --field version="${VERSION}"
```

**Why `needs: release` not `needs: [build-arm64, build-x86_64]`:** The tap script downloads binaries from the GitHub Release. The Release must exist and assets must be uploaded before the tap update downloads them to compute SHA256. The `release` job creates the release via `softprops/action-gh-release`; tap update must wait for it.

### Authentication: `TAP_GITHUB_TOKEN` Secret

| Requirement | Detail |
|-------------|--------|
| Type | Fine-grained PAT (or classic PAT) |
| Owner | `darrelltang` account |
| Repos | `darrelltang/homebrew-contentops` — read+write |
| Permissions | Actions: read+write, Contents: read+write, Workflows: read+write |
| Where stored | `contentops` repo → Settings → Secrets → `TAP_GITHUB_TOKEN` |

**Do not use `GITHUB_TOKEN` for cross-repo triggers.** `GITHUB_TOKEN` is scoped to the current repo only. Cross-repo `workflow_dispatch` requires a PAT.

### Data Flow: Tag Push to `brew install`

```
1. Developer: git push tag v0.2.0
      ↓
2. release.yml triggers (on: push tags v*)
      ↓
3. build-arm64 job → artifact: contentops-aarch64-apple-darwin
   build-x86_64 job → artifact: contentops-x86_64-apple-darwin
      ↓
4. release job:
   - Downloads both artifacts
   - Creates universal binary (lipo)
   - Generates .sha256 files
   - Uploads 6 files to GitHub Release via softprops
      ↓
5. update-tap job (needs: release):
   - Strips 'v' prefix: "v0.2.0" → "0.2.0"
   - gh workflow run bump-formula.yml --repo darrelltang/homebrew-contentops
      ↓
6. bump-formula.yml in homebrew-contentops triggers:
   - Checks out homebrew-contentops
   - update-formula script:
       - curl downloads contentops-aarch64-apple-darwin from GitHub Release
       - shasum → ARM64_SHA
       - curl downloads contentops-x86_64-apple-darwin from GitHub Release
       - shasum → X86_SHA
       - sed patches version + both SHA256 values in Formula/contentops.rb
   - git commit + push → Formula/contentops.rb updated
      ↓
7. User: brew tap darrelltang/contentops
          brew install contentops
   - Homebrew reads Formula/contentops.rb
   - Downloads arch-specific binary from GitHub Release
   - Verifies SHA256
   - Installs to /opt/homebrew/bin/contentops (arm64)
             or /usr/local/bin/contentops (intel)
```

### Component Inventory: New vs Modified

| Component | Status | Location | Notes |
|-----------|--------|----------|-------|
| `homebrew-contentops` repo | NEW | github.com/darrelltang/homebrew-contentops | Separate GitHub repo |
| `Formula/contentops.rb` | NEW | homebrew-contentops | on_arm/on_intel binary formula |
| `.github/workflows/bump-formula.yml` | NEW | homebrew-contentops | workflow_dispatch receiver |
| `.github/scripts/update-formula` | NEW | homebrew-contentops | sha256 + sed script |
| `release.yml` — `update-tap` job | MODIFIED | contentops/.github/workflows/ | Appended after existing `release` job |
| `TAP_GITHUB_TOKEN` secret | NEW | contentops repo settings | Fine-grained PAT |
| `README.md` | NEW | contentops repo root | Install + usage docs |

**Zero src/ changes required.** This milestone is pure distribution and documentation.

### README Architecture

The README is a single file at the contentops repo root. It is the only documentation file needed — no docs/ directory, no wiki.

```
README.md
├── Install (brew tap + brew install + verify)
├── Prerequisites (ffmpeg, whisper-cli, claude)
├── Commands reference (table: command | purpose | key flags)
├── Usage examples (contentops cut, caption, overlay, pipeline)
└── Uninstall
```

Target: ~60-80 lines. Scannable. No prose narrative. Commands are copy-pasteable.

## Architectural Patterns (Existing — Confirmed by Audit)

### Pattern 1: Self-Contained Command Modules

Each command module imports its own `*Args` type, calls `require_*()` guards at entry, manages its own temp files via the shared registry, and returns `anyhow::Result<()>`. No shared state between commands at runtime.

**Doctor and Pipeline follow this same shape exactly.** Doctor skips the registry arg. Pipeline adds multi-command coordination.

### Pattern 2: In-Process Command Chaining (Pipeline)

Pipeline calls `commands::cut::run()`, `commands::caption::run()`, `commands::overlay::run()` directly as Rust function calls. The shared `TempFileRegistry` handles Ctrl-C cleanup across all three steps automatically.

**Do not shell out to `contentops cut` as a subprocess.** This would lose the shared registry (Ctrl-C in a subprocess doesn't clean the parent's temp files), lose typed errors, and require the binary on PATH during development.

### Pattern 3: Prerequisite Checks as Typed Errors

`require_ffmpeg()` / `require_whisper()` return `AppError::FfmpegNotFound` / `AppError::WhisperNotFound`. The top-level error handler in main.rs catches `AppError` variants and formats them with install hints. New tools follow the same pattern.

Checks are duplicated across commands that share deps. This is acceptable — `which::which` is a filesystem stat (cheap), and it keeps each command self-documenting.

### Pattern 4: Cross-Repo Release Chain (NEW — v1.2)

The release workflow is a linear dependency chain: build → release → tap update. Each step has an explicit `needs:` dependency. The tap update cannot run until GitHub Release assets exist (the tap script downloads them to compute SHA256). The PAT scoped to the tap repo is the authentication boundary between the two repositories.

## Anti-Patterns

### Anti-Pattern 1: Pipeline as a Subprocess Chain

**What people do:** `Command::new("contentops").arg("cut").arg(input).output()`

**Why it's wrong:** Loses TempFileRegistry. Error messages go through shell and lose typed AppError. Requires the binary on PATH during development builds.

**Do this instead:** Call `commands::cut::run()` directly with constructed CutArgs.

### Anti-Pattern 2: Centralized Pre-Dispatch Prerequisite Check

**What people do:** Before the match arm in main.rs, check all tools for every command.

**Why it's wrong:** Whisper check fires for `contentops cut` (which never uses whisper). Claude check fires for `contentops caption`. Users see confusing "tool not found" errors for tools their command doesn't need.

**Do this instead:** Keep `require_*()` at the top of each command's `run()` — already the pattern.

### Anti-Pattern 3: Over-Exposing Flags on Pipeline

**What people do:** Re-expose every flag from cut, caption, and overlay on PipelineArgs.

**Why it's wrong:** 15+ fields on PipelineArgs, duplicating three commands' worth of CLI surface. Users needing fine control should use individual commands.

**Do this instead:** Expose only `input`, `model`, `lang`, `breaths`, `font`. Hardcode overlay defaults. Pipeline is the happy path.

### Anti-Pattern 4: Tap Update Before Release Assets Exist (NEW)

**What people do:** Trigger `bump-formula.yml` in parallel with or immediately after the `release` job starts, without waiting for `softprops/action-gh-release` to complete.

**Why it's wrong:** The update-formula script downloads binaries from the GitHub Release to compute SHA256. If triggered too early, the Release may not exist or assets may still be uploading. The script fails with a 404 or gets a wrong SHA.

**Do this instead:** `update-tap` job must declare `needs: release`. The release job does not complete until `softprops/action-gh-release` finishes uploading all 6 files.

### Anti-Pattern 5: Using GITHUB_TOKEN for Cross-Repo Dispatch (NEW)

**What people do:** Set `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}` in the `update-tap` job.

**Why it's wrong:** `GITHUB_TOKEN` is automatically scoped to the current repository. It cannot trigger workflows in `darrelltang/homebrew-contentops`.

**Do this instead:** Create a fine-grained PAT with actions + contents + workflows write permissions on the tap repo. Store it as `TAP_GITHUB_TOKEN` in the contentops repo secrets.

### Anti-Pattern 6: Patching SHA256 with Ambiguous sed (NEW)

**What people do:** `sed -i '' "s/OLDSHA/NEWSHA/"` when both ARM and Intel SHA placeholders share the same pattern.

**Why it's wrong:** Two SHA256 values in the formula can't be distinguished by sed if the regex matches both. First substitution may replace the wrong line.

**Do this instead:** Use line-number-aware replacement, or template the formula file (write it from scratch with interpolated values) rather than patching. Python's `re.sub` with named capture groups is more reliable than multi-line sed.

## Data Flow Summary

### Normal Commands

```
User → clap parse → *Args struct → command::run(*Args, verbose, &registry) → anyhow::Result<()>
                                        ↓
                               require_ffmpeg/whisper()
                                        ↓
                               std::process::Command (ffmpeg, whisper-cli, claude)
                                        ↓
                               output file on disk + eprintln progress
```

### Pipeline Command

```
User → clap parse → PipelineArgs → pipeline::run() → {
    cut::run(CutArgs) → input_cut.mp4
    caption::run(CaptionArgs { input: input_cut.mp4 }) → input_cut_captioned.{srt,json}
    overlay::run(OverlayArgs { auto: input_cut_captioned.json }) → input_cut_overlay.mp4
}
```

### Doctor Command

```
User → clap parse → DoctorArgs → doctor::run() → {
    which("ffmpeg")      → ok/MISSING
    which("ffprobe")     → ok/MISSING
    which("whisper-cli") → ok/MISSING
    which("claude")      → ok/MISSING (optional)
    exit 0 or exit 1
}
```

### Release + Tap Update (NEW — v1.2)

```
git push v0.2.0 tag
    ↓
release.yml
    ├── build-arm64 job  ──────────────────────────┐
    ├── build-x86_64 job ──────────────────────────┤
    │                                              ↓
    ├── release job (needs: build-arm64, build-x86_64)
    │     softprops/action-gh-release uploads 6 files
    │                                              ↓
    └── update-tap job (needs: release)
          gh workflow run bump-formula.yml \
            --repo darrelltang/homebrew-contentops \
            --field version=0.2.0
              ↓
          homebrew-contentops/bump-formula.yml
            curl arm64 binary → shasum → ARM_SHA
            curl x86_64 binary → shasum → X86_SHA
            patch Formula/contentops.rb
            git push
              ↓
          brew install contentops  [user]
            reads updated formula → downloads binary → verifies SHA → installs
```

## Build Order Recommendation (v1.1 + v1.2)

| Order | Task | Depends On | Rationale |
|-------|------|------------|-----------|
| 1 | `require_claude()` + overlay guard | `error.rs` existing pattern | One-liner additions; closes the missing check gap immediately |
| 2 | `pub fn derive_caption_output` | `caption.rs` | One-line visibility change; prerequisite for pipeline |
| 3 | `DoctorArgs` + `Commands::Doctor` in cli.rs | Nothing new | Isolated; no dependencies on other v1.1 features |
| 4 | `commands/doctor.rs` | Step 3 + error.rs which/require pattern | Standalone, self-contained, validates check infrastructure |
| 5 | `PipelineArgs` + `Commands::Pipeline` in cli.rs | Steps 1–2 complete | Can design args only after derivation API is confirmed |
| 6 | `commands/pipeline.rs` | Steps 1–5 complete | Integrates all three existing commands |
| 7 | `.github/workflows/release.yml` | None — independent | Can be written any time; test with a manual tag push |
| 8 | Create `darrelltang/homebrew-contentops` repo | Step 7 complete and tag pushed | Tap repo needs a real release to test SHA download |
| 9 | `Formula/contentops.rb` + `bump-formula.yml` + `update-formula` script | Step 8 | Core tap infrastructure |
| 10 | `TAP_GITHUB_TOKEN` secret + `update-tap` job in `release.yml` | Steps 8–9 | Wires cross-repo trigger; test with another tag push |
| 11 | `README.md` in contentops repo | Steps 8–10 (brew install must work) | README install section must reference working tap |

## Sources

- Direct codebase audit: all 12 source files in `/Users/darrelltang/darrelldoesdevops/contentops/src/`
- Homebrew tap naming and structure: https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- Homebrew formula on_arm/on_intel DSL: https://docs.brew.sh/Formula-Cookbook
- Cross-repo workflow_dispatch pattern: https://builtfast.dev/blog/automating-homebrew-tap-updates-with-github-actions/
- PAT permissions for cross-repo triggers: https://github.com/orgs/community/discussions/58868
- Formula update automation pattern: https://josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions/
- Binary formula structure: https://jonathanruiz.dev/deploy-app-homebrew-using-github-actions/

---
*Architecture research for: contentops v1.1 — doctor, pipeline, CI/CD; v1.2 — Homebrew tap, README*
*Researched: 2026-02-20*
