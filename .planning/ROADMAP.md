# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (shipped 2026-02-20)
- **v1.2 Distribution & Docs** -- Phases 10-12 (shipped 2026-02-21)
- **v1.3 Cross-Platform** -- Phases 13-15 (shipped 2026-02-23)
- **v1.4 Silero VAD** -- Phases 16-18 (shipped 2026-02-25)
- **v1.5 Upload Ready** -- Phases 19-22 (in progress)

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

### v1.5 Upload Ready (In Progress)

**Milestone Goal:** Pipeline outputs everything needed to upload a TikTok -- approved title overlay, auto-generated description, sidecar file with copy-paste metadata.

- [ ] **Phase 19: Safe Zone Fixes** - Correct overlay and subtitle margins so video elements stay within TikTok's safe area
- [x] **Phase 20: Transcript Fix Hardening** - Guard caption timing against word count corruption from fix_transcription (completed 2026-02-25)
- [x] **Phase 21: Interactive Title Approval** - Present Claude-generated title options to the user before burning overlay (completed 2026-02-25)
- [x] **Phase 22: TikTok Metadata Generation** - Generate description and write sidecar file next to output video (completed 2026-02-25)

## Phase Details

### Phase 19: Safe Zone Fixes
**Goal**: Video text elements stay within TikTok's visible safe area on all devices
**Depends on**: Phase 18 (v1.4 complete)
**Requirements**: SZ-01, SZ-02, SZ-03
**Success Criteria** (what must be TRUE):
  1. Title overlay text does not overlap TikTok's right-side icon column (profile, like, comment, share)
  2. ASS subtitle text does not extend into the right icon column
  3. Long overlay titles that would overflow are clamped to stay within safe width
  4. Named constants in code document the safe zone dimensions for future changes
**Plans**: 2 plans
- [ ] 19-01-PLAN.md -- Create tiktok.rs constants, fix overlay left position, add title word wrapping
- [ ] 19-02-PLAN.md -- Fix ASS subtitle MarginR, add scale-to-fill pipeline stage

### Phase 20: Transcript Fix Hardening
**Goal**: Caption timing is protected against word count drift when fix_transcription rewrites words
**Depends on**: Phase 19
**Requirements**: META-03
**Success Criteria** (what must be TRUE):
  1. When fix_transcription returns a different word count than the original, the original words are used and a warning is printed
  2. Caption timing never silently corrupts from a word count mismatch
**Plans**: 1 plan
- [ ] 20-01-PLAN.md -- Harden fix_transcription with interactive mismatch handling, retry, and non-TTY hard fail

### Phase 21: Interactive Title Approval
**Goal**: User selects and optionally edits a Claude-generated title before it burns into the overlay
**Depends on**: Phase 20
**Requirements**: TTL-01, TTL-02, TTL-03
**Success Criteria** (what must be TRUE):
  1. During pipeline execution, the user sees 2-3 Claude-generated title options and can pick one with arrow keys
  2. After picking a title, the user can type a custom edit before the overlay encodes
  3. Running pipeline in a non-TTY environment (CI, script) auto-selects the first title without prompting
  4. The spinner clears before the prompt renders and the prompt is not corrupted by indicatif output
**Plans**: 1 plan
- [ ] 21-01-PLAN.md -- Add --no-interactive flag, refactor title generation to multi-option, add interactive approval with dialoguer, integrate into pipeline

### Phase 22: TikTok Metadata Generation
**Goal**: Pipeline writes a sidecar file with copy-paste-ready title and description next to the output video
**Depends on**: Phase 21
**Requirements**: META-01, META-02
**Success Criteria** (what must be TRUE):
  1. After pipeline completes, a `{stem}_tiktok.json` file exists next to the output video
  2. The sidecar contains the approved title, a Claude-generated TikTok description, and hashtags
  3. The description is generated from the transcript and stays within TikTok's 4,000 character limit
**Plans**: 1 plan
- [ ] 22-01-PLAN.md -- Add metadata generation and sidecar file writing to pipeline

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
| 19. Safe Zone Fixes | v1.5 | 0/TBD | Not started | - |
| 20. Transcript Fix Hardening | 1/1 | Complete    | 2026-02-25 | - |
| 21. Interactive Title Approval | v1.5 | 1/1 | Complete | 2026-02-25 |
| 22. TikTok Metadata Generation | v1.5 | 1/1 | Complete | 2026-02-25 |
