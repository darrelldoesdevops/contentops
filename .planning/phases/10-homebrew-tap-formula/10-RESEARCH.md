# Phase 10: Homebrew Tap + Formula - Research

**Researched:** 2026-02-20
**Domain:** Homebrew tap creation, Ruby formula DSL, prebuilt binary distribution
**Confidence:** HIGH

## Summary

Creating a Homebrew tap for prebuilt binaries is well-understood territory with clear patterns. The repo naming convention (`homebrew-tap`), formula file placement (`Formula/contentops.rb`), and architecture-conditional DSL (`on_macos do` / `on_arm do` / `on_intel do`) are all stable and documented. Third-party taps distributing prebuilt binaries are explicitly supported by Homebrew — unlike homebrew-core, which requires building from source.

The release workflow already produces `contentops-aarch64-apple-darwin` and `contentops-x86_64-apple-darwin` with `.sha256` sidecar files on GitHub Releases. The formula simply needs to point to those URLs with the correct checksums, install the binary, and declare `ffmpeg` as a dependency. The sentinel comment design for Phase 11 sed-patching is the only non-trivial design decision left to this phase.

**Primary recommendation:** Use nested `on_macos do` / `on_arm do` / `on_intel do` DSL blocks (not `Hardware::CPU.arm?` inline conditionals) for architecture selection. This is the idiomatic modern approach and is more readable than if/else chains inside the formula body.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Dependency handling
- FFmpeg declared as `depends_on "ffmpeg"` — auto-installs with the formula
- whisper-cli handled via caveats only (not in Homebrew, can't be a dependency)
- Add `depends_on :macos` guard to prevent Linuxbrew install attempts
- Claude CLI mentioned in caveats as optional for AI-powered caption generation

#### Post-install caveats
- Moderate verbosity — actionable but not a full guide
- Standard Homebrew caveats formatting (plain text, indented)
- Content: list prerequisites with install hints, mention whisper model is needed (without specifying which model), note Claude CLI as optional
- Include `contentops doctor` hint at the end so users can self-diagnose
- Don't include specific model recommendations or download commands

#### Tap repo presentation
- Repo: `darrelldoesdevops/homebrew-tap` — generic, reusable for future tools
- No README — formula only
- No LICENSE file
- GitHub repo description: "Homebrew tap" (short, generic)

#### Sentinel comment design
- Visually obvious markers that stand out in the formula (e.g., `# === AUTO-UPDATE: VERSION ===`)
- Include a brief 2-3 line header comment explaining sentinel system for future maintainers
- Style is Claude's discretion — pick whatever works best for sed-based patching (inline vs block)
- Which values to mark is Claude's discretion — determine what needs sentinels for reliable auto-update

### Claude's Discretion
- Sentinel comment style (inline vs block markers) — pick what's most sed-friendly
- Which specific values get sentinel markers (version, SHAs, URLs)
- Formula test block implementation details
- brew audit compliance specifics

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BREW-01 | User can run `brew install darrelltang/tap/contentops` to install correct architecture binary on ARM Mac | Architecture-conditional formula with `on_arm do` block pointing to `contentops-aarch64-apple-darwin` |
| BREW-02 | User can run `brew install darrelltang/tap/contentops` to install correct architecture binary on Intel Mac | Architecture-conditional formula with `on_intel do` block pointing to `contentops-x86_64-apple-darwin` |
| BREW-03 | Formula passes `brew audit` and `brew test contentops` | Third-party tap audit is less strict; requires `desc`, `homepage`, `license`, `url`, `sha256`, and a `test do` block |
| BREW-04 | Formula includes `caveats` block documenting whisper model and prerequisites | Standard `def caveats` heredoc pattern; plain text, indented |
</phase_requirements>

---

## Standard Stack

### Core

| Component | Version/Source | Purpose | Why Standard |
|-----------|---------------|---------|--------------|
| GitHub repo `homebrew-tap` | new repo | Tap hosting | Homebrew convention: repo named `homebrew-<name>` enables `brew tap user/name` shorthand |
| `Formula/contentops.rb` | Ruby DSL | Formula file | Conventional placement; first dir Homebrew checks |
| `on_macos do` / `on_arm do` / `on_intel do` | Homebrew DSL | Architecture conditionals | Idiomatic modern DSL; cleaner than `Hardware::CPU.arm?` if/else |
| `depends_on "ffmpeg"` | Homebrew DSL | Runtime dependency | Auto-installs ffmpeg; standard `depends_on` pattern |
| `depends_on :macos` | Homebrew DSL | Platform guard | Prevents Linuxbrew installs; shows "macOS is required" error on Linux |
| `def caveats` heredoc | Homebrew DSL | Post-install notes | Standard pattern for `brew info` display |
| `test do` block | Homebrew DSL | Formula verification | Required by `brew audit`; `brew test contentops` runs this |

### Release Asset Shape (Already Exists from v1.1)

The existing `release.yml` produces these assets on GitHub Releases per tag:

```
contentops-aarch64-apple-darwin        # ARM64 binary (raw, no archive)
contentops-aarch64-apple-darwin.sha256 # SHA256 sidecar file
contentops-x86_64-apple-darwin        # Intel binary (raw, no archive)
contentops-x86_64-apple-darwin.sha256 # SHA256 sidecar file
contentops-universal-apple-darwin     # Universal binary (not needed by formula)
```

**Critical:** The release assets are raw binaries, not tarballs. This matters for formula design — `bin.install` works differently depending on whether the URL points to a tarball or a bare binary. For bare binaries, Homebrew downloads the file directly and `bin.install` installs it by name. The formula `url` should point to the raw binary URL, not a `.tar.gz`.

**SHA256 source:** Read from the `.sha256` sidecar files. The content format is `<hash>  <filename>` — only the hex portion is used in the formula. Strip the filename when copying.

---

## Architecture Patterns

### Recommended Formula Structure

```
darrelldoesdevops/homebrew-tap/
└── Formula/
    └── contentops.rb
```

No other files required. No README, no LICENSE (per user decision).

### Pattern: Architecture-Conditional Prebuilt Binary Formula

```ruby
# Formula/contentops.rb
#
# Sentinel system: Values marked === AUTO-UPDATE: <FIELD> === are patched
# by the update-tap GitHub Actions workflow in darrelldoesdevops/contentops.
# Do not remove sentinel comments or the auto-update script will fail.

class Contentops < Formula
  desc "Automated video content operations: silence removal, captions, overlays"
  homepage "https://github.com/darrelldoesdevops/contentops"
  version "0.1.0" # === AUTO-UPDATE: VERSION ===

  license "MIT"

  depends_on :macos
  depends_on "ffmpeg"

  on_macos do
    on_arm do
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-aarch64-apple-darwin" # === AUTO-UPDATE: ARM-URL ===
      sha256 "PLACEHOLDER_ARM64_SHA256" # === AUTO-UPDATE: ARM-SHA256 ===
    end

    on_intel do
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-x86_64-apple-darwin" # === AUTO-UPDATE: INTEL-URL ===
      sha256 "PLACEHOLDER_X86_SHA256" # === AUTO-UPDATE: INTEL-SHA256 ===
    end
  end

  def install
    bin.install "contentops-aarch64-apple-darwin" => "contentops" if Hardware::CPU.arm?
    bin.install "contentops-x86_64-apple-darwin" => "contentops" if Hardware::CPU.intel?
  end

  def caveats
    <<~EOS
      contentops requires the following to be installed:

        FFmpeg (installed automatically as a dependency)
        whisper-cli: https://github.com/ggml-org/whisper.cpp
          A whisper model file is also required (see whisper.cpp docs).
        Claude CLI (optional, for AI-powered caption generation):
          https://claude.ai/download

      Run `contentops doctor` to check your setup.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/contentops --version")
  end
end
```

**Notes on `bin.install` for bare binaries:**

When the URL points to a bare binary (not a tarball), Homebrew downloads the file with the same name as the URL filename. The `bin.install` call renames it to the desired binary name:

```ruby
bin.install "contentops-aarch64-apple-darwin" => "contentops"
```

This places the binary at `$(brew --prefix)/bin/contentops`.

**Alternative approach for `install`:** Some formulas use a single `bin.install` with `Dir["contentops-*"]` but explicit naming is cleaner and more predictable for bare binary formulas.

### Pattern: Sentinel Comment Design for sed Patching

Inline sentinels on the same line as the value are the most reliable for sed-based patching because the regex scope is limited to one line.

**Recommended inline style:**

```ruby
version "0.1.0" # === AUTO-UPDATE: VERSION ===
url "https://...v0.1.0/contentops-aarch64-apple-darwin" # === AUTO-UPDATE: ARM-URL ===
sha256 "abc123..." # === AUTO-UPDATE: ARM-SHA256 ===
url "https://...v0.1.0/contentops-x86_64-apple-darwin" # === AUTO-UPDATE: INTEL-URL ===
sha256 "def456..." # === AUTO-UPDATE: INTEL-SHA256 ===
```

**sed pattern for each field:**

```bash
# macOS sed (requires -i '' for in-place)
sed -i '' "s|version \".*\" # === AUTO-UPDATE: VERSION ===|version \"${NEW_VERSION}\" # === AUTO-UPDATE: VERSION ===|" Formula/contentops.rb
sed -i '' "s|sha256 \".*\" # === AUTO-UPDATE: ARM-SHA256 ===|sha256 \"${ARM_SHA256}\" # === AUTO-UPDATE: ARM-SHA256 ===|" Formula/contentops.rb
sed -i '' "s|sha256 \".*\" # === AUTO-UPDATE: INTEL-SHA256 ===|sha256 \"${INTEL_SHA256}\" # === AUTO-UPDATE: INTEL-SHA256 ===|" Formula/contentops.rb
sed -i '' "s|url \".*\" # === AUTO-UPDATE: ARM-URL ===|url \"${ARM_URL}\" # === AUTO-UPDATE: ARM-URL ===|" Formula/contentops.rb
sed -i '' "s|url \".*\" # === AUTO-UPDATE: INTEL-URL ===|url \"${INTEL_URL}\" # === AUTO-UPDATE: INTEL-URL ===|" Formula/contentops.rb
```

**Why `|` as sed delimiter:** The URLs contain `/` which would break `s/old/new/` syntax. Using `|` as the delimiter avoids escaping slashes.

**Values that need sentinels (5 total):**
1. `version` — the version string
2. ARM `url` — download URL embedding the version tag
3. ARM `sha256` — checksum of the ARM binary
4. Intel `url` — download URL embedding the version tag
5. Intel `sha256` — checksum of the Intel binary

The `version` field and both URLs can be derived from the tag alone, but explicit sentinel on `version` makes it searchable and auditable. Both `sha256` values require the actual checksum from the release assets.

### Anti-Patterns to Avoid

- **`Hardware::CPU.arm?` at formula top level:** Fails at parse time outside `install`/`test` blocks. Use DSL blocks (`on_arm do`) for top-level architecture selection; use `Hardware::CPU.arm?` only inside `def install` or `test do`.
- **Using `stable do` wrapping:** Not needed for simple single-version formulas without `head`.
- **`depends_on macos: :sequoia`:** This specifies minimum macOS version, not platform restriction. `depends_on :macos` is the correct guard for "macOS only."
- **Tarballs vs bare binaries:** The existing release assets are bare binaries. If wrapping in a tarball for Phase 11 compatibility, the formula would change. Keep it consistent with what the release workflow produces.
- **Placing formula in repo root:** While technically supported, Homebrew recommends `Formula/` subdirectory.
- **Using `brew bump-formula-pr`:** Only works on homebrew-core; not usable for third-party taps.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Architecture detection at install time | Custom shell script | `on_arm do` / `on_intel do` DSL | Homebrew evaluates these correctly in all installation scenarios including `--force-bottle` |
| SHA256 checksums | Custom hash library | `shasum -a 256 <file>` or read from `.sha256` sidecar | Already generated by release.yml |
| Dependency management | Bundled ffmpeg | `depends_on "ffmpeg"` | Homebrew manages upgrades, conflicts, and keg linking |
| Formula testing | Ad-hoc verification | `test do` block with `assert_match` | `brew test` runs in a clean sandbox; only this mechanism is reliable |

**Key insight:** Homebrew's DSL handles all edge cases in binary selection (Rosetta 2 detection, cross-compilation scenarios). Hand-rolling CPU detection with shell scripts inside `install` is fragile and has broken for users running Rosetta 2.

---

## Common Pitfalls

### Pitfall 1: Hardware::CPU at Formula Top Level

**What goes wrong:** Using `if Hardware::CPU.arm?` to set `url` and `sha256` at the formula class body level raises a parse error. Homebrew evaluates the formula class body at parse time when both architectures may not be present.

**Why it happens:** `Hardware::CPU` methods are runtime, not DSL declarations. The formula class body is parsed before installation context is available.

**How to avoid:** Use `on_arm do` / `on_intel do` blocks for top-level `url` and `sha256`. Use `Hardware::CPU.arm?` only inside `def install` or `test do` where it runs at install time.

**Warning signs:** `Error: undefined method 'arm?' for Hardware::CPU:Module` during formula parse.

### Pitfall 2: sed Breaking on URLs with Slashes

**What goes wrong:** `sed -i '' "s/url \"old\"/url \"new\"/"` fails when the URL contains `/` because sed interprets those as delimiters.

**Why it happens:** Default `s/old/new/` uses `/` as delimiter.

**How to avoid:** Use `|` as the delimiter: `s|url ".*"|url "new"|`. This is the standard workaround.

**Warning signs:** `sed: 1: "s/url..."`: unterminated substitute pattern.

### Pitfall 3: macOS vs Linux sed Differences

**What goes wrong:** `sed -i "s/.../.../"` (Linux syntax) fails on macOS with "invalid command code".

**Why it happens:** macOS `sed` requires an explicit suffix argument for `-i`; on macOS the empty string `''` is required: `sed -i '' "s/.../.../"`.

**How to avoid:** In the tap's GitHub Actions workflow (which runs on ubuntu), use Linux sed syntax. In local development scripts (which run on macOS), use `sed -i ''`. The update-tap workflow in Phase 11 runs on ubuntu, so Linux syntax is correct there.

**Warning signs:** `sed: 1: "Formula/contentops.rb"`: invalid command code.

### Pitfall 4: Brew Audit Stricter Than Expected for Taps

**What goes wrong:** `brew audit Formula/contentops.rb` passes but `brew audit --strict contentops` fails with style violations.

**Why it happens:** The formula must be tapped first before `brew audit` can find it by name. Path-based audit (`brew audit Formula/contentops.rb`) was deprecated; name-based audit requires the tap to be registered.

**How to avoid:** Run `brew tap darrelldoesdevops/tap path/to/local/homebrew-tap` to tap locally, then run `brew audit contentops`. For CI, the formula must be committed and the tap registered before auditing.

**Warning signs:** `Error: No available formula with the name "contentops"` when running audit by name.

### Pitfall 5: `bin.install` Filename Mismatch

**What goes wrong:** `bin.install "contentops"` fails because the downloaded file is named `contentops-aarch64-apple-darwin`, not `contentops`.

**Why it happens:** Homebrew names the downloaded file after the URL filename when downloading bare binaries. The URL ends in `contentops-aarch64-apple-darwin`, so that's what's on disk after download.

**How to avoid:** Use the rename form: `bin.install "contentops-aarch64-apple-darwin" => "contentops"`. Wrap in architecture check inside `def install`.

### Pitfall 6: Formula Name vs Tap Name Confusion

**What goes wrong:** User runs `brew install darrelldoesdevops/homebrew-tap/contentops` (using the repo name) instead of `brew install darrelldoesdevops/tap/contentops` (using the tap shorthand).

**Why it happens:** Homebrew strips the `homebrew-` prefix from the repo name to form the tap name. Repo `homebrew-tap` → tap name `tap`.

**How to avoid:** Document the install command as `brew install darrelldoesdevops/tap/contentops`. Both forms work but the short form is conventional.

---

## Code Examples

### Complete Formula (Production-Ready Template)

```ruby
# Formula/contentops.rb
#
# Sentinel system: Lines ending with === AUTO-UPDATE: <FIELD> === are patched
# automatically by the update-tap workflow. Do not remove sentinel comments.
# See darrelldoesdevops/contentops/.github/workflows/release.yml for the patcher.

class Contentops < Formula
  desc "Automated video content operations: silence removal, captions, overlays"
  homepage "https://github.com/darrelldoesdevops/contentops"
  version "0.1.0" # === AUTO-UPDATE: VERSION ===

  license "MIT"

  depends_on :macos
  depends_on "ffmpeg"

  on_macos do
    on_arm do
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-aarch64-apple-darwin" # === AUTO-UPDATE: ARM-URL ===
      sha256 "REPLACE_WITH_ARM64_SHA256" # === AUTO-UPDATE: ARM-SHA256 ===
    end

    on_intel do
      url "https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-x86_64-apple-darwin" # === AUTO-UPDATE: INTEL-URL ===
      sha256 "REPLACE_WITH_X86_SHA256" # === AUTO-UPDATE: INTEL-SHA256 ===
    end
  end

  def install
    if Hardware::CPU.arm?
      bin.install "contentops-aarch64-apple-darwin" => "contentops"
    else
      bin.install "contentops-x86_64-apple-darwin" => "contentops"
    end
  end

  def caveats
    <<~EOS
      contentops requires the following:

        whisper-cli (not a Homebrew package):
          https://github.com/ggml-org/whisper.cpp
          A whisper model file is also required — see the whisper.cpp docs.

        Claude CLI (optional, for AI-powered caption generation):
          https://claude.ai/download

      FFmpeg is installed automatically as a dependency.

      Run `contentops doctor` to check your environment.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/contentops --version")
  end
end
```

### Getting SHA256 from Release Assets

```bash
# Option 1: Read from the .sha256 sidecar file (already computed by release.yml)
ARM_SHA=$(curl -sL https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-aarch64-apple-darwin.sha256 | awk '{print $1}')

# Option 2: Compute directly from binary download
ARM_SHA=$(curl -sL https://github.com/darrelldoesdevops/contentops/releases/download/v0.1.0/contentops-aarch64-apple-darwin | shasum -a 256 | awk '{print $1}')
```

The `.sha256` sidecar format is `<hash>  <filename>`. The `awk '{print $1}'` extracts just the hex hash.

### Local Tap Registration for Testing

```bash
# Register local tap for brew audit / brew install testing
brew tap darrelldoesdevops/tap /path/to/homebrew-tap

# Audit the formula
brew audit contentops
brew audit --strict contentops

# Test the formula
brew test contentops

# Install from local tap
brew install darrelldoesdevops/tap/contentops

# Uninstall and remove tap when done
brew uninstall contentops
brew untap darrelldoesdevops/tap
```

### Verify Correct Architecture After Install

```bash
# Should show arm64 on Apple Silicon, x86_64 on Intel
file $(brew --prefix)/bin/contentops
# → .../contentops: Mach-O 64-bit executable arm64

# Or use --version to verify it runs at all
contentops --version
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|-----------------|--------------|--------|
| `Hardware::CPU.arm?` inline for `url`/`sha256` | `on_arm do` / `on_intel do` DSL blocks | ~2021 (Apple Silicon era) | DSL blocks are evaluated safely at parse time; inline conditionals break at formula body level |
| `depends_on :macos` as version guard | `depends_on :macos` for platform, `depends_on macos: :version` for version | Ongoing | These are distinct — `:macos` means "macOS only"; `macos: :sonoma` means "minimum version" |
| Bintray bottle hosting | GitHub Releases | 2021 (Bintray shutdown) | All custom bottle hosting now uses GitHub Releases |
| `brew audit [path]` | `brew audit [name]` (tap must be registered) | ~2023 | Path-based audit was removed; must tap locally before auditing |
| `mislav/bump-homebrew-formula-action` | Custom `workflow_dispatch` + sed | N/A for this project | The action cannot handle `on_arm do` / `on_intel do` structure with dual sha256; ruled out in prior decisions |

---

## Open Questions

1. **License field value**
   - What we know: Formula requires a `license` field to pass `brew audit`. contentops is a personal project.
   - What's unclear: Actual license of contentops — Cargo.toml doesn't specify one.
   - Recommendation: Add `license "MIT"` to the formula (and optionally to Cargo.toml). If no license is chosen, use `license :cannot_represent` as a fallback but this may trigger audit warnings.

2. **Bare binary vs tarball for formula URL**
   - What we know: Current release.yml uploads bare binaries. Homebrew handles bare binary URLs correctly with `bin.install "filename" => "new-name"`.
   - What's unclear: Whether `brew audit --strict` objects to non-archive URLs for formulas (it does not for casks, which routinely use binary URLs).
   - Recommendation: Use the bare binary URL as-is. If audit complains, wrap binaries in a tarball in the release workflow (but this would require Phase 11 updates too, so avoid unless necessary).

3. **`brew audit` without `--strict` vs with `--strict`**
   - What we know: Phase success criteria says "passes with no errors or warnings." The `--strict` flag enables additional RuboCop-style checks.
   - What's unclear: Whether any of those strict checks apply specifically to prebuilt binary formulas.
   - Recommendation: Target passing both `brew audit contentops` and `brew audit --strict contentops`. The formula template above should satisfy both.

---

## Sources

### Primary (HIGH confidence)
- [Homebrew How-to-Create-and-Maintain-a-Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap) — tap naming, directory structure, formula placement
- [Homebrew Taps documentation](https://docs.brew.sh/Taps) — shorthand install command, naming conventions, `homebrew-` prefix requirement
- [Hardware::CPU Ruby API](https://docs.brew.sh/rubydoc/Hardware/CPU.html) — `arm?`, `intel?`, `arm64?`, `in_rosetta2?`, `physical_cpu_arm64?` methods verified
- [Formula Cookbook](https://docs.brew.sh/Formula-Cookbook) — `depends_on :macos`, `on_macos do`, `on_arm do`, `on_intel do`, `test do`, `caveats`
- [Homebrew Bottles documentation](https://docs.brew.sh/Bottles) — `root_url`, architecture-tagged `sha256`
- [CockroachDB homebrew-tap](https://github.com/cockroachdb/homebrew-tap/blob/master/Formula/cockroach.rb) — real-world `on_intel do` / `on_arm do` with prebuilt binaries
- contentops `release.yml` — verified exact asset names: `contentops-aarch64-apple-darwin`, `contentops-x86_64-apple-darwin`, `.sha256` sidecars

### Secondary (MEDIUM confidence)
- [GitHub Discussion: Shipping binaries with homebrew](https://github.com/orgs/Homebrew/discussions/4439) — confirms homebrew-core rejects prebuilt binaries; third-party taps are the correct path
- [GitHub Discussion: Dynamic URL for binary packages](https://github.com/orgs/Homebrew/discussions/1069) — `Hardware::CPU.arm?` pattern inside `stable do` resource blocks
- [GitHub Discussion: macOS-only formula syntax](https://github.com/orgs/Homebrew/discussions/4914) — `depends_on :macos` vs `depends_on macos: :version`
- [BuiltFast: Automating Homebrew Tap Updates](https://builtfast.dev/blog/automating-homebrew-tap-updates-with-github-actions/) — sed command patterns for version/sha256 update automation
- [brew/audit.rb source](https://github.com/Homebrew/brew/blob/master/Library/Homebrew/dev-cmd/audit.rb) — confirmed tap audits are lighter than core audits

### Tertiary (LOW confidence — flag for validation)
- Assertion that `brew audit [path]` was removed in ~2023: Confirmed by community discussion but could not find official changelog entry. Validate by running `brew audit Formula/contentops.rb` vs `brew audit contentops` locally.
- Whether `brew audit --strict` objects to bare binary (non-archive) `url` values: Not directly confirmed by official docs. Validate during formula testing.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — tap naming, formula DSL, dependency syntax all verified against official docs and real-world examples
- Architecture: HIGH — `on_arm do` / `on_intel do` pattern verified in CockroachDB's production tap; `Hardware::CPU` API verified from Ruby API docs
- Sentinel design: MEDIUM — sed pattern approach is well-documented; specific sentinel comment style is a design decision not covered by existing sources
- Pitfalls: HIGH — most pitfalls verified from official source code or official GitHub discussions

**Research date:** 2026-02-20
**Valid until:** Stable — Homebrew formula DSL changes slowly; valid for 90+ days
