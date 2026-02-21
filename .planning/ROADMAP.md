# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (shipped 2026-02-20)
- **v1.2 Distribution & Docs** -- Phases 10-12 (shipped 2026-02-21)
- **v1.3 Cross-Platform** -- Phases 13-15 (planned)

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

### v1.3 Cross-Platform (Planned)

- [x] **Phase 13: Platform-Portable Code** - Fix macOS-specific code paths: platform-conditional default font, Windows `/dev/null` → `NUL`, platform-aware error hints (completed 2026-02-21)
- [ ] **Phase 14: Linux & Windows CI/Release** - Add `ubuntu-latest` and `windows-latest` build matrix to release.yml, produce Linux and Windows binaries alongside macOS
- [ ] **Phase 15: Cross-Platform Docs & Install** - Update README with Linux/Windows install paths, platform-specific prerequisites, update doctor hints

## Phase Details

### Phase 13: Platform-Portable Code
**Goal**: contentops compiles and runs correctly on Linux and Windows without `--font` workaround
**Depends on**: Nothing (first phase of v1.3)
**Requirements**: XPLAT-01, XPLAT-02, XPLAT-03
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds on `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc` targets
  2. Default font path resolves to a valid system font on Linux, Windows, and macOS
  3. `normalize` command uses platform-appropriate null device (`/dev/null` on Unix, `NUL` on Windows)
  4. Error hints show platform-appropriate install commands (brew on macOS, apt/choco on Linux/Windows)
**Plans:** 1/1 plans complete
Plans:
- [ ] 13-01-PLAN.md -- Platform-conditional font, null muxer, error hints

### Phase 14: Linux & Windows CI/Release
**Goal**: Tag push produces downloadable binaries for Linux and Windows alongside existing macOS binaries
**Depends on**: Phase 13 (code must compile cross-platform before building release binaries)
**Requirements**: XPLAT-04, XPLAT-05
**Success Criteria** (what must be TRUE):
  1. `release.yml` build matrix includes `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`
  2. GitHub Release contains Linux and Windows binaries alongside existing macOS binaries
  3. CI runs tests on all three platforms
**Plans:** 1 plan
Plans:
- [ ] 14-01-PLAN.md -- Linux/Windows build jobs in release.yml + three-platform CI matrix

### Phase 15: Cross-Platform Docs & Install
**Goal**: README covers Linux and Windows users with install instructions and platform-specific prerequisites
**Depends on**: Phase 13 (portable code), Phase 14 (binaries available)
**Requirements**: XPLAT-06
**Success Criteria** (what must be TRUE):
  1. README includes Linux and Windows install paths (direct download curl/Invoke-WebRequest one-liners)
  2. Prerequisites section shows platform-specific install commands (apt, choco alongside brew)
  3. Doctor subcommand hints are platform-aware
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
