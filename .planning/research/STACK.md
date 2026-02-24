# Stack Research

**Domain:** Silero VAD integration into existing Rust CLI (contentops)
**Researched:** 2026-02-24
**Confidence:** HIGH (all crate versions verified against crates.io API and GitHub source)

## Recommended Stack

### Core Addition: VAD Crate

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `voice_activity_detector` | 0.2.1 | Silero VAD inference in Rust | Bundles model via `include_bytes!` (no runtime download), ships Silero VAD V5, verified on Windows/macOS/Linux, 42K total downloads. Only crate of the three candidates that supports static binary distribution without a runtime dylib dependency |

### ONNX Runtime (transitive, managed by voice_activity_detector)

| Technology | Version | Purpose | Notes |
|------------|---------|---------|-------|
| `ort` | =2.0.0-rc.10 (pinned exactly by voice_activity_detector) | ONNX Runtime Rust bindings | Wraps ONNX Runtime 1.22.0. Has prebuilt binaries for all four target platforms. Do NOT upgrade to rc.11 — it drops x86_64-apple-darwin from prebuilt support |
| `ort-sys` | =2.0.0-rc.10 (transitive) | C FFI bindings layer | Pulled in automatically; no direct entry needed in contentops Cargo.toml |

### Audio Input (optional, already FFmpeg-produced)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `hound` | 3.5.1 | WAV file reading for VAD input | Only needed if contentops reads the WAV file directly for VAD. If piping PCM bytes from FFmpeg stdout, hound is not required. hound is a dev-dependency of voice_activity_detector, not runtime |

## Cargo.toml Addition

```toml
[dependencies]
voice_activity_detector = "0.2.1"
# Do NOT add ort directly — voice_activity_detector pins ort = "=2.0.0-rc.10" exactly.
# Adding ort at a different version will cause Cargo resolver conflict.
```

If reading the WAV from disk (simplest approach given existing FFmpeg extraction):

```toml
[dependencies]
voice_activity_detector = "0.2.1"
hound = "3.5.1"
```

## Audio Format Requirements

| Parameter | Required Value | contentops Status |
|-----------|---------------|-------------------|
| Sample rate | 16 kHz (or 8 kHz) | Already produced at 16 kHz by FFmpeg for Whisper — no change needed |
| Channels | Mono (1 channel) | Already extracted mono via FFmpeg `-ac 1` — no change needed |
| Encoding | LPCM i16, i8, u8, u16, or f32 | hound decodes WAV to i16 by default — compatible |
| Chunk size | 512 samples at 16 kHz (= 32ms) | Enforced by Silero VAD V5 model; voice_activity_detector pads/truncates automatically |

The existing FFmpeg extraction pass for Whisper (`-ar 16000 -ac 1`) produces a file that is directly valid VAD input. No additional FFmpeg pass is required.

## Model Bundling Approach

**Use `include_bytes!` — handled entirely by voice_activity_detector.**

The Silero VAD V5 ONNX model is embedded in the `voice_activity_detector` crate source at `src/silero_vad.onnx` via:

```rust
// inside voice_activity_detector/src/vad.rs
const MODEL: &[u8] = include_bytes!("silero_vad.onnx");

static DEFAULT_SESSION: LazyLock<Arc<Mutex<Session>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(
        Session::builder()
            .unwrap()
            .commit_from_memory(MODEL)
            .unwrap()
    ))
});
```

Implications for contentops:
- No `build.rs` modifications needed
- No model file path to manage at runtime
- Binary size increase: ~2MB (the ONNX model, embedded at link time)
- Identical behavior on all platforms — the model is baked into the binary

## Cross-Platform Prebuilt Binary Support (ort 2.0.0-rc.10)

ort's `download-binaries` feature (enabled by default) downloads prebuilt ONNX Runtime 1.22.0 binaries at build time from `cdn.pyke.io`. All four contentops CI targets are covered:

| Target Triple | ort rc.10 Support | Source |
|--------------|-------------------|--------|
| `aarch64-apple-darwin` (macOS ARM64) | YES | `ms@1.22.0/aarch64-apple-darwin.tgz` |
| `x86_64-apple-darwin` (macOS Intel) | YES | `ms@1.22.0/x86_64-apple-darwin.tgz` |
| `x86_64-pc-windows-msvc` (Windows) | YES | `ms@1.22.0/x86_64-pc-windows-msvc.tgz` |
| `x86_64-unknown-linux-gnu` (Linux) | YES | `ms@1.22.0/x86_64-unknown-linux-gnu.tgz` |

Verified directly from `github.com/pykeio/ort/blob/v2.0.0-rc.10/ort-sys/dist.txt`.

**Note:** ort 2.0.0-rc.11 (the current latest) removed `x86_64-apple-darwin` from its prebuilt list. This is why voice_activity_detector pins `=2.0.0-rc.10` exactly. Do not attempt to update ort independently.

