# Phase 23: Pipeline Reorder - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Reorder pipeline stages so silence cutting happens before transcription. Whisper runs on the cut video, producing timestamps that match the final timeline. The `adjust_timestamps` call is removed from the pipeline execution path. No new features, no changes to output format.

</domain>

<decisions>
## Implementation Decisions

### Stage order
- **D-01:** New pipeline order: scale → normalize → cut → transcribe → fix → caption → overlay
- **D-02:** Stage numbering in user-facing output updates to reflect new order

### WAV extraction
- **D-03:** Extract WAV once for VAD (from normalized, pre-cut audio). Let `caption::transcribe()` extract its own WAV from the cut mp4 — it already does this internally with identical parameters (`-ar 16000 -ac 1 -f wav`) when no pre-extracted WAV is passed
- **D-04:** Remove the `Some(&wav_path)` argument from the `caption::transcribe()` call in pipeline; pass `None` so it self-extracts from cut video

### Timestamp adjustment removal
- **D-05:** Remove the `adjust_timestamps` call (pipeline.rs lines 439-448) and the `word_data`/`adjusted`/`adjusted_words` transformation block
- **D-06:** Pass `words` directly to `finish_stages()` instead of `adjusted_words` — Whisper timestamps are already on the correct timeline

### Claude's Discretion
- Cleanup of the shared WAV lifecycle (registration, deletion) after VAD no longer shares with Whisper
- Whether to clean up the VAD WAV before or after the cut FFmpeg call

</decisions>

<specifics>
## Specific Ideas

No specific requirements — the fix is mechanically moving code blocks and removing the adjustment step.

</specifics>

<canonical_refs>
## Canonical References

No external specs — requirements are fully captured in decisions above.

### Key source files
- `src/commands/pipeline.rs` — Pipeline orchestrator, stages 1-7, `adjust_timestamps` call at lines 439-448
- `src/silence.rs` — `adjust_timestamps` function (lines 32-78), `build_concat_filter`, `total_silence_from_speeches`
- `src/commands/caption.rs` — `transcribe()` accepts optional WAV path, self-extracts at line 871 if None
- `src/vad.rs` — `run_vad()` requires 16kHz mono WAV input

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `caption::transcribe()` already has fallback WAV extraction — no new code needed for Whisper to work on cut video
- `ffmpeg::extract_16k_wav()` stays as-is for VAD's pre-cut WAV

### Established Patterns
- Pipeline stages are sequential blocks in `run_pipeline()` with stage banners (`"Stage N/7: name"`)
- Temp file lifecycle managed via `TempFileRegistry`
- `finish_stages()` handles caption + overlay + metadata (unchanged)

### Integration Points
- `caption::transcribe()` call changes from `Some(&wav_path)` to `None`
- `finish_stages()` receives `words` instead of `adjusted_words`
- No changes needed to `finish_stages`, `burn_captions`, `overlay`, or metadata generation

</code_context>

<deferred>
## Deferred Ideas

- Caption highlight style/color changes — next milestone per user request
- Dead code removal of `adjust_timestamps` from `silence.rs` — Phase 24

</deferred>

---

*Phase: 23-pipeline-reorder*
*Context gathered: 2026-03-22*
