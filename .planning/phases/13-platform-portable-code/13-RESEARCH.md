# Phase 13: Platform-Portable Code - Research

**Researched:** 2026-02-21
**Domain:** Rust cross-platform compilation, conditional compilation, font discovery
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| XPLAT-01 | Default font path resolves to a valid system font on macOS, Linux, and Windows without requiring `--font` | Cfg-based platform constants OR runtime font-kit lookup; hard-coded paths verified for each OS |
| XPLAT-02 | Null device uses `/dev/null` on Unix and `NUL` on Windows | Best approach: use `-f null -` which is truly cross-platform; OR cfg-based constant |
| XPLAT-03 | Error hints show platform-appropriate install commands (brew on macOS, apt on Linux, choco on Windows) | `cfg!` macro at call site in `error.rs` format_error(); `#[cfg]` for AppError display strings |
</phase_requirements>

## Summary

Phase 13 is a pure code-level portability fix with three isolated change sites in the codebase. No new dependencies are strictly required — Rust's built-in `#[cfg]` attribute and `cfg!()` macro cover all three requirements using compile-time platform detection. The phase requires no architectural changes: it's targeted surgery on three files (`overlay.rs`, `normalize.rs`, `error.rs`).

The hardest requirement is XPLAT-01 (font path). Impact is a Windows system font (guaranteed in `C:\Windows\Fonts\impact.ttf`), a macOS supplemental font (guaranteed at `/System/Library/Fonts/Supplemental/Impact.ttf`), but NOT guaranteed on Linux — it must be installed separately. The planner must decide whether to: (a) resolve to a known-good fallback (DejaVu Sans Bold) on Linux when Impact is absent, or (b) probe for Impact at runtime and fall back gracefully, or (c) document Impact as a requirement and emit a clear error. A runtime probe with fallback is the most user-friendly approach and requires no extra dependencies (only `std::path::Path::exists()`).

XPLAT-02 is the simplest fix: replace `"/dev/null"` with `"-"` and keep `-f null` already present, making it truly cross-platform via ffmpeg's own null muxer (`-f null -`). No cfg needed.

**Primary recommendation:** Use `#[cfg]` for compile-time constants (font paths, install hints). Use `-f null -` for null device. Probe for Impact at runtime with DejaVu/Liberation fallback on Linux.

## Standard Stack

### Core (no new dependencies needed)

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| Rust `#[cfg(target_os)]` | built-in | Compile-time platform branching | Zero cost, no deps, idiomatic Rust |
| Rust `cfg!()` macro | built-in | Runtime-resolved platform booleans | Works in match arms and string formatting |
| `std::path::Path::exists()` | built-in | Runtime font path probe | No deps, sufficient for XPLAT-01 |

### If Runtime Font Discovery Is Required (optional, avoid if possible)

| Library | Version | Purpose | Cost |
|---------|---------|---------|------|
| `font-kit` | 0.14.x | System font database lookup by name | Pulls in freetype + fontconfig on Linux — heavy system deps, complicates cross-compile |

**Decision: Do NOT add font-kit.** The system dependency footprint (libfreetype6-dev, libfontconfig1-dev required on Linux build machine) conflicts with the phase goal of clean cross-compilation. Hard-coded paths + existence probe is sufficient.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hard-coded paths + existence probe | font-kit | font-kit is more robust for arbitrary system configs but adds heavy native deps that break `cargo build` on minimal CI runners |
| `-f null -` (null muxer) | `cfg!`-based `/dev/null` or `NUL` | `-f null -` is simpler and already works; cfg-based approach also valid but unnecessary |

## Architecture Patterns

### Pattern 1: Platform-Specific `const` via `#[cfg]`

**What:** Define multiple `const` items with the same name, each guarded by `#[cfg]`. Rust selects the matching one at compile time.

**When to use:** For string literals that differ by OS (font paths, install commands).

**Example:**
```rust
// Source: https://doc.rust-lang.org/reference/conditional-compilation.html
#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "/System/Library/Fonts/Supplemental/Impact.ttf";

#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "C:\\Windows\\Fonts\\impact.ttf";

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_FONT: &str = "/usr/share/fonts/truetype/msttcorefonts/Impact.ttf";
```

**Constraint:** Must cover all cfg branches — the compiler will error if none matches.

### Pattern 2: Runtime Existence Probe with Fallback

**What:** Check if preferred font exists at runtime; fall back to alternative paths or known-good substitutes.

**When to use:** On Linux, where Impact may or may not be installed.

```rust
fn resolve_default_font() -> String {
    let candidates = [
        "/usr/share/fonts/truetype/msttcorefonts/Impact.ttf",    // ubuntu ttf-mscorefonts-installer
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", // liberation-fonts
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",        // dejavu-fonts (near-universal)
    ];
    candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
        .unwrap_or_else(|| candidates[0].to_string()) // let ffmpeg fail with clear path
}
```

### Pattern 3: `cfg!` Macro in Format Strings for Install Hints

