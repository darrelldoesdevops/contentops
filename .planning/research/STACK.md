# Stack Research

**Domain:** Homebrew personal tap + auto-update + CLI README
**Researched:** 2026-02-20
**Confidence:** HIGH — all formula patterns verified against locally installed tap formulas; workflow patterns verified against official action READMEs

---

## What This Milestone Adds

| Capability | Approach | New file? |
|------------|----------|-----------|
| Homebrew personal tap | New GitHub repo `darrelldoesdevops/homebrew-tap` | Yes — new repo |
| Formula for pre-built binaries | Ruby `.rb` formula with `Hardware::CPU` conditionals | Yes — `Formula/contentops.rb` |
| Auto-update formula on release | GitHub Actions workflow in `contentops` repo dispatching to tap repo | Yes — workflow in both repos |
| Comprehensive README | Markdown in `contentops` repo root | Yes — `README.md` |

---

## Homebrew Tap Repository

### Naming and Setup

| Requirement | Value | Why |
|-------------|-------|-----|
| Repo name | `homebrew-tap` | Homebrew convention: prefix `homebrew-` is mandatory for short-form tap command |
| Install command | `brew tap darrelldoesdevops/tap` | Expands to `github.com/darrelldoesdevops/homebrew-tap` |
| Formula location | `Formula/contentops.rb` | Standard `Formula/` subdirectory; first path Homebrew checks |
| Visibility | Public | Required for unauthenticated `brew tap` to work |

Create via: `brew tap-new darrelldoesdevops/tap` (scaffolds directory with default GitHub Actions — delete the default bottle-building workflows; they apply to source builds, not pre-built binaries).

**Confidence: HIGH** — Verified against Homebrew official tap documentation.

---

## Homebrew Formula Syntax

### Pattern: Bare Binary with Architecture Conditionals

The release workflow produces bare binaries (not tarballs): `contentops-aarch64-apple-darwin`, `contentops-x86_64-apple-darwin`. Homebrew supports direct binary URLs. The install block renames the arch-suffixed binary to the canonical `contentops` name.

**Verified pattern** from `loft-sh/tap/vcluster` (locally installed tap, macOS-only tool with same binary structure):

```ruby
# typed: false
# frozen_string_literal: true

class Contentops < Formula
  desc "CLI for video post-production automation"
  homepage "https://github.com/darrelldoesdevops/contentops"
  version "1.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v1.1.0/contentops-aarch64-apple-darwin"
      sha256 "ec58e2d8106c84de25ae20641a060cbf85a91bb7cab4f0f60f27577f8333f0ba"

      def install
        bin.install "contentops-aarch64-apple-darwin" => "contentops"
      end
    end
    if Hardware::CPU.intel?
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v1.1.0/contentops-x86_64-apple-darwin"
      sha256 "<X86_SHA256>"

      def install
        bin.install "contentops-x86_64-apple-darwin" => "contentops"
      end
    end
  end

  test do
    assert_match "contentops", shell_output("#{bin}/contentops --help")
  end
end
```

**Key syntax decisions:**

| Decision | Rationale |
|----------|-----------|
| `Hardware::CPU.arm?` / `.intel?` inside `on_macos do` | Verified pattern from GoReleaser-generated formulas and `loft-sh/tap`. The `on_arm do` / `on_intel do` block syntax also exists but `Hardware::CPU` inside `on_macos` is more common for tools shipping bare binaries from GitHub Releases |
| `def install` inside each conditional | Required when using bare binary URLs with different filenames per arch; the rename `=> "contentops"` maps the arch-named file to the installed binary name |
| No `on_linux do` block | contentops is macOS-only by design; omitting Linux blocks is correct |
| `version` explicit field | Required when URL doesn't contain a tag path (bare binary URL lacks version in path); Homebrew cannot infer version from the URL |
| `# frozen_string_literal: true` | Homebrew linting convention; `brew style` will warn without it |

**Confidence: HIGH** — Pattern verified from locally installed `loft-sh/tap/vcluster` which uses identical structure (GoReleaser-generated, bare binary, same arch conditional approach).

