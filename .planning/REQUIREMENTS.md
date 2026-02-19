# Requirements: contentops

**Defined:** 2026-02-19
**Core Value:** Take a raw video file and remove dead air automatically

## v1 Requirements

### Pipeline Infrastructure

- [x] **PIPE-01**: User can run contentops and it detects FFmpeg on PATH, failing with a clear error if missing
- [x] **PIPE-02**: FFmpeg subprocess wrapper handles pipe safety, exit code checking, and always passes `-y -nostdin`
- [x] **PIPE-03**: Temporary files are automatically cleaned up after successful processing
- [x] **PIPE-04**: Processing errors are reported with context (which stage failed, FFmpeg stderr output)
- [ ] **PIPE-05**: User sees a progress bar during FFmpeg processing stages

### Silence Removal

- [x] **SIL-01**: User can detect silent segments in a video using `contentops process input.mp4 --remove-silence`
- [x] **SIL-02**: Silent segments are removed from both audio and video tracks using select/aselect filters
- [x] **SIL-03**: Cuts include 200-500ms margin/padding to avoid clipping word starts
- [x] **SIL-04**: User can run `--remove-silence --dry-run` to see what would be cut without modifying the video
- [x] **SIL-05**: Output is in TikTok-standard format (H.264/AAC, yuv420p, CRF 23, AAC 192kbps)

### Auto-Captioning

- [ ] **CAP-01**: User can generate captions using `contentops process input.mp4 --caption`
- [ ] **CAP-02**: Audio is extracted from video and transcribed via local Whisper with word-level timestamps
- [ ] **CAP-03**: Subtitles are burned into the video as hard subs
- [ ] **CAP-04**: Captions use karaoke-style word-by-word highlighting (ASS format with \k tags)
- [ ] **CAP-05**: Caption positioning respects TikTok safe zones (avoids top 250px and bottom 320px)

### Text Overlays

- [ ] **OVL-01**: User can add title text using `contentops process input.mp4 --overlay "Title Text"`
- [ ] **OVL-02**: User can control font, color, position, and duration of overlay text
- [ ] **OVL-03**: Overlay positioning respects TikTok safe zones

### Audio

- [ ] **AUD-01**: User can normalize audio loudness to a target LUFS value

## v2 Requirements

### Silence Removal

- **SIL-06**: User can configure silence threshold (dB) via CLI flag
- **SIL-07**: User can configure minimum silence duration via CLI flag

### Pipeline

- **PIPE-06**: User can process multiple input files in batch
- **PIPE-07**: User can define pipeline presets for common workflows
- **PIPE-08**: User can define pipeline stages via TOML config file

### Content Intelligence

- **INTL-01**: User can remove filler words (um, uh) detected via Whisper transcription

## Out of Scope

| Feature | Reason |
|---------|--------|
| GUI or web interface | CLI-only personal tool |
| Cloud/API transcription | Local Whisper only for privacy and cost |
| Resolution/aspect ratio conversion | TikTok standard only for now |
| Cross-platform testing | macOS primary, personal tool |
| Real-time preview | Batch processing only |
| Video concatenation/joining | Single input file processing |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PIPE-01 | Phase 1 | Done |
| PIPE-02 | Phase 1 | Done |
| PIPE-03 | Phase 1 | Done |
| PIPE-04 | Phase 1 | Done |
| PIPE-05 | Phase 5 | Pending |
| SIL-01 | Phase 2 | Done |
| SIL-02 | Phase 2 | Done |
| SIL-03 | Phase 2 | Done |
| SIL-04 | Phase 2 | Done |
| SIL-05 | Phase 2 | Done |
| CAP-01 | Phase 3 | Pending |
| CAP-02 | Phase 3 | Pending |
| CAP-03 | Phase 4 | Pending |
| CAP-04 | Phase 4 | Pending |
| CAP-05 | Phase 4 | Pending |
| OVL-01 | Phase 5 | Pending |
| OVL-02 | Phase 5 | Pending |
| OVL-03 | Phase 5 | Pending |
| AUD-01 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0

---
*Requirements defined: 2026-02-19*
*Last updated: 2026-02-20 after Phase 2 Plan 2 completion*
