# Phase 17: Core VAD Integration - Context

**Gathered:** 2026-02-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace FFmpeg silencedetect with Silero VAD neural inference in both `cut` and `pipeline` commands. Create a shared 16kHz mono WAV extraction helper reused by both VAD and Whisper. No new CLI flags or doctor changes -- those are Phase 18.

</domain>

<decisions>
## Implementation Decisions

### Switchover strategy
- Full replacement: VAD completely replaces silencedetect in both cut and pipeline commands -- no fallback path
- Old amplitude-based code paths should be commented out (marked deprecated) so Phase 18 knows exactly what to remove
- The `--breaths` flag is silently ignored in this phase (sole user, no deprecation warning needed); Phase 18 removes it entirely

### Speech detection behavior
- Aggressive cutting: tight speech boundaries, minimal padding -- maximize dead air removal for fast-paced talking-head content
- Remove all detected silence gaps, even very short ones -- no merging of close speech segments
- Videos are pure talking-head (no intro music, no SFX) -- VAD's speech/non-speech classification is sufficient
- Show summary stats after VAD processing: "Found X speech segments, removing Y seconds of silence"

### Shared audio extraction
- Share one WAV file: extract 16kHz mono WAV once, reuse for both VAD and Whisper in pipeline
- `cut` command uses the same shared helper (one code path for audio extraction everywhere)
- Temp WAV file registered with existing TempFileRegistry -- cleaned up when command completes

### Error handling
- Fail with clear error if ONNX Runtime fails to initialize -- no silent fallback to silencedetect
- Error with warning if VAD produces zero speech segments: "No speech detected in input" and exit with error
- Existing spinner is sufficient for VAD progress -- no VAD-specific progress indicator needed
- Doctor updates deferred to Phase 18

### Claude's Discretion
- Whether shared helper lives in ffmpeg.rs or a new module
- Whether to extract WAV as temp file or stream audio to VAD
- VAD chunk size and accumulation loop implementation details
- Exact spinner message text during VAD processing

</decisions>

<specifics>
## Specific Ideas

No specific requirements -- open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 17-core-vad-integration*
*Context gathered: 2026-02-24*
