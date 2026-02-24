# Phase 16: Build & CI Verification - Research

**Researched:** 2026-02-24
**Domain:** Rust cross-platform CI, ONNX Runtime caching, GitHub Actions
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Full CI pass required: build + tests + clippy + tree check — ensure nothing regresses from adding the dependency
- Single PR: add voice_activity_detector dependency and CI cache changes together in one PR
- Any regression caused by adding the dependency gets fixed in this phase, not deferred
- Modify the existing CI workflow — no separate workflow file
- Update both CI (PR/push) and release workflows so both benefit from cached ORT
- All 4 platform builds run in parallel via matrix strategy (existing pattern)
- All 4 platforms must pass for CI to be green — any platform failure = red
- Exception: if Windows ORT cache path is problematic, ship with 3 platforms passing and file a GitHub issue for Windows specifically
- Fix any test/build regressions in this phase before moving on

### Claude's Discretion
- ORT cache strategy: whether to include ORT path in existing Cargo cache or use a separate cache step
- ort version conflict checking: whether to add a cargo tree CI step or verify manually
- ORT download retry behavior in CI
- Cache key design and invalidation strategy

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CI-01 | `voice_activity_detector` 0.2.1 compiles on macOS ARM64, macOS Intel, Linux x86_64, Windows x86_64 | Dependency analysis confirms exact version pins and default feature set; platform cache paths verified from source |
| CI-02 | ONNX Runtime binary cached in GitHub Actions to avoid repeated downloads | ORT cache directory paths verified from source code; `ORT_CACHE_DIR` env var confirmed; Swatinem `cache-directories` param confirmed as the integration mechanism |
</phase_requirements>

## Summary

`voice_activity_detector` 0.2.1 depends on `ort =2.0.0-rc.10` with the `ndarray` feature and `ort-sys =2.0.0-rc.10`. The `ort` crate's default feature set includes `download-binaries` and `copy-dylibs`, which means ONNX Runtime is downloaded at build time on first compile and cached in a platform-specific directory. The Silero VAD V5 ONNX model (1.8 MB) is bundled via `include_bytes!` in the crate source — no model download needed.

The primary CI challenge is two-fold: (1) the existing CI matrix uses only 3 runners but needs 4 platform targets, and (2) the ORT binary cache directory must be explicitly added to `Swatinem/rust-cache`'s `cache-directories` parameter so it persists between runs. The ORT cache path is well-documented from source code: it is controlled by `ORT_CACHE_DIR` env var, with platform defaults of `~/.cache/ort.pyke.io` (Linux), `~/Library/Caches/ort.pyke.io` (macOS), and `%LOCALAPPDATA%\ort.pyke.io` (Windows).

For macOS Intel, the current `macos-latest` runner is ARM64. The canonical label for an Intel runner is `macos-15-intel`, available until August 2027. The alternative is cross-compiling `x86_64-apple-darwin` on the `macos-latest` ARM64 runner — which is exactly what the release workflow already does and is the cleaner approach for CI builds (avoids a second, more expensive Intel runner).

**Primary recommendation:** Add `voice_activity_detector = "0.2.1"` to Cargo.toml, extend `Swatinem/rust-cache` with `cache-directories` pointing to the ORT cache path per platform (using `$HOME` prefix for cross-platform compatibility), and add macOS x86_64 cross-compilation as a fourth matrix entry on the existing ARM64 runner.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `voice_activity_detector` | 0.2.1 | Silero VAD V5 inference; bundles model via `include_bytes!` | Locked decision |
| `ort` | =2.0.0-rc.10 | ONNX Runtime Rust bindings (transitive, pinned by VAD) | Pinned by voice_activity_detector; must not be added independently |
| `ort-sys` | =2.0.0-rc.10 | Low-level ORT sys bindings (transitive) | Same pin rationale |
| `Swatinem/rust-cache@v2` | v2 (current) | Cargo registry + target dir cache | Already in use; extended via `cache-directories` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `actions/cache@v4` | v4 | Alternative for ORT-only cache step | Only if separate cache step is preferred over `cache-directories` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `cache-directories` in rust-cache | Separate `actions/cache` step | Separate step gives more explicit key control; `cache-directories` is simpler since rust-cache is already in use |
| ARM64 runner cross-compile for x86_64 mac | `macos-15-intel` runner | Intel runner is a real native check but costs more minutes; cross-compile is sufficient for dependency build verification and mirrors the release workflow pattern |