**What:** Use `cfg!()` in `format_error()` to select the correct install command string.

**When to use:** In `error.rs` `format_error()` where the hint text differs per platform.

```rust
// Source: https://doc.rust-lang.org/reference/conditional-compilation.html
let install_hint = if cfg!(target_os = "macos") {
    "brew install ffmpeg"
} else if cfg!(target_os = "windows") {
    "choco install ffmpeg"
} else {
    "apt install ffmpeg"
};
```

**Alternative:** Use `#[cfg]` on the `AppError` display string — but this requires conditional compilation on the entire enum variant, which is noisier. The `cfg!()` approach in `format_error()` is cleaner.

### Pattern 4: FFmpeg Null Muxer (XPLAT-02 — simplest fix)

**What:** Replace `/dev/null` with `-` and keep `-f null`. The null muxer in ffmpeg (`-f null -`) discards output and works identically on all platforms.

**Current code at normalize.rs:66-74:**
```rust
let measure_args = [
    "-i", &input_str,
    "-af", "loudnorm=I=-14:TP=-1.5:LRA=11:print_format=json",
    "-f", "null",
    "/dev/null",   // <- change this to "-"
];
```

**After fix:**
```rust
let measure_args = [
    "-i", &input_str,
    "-af", "loudnorm=I=-14:TP=-1.5:LRA=11:print_format=json",
    "-f", "null",
    "-",   // null muxer sink — cross-platform
];
```

Source: ffmpeg documentation confirms `-f null -` is the portable null output approach.

### Anti-Patterns to Avoid

- **Adding `font-kit` as a dependency:** Pulls in native C library deps (freetype, fontconfig) that are NOT available on minimal Linux CI runners and complicate cross-compilation toolchain setup.
- **Using `cfg!()` for the font path constant:** `cfg!()` returns a `bool`, not a string — you cannot embed it in a `const &str`. Use `#[cfg]` on multiple `const` definitions instead.
- **Assuming Impact exists on all Linux systems:** It requires the `ttf-mscorefonts-installer` package on Debian/Ubuntu. Never guaranteed. Must probe or fallback.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Platform null device | Custom cfg-per-platform string | `-f null -` (ffmpeg native) | ffmpeg already handles this; no Rust code needed |
| Font system enumeration | Custom OS font scanner | `std::path::Path::exists()` probe with candidate list | Sufficient for the 3 known platforms; font-kit is overkill |

## Common Pitfalls

### Pitfall 1: Linux Without Impact Font Installed

**What goes wrong:** `DEFAULT_FONT` points to `/usr/share/fonts/truetype/msttcorefonts/Impact.ttf` which does not exist unless `ttf-mscorefonts-installer` is installed. ffmpeg drawtext fails with "No such file or directory".

**Why it happens:** Impact is a Microsoft font, not installed by default on any Linux distribution.

**How to avoid:** Use the runtime probe pattern (Pattern 2 above). Candidate fallbacks that ARE near-universally present on Linux: DejaVu Sans Bold, Liberation Sans Bold.

**Warning signs:** `cargo build` succeeds but `overlay` command fails on Linux in CI.

### Pitfall 2: `#[cfg]` Without Exhaustive Else Branch

**What goes wrong:** Defining `const DEFAULT_FONT` only for macOS and Windows leaves Linux builds with a compile error (symbol not found).

**Why it happens:** Each `#[cfg]`-guarded `const` is independent; Rust does not auto-generate a fallback.

**How to avoid:** Always include a `#[cfg(not(any(...)))]` catch-all, or explicitly add `#[cfg(target_os = "linux")]`.

### Pitfall 3: Error Hint Mismatch (display vs. format_error)

**What goes wrong:** `AppError` has two representations: the `#[error(...)]` derive string (used when downcasting fails) and `format_error()` (used by main). Only `format_error()` is actually called. Fixing only the `#[error]` string but not `format_error()` leaves the user-visible hint unchanged.

**Why it happens:** The code has both a `thiserror`-derived Display and a custom `format_error()` function. The custom one wins.

**How to avoid:** Fix both the `#[error(...))]` attributes on the enum variants AND the `format_error()` match arms. Locations in `error.rs`:
- Line 8: `#[error("ffmpeg not found on PATH\n  hint: brew install ffmpeg")]`
- Line 28: `#[error("whisper-cli not found on PATH\n  hint: brew install whisper-cli")]`
- Line 85: `format_error()` FfmpegNotFound arm — `"brew install ffmpeg"`
- Line 125: `format_error()` WhisperNotFound arm — `"brew install whisper-cli"`

### Pitfall 4: Windows Font Path Casing

**What goes wrong:** Windows is case-insensitive but ffmpeg's fontfile parameter may be case-sensitive on the value passed in. The file is `impact.ttf` (lowercase) in `C:\Windows\Fonts\` on modern Windows but was `Impact.ttf` historically.

