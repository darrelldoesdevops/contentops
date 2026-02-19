---
phase: 04-caption-rendering
plan: 01
subsystem: video-processing
tags: [ass, ffmpeg, karaoke, subtitles, tiktok]

requires:
  - phase: 03-caption-generation
    provides: "per-word JSON sidecar with Word structs"
provides:
  - "ASS subtitle generation with karaoke kf tags"
  - "FFmpeg burn pipeline via ass= filter"
  - "--burn flag on caption subcommand"
affects: [05-silence-removal]

tech-stack:
  added: [ASS subtitle format, FFmpeg ass filter]
  patterns: [karaoke kf tags for word-by-word highlighting, ASS MarginV for safe zones]

key-files:
  created: []
  modified: [src/cli.rs, src/commands/caption.rs]

key-decisions:
  - "ASS MarginV=320 for TikTok bottom safe zone avoidance"
  - "kf tags (smooth fill) over k tags (instant swap) for karaoke highlighting"

patterns-established:
  - "ASS generation reuses same word grouping logic (3-5 words, punctuation break) as SRT"
  - "Burn pipeline runs after SRT+JSON generation -- both outputs always produced when --burn"

requirements-completed: [CAP-03, CAP-04, CAP-05]

duration: 2min
completed: 2026-02-20
---

# Phase 4 Plan 1: Caption Rendering Summary

**ASS subtitle generation with karaoke kf tags and FFmpeg burn pipeline for TikTok-safe caption rendering**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-19T16:26:04Z
- **Completed:** 2026-02-19T16:27:50Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- ASS subtitle generation with karaoke `\kf` tags for smooth word-by-word highlighting
- FFmpeg burn pipeline using `ass=` filter with H.264/yuv420p/CRF 23 output
- TikTok safe zone positioning via PlayRes 1080x1920 and MarginV=320

## Task Commits

Each task was committed atomically:

1. **Task 1: Add --burn flag and ASS subtitle generation** - `aea8c15` (feat)
2. **Task 2: FFmpeg burn pipeline and caption command integration** - `63b064d` (feat)

## Files Created/Modified
- `src/cli.rs` - Added `burn: bool` field to CaptionArgs
- `src/commands/caption.rs` - ASS generation functions (generate_ass, group_words_for_ass, format_ass_time) and FFmpeg burn pipeline

## Decisions Made
- Used MarginV=320 for TikTok bottom safe zone (matches context spec)
- kf tags for smooth karaoke fill effect rather than k tags for instant swap
- Burn pipeline produces burned video alongside SRT+JSON (not instead of)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Caption rendering complete, --burn flag functional
- Phase 5 (silence removal) can proceed independently

## Self-Check: PASSED

All files exist, all commits verified.

---
*Phase: 04-caption-rendering*
*Completed: 2026-02-20*
