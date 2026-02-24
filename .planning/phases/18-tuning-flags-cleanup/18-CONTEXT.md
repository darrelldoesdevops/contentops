# Phase 18: Tuning Flags & Cleanup - Context

**Gathered:** 2026-02-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose VAD tuning flags (`--vad-threshold`, `--min-silence-ms`) on both cut and pipeline commands. Remove the `--breaths` flag entirely. Delete all dead amplitude-based silence detection code. Add VAD health check to doctor. Update README doctor output example.

</domain>

<decisions>
## Implementation Decisions

### Flag defaults & validation
- `--vad-threshold` default: 0.5 (Silero recommended default)
- `--min-silence-ms` default: 400ms (aggressive cutting for fast-paced content)
- Both flags apply to both `cut` and `pipeline` commands
- Invalid values (out of range) rejected with clear error message -- no clamping

### --breaths removal
- Hard remove: delete the flag entirely from clap args in both cut and pipeline
- Scripts using `--breaths` will get clap's standard "unknown argument" error
- No migration hint, no deprecation warning
- Quiet update to docs -- no changelog note about removal

### Dead code cleanup
- Full delete: remove all commented-out DEPRECATED code from Phase 17, not just uncomment
- Delete `run_silencedetect()` from ffmpeg.rs entirely
- Delete `parse_silencedetect`, `silence_to_speech`, `filter_silences_by_words`, `SilenceInterval`, `total_silence_removed` from silence.rs
- Keep `total_silence_from_speeches` in silence.rs (Phase 17 addition, actively used)
- Manual audit: systematically search for all silencedetect/amplitude references and remove, don't rely on compiler warnings alone
- All amplitude-related constants (SILENCE_THRESHOLD_DB, SILENCE_MIN_DURATION, BREATH_THRESHOLD_DB, BREATH_MIN_DURATION, SPEECH_PADDING) deleted

### Doctor updates
- Add VAD health check: create a VoiceActivityDetector instance to verify ONNX Runtime initializes
- Display: "VAD (Silero V5): OK" or "VAD (Silero V5): FAILED - {error}" -- simple pass/fail matching existing doctor style
- Update README doctor output example to include the VAD check line

### Claude's Discretion
- Whether to review/remove old silencedetect-related doctor checks (if any exist beyond FFmpeg presence)
- Valid ranges for --vad-threshold (likely 0.0-1.0) and --min-silence-ms (likely > 0)
- Exact error message wording for out-of-range values
- How to structure the VAD doctor check function

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

*Phase: 18-tuning-flags-cleanup*
*Context gathered: 2026-02-24*
