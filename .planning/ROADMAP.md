# Roadmap: contentops

## Milestones

- **v1.0 MVP** -- Phases 1-5 (shipped 2026-02-20)
- **v1.1 Polish & Pipeline** -- Phases 6-9 (shipped 2026-02-20)
- **v1.2 Distribution & Docs** -- Phases 10-12 (in progress)

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

### v1.2 Distribution & Docs (In Progress)

- [x] **Phase 10: Homebrew Tap + Formula** - Create `darrelldoesdevops/homebrew-tap` repo with architecture-conditional formula; verify `brew install` works on ARM and Intel (completed 2026-02-20)
- [ ] **Phase 11: GitHub Actions Auto-Update** - Wire cross-repo `workflow_dispatch` so pushing a version tag auto-updates the tap formula within minutes
- [ ] **Phase 12: Comprehensive README** - Write `README.md` from live `--help` output: prerequisites, Homebrew install, pipeline-first usage, flag reference tables, troubleshooting

## Phase Details

### Phase 10: Homebrew Tap + Formula
**Goal**: Users can install contentops on ARM and Intel Macs via `brew install darrelldoesdevops/tap/contentops`
**Depends on**: Nothing (first phase of v1.2; release assets already exist from v1.1)
**Requirements**: BREW-01, BREW-02, BREW-03, BREW-04
**Success Criteria** (what must be TRUE):
  1. `brew install darrelldoesdevops/tap/contentops` completes without error on ARM Mac and installs an ARM64 binary
  2. `brew install darrelldoesdevops/tap/contentops` completes without error on Intel Mac and installs an x86_64 binary
  3. `brew audit Formula/contentops.rb` passes with no errors or warnings
  4. `brew test contentops` passes (runs `contentops --version` successfully)
  5. Post-install `brew info contentops` displays caveats documenting whisper model and prerequisites
**Plans:** 1/1 plans complete
Plans:
- [ ] 10-01-PLAN.md — Create homebrew-tap repo with architecture-conditional formula, verify brew install/audit/test

### Phase 11: GitHub Actions Auto-Update
**Goal**: Pushing a version tag to contentops automatically updates the tap formula version and SHA256 within minutes, requiring zero manual steps
**Depends on**: Phase 10 (tap repo and formula with sentinel comments must exist before automation can patch them)
**Requirements**: AUTO-01, AUTO-02
**Success Criteria** (what must be TRUE):
  1. Pushing `git push v*` tag triggers `update-tap` job in contentops `release.yml` after the release job completes
  2. Tap repo receives a commit with updated version string and correct SHA256 for both architectures within minutes of the GitHub Release completing
  3. `brew update && brew upgrade contentops` after a new release installs the new version with no manual formula edits
**Plans**: TBD

### Phase 12: Comprehensive README
**Goal**: A user landing on the contentops repo can understand what the tool does, install it, and run their first pipeline command without consulting any other source
**Depends on**: Phase 10 (Homebrew install path must be live before documenting it), Phase 11 (auto-update wired so README install instructions stay valid)
**Requirements**: DOCS-01, DOCS-02, DOCS-03, DOCS-04, DOCS-05, DOCS-06
**Success Criteria** (what must be TRUE):
  1. README prerequisites section lists FFmpeg, whisper-cli, and Claude CLI (optional) with install commands
  2. README includes both `brew install darrelldoesdevops/tap/contentops` and direct-download install paths as copy-paste one-liners
  3. README leads with a complete `contentops pipeline` example a user can copy-paste and run immediately
  4. README contains a flag reference table for each subcommand; every flag matches `contentops <subcommand> --help` output exactly with no undocumented or phantom flags
  5. README troubleshooting section maps common failure messages from `contentops doctor` and error hints to resolution steps
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
| 10. Homebrew Tap + Formula | 1/1 | Complete    | 2026-02-20 | - |
| 11. GitHub Actions Auto-Update | v1.2 | 0/? | Not started | - |
| 12. Comprehensive README | v1.2 | 0/? | Not started | - |
