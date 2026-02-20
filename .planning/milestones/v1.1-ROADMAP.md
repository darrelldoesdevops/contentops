# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (in progress)

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

### v1.1 Polish & Pipeline (In Progress)

**Milestone Goal:** Harden the codebase, add a one-command pipeline, and ship installable binaries.

- [ ] **Phase 6: Audit & Cleanup** - Clippy-clean codebase with extracted shared utilities and consistent error handling
- [ ] **Phase 7: Doctor Subcommand** - `contentops doctor` checks all prerequisites with colored pass/warn/fail output
- [ ] **Phase 8: Pipeline Subcommand** - `contentops pipeline` chains cut, caption, and overlay in one command
- [ ] **Phase 9: CI/CD** - GitHub Actions CI on push/PR; tagged releases ship ARM64 and Intel macOS binaries

## Phase Details

### Phase 6: Audit & Cleanup
**Goal**: The codebase is clean, idiomatic, and free of accumulated debt before new features are added
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: AUDIT-01, AUDIT-02, AUDIT-03, AUDIT-04, AUDIT-05
**Success Criteria** (what must be TRUE):
  1. `cargo clippy -D warnings` passes with zero warnings and zero suppression attributes added
  2. A written findings report exists before any code changes are made
  3. Duplicate spinner factories across cut.rs, caption.rs, overlay.rs are replaced by a single shared utility
  4. All commands use AppError-based error handling with no bare `anyhow::bail!` inconsistencies
  5. Dead code is either removed or has a documented justification comment
**Plans**: 3 plans
- [ ] 06-01-PLAN.md -- Audit findings report (AUDIT-05)
- [ ] 06-02-PLAN.md -- Dead code removal + spinner extraction (AUDIT-01, AUDIT-02, AUDIT-03)
- [ ] 06-03-PLAN.md -- Consistent AppError error handling (AUDIT-04)

### Phase 7: Doctor Subcommand
**Goal**: Users can verify their environment is ready to run any contentops command before attempting video processing
**Depends on**: Phase 6
**Requirements**: DOCT-01, DOCT-02, DOCT-03, DOCT-04, DOCT-05, DOCT-06, DOCT-07
**Success Criteria** (what must be TRUE):
  1. `contentops doctor` prints colored [ok]/[warn]/[fail] status for ffmpeg, ffprobe, whisper-cli, and claude
  2. Doctor output includes a per-subcommand readiness summary (e.g., "cut: ready, caption: missing whisper-cli")
  3. Doctor checks that ffmpeg version is >= 6.0 and reports if it is not
  4. `contentops doctor` exits 0 by default; exits 1 only with `--strict`
  5. Running `contentops overlay --auto` without claude on PATH shows an error suggesting `contentops doctor`
**Plans**: TBD

### Phase 8: Pipeline Subcommand
**Goal**: Users can process a raw video through cut, caption, and overlay with a single command
**Depends on**: Phase 7
**Requirements**: PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05
**Success Criteria** (what must be TRUE):
  1. `contentops pipeline input.mp4 --model ggml-base.bin` produces a fully processed video without manual intermediate steps
  2. Intermediate files appear in a temp directory, not the working directory
  3. On pipeline failure, the temp directory is preserved and the path is printed with a recovery hint
  4. `contentops pipeline --dry-run input.mp4` prints the planned stages without executing any FFmpeg or Whisper calls
**Plans**: TBD

### Phase 9: CI/CD
**Goal**: The codebase is gated by automated checks on every push and installable macOS binaries ship on every release tag
**Depends on**: Phase 8
**Requirements**: CICD-01, CICD-02, CICD-03, CICD-04, CICD-05
**Success Criteria** (what must be TRUE):
  1. A push or PR to main triggers CI that runs fmt, clippy, tests, and cargo-audit and fails the check if any step fails
  2. Pushing a version tag triggers a release build producing separate ARM64 and x86_64 macOS binaries
  3. The GitHub Release includes a universal macOS binary built with lipo
  4. Each release artifact has a corresponding SHA256 checksum file
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 2/2 | Complete | 2026-02-19 |
| 2. Silence Removal | v1.0 | 2/2 | Complete | 2026-02-20 |
| 3. Caption Generation | v1.0 | 1/1 | Complete | 2026-02-20 |
| 4. Caption Rendering | v1.0 | 1/1 | Complete | 2026-02-20 |
| 5. Overlays and Polish | v1.0 | 2/2 | Complete | 2026-02-20 |
| 6. Audit & Cleanup | v1.1 | 0/3 | Not started | - |
| 7. Doctor Subcommand | v1.1 | 0/? | Not started | - |
| 8. Pipeline Subcommand | v1.1 | 0/? | Not started | - |
| 9. CI/CD | v1.1 | 0/? | Not started | - |