## Architecture Patterns

### ORT Binary Cache Directory Locations (verified from source)

From `ort-sys/src/internal/dirs.rs` in `v2.0.0-rc.10`:

```
Linux:   $XDG_CACHE_HOME/ort.pyke.io  (or ~/.cache/ort.pyke.io)
macOS:   ~/Library/Caches/ort.pyke.io
Windows: %LOCALAPPDATA%\ort.pyke.io   (FOLDERID_LocalAppData via SHGetKnownFolderPath)
```

Within that root, binaries land at: `{cache_root}/dfbin/{target_triple}/{hash}/onnxruntime/`

The `ORT_CACHE_DIR` environment variable overrides the platform default. Setting it to a known path (e.g., `~/.ort-cache`) simplifies cross-platform CI configuration.

### Pattern 1: Extend rust-cache with ORT directories

**What:** Add `cache-directories` to the existing `Swatinem/rust-cache@v2` step to include the ORT binary cache path.

**When to use:** When the ORT cache directory is on the same filesystem as the Cargo cache and a single cache key is acceptable.

```yaml
# Source: Swatinem/rust-cache README + ort-sys source code
- uses: Swatinem/rust-cache@v2
  with:
    cache-directories: |
      ~/.cache/ort.pyke.io      # Linux
      ~/Library/Caches/ort.pyke.io  # macOS (both ARM and Intel)
```

**Problem:** The `cache-directories` parameter doesn't support conditional paths per OS inside a single step. If the matrix includes all three OS families, the non-applicable paths are simply absent (no error).

**Recommended approach:** Set `ORT_CACHE_DIR` to a single portable path like `~/.ort-cache` in CI env vars, then cache that one directory regardless of platform. This avoids platform-conditional path logic entirely.

```yaml
env:
  CARGO_TERM_COLOR: always
  ORT_CACHE_DIR: ${{ runner.temp }}/ort-cache

# Then in the rust-cache step:
- uses: Swatinem/rust-cache@v2
  with:
    cache-directories: ${{ runner.temp }}/ort-cache
```

**Note:** `runner.temp` is a GHA built-in that resolves to a writable temp dir per runner, but it is NOT cached between runs by default. Use `~/.ort-cache` or a workspace-relative path instead if you want persistence.

### Pattern 2: Use ORT_CACHE_DIR with a fixed path

The cleanest approach: set `ORT_CACHE_DIR` to a fixed, known path; add that path to `cache-directories`.

```yaml
# In env section at workflow level:
env:
  CARGO_TERM_COLOR: always
  ORT_CACHE_DIR: ~/.ort-cache

# In each job's steps:
- uses: Swatinem/rust-cache@v2
  with:
    cache-directories: ~/.ort-cache
```

This works on Linux, macOS, and Windows (GitHub Actions expands `~` to the home directory on all platforms).

### Pattern 3: Adding macOS x86_64 to CI matrix (cross-compile)

The current CI matrix (`macos-latest`, `ubuntu-latest`, `windows-latest`) runs native architectures only. To add macOS Intel (x86_64) without a separate Intel runner:

```yaml
strategy:
  matrix:
    include:
      - os: macos-latest
        target: aarch64-apple-darwin
      - os: macos-latest
        target: x86_64-apple-darwin
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: windows-latest
        target: x86_64-pc-windows-msvc
```

When `target` differs from the host, `cargo build --target ${{ matrix.target }}` cross-compiles. The `dtolnay/rust-toolchain` action handles target installation via `targets:` parameter.

