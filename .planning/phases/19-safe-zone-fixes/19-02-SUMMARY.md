# Plan 19-02 Summary: Subtitle Margins + Scale-to-Fill Pipeline

**Status:** Complete
**Commit:** 450e182

## What Changed

- Fixed ASS subtitle margins in caption.rs: MarginL 40->60, MarginR 40->120, MarginV stays 480 (all from tiktok constants)
- Added `scale_to_tiktok()` in ffmpeg.rs -- scales to fill 1080x1920 with center crop, no black bars
- Added Stage 1 (scale) to pipeline.rs before normalize -- probes dimensions, skips if already 1080x1920
- Updated pipeline from 6 stages to 7 stages with correct numbering
- Updated dry_run output to show new scale stage
- Cleaned up scaled temp file after normalize

## Files Modified

- src/commands/caption.rs -- import tiktok, dynamic ASS Style line with named constants
- src/ffmpeg.rs -- added scale_to_tiktok() function
- src/commands/pipeline.rs -- import tiktok, added scale stage, updated stage numbering 1/7..7/7
- src/commands/overlay.rs -- removed redundant trim() before split_whitespace (clippy fix)

## Decisions

- Scale stage uses libx264 CRF 14 / slow preset (matches existing encode settings)
- Skip scale if already 1080x1920 to avoid unnecessary re-encode
- ASS margins now reference tiktok constants (not hardcoded numbers)
