# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (shipped 2026-02-20)
- **v1.2 Distribution & Docs** -- Phases 10-12 (shipped 2026-02-21)
- **v1.3 Cross-Platform** -- Phases 13-15 (shipped 2026-02-23)
- **v1.4 Silero VAD** -- Phases 16-18 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 1-5) -- SHIPPED 2026-02-20</summary>

- [x] Phase 1: Foundation (2/2 plans) -- completed 2026-02-19
- [x] Phase 2: Silence Removal (2/2 plans) -- completed 2026-02-20
- [x] Phase 3: Caption Generation (1/1 plan) -- completed 2026-02-20
- [x] Phase 4: Caption Rendering (1/1 plan) -- completed 2026-02-20
- [x] Phase 5: Overlays and Polish (2/2 plans) -- completed 2026-02-20

See: `.planning/milestones/v1.0-ROADMAP.md` for full details.

</details>

<details>
<summary>v1.1 Polish & Pipeline (Phases 6-9) -- SHIPPED 2026-02-20</summary>

- [x] Phase 6: Audit & Cleanup (3/3 plans) -- completed 2026-02-20
- [x] Phase 7: Doctor Subcommand (quick execution) -- completed 2026-02-20
- [x] Phase 8: Pipeline Subcommand (quick execution) -- completed 2026-02-20
- [x] Phase 9: CI/CD (quick execution) -- completed 2026-02-20

See: `.planning/milestones/v1.1-ROADMAP.md` for full details.

</details>

<details>
<summary>v1.2 Distribution & Docs (Phases 10-12) -- SHIPPED 2026-02-21</summary>

- [x] Phase 10: Homebrew Tap + Formula (1/1 plan) -- completed 2026-02-20
- [x] Phase 11: GitHub Actions Auto-Update (1/1 plan) -- completed 2026-02-21
- [x] Phase 12: Comprehensive README (1/1 plan) -- completed 2026-02-21

See: `.planning/milestones/v1.2-ROADMAP.md` for full details.

</details>

<details>
<summary>v1.3 Cross-Platform (Phases 13-15) -- SHIPPED 2026-02-23</summary>

- [x] Phase 13: Platform-Portable Code (1/1 plan) -- completed 2026-02-21
- [x] Phase 14: Linux & Windows CI/Release (1/1 plan) -- completed 2026-02-21
- [x] Phase 15: Cross-Platform Docs & Install (1/1 plan) -- completed 2026-02-21

See: `.planning/milestones/v1.3-ROADMAP.md` for full details.

</details>

### v1.4 Silero VAD (In Progress)

**Milestone Goal:** Replace amplitude-based silence detection with Silero VAD neural network for accurate silence and breath removal across all platforms.

- [x] **Phase 16: Build & CI Verification** - voice_activity_detector compiles and ONNX Runtime is cached on all four CI targets (completed 2026-02-24)
- [ ] **Phase 17: Core VAD Integration** - vad.rs module and shared audio helper wired into cut and pipeline commands
- [ ] **Phase 18: Tuning Flags & Cleanup** - --vad-threshold and --min-silence-ms exposed; --breaths and dead amplitude code removed

## Phase Details

### Phase 16: Build & CI Verification
**Goal**: voice_activity_detector 0.2.1 compiles on all four CI targets with ONNX Runtime cached so no build blocks the feature work
**Depends on**: Nothing (first phase of milestone)
**Requirements**: CI-01, CI-02
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds on macOS ARM64, macOS Intel, Linux x86_64, and Windows x86_64 with voice_activity_detector 0.2.1 in Cargo.toml
  2. GitHub Actions CI completes without downloading ONNX Runtime on a second run (cache hit confirmed in workflow logs)
  3. No ort version conflicts appear in `cargo tree` output
**Plans**: 1 plan
Plans:
- [ ] 16-01-PLAN.md -- Add voice_activity_detector dep, update CI/release for 4-platform ORT-cached builds

### Phase 17: Core VAD Integration
**Goal**: Users running `contentops cut` or `contentops pipeline` get silence removed via Silero VAD neural inference instead of FFmpeg silencedetect
**Depends on**: Phase 16
**Requirements**: VAD-03, VAD-01, VAD-02
**Success Criteria** (what must be TRUE):
  1. `contentops cut input.mp4 output.mp4` removes silence using VAD and produces a correctly trimmed video
  2. `contentops pipeline input.mp4 output.mp4` completes end-to-end with VAD-based silence removal
  3. The 16kHz mono WAV extraction helper is shared between VAD inference and Whisper transcription (no duplication in ffmpeg.rs)
  4. VAD produces correct speech intervals on a real talking-head video (no over-cutting of quiet speech, no under-cutting of breaths)
**Plans**: TBD

### Phase 18: Tuning Flags & Cleanup
**Goal**: Users can tune VAD sensitivity via CLI flags, and the codebase contains no dead amplitude-based detection code or the obsolete --breaths flag
**Depends on**: Phase 17
**Requirements**: VAD-04, VAD-05, CLN-01, CLN-02
**Success Criteria** (what must be TRUE):
  1. `contentops cut input.mp4 output.mp4 --vad-threshold 0.3` applies a lower speech probability threshold without error
  2. `contentops cut input.mp4 output.mp4 --min-silence-ms 600` applies a longer minimum silence duration without error
  3. `contentops cut --help` and `contentops pipeline --help` show no --breaths flag
  4. `silence.rs` contains only SpeechInterval, build_concat_filter, and adjust_timestamps -- no parse_silencedetect, silence_to_speech, filter_silences_by_words, or SilenceInterval
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 2/2 | Complete | 2026-02-19 |
| 2. Silence Removal | v1.0 | 2/2 | Complete | 2026-02-20 |
| 3. Caption Generation | v1.0 | 1/1 | Complete | 2026-02-20 |
| 4. Caption Rendering | v1.0 | 1/1 | Complete | 2026-02-20 |
| 5. Overlays and Polish | v1.0 | 2/2 | Complete | 2026-02-20 |
| 6. Audit & Cleanup | v1.1 | 3/3 | Complete | 2026-02-20 |
| 7. Doctor Subcommand | v1.1 | quick | Complete | 2026-02-20 |
| 8. Pipeline Subcommand | v1.1 | quick | Complete | 2026-02-20 |
| 9. CI/CD | v1.1 | quick | Complete | 2026-02-20 |
| 10. Homebrew Tap + Formula | v1.2 | 1/1 | Complete | 2026-02-20 |
| 11. GitHub Actions Auto-Update | v1.2 | 1/1 | Complete | 2026-02-21 |
| 12. Comprehensive README | v1.2 | 1/1 | Complete | 2026-02-21 |
| 13. Platform-Portable Code | v1.3 | 1/1 | Complete | 2026-02-21 |
| 14. Linux & Windows CI/Release | v1.3 | 1/1 | Complete | 2026-02-21 |
| 15. Cross-Platform Docs & Install | v1.3 | 1/1 | Complete | 2026-02-21 |
| 16. Build & CI Verification | 1/1 | Complete   | 2026-02-24 | - |
| 17. Core VAD Integration | v1.4 | 0/TBD | Not started | - |
| 18. Tuning Flags & Cleanup | v1.4 | 0/TBD | Not started | - |