## CI/CD Impact

No new CI dependencies beyond internet access during the first `cargo build` per target.

Add ort's binary cache directory to GitHub Actions cache:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
      ~/.cache/ort        # ort prebuilt binary cache on Linux/macOS
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

Windows caches `%LOCALAPPDATA%\pyke\ort` — handled by standard Cargo cache action configurations.

The ort binary download (~30MB compressed per platform) is a one-time cost per cache miss. No `apt-get install`, `brew install`, or `choco install` commands are required.

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `voice_activity_detector` 0.2.1 | `silero-vad-rust` 6.2.1 | Requires `load-dynamic` ort feature — forces `libonnxruntime.dylib/.so/.dll` to ship alongside the binary at runtime. Incompatible with single-binary Homebrew distribution. Only 1.7K downloads total, published November 2025 |
| `voice_activity_detector` 0.2.1 | `silero-vad-rs` 0.1.2 | Pins `ort = "=2.0.0-rc.9"` (one version older, rc.9 has different behavior), only 2.2K downloads, last updated April 2025. Also needs verification on x86_64-apple-darwin for rc.9 |
| `include_bytes!` via voice_activity_detector | Download model at runtime | Runtime download fails in offline environments, adds startup latency, complicates distribution and error handling |
| Single VAD crate dependency | Direct `ort` + manual model handling | voice_activity_detector handles model embedding, session lifecycle, chunk sizing, and the Sample trait conversion — saves substantial integration work |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `ort` 2.0.0-rc.11 directly | Dropped x86_64-apple-darwin prebuilt binary — requires compiling ONNX Runtime from source for macOS Intel CI, which takes 20+ minutes and requires cmake | voice_activity_detector which pins ort 2.0.0-rc.10 |
| `silero-vad-rust` 6.2.1 `load-dynamic` mode | Requires `ORT_DYLIB_PATH` env var at runtime; dylib must ship alongside binary — incompatible with Homebrew single-binary formula | `voice_activity_detector` with static `download-binaries` approach |
| Manually calling ort with a model file path | Requires bundling/shipping the ONNX model separately from the binary | `voice_activity_detector` with `include_bytes!` embedded model |

## Version Compatibility Matrix

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `voice_activity_detector` | 0.2.1 | `ort` =2.0.0-rc.10 | Uses exact version pin |
| `ort` | 2.0.0-rc.10 | ONNX Runtime 1.22.0 | Prebuilt binary wraps ORT 1.22.0 |
| `ort` | 2.0.0-rc.10 | Rust 1.81+ | contentops uses edition 2024, requires recent Rust; no conflict |
| `ndarray` | 0.16.x | `ort` rc.10 | ort rc.10 requires `ndarray ^0.16`; voice_activity_detector uses 0.16.1; resolved by Cargo automatically |
| Silero VAD V5 ONNX model | bundled | ORT 1.22.0 | Model is opset 15/16 compatible; embedded in voice_activity_detector 0.2.1 |

## Sources

- crates.io API `/api/v1/crates/voice_activity_detector/0.2.1/dependencies` — exact ort pin `=2.0.0-rc.10`, ndarray 0.16.1 — HIGH confidence
- crates.io API `/api/v1/crates/silero-vad-rs/0.1.2/dependencies` — ort pin `=2.0.0-rc.9` — HIGH confidence
- crates.io API `/api/v1/crates/silero-vad-rust/6.2.1/dependencies` — ort features `["load-dynamic", "ndarray"]` confirmed — HIGH confidence
- GitHub `nkeenan38/voice_activity_detector` `src/vad.rs` — `include_bytes!("silero_vad.onnx")` and `commit_from_memory(MODEL)` confirmed — HIGH confidence
- GitHub `nkeenan38/voice_activity_detector` `src/sample.rs` — LPCM i8/i16/u8/u16/f32 via Sample trait confirmed — HIGH confidence
- GitHub `nkeenan38/voice_activity_detector` README — 16 kHz / 512-sample window, mono-only, Windows/macOS/Linux verified — HIGH confidence
- GitHub `pykeio/ort` `ort-sys/dist.txt` at tag `v2.0.0-rc.10` — all four target triples confirmed with x86_64-apple-darwin — HIGH confidence
- GitHub `pykeio/ort` `ort-sys/build/download/dist.txt` at `main` (rc.11) — x86_64-apple-darwin absent, ORT 1.23.2 — HIGH confidence
- crates.io API `/api/v1/crates/ort/2.0.0-rc.10` — Rust 1.81 minimum, ndarray ^0.16 — HIGH confidence

---
*Stack research for: Silero VAD integration into contentops Rust CLI*
*Researched: 2026-02-24*
