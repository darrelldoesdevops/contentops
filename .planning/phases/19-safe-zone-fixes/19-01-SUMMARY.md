# Plan 19-01 Summary: TikTok Safe Zone Constants + Overlay Fix

**Status:** Complete
**Commit:** b96ede9

## What Changed

- Created `src/tiktok.rs` with named constants for TikTok safe zones (OUTPUT_WIDTH, OUTPUT_HEIGHT, margins, SAFE_WIDTH)
- Fixed overlay `final_x` from `scale_i32(30, ...)` to `scale_i32(80, ...)` -- accent bar now clears 60px left safe margin
- Added `wrap_title_lines()` function in overlay.rs -- char-count estimation wraps long titles to fit within 900px safe width
- `build_title_filter()` now uses `wrap_title_lines()` instead of raw `text.split('\n')`

## Files Modified

- src/tiktok.rs -- NEW: 7 named constants for TikTok canvas and safe zones
- src/main.rs -- added `mod tiktok;`
- src/commands/overlay.rs -- import tiktok, fixed final_x, added wrap_title_lines(), integrated wrapping

## Decisions

- Used 0.55 char-width multiplier for Impact font (conservative, wraps slightly early rather than late)
- SAFE_WIDTH = 900px (1080 - 60 left - 120 right) as max title width