**How to avoid:** Use the documented filename `impact.ttf` (lowercase) as the constant. ffmpeg on Windows handles path separators (both `/` and `\` work).

## Code Examples

Verified patterns applicable to this phase:

### XPLAT-01: Platform Font Constants + Linux Probe

```rust
// overlay.rs — replace the single DEFAULT_FONT const

#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "/System/Library/Fonts/Supplemental/Impact.ttf";

#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "C:\\Windows\\Fonts\\impact.ttf";

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const IMPACT_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/msttcorefonts/Impact.ttf",
    "/usr/share/fonts/msttcorefonts/Impact.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn resolve_default_font() -> String {
    IMPACT_FONT_CANDIDATES
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
        .unwrap_or_else(|| IMPACT_FONT_CANDIDATES[0].to_string())
}

// Then in build_title_filter or the call site:
let font_path = args.font
    .as_ref()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        { DEFAULT_FONT.to_string() }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        { resolve_default_font() }
    });
```

### XPLAT-02: Null Device Fix

```rust
// normalize.rs line 66-74 — change "/dev/null" to "-"
let measure_args = [
    "-i", &input_str,
    "-af", "loudnorm=I=-14:TP=-1.5:LRA=11:print_format=json",
    "-f", "null",
    "-",
];
```

### XPLAT-03: Platform Install Hints in format_error()

```rust
// error.rs — helper function for install hints
fn ffmpeg_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install ffmpeg"
    } else if cfg!(target_os = "windows") {
        "choco install ffmpeg"
    } else {
        "apt install ffmpeg"
    }
}

fn whisper_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install whisper-cli"
    } else if cfg!(target_os = "windows") {
        "choco install whisper-cli"  // or: winget install Whisper
    } else {
        "apt install whisper-cpp  # or build from source"
    }
}
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|-----------------|--------|
| Hard-coded macOS font path | `#[cfg]` multi-const + Linux runtime probe | Enables Linux/Windows builds |
| `/dev/null` literal | `-f null -` (ffmpeg null muxer) | Cross-platform without any cfg needed |
| Static `brew install` hints | `cfg!()` macro per platform | Correct hint on each OS |

## Open Questions

1. **whisper-cli on Windows/Linux**
   - What we know: `brew install whisper-cli` is macOS-only; no official Windows/Linux package name is established
   - What's unclear: Whether `whisper-cpp` is the right apt package name; whether choco has it
   - Recommendation: Use `"build from source: https://github.com/ggerganov/whisper.cpp"` as the non-macOS hint for whisper; it's the honest answer

2. **Linux font fallback quality**
   - What we know: DejaVu Sans Bold is near-universally present on Linux but is a sans-serif, not condensed like Impact
   - What's unclear: Whether the visual output with a non-Impact fallback is acceptable for the project's aesthetic goals
   - Recommendation: Document in code that Impact is preferred; user can install `ttf-mscorefonts-installer` for Impact; tool falls back gracefully

3. **Cargo build targets vs. actual runtime portability**
   - What we know: The success criterion says `cargo build` must succeed on linux/windows targets; it does NOT say the tool must produce identical video output
   - What's unclear: Does the font fallback on Linux need to produce aesthetically equivalent output or just not crash?
   - Recommendation: Not-crash is sufficient for the build success criterion; document aesthetic difference

## Sources

### Primary (HIGH confidence)
- Rust Reference: Conditional Compilation — https://doc.rust-lang.org/reference/conditional-compilation.html (`#[cfg]`, `cfg!()`, target_os values)
- FFmpeg null muxer wiki — https://trac.ffmpeg.org/wiki/Null (confirmed `-f null -` is portable)
- Microsoft Typography: Impact font — https://learn.microsoft.com/en-us/typography/font-list/impact (confirmed `impact.ttf` in Windows 7–11)
- font-kit Handle docs — https://docs.rs/font-kit/latest/font_kit/handle/enum.Handle.html (Handle::Path variant for file path extraction)

### Secondary (MEDIUM confidence)
- font-kit Source trait docs — https://docs.rs/font-kit/latest/font_kit/source/trait.Source.html (`select_best_match`, `FamilyName::Title`)
- font-kit GitHub issues — https://github.com/servo/font-kit/issues/181 (fontconfig dependency issues on Linux)
- crates.io font-kit — version 0.14.x, confirmed as current latest

### Tertiary (LOW confidence)
- Linux font locations inferred from Ubuntu wiki and arch wiki — Impact path candidates are convention-based, not guaranteed; probe at runtime

## Metadata

**Confidence breakdown:**
- XPLAT-02 (null device): HIGH — FFmpeg docs are authoritative; `-f null -` is unambiguous
- XPLAT-03 (error hints): HIGH — Rust cfg docs authoritative; fix is mechanical
- XPLAT-01 (font path macOS/Windows): HIGH — OS font paths verified with official docs
- XPLAT-01 (font path Linux): MEDIUM — Impact not guaranteed; fallback paths are distro-convention

**Research date:** 2026-02-21
**Valid until:** 2026-08-21 (stable domain — Rust cfg, ffmpeg null muxer, Windows font paths are not fast-moving)
