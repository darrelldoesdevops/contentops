# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (shipped 2026-02-20)
- **v1.2 Distribution & Docs** -- Phases 10-12 (shipped 2026-02-21)
- **v1.3 Cross-Platform** -- Phases 13-15 (shipped 2026-02-23)
- **v1.4 Silero VAD** -- Phases 16-18 (shipped 2026-02-25)
- **v1.5 Upload Ready** -- Phases 19-22 (shipped 2026-02-25)
- **v1.7 Pipeline Reorder** -- Phases 23-24 (active)

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

<details>
<summary>v1.4 Silero VAD (Phases 16-18) -- SHIPPED 2026-02-25</summary>

- [x] Phase 16: Build & CI Verification (1/1 plan) -- completed 2026-02-24
- [x] Phase 17: Core VAD Integration (2/2 plans) -- completed 2026-02-24
- [x] Phase 18: Tuning Flags & Cleanup (2/2 plans) -- completed 2026-02-24

See: `.planning/milestones/v1.4-ROADMAP.md` for full details.

</details>

<details>
<summary>v1.5 Upload Ready (Phases 19-22) -- SHIPPED 2026-02-25</summary>

- [x] Phase 19: Safe Zone Fixes (2/2 plans) -- completed 2026-02-25
- [x] Phase 20: Transcript Fix Hardening (1/1 plan) -- completed 2026-02-25
- [x] Phase 21: Interactive Title Approval (1/1 plan) -- completed 2026-02-25
- [x] Phase 22: TikTok Metadata Generation (1/1 plan) -- completed 2026-02-25

See: `.planning/milestones/v1.5-ROADMAP.md` for full details.

</details>

### v1.7 Pipeline Reorder

- [x] **Phase 23: Pipeline Reorder** - Reorder pipeline so cut runs before transcription; remove adjust_timestamps call (completed 2026-03-23)
- [ ] **Phase 24: Dead Code Removal** - Excise adjust_timestamps function and monotonicity clamping from silence.rs

## Phase Details

### Phase 23: Pipeline Reorder
**Goal**: The pipeline runs cut before transcription so Whisper timestamps align with the final video and captions highlight correctly
**Depends on**: Nothing (first phase of milestone)
**Requirements**: PIPE-01, PIPE-02, PIPE-03
**Success Criteria** (what must be TRUE):
  1. Running `contentops pipeline` on a raw video produces a captioned output where highlighted words match speech timing with no visible drift
  2. The pipeline stage order is: scale -> normalize -> cut -> transcribe -> fix -> caption -> overlay
  3. No call to `adjust_timestamps` appears in the pipeline execution path
  4. Caption word boundaries on a real talking-head video do not creep ahead of or behind actual speech
**Plans**: 1 plan
Plans:
- [x] 23-01-PLAN.md -- Reorder run_stages() so cut runs before transcribe, remove adjust_timestamps call

### Phase 24: Dead Code Removal
**Goal**: The `adjust_timestamps` function and monotonicity clamping code are gone from the codebase
**Depends on**: Phase 23
**Requirements**: CLN-01
**Success Criteria** (what must be TRUE):
  1. `cargo clippy -- -D warnings` passes with zero warnings (no dead_code lint suppression needed)
  2. Searching the codebase for `adjust_timestamps` returns no results
  3. Monotonicity clamping logic no longer exists in `silence.rs`
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
| 16. Build & CI Verification | v1.4 | 1/1 | Complete | 2026-02-24 |
| 17. Core VAD Integration | v1.4 | 2/2 | Complete | 2026-02-24 |
| 18. Tuning Flags & Cleanup | v1.4 | 2/2 | Complete | 2026-02-24 |
| 19. Safe Zone Fixes | v1.5 | 2/2 | Complete | 2026-02-25 |
| 20. Transcript Fix Hardening | v1.5 | 1/1 | Complete | 2026-02-25 |
| 21. Interactive Title Approval | v1.5 | 1/1 | Complete | 2026-02-25 |
| 22. TikTok Metadata Generation | v1.5 | 1/1 | Complete | 2026-02-25 |
| 23. Pipeline Reorder | v1.7 | 1/1 | Complete    | 2026-03-23 |
| 24. Dead Code Removal | v1.7 | 0/1 | Not started | - |
