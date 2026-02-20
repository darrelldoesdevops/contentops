# Technology Stack

**Project:** contentops (Milestone 2 — audit tooling, doctor subcommand, pipeline subcommand, CI/CD)
**Researched:** 2026-02-20
**Scope:** NEW capabilities only. Existing stack (clap, serde, indicatif, owo-colors, anyhow, which, tempfile) is validated and unchanged.

---

## What This Milestone Adds

| Capability | Approach | New Cargo dependency? |
|------------|----------|-----------------------|
| Codebase audit (clippy, fmt) | Dev tooling, not runtime | No — cargo built-in |
| Security audit | `cargo-audit` CLI tool | No — installed separately |
| Dependency audit | `cargo-deny` CLI tool | No — installed separately |
| Prerequisite checking (doctor) | `which` (already in Cargo.toml) + `std::process::Command` | No |
| Pipeline subcommand | Internal code structure in clap | No |
| GitHub Actions CI | YAML workflow files | No |
| GitHub Actions Releases | `taiki-e/upload-rust-binary-action@v1` | No — GitHub Action |

**Zero new Cargo.toml dependencies for this milestone.** All additions are tooling, configuration files, and CI YAML.

---

## Codebase Audit Tooling

### Clippy Configuration

Use a `clippy.toml` at the project root. Clippy reads it automatically via `CARGO_MANIFEST_DIR`.

**Recommended `clippy.toml`:**
```toml
# Suppress false positives from the pedantic group
avoid-breaking-exported-api = false
msrv = "1.75.0"
```

**CI invocation:**
```bash
cargo clippy --all-targets -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions
```

Why `-D warnings`: Fails CI on any lint. Why `clippy::pedantic`: Catches common correctness issues beyond `clippy::all`. Why `-A clippy::module_name_repetitions`: This pedantic lint fires constantly in a codebase with modules named after their domain (e.g., `commands::cut::CutArgs`) and provides no value.

Do NOT enable `clippy::restriction` as a group — it contains mutually contradictory lints. Cherry-pick from it only if a specific lint is needed.

**Confidence: HIGH** — Verified against official Clippy documentation (doc.rust-lang.org/clippy).

### rustfmt

No configuration needed. `cargo fmt --check` as CI gate is sufficient. The default rustfmt style is stable and opinionated — don't fight it.

```bash
cargo fmt --check   # CI gate
cargo fmt           # Developer workflow
```

### cargo-audit (Security Advisories)

**Tool:** `cargo-audit` v0.22.1 (released 2026-02-04)
**Install:** `cargo install cargo-audit --locked`
**Source:** RustSec Advisory Database (rustsec.org)

This is a standalone tool, not a Cargo dependency. Run it against `Cargo.lock`. It checks for known CVEs in the dependency tree.

```bash
cargo audit                    # Local check
cargo audit --deny warnings    # CI — fail on warnings too
```

For CI, use `actions-rust-lang/audit@v1` (the actively maintained action — `actions-rs/audit-check` is unmaintained).

**Confidence: HIGH** — Verified via docs.rs, crates.io.

### cargo-deny (License + Duplicate Dependency Audit)

**Tool:** `cargo-deny` v0.18.5
**Install:** `cargo install cargo-deny --locked`

More comprehensive than cargo-audit: checks licenses, bans specific crates, detects duplicate dependency versions, and also checks advisories. For a personal tool, this is optional but worth the `deny.toml` setup cost because it catches license issues before they matter.

**Minimal `deny.toml`:**
```toml
[advisories]
# Covered by cargo-audit; keep in sync

[bans]
multiple-versions = "warn"
wildcards = "deny"

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "ISC",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "Unicode-3.0",
]
```

Initialize with: `cargo deny init`

**Confidence: MEDIUM** — cargo-deny is optional for a personal tool. Useful if the project is distributed. Add if the GitHub Actions CI pipeline has a slot for it.

---

## Doctor Subcommand (Runtime Prerequisite Checking)

The `doctor` subcommand checks that all external dependencies are installed and meet minimum version requirements. No new Cargo dependencies are needed — `which` (already in Cargo.toml at v8.0.0) handles PATH lookup, and `std::process::Command` handles version parsing.

### Pattern

```rust
// Check presence
use which::which;
which("ffmpeg").map_err(|_| anyhow!("ffmpeg not found in PATH — install with: brew install ffmpeg"))?;

// Check minimum version
let output = Command::new("ffmpeg").arg("-version").output()?;
let stdout = String::from_utf8_lossy(&output.stdout);
// Parse "ffmpeg version 7.1..." from first line
```

