# Phase 5: Overlays and Polish - Context

**Gathered:** 2026-02-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Text overlays via FFmpeg drawtext filter, audio loudness normalization via loudnorm filter, and progress bar upgrade from spinner to real indicatif progress bar. Three new subcommands/features that complete the full feature set.

Requirements: OVL-01, OVL-02, OVL-03, AUD-01, PIPE-05

</domain>

<decisions>
## Implementation Decisions

### New `overlay` subcommand
- Command: `contentops overlay input.mp4 --text "Title Text"`
- Uses FFmpeg `drawtext` filter to burn text onto video
- Output suffix: `_overlay.mp4` (e.g., `input_overlay.mp4`)
- `-o` flag overrides output path (same pattern as `cut`)
- Text is required via `--text` flag

### Overlay user controls (CLI flags)
- `--font <path>`: Path to a .ttf font file (default: FFmpeg's default font)
- `--font-size <int>`: Font size in pixels (default: 48)
- `--color <string>`: Font color as FFmpeg color name or hex (default: "white")
- `--position <preset>`: Position preset — `top`, `center`, `bottom` (default: `center`)
- `--start <seconds>`: When overlay appears in seconds (default: 0.0)
- `--duration <seconds>`: How long overlay is visible in seconds (default: entire video)

### TikTok safe zone positioning
- TikTok safe zones: avoid top 250px (status bar, username) and bottom 320px (captions, buttons)
- `top` position: y=260 (just below safe zone)
- `center` position: centered vertically in safe area
- `bottom` position: y=(h-330) (just above bottom safe zone)
- All positions: horizontally centered using `x=(w-text_w)/2`

### New `normalize` subcommand
- Command: `contentops normalize input.mp4`
- Uses FFmpeg `loudnorm` two-pass filter for EBU R128 loudness normalization
- Target: -14 LUFS (standard for social media platforms)
- Two-pass approach: first pass measures loudness, second pass applies correction
- Output suffix: `_normalized.mp4` (e.g., `input_normalized.mp4`)
- `-o` flag overrides output path
- Video stream is copied (`-c:v copy`) — only audio is re-encoded

### Progress bar upgrade (PIPE-05)
- Replace Phase 1's `indicatif` spinner with a real progress bar
- Parse FFmpeg's stderr progress output (`time=HH:MM:SS.ms` lines) to track progress
- Requires knowing total duration: probe input with `ffprobe -show_entries format=duration`
- New FFmpeg runner function that spawns child process, reads stderr line-by-line, updates progress bar
- Falls back to spinner if duration can't be determined
- Applies to all subcommands (cut, overlay, normalize)
- Verbose mode (`--verbose`) still streams raw FFmpeg stderr (no progress bar)

### FFmpeg wrapper changes
- New `run_ffmpeg_with_progress()` function that:
  1. Spawns FFmpeg as child process (not `.output()`)
  2. Adds `-progress pipe:2` or parses `time=` from stderr
  3. Reads stderr line-by-line via BufReader
  4. Parses `out_time_ms=` or `time=` to compute percentage
  5. Updates indicatif ProgressBar
  6. Returns FfmpegOutput on completion
- New `probe_duration(input)` function that runs `ffprobe` to get video duration in seconds
- Existing `run_ffmpeg()` and `run_ffmpeg_verbose()` remain for backwards compatibility

### Claude's Discretion
- Exact drawtext filter syntax and escaping strategy
- Progress bar style (percentage, ETA, etc.)
- Whether to add `regex` crate or use simple string parsing for FFmpeg output
- Internal code organization for the new commands

</decisions>

<specifics>
## Specific Ideas

- Overlay should feel like a quick title card feature — defaults should produce good-looking results without tweaking
- Normalize should be fire-and-forget: run it, get -14 LUFS output, done
- Progress bar should show percentage and elapsed time at minimum

</specifics>

<deferred>
## Deferred Ideas

- Overlay animation (fade in/out) — v2 enhancement
- Multiple text overlays in one pass — v2
- Custom LUFS target flag — v2
- Background box/shadow behind overlay text — v2

</deferred>

---

*Phase: 05-overlays-polish*
*Context gathered: 2026-02-20*
