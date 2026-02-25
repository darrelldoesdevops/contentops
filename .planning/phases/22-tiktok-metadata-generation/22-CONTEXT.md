# Phase 22: TikTok Metadata Generation - Context

**Gathered:** 2026-02-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Generate a TikTok description from the transcript via Claude and write a sidecar JSON file next to the output video containing the approved title, description, and hashtags. This is the final phase of v1.5 Upload Ready.

</domain>

<decisions>
## Implementation Decisions

### Sidecar file format
- Filename: `{stem}_tiktok.json` where stem matches the output video filename (already decided in STATE.md)
- JSON structure: `{ "title": "...", "description": "...", "hashtags": ["..."] }`
- Minimal format for copy-paste -- not TikTok API-shaped (already decided in STATE.md)
- Written next to the output video path, not in temp directory

### Description generation
- Use Claude haiku (same model as title generation) to generate TikTok description from transcript
- Description should be engaging, include key points from the video
- Must stay within TikTok's 4,000 character limit
- Include 3-5 relevant hashtags derived from transcript topics
- No interactive approval for description -- auto-generate and write (only title gets approval)

### Pipeline integration
- Description generation happens AFTER overlay (Stage 7) completes, as a new post-pipeline step
- Receives: approved title (from Phase 21 approval), transcript words, output video path
- Not a numbered stage in the pipeline -- it's a metadata side-effect after all video processing is done
- If Claude call fails for description, warn but don't fail the pipeline (video is already produced)

### Standalone usage
- No standalone subcommand for metadata generation -- only runs as part of pipeline
- If user passes `--text` (manual title, no auto-generation), still generate description from transcript and write sidecar

### Claude's Discretion
- Exact prompt wording for description generation
- How to extract hashtag topics from transcript
- Whether to generate description and hashtags in one Claude call or two
- Error message wording when description generation fails

</decisions>

<specifics>
## Specific Ideas

- Sidecar file is the last thing the pipeline produces -- user sees it in the final output alongside the video
- Description should read naturally as a TikTok post caption, not as a summary
- Hashtags should be specific to the topic, not generic (#fyp, #viral) -- those add no discovery value

</specifics>

<deferred>
## Deferred Ideas

- TikTok API upload -- v2 (UPL-01)
- Hashtag generation as separate feature -- out of scope, included inline with description

</deferred>

---

*Phase: 22-tiktok-metadata-generation*
*Context gathered: 2026-02-25*