### Prerequisites to Check

| Tool | How to Check | Minimum Version | Why |
|------|-------------|-----------------|-----|
| `ffmpeg` | `ffmpeg -version` stdout | 6.0+ | concat filter, silencedetect, drawtext filter with timeline options |
| `ffprobe` | `ffprobe -version` stdout | 6.0+ | Ships with FFmpeg; same version |
| `whisper` (optional) | `which whisper` only | any | Project shells out to whisper-cli; version flexibility needed |
| `claude` (optional) | `which claude` only | any | Claude CLI; used for overlay --auto |

Mark optional tools as warnings, not errors. A user running only `cut` doesn't need whisper installed.

**Output format:** Use `owo-colors` (already in Cargo.toml) for colored check/cross/warning symbols. Pattern: `[check] ffmpeg 7.1 (ok)` or `[cross] ffmpeg not found`.

**Confidence: HIGH** — `which` crate v8.0.0 is already in Cargo.toml. Pattern is standard for CLI tools.

---

## Pipeline Subcommand

The `pipeline` subcommand chains `cut → caption → overlay` operations sequentially on a single input. This is pure internal code — no new dependencies.

### Design: Clap Args + Internal Dispatch

```rust
// cli.rs addition
#[derive(Subcommand)]
enum Commands {
    Cut(CutArgs),
    Caption(CaptionArgs),
    Overlay(OverlayArgs),
    Pipeline(PipelineArgs),  // new
    Doctor,                   // new
}

#[derive(Args)]
struct PipelineArgs {
    pub input: PathBuf,
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,
    // Inline the relevant subset of each subcommand's args
    // or use #[command(flatten)] on sub-arg structs
    #[arg(long)] pub model: Option<PathBuf>,   // for caption stage
    #[arg(long)] pub text: Option<String>,      // for overlay stage
    // ...skip stages if args are absent
}
```

### Stage Skipping

If `--model` is not provided, skip caption stage. If neither `--text` nor `--auto` is provided, skip overlay stage. This keeps pipeline flexible without requiring all stages.

### Intermediate Files

Use `TempFileRegistry` (already exists in `src/temp.rs`) for intermediate files between stages. Stage N output is stage N+1 input. On success, move final output to destination; on failure, temp registry cleans up.

**Confidence: HIGH** — All components exist. This is a composition problem, not a new technology problem.

---

## GitHub Actions CI/CD

### Workflow Structure

Two separate workflow files:

| File | Trigger | Purpose |
|------|---------|---------|
| `.github/workflows/ci.yml` | Push to any branch, PR | clippy, fmt, test, audit |
| `.github/workflows/release.yml` | Push tag `v[0-9]+.*` | Build binaries, upload to GitHub Releases |

### CI Workflow (`ci.yml`)

**Actions used:**
- `actions/checkout@v4` — standard
- `dtolnay/rust-toolchain@stable` — preferred over deprecated `actions-rs/toolchain`; supports `components: clippy,rustfmt`
- `Swatinem/rust-cache@v2` — caches `~/.cargo` and `target/`; keys off Cargo.lock hash; critical for CI speed
- `actions-rust-lang/audit@v1` — actively maintained cargo-audit wrapper; creates GitHub Issues on advisory hits

```yaml
name: CI
on:
  push:
    branches: ["main"]
  pull_request:
jobs:
  check:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy,rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions
      - run: cargo test

  audit:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      issues: write
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/audit@v1
```

**Why `macos-latest` for CI?** The project is macOS-primary. Running CI on Linux would miss macOS-specific path and linking behavior. The audit job uses ubuntu-latest because it only reads Cargo.lock — no compilation needed.

**Confidence: HIGH** — All actions verified against GitHub Marketplace and official documentation.

### Release Workflow (`release.yml`)

**Strategy:** Native macOS compilation (not cross-compilation via Docker/cross) because:
- GitHub provides both `macos-latest` (aarch64) and `macos-13` (x86_64) runners
- No cross-compilation toolchain complexity
- Produces native binaries with correct arch-specific optimizations

**Actions used:**
- `taiki-e/create-gh-release-action@v1` — creates GitHub Release from CHANGELOG.md entry
- `taiki-e/upload-rust-binary-action@v1` — builds `--release`, tars binary, uploads to Release

```yaml
name: Release
permissions:
  contents: write
on:
  push:
    tags:
      - v[0-9]+.*

jobs:
  create-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/create-gh-release-action@v1
        with:
          changelog: CHANGELOG.md
          token: ${{ secrets.GITHUB_TOKEN }}

  upload-assets:
    needs: create-release
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
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: contentops
          target: ${{ matrix.target }}
          tar: unix
          token: ${{ secrets.GITHUB_TOKEN }}
```

