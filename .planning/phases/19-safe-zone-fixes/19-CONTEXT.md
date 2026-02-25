# Phase 19: Safe Zone Fixes - Context

**Gathered:** 2026-02-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Correct overlay, subtitle, and video output dimensions so all text and content stays within TikTok's visible safe area. No new features -- just positioning and resolution fixes.

</domain>

<decisions>
## Implementation Decisions

### Subtitle placement
- Claude's discretion on vertical position -- current MarginV: 480 may be fine, validate against TikTok safe zones
- Claude's discretion on horizontal margins -- add side margins or max width to avoid right-side icon column
- Check whether word wrapping causes overflow issues at current settings

### Overlay title position
- Title currently renders at top of frame
- Left edge of title was clipped when uploaded to TikTok -- likely a resolution/padding issue
- Claude's discretion on padding from edges -- ensure title text never touches frame edges
- Probe source video resolution (iPhone portrait) and compare to output resolution

### Long title handling
- Wrap to next line when title is too wide (not shrink font)
- Title length varies widely (2-8+ words) -- wrapping must handle both short and long
- Title width must be clamped to stay within safe zone after padding

### Video output resolution
- Output must always be exactly 1080x1920 (9:16 TikTok standard)
- Scale source to fill 1080x1920 -- crop edges if aspect ratio doesn't match exactly (no letterboxing, no black bars)
- iPhone shoots portrait so source is close to 9:16 already -- cropping should be minimal

### Claude's Discretion
- Exact pixel values for safe zone margins (top, bottom, left, right)
- Whether to add a dedicated scale/crop stage or integrate into existing ffmpeg pipeline
- ASS subtitle PlayResX/PlayResY values if they need adjustment
- Overlay drawtext x/y coordinate changes

</decisions>

<specifics>
## Specific Ideas

- "The very far left edge of the title was clipped" -- this is the primary visual bug to fix
- "The subtitles are running into the overlay for TikTok" -- subtitle/TikTok UI overlap needs fixing
- User wants zero black bars -- crop-to-fill, not letterbox

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 19-safe-zone-fixes*
*Context gathered: 2026-02-25*