---

## Auto-Update: Formula Patching on Release

### Tool Decision: Custom Shell Script Over `mislav/bump-homebrew-formula-action`

`mislav/bump-homebrew-formula-action@v3` (latest: v3.6) **explicitly cannot** update formulas with `if...else` or `Hardware::CPU` conditionals. From its README:

> Cannot bump formulae which use Ruby `if...else` conditions to determine alternate download locations at runtime

Since contentops requires per-architecture URLs and SHA256 values, this action is ruled out.

**Recommended approach:** Custom script in the tap repo, triggered via `workflow_dispatch` from the main release workflow.

### Architecture: Two-Repo Pattern

```
contentops repo (release.yml)
    │
    │  gh workflow run update-formula.yml \
    │    -f version=$VERSION \
    │    -R darrelldoesdevops/homebrew-tap
    ▼
homebrew-tap repo (update-formula.yml)
    │
    ├── Download contentops-aarch64-apple-darwin.sha256 from release
    ├── Download contentops-x86_64-apple-darwin.sha256 from release
    ├── Parse hash (awk '{print $1}') from "hash  filename" format
    ├── sed replace version, ARM sha256, Intel sha256 in formula
    └── git commit + push
```

### Workflow: contentops release.yml addition

Add this step to the existing `release` job in `.github/workflows/release.yml`, after "Create GitHub Release":

```yaml
- name: Update Homebrew formula
  run: |
    gh workflow run update-formula.yml \
      -f version=${GITHUB_REF#refs/tags/v} \
      -R darrelldoesdevops/homebrew-tap
  env:
    GITHUB_TOKEN: ${{ secrets.TAP_UPDATE_TOKEN }}
```

`TAP_UPDATE_TOKEN` must be a classic PAT with `repo` and `workflow` scopes (cross-repo workflow dispatch requires `workflow` scope; `GITHUB_TOKEN` is scoped to the current repo only).

### Workflow: homebrew-tap update-formula.yml

```yaml
name: Update Formula

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version (without v prefix, e.g. 1.2.0)'
        required: true
        type: string

jobs:
  update:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Compute SHA256 values
        id: sha
        run: |
          ARM_SHA=$(curl -sL \
            "https://github.com/darrelldoesdevops/contentops/releases/download/v${{ inputs.version }}/contentops-aarch64-apple-darwin.sha256" \
            | awk '{print $1}')
          X86_SHA=$(curl -sL \
            "https://github.com/darrelldoesdevops/contentops/releases/download/v${{ inputs.version }}/contentops-x86_64-apple-darwin.sha256" \
            | awk '{print $1}')
          echo "arm_sha=$ARM_SHA" >> $GITHUB_OUTPUT
          echo "x86_sha=$X86_SHA" >> $GITHUB_OUTPUT

      - name: Patch formula
        env:
          VERSION: ${{ inputs.version }}
          ARM_SHA: ${{ steps.sha.outputs.arm_sha }}
          X86_SHA: ${{ steps.sha.outputs.x86_sha }}
        run: |
          FORMULA="Formula/contentops.rb"
          sed -i "s|version \".*\"|version \"${VERSION}\"|" "$FORMULA"
          sed -i "s|/v[0-9.]*/contentops-aarch64-apple-darwin\"|/v${VERSION}/contentops-aarch64-apple-darwin\"|" "$FORMULA"
          sed -i "s|/v[0-9.]*/contentops-x86_64-apple-darwin\"|/v${VERSION}/contentops-x86_64-apple-darwin\"|" "$FORMULA"
          # Replace SHA256 values - requires stable ordering in formula file
          # Use line-number-anchored sed or maintain unique sentinel comments
          python3 - <<'PYEOF'
          import re, os
          formula = open('Formula/contentops.rb').read()
          arm_sha = os.environ['ARM_SHA']
          x86_sha = os.environ['X86_SHA']
          # ARM block comes first in formula - replace first sha256 occurrence
          formula = re.sub(
              r'(CPU\.arm\?.*?sha256 ")([a-f0-9]{64})(")',
              lambda m: m.group(1) + arm_sha + m.group(3),
              formula, count=1, flags=re.DOTALL
          )
          formula = re.sub(
              r'(CPU\.intel\?.*?sha256 ")([a-f0-9]{64})(")',
              lambda m: m.group(1) + x86_sha + m.group(3),
              formula, count=1, flags=re.DOTALL
          )
          open('Formula/contentops.rb', 'w').write(formula)
          PYEOF

      - name: Commit and push
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/contentops.rb
          git commit -m "contentops ${{ inputs.version }}"
          git push
```