**Why not universal-apple-darwin?** The universal binary target builds on a single runner and requires cross-compilation from one arch to the other. Using separate runners per target is simpler and more debuggable.

**Why `macos-13` for x86_64?** GitHub changed `macos-latest` to aarch64 starting with macos-14. `macos-13` is the last Intel runner. Explicitly specifying both avoids arch ambiguity.

**Confidence: MEDIUM** — `macos-13` availability and runner availability is a GitHub infrastructure detail. Verify runner availability at implementation time; GitHub may have changed runner naming.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `actions-rust-lang/audit@v1` | `actions-rs/audit-check@v1` | `actions-rs` org is unmaintained as of 2023; `actions-rust-lang` is the community successor |
| `dtolnay/rust-toolchain@stable` | `actions-rs/toolchain@v1` | Same reason — `actions-rs` unmaintained |
| Native macOS runners per arch | `cross` + Docker cross-compilation | Cross requires Docker on macOS runners; native runners are simpler and faster for macOS targets |
| `cargo-audit` + `cargo-deny` | `cargo-audit` alone | `cargo-deny` catches license issues and duplicate dependencies that `cargo-audit` misses; small config cost |
| `which` (already installed) + `Command::output()` | `assert_cmd` for doctor checks | `assert_cmd` is a test helper, not a runtime tool. Doctor runs in production, not tests |

---

## What NOT to Add

| Avoid | Why |
|-------|-----|
| `tokio` or any async runtime | Pipeline stages are sequential. No async value. Adds significant compile time and complexity |
| `cargo-make` or `just` | Task runner for build scripts. The CI YAML and Cargo built-ins (`cargo clippy`, `cargo fmt`) are sufficient |
| `cross` crate | Docker-based cross-compilation. Unnecessary when GitHub provides native macOS runners for both arches |
| `actions-rs/*` actions | Unmaintained since 2023. Use `dtolnay/rust-toolchain` and `actions-rust-lang/audit` instead |
| `cargo-watch` in CI | Dev tool only. Not CI-relevant |
| New Cargo.toml runtime dependencies | This milestone is entirely tooling and code structure. Zero new runtime deps needed |

---

## Cargo.toml — No Changes Required

The existing Cargo.toml already has everything needed:

```toml
[dependencies]
which = "8.0"          # doctor prerequisite checks
anyhow = "1.0"         # error handling throughout
clap = { version = "4.5", features = ["derive"] }  # pipeline + doctor subcommands
owo-colors = "4.2"     # doctor output formatting
```

The `which` crate (v8.0.0) is already present and provides the PATH lookup needed for doctor checks. No new dependencies.

---

## Development Setup

```bash
# Install audit tooling (one-time, per developer machine)
cargo install cargo-audit --locked
cargo install cargo-deny --locked

# Initialize deny.toml (one-time, per project)
cargo deny init

# Audit workflow
cargo audit                  # Check advisories
cargo deny check             # Check licenses, bans, advisories
cargo clippy --all-targets -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions
cargo fmt --check

# Release (CI handles this; for local testing)
cargo build --release
```

---

## Sources

- [cargo-audit 0.22.1](https://docs.rs/crate/cargo-audit/latest) — docs.rs, verified 2026-02-20
- [cargo-deny 0.18.5](https://docs.rs/crate/cargo-deny/latest) — docs.rs, verified 2026-02-20
- [Clippy Configuration](https://doc.rust-lang.org/clippy/configuration.html) — official Rust docs, verified 2026-02-20
- [Clippy Usage/CI](https://doc.rust-lang.org/clippy/usage.html) — official Rust docs, verified 2026-02-20
- [which 8.0.0](https://docs.rs/which/latest/which/) — docs.rs, verified 2026-02-20
- [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action) — GitHub, verified 2026-02-20
- [taiki-e/create-gh-release-action](https://github.com/taiki-e/create-gh-release-action) — GitHub, verified 2026-02-20
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain) — GitHub, verified 2026-02-20
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) — GitHub, verified 2026-02-20
- [actions-rust-lang/audit](https://github.com/actions-rust-lang/audit) — GitHub, verified 2026-02-20
- [EmbarkStudios/cargo-deny](https://github.com/EmbarkStudios/cargo-deny) — GitHub, verified 2026-02-20
- [RustSec Advisory Database](https://rustsec.org/) — rustsec.org, verified 2026-02-20
