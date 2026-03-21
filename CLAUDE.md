# contentops

Rust CLI for TikTok/short-form video post-production. Removes silence (VAD-based), generates word-by-word captions, adds title overlays.

## Project Structure

| Path | Purpose |
|------|---------|
| `src/cli.rs` | CLI argument definitions (clap derive) |
| `src/vad.rs` | Voice Activity Detection — speech interval detection |
| `src/silence.rs` | FFmpeg filter chain builder for silence removal |
| `src/commands/cut.rs` | Silence removal command |
| `src/commands/pipeline.rs` | Full workflow orchestrator (cut -> caption -> overlay) |
| `src/commands/caption.rs` | Whisper transcription + caption burn |
| `src/commands/overlay.rs` | Title card overlay |
| `src/commands/normalize.rs` | Audio normalization |
| `src/ffmpeg.rs` | FFmpeg wrapper |
| `src/tiktok.rs` | TikTok format constants (1080x1920) |

## Before Committing

Always run before committing — the CI checks these:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

A pre-commit hook at `.githooks/pre-commit` runs fmt and clippy automatically. Enable it with:

```bash
git config core.hooksPath .githooks
```

## Release Process

Releases are fully automated via GitHub Actions (`.github/workflows/release.yml`):

1. Bump version in `Cargo.toml`
2. Commit and push
3. Tag with `vX.Y.Z` and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`
4. The workflow handles everything else:
   - Builds arm64 macOS, x86_64 macOS, Linux, Windows
   - Creates universal macOS binary
   - Publishes GitHub release with checksums
   - Triggers `update-tap.yml` in `darrelldoesdevops/homebrew-tap` to update the Homebrew formula

Do NOT manually build release binaries, create GitHub releases, or update the brew formula — the workflow does all of this.

## Homebrew

- Tap: `darrelldoesdevops/tap` (repo: `darrelldoesdevops/homebrew-tap`)
- Install: `brew install darrelldoesdevops/tap/contentops`
- Formula auto-updates via workflow dispatch from the release pipeline

## CI

CI (`.github/workflows/ci.yml`) runs on every push to main and PRs:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- Build on all 4 targets (arm64 macOS, x86_64 macOS, Linux, Windows)
- Tests (skipped for x86_64-apple-darwin cross-compile)
- Security audit (`cargo audit`, arm64 only)

## GitHub Account

This repo is under the `darrelldoesdevops` org — use the `DarrellTang` GitHub account.