**Important:** The CI workflow currently runs `cargo test` without a `--target` flag, meaning it runs tests natively. For the macOS x86_64 cross-compile case, `cargo test --target x86_64-apple-darwin` will fail on an ARM64 runner (can't execute x86_64 binaries). The check job in CI should be structured as build + clippy (with target) + native tests (without target, skipped for cross targets) OR use `cargo check` instead of `cargo test` for cross-compile verification.

**Simpler alternative:** Keep the existing 3-runner matrix for `check` (build/test/clippy), and add macOS x86_64 as a build-only job (`cargo build --target x86_64-apple-darwin`) to satisfy CI-01 without test execution issues.

### Anti-Patterns to Avoid
- **Adding `ort` to your own Cargo.toml:** `voice_activity_detector` pins `ort =2.0.0-rc.10` exactly. If contentops also adds `ort` with any other version specifier, Cargo will fail to resolve unless it's an identical `=2.0.0-rc.10` pin. Do NOT add ort independently.
- **Using `%LOCALAPPDATA%` in GitHub Actions cache path:** The `actions/cache` and `cache-directories` parameter do not expand Windows env vars using `%VAR%` syntax. Use `~` which expands on all platforms, or use `ORT_CACHE_DIR` to normalize the path.
- **Caching `runner.temp`:** This directory is ephemeral per job and not persisted — caching it does nothing.
- **Running `cargo test` cross-target on GHA:** Cross-compiled binaries cannot execute on the host runner. Use `cargo check` or `cargo build` for cross-targets.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ORT binary download | Custom download script | ORT's built-in `download-binaries` feature (default) | ORT handles hash verification, extraction, platform-specific URLs |
| Cache key management | Custom key with timestamp | Swatinem/rust-cache key derivation | rust-cache uses Cargo.lock hash + toolchain hash as key — already invalidates correctly on dependency changes |
| macOS Intel cross-compile | Manual toolchain setup | `dtolnay/rust-toolchain` with `targets: x86_64-apple-darwin` | Handles rustup target add automatically |

**Key insight:** The ORT download and caching mechanism is already implemented in the build script. The only CI requirement is persisting the `ORT_CACHE_DIR` between runs using `Swatinem/rust-cache`'s `cache-directories` parameter.

## Common Pitfalls

### Pitfall 1: ort version conflict via indirect dependency
**What goes wrong:** If any other future dependency in Cargo.toml also depends on `ort` with a non-exact version, Cargo may try to resolve two ort versions and fail.
**Why it happens:** `voice_activity_detector` uses `=2.0.0-rc.10` (exact pin). Any other dep using `ort = "2"` or `ort = "2.0.0-rc.10"` (caret) could conflict.
**How to avoid:** Run `cargo tree -d` after adding the dependency to detect any duplicate/conflicting ort versions before pushing.
**Warning signs:** Build error: `"failed to select a version for the requirement ort"` or multiple `ort` entries in `cargo tree`.

### Pitfall 2: ORT binary re-downloaded every CI run
**What goes wrong:** Without caching the ORT binary directory, ORT downloads the runtime binary (~30 MB) on every CI run, slowing builds significantly.
**Why it happens:** `Swatinem/rust-cache` caches the Cargo registry and `target/` directory but not the ORT binary cache in `~/.cache/ort.pyke.io` (or platform equivalent).
**How to avoid:** Add `cache-directories: ~/.ort-cache` to `Swatinem/rust-cache` and set `ORT_CACHE_DIR: ~/.ort-cache` in the workflow env.
**Warning signs:** CI logs show "Downloading ONNX Runtime" on every run, never a cache hit.

### Pitfall 3: Windows ORT cache path resolution failure
**What goes wrong:** The Windows ORT default path uses `FOLDERID_LocalAppData` (via Win32 API call in build.rs), which resolves to `C:\Users\runneruser\AppData\Local\ort.pyke.io`. This path may not be covered by `~` expansion in some GitHub Actions contexts.
**Why it happens:** `%LOCALAPPDATA%` is not expanded by `cache-directories`; `~` expands to `C:\Users\runneruser` on Windows in GHA, so `~\AppData\Local\ort.pyke.io` should work but is less proven.
**How to avoid:** Set `ORT_CACHE_DIR: ~/.ort-cache` globally in the workflow env — this normalizes the path to `C:\Users\runneruser\.ort-cache` on Windows, which `~` expands correctly.
**Warning signs:** Windows CI downloads ORT every run despite cache-directories being configured.

### Pitfall 4: cargo test fails on cross-compiled macOS x86_64 target
**What goes wrong:** `cargo test --target x86_64-apple-darwin` on an ARM64 runner tries to execute x86_64 binaries which the kernel refuses to run (wrong arch).
**Why it happens:** Cross-compilation produces binaries for a different architecture than the host.
**How to avoid:** Use `cargo build --target x86_64-apple-darwin` and `cargo clippy --target x86_64-apple-darwin` for the cross-compile job; do not run `cargo test` with a target flag for the macOS Intel entry.
**Warning signs:** CI error: `exec format error` or `bad CPU type in executable`.

### Pitfall 5: CI matrix still only has 3 platforms
**What goes wrong:** The existing CI matrix (`macos-latest`, `ubuntu-latest`, `windows-latest`) only covers 3 native platforms. CI-01 requires 4 platforms including macOS Intel.
**Why it happens:** `macos-latest` resolves to ARM64 (`aarch64-apple-darwin`). There is no automatic Intel build.
**How to avoid:** Add a 4th matrix entry: `os: macos-latest, target: x86_64-apple-darwin` with cross-compilation steps.

## Code Examples

### Adding the dependency (Cargo.toml)
```toml
# No special feature flags needed — defaults are fine
# Do NOT add ort separately; voice_activity_detector manages that pin
voice_activity_detector = "0.2.1"
```

### CI workflow modifications (ci.yml)
```yaml
env:
  CARGO_TERM_COLOR: always
  ORT_CACHE_DIR: ~/.ort-cache   # Normalize ORT cache path across all platforms

jobs:
  check:
    name: Check (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}
          cache-directories: ~/.ort-cache

      - name: Format check
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --target ${{ matrix.target }} -- -D warnings

      - name: Build
        run: cargo build --target ${{ matrix.target }}

      # Tests only run natively (skip for cross-compiled macOS Intel)
      - name: Tests
        if: matrix.target != 'x86_64-apple-darwin'
        run: cargo test

      - name: Security audit
        run: |
          cargo install cargo-audit --locked
          cargo audit
```

### Verifying no ort version conflicts (local and CI)
```bash
# Run locally after adding voice_activity_detector to Cargo.toml:
cargo tree -d | grep ort

# In CI, add optional step:
- name: Check for ort version conflicts
  run: cargo tree -d | grep -c ort || true
```

### Release workflow additions (release.yml)

Each build job already uses `Swatinem/rust-cache@v2`. Add `cache-directories` and the env var:

```yaml
env:
  CARGO_TERM_COLOR: always
  BINARY_NAME: contentops
  ORT_CACHE_DIR: ~/.ort-cache  # Add this

# In each build job, update the rust-cache step:
- uses: Swatinem/rust-cache@v2
  with:
    cache-directories: ~/.ort-cache
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `macos-13` runner for Intel macOS | `macos-15-intel` runner (or cross-compile) | macOS-13 deprecated Sep 2025, removed Dec 2025 | Must use `macos-15-intel` if a native Intel runner is needed; cross-compile on ARM64 runner is the alternative |
| ORT dynamic loading via `ORT_DYLIB_PATH` | ORT `download-binaries` default feature | ort 2.x | Default behavior downloads + caches ORT binary automatically; no manual dylib management needed |

**Deprecated/outdated:**
- `macos-13` runner label: deprecated September 2025, unsupported December 2025. Use `macos-15-intel` for native Intel, or cross-compile on `macos-latest` (ARM64).
- `%LOCALAPPDATA%` in `cache-directories`: Windows env var syntax not expanded by GHA cache actions.

## Open Questions

1. **Does `cargo test` for `voice_activity_detector` require the ORT binary at test time, or only at link time?**
   - What we know: The VAD crate initializes a `LazyLock<Session>` — ORT is loaded at first use, not import time. Tests that exercise VAD will trigger ORT initialization.
   - What's unclear: Whether contentops's own tests exercise any VAD code (they don't yet — no VAD logic is wired in this phase).
   - Recommendation: Since no VAD logic is wired in Phase 16, existing tests will pass without ORT binary being present at test runtime. The ORT binary IS needed at build time (link step). Cache it to avoid build-time downloads.

2. **Will `cargo fmt --check` and `cargo clippy` require any changes for the new dependency?**
   - What we know: Adding a dependency to Cargo.toml doesn't affect fmt. Clippy may flag issues in voice_activity_detector itself, but `--no-deps` suppresses that.
   - What's unclear: Whether the current `cargo clippy` invocation in ci.yml uses `--no-deps`.
   - Recommendation: Current ci.yml runs `cargo clippy -- -D warnings` without `--no-deps`. If clippy fails due to warnings in the VAD dependency's exported types, add `--no-deps`.

3. **How large is the ORT binary download per target?**
   - What we know: ORT 2.0.0-rc.10 binaries are downloaded from Microsoft's release assets. Typical ORT builds are 15-40 MB compressed.
   - What's unclear: Exact size for rc.10 builds per platform.
   - Recommendation: The cache will avoid repeated downloads after first run; size is not a blocking concern.

## Sources

### Primary (HIGH confidence)
- `pykeio/ort` repo, `v2.0.0-rc.10`, `ort-sys/src/internal/dirs.rs` — platform-specific ORT cache directory logic verified from source
- `pykeio/ort` repo, `v2.0.0-rc.10`, `ort-sys/build.rs` — `ORT_CACHE_DIR` env var, download behavior, `download-binaries` feature gate verified
- `pykeio/ort` repo, `v2.0.0-rc.10`, `Cargo.toml` — `download-binaries` and `copy-dylibs` confirmed as default features
- `nkeenan38/voice_activity_detector` repo, `v0.2.1`, `Cargo.toml` — exact `ort =2.0.0-rc.10` and `ort-sys =2.0.0-rc.10` pins confirmed
- `nkeenan38/voice_activity_detector` repo, `v0.2.1`, `src/vad.rs` — Silero model embedded via `include_bytes!`, confirmed no model download needed
- GitHub Docs (current): `macos-15-intel` is the current Intel x86_64 runner label; `macos-13` deprecated Dec 2025

### Secondary (MEDIUM confidence)
- Swatinem/rust-cache README: `cache-directories` parameter supports newline-separated paths, confirmed from official repo
- DeepWiki/pykeio/ort: ORT cache path `~/.cache/ort/dfbin/` structure — consistent with source code reading

### Tertiary (LOW confidence)
- WebSearch: `~` expands correctly on Windows GHA for cache paths — not directly verified with official GHA docs but consistent across multiple community reports

## Metadata

**Confidence breakdown:**
- ORT cache paths: HIGH — verified directly from `ort-sys` source code
- voice_activity_detector deps: HIGH — verified from actual Cargo.toml at v0.2.1 tag
- GHA runner labels: HIGH — verified from GitHub Docs (current)
- Cache key/invalidation strategy: MEDIUM — Swatinem docs are clear but Windows `~` expansion is community-reported
- macOS Intel cross-compile behavior: HIGH — mirrors existing release.yml pattern which already works

**Research date:** 2026-02-24
**Valid until:** 2026-03-24 (30 days — stable area, but GHA runner labels can change)
