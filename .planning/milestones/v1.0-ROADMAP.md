# Roadmap: contentops

## Overview

Build a Rust CLI that replaces CapCut by orchestrating FFmpeg for automated video processing. Start with the FFmpeg subprocess foundation, then deliver the core value (silence removal) as a usable tool, then layer on captioning (generation then rendering), and finish with overlays and polish. After Phase 2, the tool ships real value -- every subsequent phase is additive.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - CLI skeleton, FFmpeg wrapper, error handling, temp file lifecycle
- [x] **Phase 2: Silence Removal** - Core value: detect and remove silence from video with clean cuts
- [x] **Phase 3: Caption Generation** - Whisper integration producing word-level subtitle files
- [x] **Phase 4: Caption Rendering** - Burn styled karaoke captions into video within safe zones
- [x] **Phase 5: Overlays and Polish** - Text overlays, audio normalization, progress feedback

## Phase Details

### Phase 1: Foundation
**Goal**: User can invoke the CLI, it validates FFmpeg is available, and the subprocess/error/temp-file infrastructure is solid enough to build every pipeline stage on top of
**Depends on**: Nothing (first phase)
**Requirements**: PIPE-01, PIPE-02, PIPE-03, PIPE-04
**Success Criteria** (what must be TRUE):
  1. Running `contentops` without FFmpeg installed prints a clear error naming FFmpeg and exits non-zero
  2. Running `contentops process input.mp4 --remove-silence` with a valid file invokes FFmpeg with `-y -nostdin` flags and reports FFmpeg's exit code on failure
  3. Temporary files created during processing are cleaned up after both successful and failed runs
  4. When FFmpeg fails, the error message includes which pipeline stage failed and relevant FFmpeg stderr output
**Plans**: 2 plans

Plans:
- [x] 01-01-PLAN.md -- Project bootstrap, CLI skeleton, error types, FFmpeg detection
- [x] 01-02-PLAN.md -- FFmpeg subprocess wrapper, temp file lifecycle, spinner, cut command

### Phase 2: Silence Removal
**Goal**: User can take a raw video and get back a jump-cut version with dead air removed -- the tool delivers its core value and is usable from this point forward
**Depends on**: Phase 1
**Requirements**: SIL-01, SIL-02, SIL-03, SIL-04, SIL-05
**Success Criteria** (what must be TRUE):
  1. Running `contentops cut input.mp4` produces an output video with silent segments removed from both audio and video tracks
  2. Cuts have 200-500ms padding so words are not clipped at segment boundaries
  3. Running `contentops cut input.mp4 --dry-run` prints detected silent segments and what would be cut, without producing an output file
  4. Output video is H.264/AAC with yuv420p pixel format, playable on TikTok without re-encoding
  5. Audio and video remain in sync throughout the output (no drift from cutting)
**Plans**: 2 plans

Plans:
- [x] 02-01-PLAN.md -- Silence detection parsing, padding logic, filter expression building (TDD)
- [x] 02-02-PLAN.md -- FFmpeg pipeline integration, dry-run, TikTok-standard output

### Phase 3: Caption Generation
**Goal**: User can extract speech from a video as word-level timestamped subtitles via local Whisper
**Depends on**: Phase 1
**Requirements**: CAP-01, CAP-02
**Success Criteria** (what must be TRUE):
  1. Running `contentops process input.mp4 --caption` extracts audio and produces a transcription with word-level timestamps
  2. Transcription runs locally via Whisper (no cloud API calls)
  3. Generated subtitle data includes per-word start/end times accurate enough for karaoke rendering
**Plans**: 1 plan

Plans:
- [x] 03-01-PLAN.md -- Caption subcommand, whisper-cpp integration, SRT + JSON output

### Phase 4: Caption Rendering
**Goal**: User gets a video with styled, animated captions burned directly into the frame, positioned within TikTok safe zones
**Depends on**: Phase 3
**Requirements**: CAP-03, CAP-04, CAP-05
**Success Criteria** (what must be TRUE):
  1. Running `contentops process input.mp4 --caption` produces a video with hard-burned subtitles visible in the output
  2. Captions use karaoke-style word-by-word highlighting (words light up as they are spoken)
  3. Caption text is positioned within TikTok safe zones (not obscured by the top status bar or bottom UI controls)
**Plans**: 1 plan

Plans:
- [x] 04-01-PLAN.md -- ASS karaoke subtitle generation and FFmpeg burn pipeline with TikTok safe zones

### Phase 5: Overlays and Polish
**Goal**: User can add text overlays, normalize audio loudness, and see progress during processing -- completing the full feature set
**Depends on**: Phase 1
**Requirements**: OVL-01, OVL-02, OVL-03, AUD-01, PIPE-05
**Success Criteria** (what must be TRUE):
  1. Running `contentops process input.mp4 --overlay "Title Text"` burns the specified text into the video
  2. User can control overlay font, color, position, and duration via CLI flags
  3. Overlay text is positioned within TikTok safe zones by default
  4. Running `contentops process input.mp4 --normalize` adjusts audio loudness to a target LUFS value
  5. User sees a progress indicator during FFmpeg processing stages
**Plans**: 2 plans

Plans:
- [x] 05-01-PLAN.md -- Overlay and normalize subcommands
- [x] 05-02-PLAN.md -- Progress bar upgrade with ffprobe duration detection

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 2/2 | Complete | 2026-02-19 |
| 2. Silence Removal | 2/2 | Complete | 2026-02-20 |
| 3. Caption Generation | 1/1 | Complete | 2026-02-20 |
| 4. Caption Rendering | 1/1 | Complete | 2026-02-20 |
| 5. Overlays and Polish | 2/2 | Complete | 2026-02-20 |