**SHA256 file format:** The release workflow generates files with content `hash  filename` (standard `shasum -a 256` output). The `awk '{print $1}'` extracts just the hex hash. Verified against actual v1.1.0 release assets.

**Why Python for SHA256 replacement:** `sed` regex for matching a 64-char hex string inside a multiline conditional block is fragile and platform-specific (macOS `sed -i ''` vs GNU `sed -i`). Since the tap's update workflow runs on `ubuntu-latest`, GNU sed is available, but the multiline match requirement makes Python cleaner and more robust. Python 3 is always available on GitHub Actions runners.

**Why not re-download and compute SHA256 from the binary:** The `.sha256` files are already present in the GitHub Release (generated by the release workflow). Re-downloading the binary to re-compute the hash introduces a redundant 50-100MB download per run. Trust the pre-computed checksums.

**Confidence: HIGH for workflow structure.** MEDIUM for the Python sed approach — the regex pattern depends on formula layout staying stable. An alternative is to use unique sentinel comments (`# ARM-SHA`, `# INTEL-SHA`) on the sha256 lines and sed on those, which is simpler and more brittle-proof.

### Alternative: Simpler Sentinel Comment Approach

If the Python regex feels over-engineered, add sentinel comments to the formula and use simple grep+sed:

```ruby
      sha256 "ec58e2d8106c84de25ae20641a060cbf85a91bb7cab4f0f60f27577f8333f0ba" # ARM-SHA
      ...
      sha256 "abc123..." # INTEL-SHA
```

Then in the update script:
```bash
sed -i "s|sha256 \"[a-f0-9]*\" # ARM-SHA|sha256 \"${ARM_SHA}\" # ARM-SHA|" Formula/contentops.rb
sed -i "s|sha256 \"[a-f0-9]*\" # INTEL-SHA|sha256 \"${INTEL_SHA}\" # INTEL-SHA|" Formula/contentops.rb
```

This is the recommended approach — simpler, readable, no Python dependency.

**Confidence: HIGH** — Pattern used by multiple real-world taps documented in builtfast.dev article (2025) and josh.fail (2023).

---

## README Structure

### Recommended Sections for a CLI Tool (personal, macOS, video production)

| Section | Content | Notes |
|---------|---------|-------|
| Title + one-liner | Tool name, what it does in one sentence | No badges needed for personal tool |
| Prerequisites | ffmpeg, whisper-cli, minimum versions | Critical UX: users hit this before install works |
| Installation | `brew tap` + `brew install` as primary path | Direct download as fallback |
| Subcommands reference | Table: command, purpose, key flags | Scannable over narrative |
| Usage examples | One concrete example per subcommand | Show actual commands with real-ish filenames |
| `contentops doctor` | Call out as the "start here if broken" command | Reduce support burden |
| Configuration/output | Where files go, naming conventions | |

### Section Order Rationale

Prerequisites before Installation because Homebrew installs the binary but brew doesn't install ffmpeg/whisper. Users who install first, read docs second, will hit runtime errors from `doctor`. Frontloading prerequisites prevents confusion.

### What NOT to Include

| Avoid | Why |
|-------|-----|
| Contributing section | Personal tool, not open source project |
| Badges (CI status, crates.io, etc.) | Adds noise, breaks if repo is private |
| Architecture/internals documentation | Wrong audience for README; belongs in `.planning/` |
| Changelog in README | Already generated by GitHub Releases; duplication |
| License badge / full license text in README | Single line "MIT License" is sufficient |

