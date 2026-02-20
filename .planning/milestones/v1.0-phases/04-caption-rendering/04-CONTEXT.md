# Phase 4: Caption Rendering — Context

## Goal
Burn styled karaoke captions into video within TikTok safe zones via the existing `caption` subcommand's new `--burn` flag.

## Approach

### CLI Extension
- Add `--burn` flag to the existing `CaptionArgs` struct
- When `--burn` is passed: generate captions (SRT + JSON) AND burn them into the video in one pipeline
- Output: `input_captioned.mp4` (video with hard-burned subs)
- Without `--burn`: existing behavior unchanged (SRT + JSON only)

### ASS Subtitle Generation
- Generate ASS (Advanced SubStation Alpha) format subtitle file from the per-word JSON sidecar
- Use ASS `\k` tags for karaoke-style word-by-word highlighting (words light up as spoken)
- Each subtitle event contains a group of words (same 3-5 word grouping as SRT)
- Within each event, individual words get `\kf` duration tags so they highlight progressively

### Caption Style
- Font: bold, ~48pt (PlayResY-scaled)
- Color: white text with black outline (BorderStyle 3, Outline 2-3px)
- Alignment: bottom-center (ASS alignment 2)
- Shadow: none or minimal

### TikTok Safe Zone Positioning
- Avoid top 250px (status bar, username overlay)
- Avoid bottom 320px (description, buttons, progress bar)
- Use ASS `MarginV` to push captions up from the bottom safe zone boundary
- Based on 1920x1080 resolution: MarginV = 320, with top margin via PlayResY constraints

### Burn Pipeline
1. Generate ASS file from JSON sidecar (temp file)
2. Run FFmpeg with `ass` subtitle filter: `ffmpeg -i input.mp4 -vf "ass=temp.ass" -c:a copy output.mp4`
3. Output codec: H.264/AAC, yuv420p (TikTok standard), CRF 23
4. Clean up temp ASS file after burn

## Requirements Covered
- **CAP-03**: Subtitles burned into video as hard subs
- **CAP-04**: Karaoke-style word-by-word highlighting (ASS `\k` tags)
- **CAP-05**: Caption positioning respects TikTok safe zones

## Key Decisions
| Decision | Choice | Rationale |
|----------|--------|-----------|
| Subtitle format | ASS | Only format supporting karaoke `\k` tags via FFmpeg filters |
| Highlight style | `\kf` tags | Smooth fill effect (vs `\k` which is instant swap) |
| FFmpeg filter | `ass=` filter | Direct ASS rendering, supports all ASS features |
| Font size | ~48pt | Readable on mobile at TikTok 1080x1920 |
| Safe zone impl | ASS MarginV/MarginT | Native ASS positioning, no manual coordinate math |
| Output suffix | `_captioned.mp4` | Consistent with existing `_captioned.srt` naming |

## Dependencies
- Phase 3 output: per-word JSON sidecar (`[{word, start, end}, ...]`)
- FFmpeg with libass support (standard in Homebrew FFmpeg)
- Existing `caption.rs` module as the integration point

## Files to Modify
- `src/cli.rs` — add `--burn` flag to CaptionArgs
- `src/commands/caption.rs` — ASS generation + FFmpeg burn pipeline
- No new modules needed; extends existing caption command
