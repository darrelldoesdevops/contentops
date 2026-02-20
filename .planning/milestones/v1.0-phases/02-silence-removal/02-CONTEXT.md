# Phase 2: Silence Removal - Context

**Gathered:** 2026-02-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Detect and remove silent segments from video using FFmpeg's silencedetect filter and select/aselect filter approach. Extends the existing `cut` subcommand to perform silence removal. Adds `--dry-run` flag for previewing detected silence. Output in TikTok-standard format (H.264/AAC, yuv420p, CRF 23, AAC 192kbps).

</domain>

<decisions>
## Implementation Decisions

### Silence detection approach
- Use FFmpeg's `silencedetect` audio filter to detect silent segments
- Parse silence_start/silence_end timestamps from FFmpeg stderr output
- Hardcoded defaults for v0.1: silence threshold -30dB, minimum duration 0.5s
- Handle edge cases: trailing silence (no silence_end emitted), leading silence (silence_end before silence_start)
- Use video duration as fallback end boundary for unmatched silence_start

### Silence removal approach (LOCKED DECISION)
- Use `select`/`aselect` filter approach (NOT segment-and-concat)
- Build FFmpeg filter expressions that select non-silent time ranges:
  ```
  -vf "select='between(t,S1,E1)+between(t,S2,E2)+...',setpts=N/FRAME_RATE/TB"
  -af "aselect='between(t,S1,E1)+between(t,S2,E2)+...',asetpts=N/SR/TB"
  ```
- `setpts=N/FRAME_RATE/TB` and `asetpts=N/SR/TB` rebuild timestamps from scratch, eliminating drift
- Single FFmpeg pass for both audio and video selection ensures sync

### Padding/margin
- 200ms padding on each side of speech segments to avoid clipping words
- Padding is subtracted from silence boundaries (expand speech segments into adjacent silence)
- Clamp padding so speech segments don't overlap or go negative

### CLI integration
- Extend existing `cut` subcommand — silence removal is what `cut` does
- Add `--dry-run` flag to CutArgs: prints detected silent segments and planned cuts without producing output
- Dry-run output shows: each silent segment (start-end, duration) and total time to be removed
- No new subcommands needed

### Output format
- H.264 video with libx264, CRF 23, yuv420p pixel format
- AAC audio at 192kbps
- TikTok-standard playable format
- Replace the current passthrough re-encode in cut.rs with the silence-removal filter pipeline

### FFmpeg wrapper changes
- Add a function to run silencedetect and capture stderr for parsing
- The existing `run_ffmpeg` / `run_ffmpeg_verbose` can be reused for the final encode step
- Silencedetect needs stderr captured (use Command::output() with stdout null)

### Claude's Discretion
- Exact stderr parsing implementation (regex vs string splitting)
- Whether to add `regex` crate or parse with string operations
- Internal struct naming for silence intervals
- Exact dry-run output formatting
- Whether probe_duration uses ffprobe or parses from silencedetect stderr

</decisions>

<specifics>
## Specific Ideas

- The cut command currently does a simple re-encode passthrough — replace that with the actual silence removal pipeline
- Two-stage process: (1) detect silence, (2) build filter and encode
- Spinner should update to show "Detecting silence..." then "Removing silence..."
- Success message should include how much silence was removed (e.g., "Removed 45.2s of silence")

</specifics>

<deferred>
## Deferred Ideas

- Configurable silence threshold (SIL-06, v2)
- Configurable minimum silence duration (SIL-07, v2)

</deferred>

---

*Phase: 02-silence-removal*
*Context gathered: 2026-02-20*
