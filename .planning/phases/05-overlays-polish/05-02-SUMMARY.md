# Plan 05-02 Summary: Progress Bar Upgrade

**Completed:** 2026-02-20

## What was built

- **`probe_duration()`**: Runs ffprobe to get input video duration in seconds
- **`run_ffmpeg_with_progress()`**: Spawns FFmpeg with `-progress pipe:2`, reads stderr line-by-line, parses `out_time_us=` to compute percentage, updates indicatif ProgressBar
- **Spinner fallback**: When duration can't be determined, falls back to indeterminate spinner
- **All commands upgraded**: cut, overlay, and normalize pass 2 use progress bar

## Files modified
- `src/ffmpeg.rs` — Added probe_duration, run_ffmpeg_with_progress, run_ffmpeg_with_spinner
- `src/commands/cut.rs` — Simplified to use progress bar runner
- `src/commands/overlay.rs` — Simplified to use progress bar runner
- `src/commands/normalize.rs` — Pass 2 uses progress bar, pass 1 keeps spinner

## Requirements satisfied
- PIPE-05: User sees progress bar during FFmpeg processing stages