**Confidence: MEDIUM** — Derived from general CLI documentation best practices; no domain-specific source for video production CLI tools specifically.

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Formula architecture handling | `Hardware::CPU.arm?` inside `on_macos do` | `on_arm do` / `on_intel do` top-level blocks | `on_arm`/`on_intel` blocks work but `Hardware::CPU` inside `on_macos` is the pattern used by GoReleaser-generated formulas and matches the verified `loft-sh/tap/vcluster` example; either works |
| Auto-update action | Custom script + workflow_dispatch | `mislav/bump-homebrew-formula-action@v3` | mislav action explicitly cannot handle `if...else` or `Hardware::CPU` conditionals; documented limitation in its own README |
| Auto-update action | Custom script | `dawidd6/action-homebrew-bump-formula` | Wraps `brew bump-formula-pr`; designed for homebrew-core PRs, not direct-push personal taps; more complexity than needed |
| SHA256 source | Download pre-computed `.sha256` from release | Re-compute by downloading binary | Binary assets are 10-30MB each; pre-computed files already exist in the release; re-downloading wastes bandwidth and adds latency |
| Binary format | Bare binary (current) | Tarball `.tar.gz` | Current release workflow produces bare binaries; switching to tarballs would simplify URL version-embedding but requires release workflow change; not worth it |
| Token for cross-repo dispatch | Classic PAT with `repo`+`workflow` scopes | Fine-grained PAT | Fine-grained PATs can grant repo-specific write access, but `workflow` scope (for triggering workflows) is only on classic PATs as of 2026-02 |

---

## Token Setup (One-Time)

```
1. github.com/settings/tokens → Generate new token (classic)
2. Scopes: repo, workflow
3. Name: contentops-tap-update
4. Add to contentops repo: Settings → Secrets → TAP_UPDATE_TOKEN
```

---

## Installation Commands (for README)

```bash
# Add tap (one-time)
brew tap darrelldoesdevops/tap

# Install
brew install contentops

# Upgrade
brew upgrade contentops

# Direct install without tapping first
brew install darrelldoesdevops/tap/contentops
```

---

## Sources

- `brew cat loft-sh/tap/vcluster` — locally installed tap formula; verified `Hardware::CPU.arm?` / `.intel?` inside `on_macos do` with `bin.install "name" => "binary"` pattern; GoReleaser-generated; HIGH confidence
- [Homebrew How-to-Create-and-Maintain-a-Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap) — official docs; naming convention, directory structure; HIGH confidence
- [Homebrew Taps documentation](https://docs.brew.sh/Taps) — `brew tap user/repo` convention, `homebrew-` prefix requirement; HIGH confidence
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook) — `on_arm`, `on_intel`, `on_macos` block syntax, `bin.install` method; HIGH confidence
- [mislav/bump-homebrew-formula-action README](https://raw.githubusercontent.com/mislav/bump-homebrew-formula-action/main/README.md) — v3.6 (latest); documented limitation re: `if...else` conditionals; HIGH confidence
- [builtfast.dev: Automating Homebrew Tap Updates with GitHub Actions](https://builtfast.dev/blog/automating-homebrew-tap-updates-with-github-actions/) (2025) — two-repo pattern, `gh workflow run`, bash script with sed; MEDIUM confidence
- [josh.fail: Automate updating custom Homebrew formulae](https://josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions/) — `workflow_dispatch` tap update approach; MEDIUM confidence
- `gh release download v1.1.0 --pattern "*.sha256"` — verified SHA256 file format is `hash  filename` (shasum -a 256 output); awk '{print $1}' extracts hash; HIGH confidence
- `gh release view v1.1.0 --json assets` — verified asset names: `contentops-aarch64-apple-darwin`, `contentops-x86_64-apple-darwin`, `contentops-universal-apple-darwin` (bare binaries, not tarballs); HIGH confidence

---
*Stack research for: Homebrew tap + auto-update + CLI README (Milestone 3)*
*Researched: 2026-02-20*
