# Phase 3: Caption Generation - Context

**Gathered:** 2026-02-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Whisper integration producing word-level subtitle files from video input. User runs `contentops caption input.mp4` and gets a subtitle file (SRT) with per-word timestamps. This phase ONLY generates subtitles — burning them into video is Phase 4.

</domain>

<decisions>
## Implementation Decisions

### Whisper approach: shell out to whisper-cpp CLI (not whisper-rs)
- **whisper-rs v0.15.1**: GitHub repo archived (moved to Codeberg), word-level timestamps marked "EXPERIMENTAL" via DTW, binding compilation issues reported
- **whisper.cpp CLI**: available via `brew install whisper-cpp` (v1.8.3), stable, actively maintained, supports word-level timestamps via `--max-len 1` and `--dtw` flags
- **Decision**: shell out to whisper-cpp binary (same pattern as FFmpeg — consistent architecture)
- Detection: `which::which("whisper-cpp")` with actionable error like FFmpeg detection
- Model files: user provides path via `--model` flag, default to `base.en` model name

### CLI subcommand
- `contentops caption input.mp4` — new subcommand following established verb pattern
- Output: `input_captioned.srt` alongside input (matches `_cut.mp4` naming pattern)
- `-o` flag overrides output path
- `--model` flag to specify whisper model path (required — no bundled models)
- `--lang` flag defaults to `en`

### Pipeline stages
1. **Audio extraction**: FFmpeg extracts audio to 16kHz mono WAV (Whisper's required input format)
   - `ffmpeg -i input.mp4 -ar 16000 -ac 1 -f wav temp.wav`
   - Uses existing `ffmpeg::run_ffmpeg()` wrapper
   - Temp WAV file registered with TempFileRegistry for cleanup
2. **Transcription**: Shell out to whisper-cpp with word-level timestamps
   - `whisper-cpp -m model.bin -f temp.wav --output-json --max-len 1 -l en`
   - Parse JSON output for word-level timing data
   - Each token has start/end timestamps in the JSON output
3. **SRT generation**: Convert word timestamps to SRT subtitle format
   - Group words into display lines (3-5 words per subtitle entry)
   - Each entry has start time of first word, end time of last word
   - Standard SRT format for maximum compatibility (Phase 4 can convert to ASS for karaoke)

### Word-level timestamp strategy
- whisper-cpp `--max-len 1` forces one-word-per-segment output with timestamps
- Alternative: `--dtw` flag for more accurate token-level timing (available in whisper-cpp >1.0.55)
- Parse the JSON output which contains per-token timing data
- Store word-level data in internal struct even though SRT groups words (Phase 4 needs individual word times for karaoke)

### Output format
- SRT file as primary output (universal subtitle format)
- Also write a `.json` sidecar with raw word-level timestamps for Phase 4 consumption
- JSON sidecar contains: `[{word, start, end}, ...]` array
- This decouples caption generation from rendering — Phase 4 reads the JSON, not re-running Whisper

### Error handling
- whisper-cpp not found: actionable error with `brew install whisper-cpp` hint
- Model file not found: clear error with download instructions
- Transcription failure: capture whisper-cpp stderr, report with stage context
- Empty transcription: warn but don't error (video might have no speech)

### Claude's Discretion
- Exact word grouping algorithm for SRT entries (3-5 words, break on punctuation)
- JSON sidecar schema details
- Whether to validate WAV extraction succeeded before proceeding to Whisper
- Spinner messages during each stage

</decisions>

<specifics>
## Specific Ideas

- Reuse derive_output_path from cut.rs with "captioned" suffix for .srt output
- Follow same spinner pattern as cut command for each stage
- whisper-cpp model detection: check common paths like `~/.local/share/whisper-cpp/` or require explicit `--model` flag

</specifics>

<deferred>
## Deferred Ideas

- ASS format output with karaoke tags — Phase 4 concern
- Burning subtitles into video — Phase 4 concern
- Filler word removal using transcription — v2 feature (INTL-01)
- Model auto-download — out of scope, user manages models

</deferred>

---

*Phase: 03-caption-generation*
*Context gathered: 2026-02-20*
