# Requirements: contentops

**Defined:** 2026-02-20
**Core Value:** Take a raw video file and remove dead air automatically

## v1.1 Requirements

Requirements for milestone v1.1 Polish & Pipeline. Each maps to roadmap phases.

### Audit & Cleanup

- [ ] **AUDIT-01**: Codebase passes `cargo clippy -D warnings` with zero warnings
- [ ] **AUDIT-02**: All dead code removed or justified with documented reason
- [ ] **AUDIT-03**: Duplicate spinner factories extracted to shared utility
- [ ] **AUDIT-04**: Consistent AppError-based error handling across all commands
- [x] **AUDIT-05**: Written findings report delivered before any code changes

### Doctor

- [ ] **DOCT-01**: User can run `contentops doctor` to check all prerequisites
- [ ] **DOCT-02**: Doctor checks ffmpeg, ffprobe, whisper-cli, and claude on PATH with colored pass/warn/fail
- [ ] **DOCT-03**: Doctor shows per-subcommand readiness summary
- [ ] **DOCT-04**: Doctor checks minimum ffmpeg version (>= 6.0)
- [ ] **DOCT-05**: Doctor exits 0 by default, exits 1 with `--strict`
- [ ] **DOCT-06**: Commands auto-suggest `contentops doctor` when failing due to missing prerequisite
- [ ] **DOCT-07**: `require_claude()` added and enforced in overlay `--auto` path

### Pipeline

- [ ] **PIPE-01**: User can run `contentops pipeline input.mp4` to chain cut, caption, and overlay
- [ ] **PIPE-02**: Pipeline manages intermediate files in a temp directory, not working directory
- [ ] **PIPE-03**: Pipeline preserves temp directory on failure with path and recovery hint
- [ ] **PIPE-04**: Pipeline supports `--dry-run` showing planned stages without executing
- [ ] **PIPE-05**: Pipeline accepts `--model` flag for whisper model path

### CI/CD

- [ ] **CICD-01**: GitHub Actions runs fmt, clippy, and tests on push/PR
- [ ] **CICD-02**: Tag-triggered release builds ARM64 and x86_64 macOS binaries
- [ ] **CICD-03**: Release includes universal macOS binary via lipo
- [ ] **CICD-04**: Release artifacts include SHA256 checksums
- [ ] **CICD-05**: CI pipeline includes cargo-audit security check

## Future Requirements

### Audit Subcommand

- **AUDIT-06**: `contentops audit` subcommand with colored summary output
- **AUDIT-07**: `contentops audit --fix` wrapping clippy --fix + cargo fmt

### Pipeline Enhancements

- **PIPE-06**: `--keep-intermediates` flag preserves intermediate files for debugging

## Out of Scope

| Feature | Reason |
|---------|--------|
| YAML/TOML pipeline config | Shell aliases cover preset combinations; one user doesn't need config files |
| Re-entrant pipeline (resume from failed stage) | Complex state tracking; manual rerun via individual commands is acceptable |
| Linux/Windows builds | Hard-coded macOS font paths; no current users on other platforms |
| Homebrew formula | Ongoing maintenance burden; GitHub Releases download is sufficient |
| crates.io publish | Application binary, not a library; pre-built binaries have better UX |
| Auto-installing missing tools | Running brew install from within tool is surprising; print hints instead |
| In-process linting | Shell out to cargo clippy; reimplementing is massive scope |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUDIT-01 | Phase 6 | Pending |
| AUDIT-02 | Phase 6 | Pending |
| AUDIT-03 | Phase 6 | Pending |
| AUDIT-04 | Phase 6 | Pending |
| AUDIT-05 | Phase 6 | Complete |
| DOCT-01 | Phase 7 | Pending |
| DOCT-02 | Phase 7 | Pending |
| DOCT-03 | Phase 7 | Pending |
| DOCT-04 | Phase 7 | Pending |
| DOCT-05 | Phase 7 | Pending |
| DOCT-06 | Phase 7 | Pending |
| DOCT-07 | Phase 7 | Pending |
| PIPE-01 | Phase 8 | Pending |
| PIPE-02 | Phase 8 | Pending |
| PIPE-03 | Phase 8 | Pending |
| PIPE-04 | Phase 8 | Pending |
| PIPE-05 | Phase 8 | Pending |
| CICD-01 | Phase 9 | Pending |
| CICD-02 | Phase 9 | Pending |
| CICD-03 | Phase 9 | Pending |
| CICD-04 | Phase 9 | Pending |
| CICD-05 | Phase 9 | Pending |

**Coverage:**
- v1.1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0

---
*Requirements defined: 2026-02-20*
*Last updated: 2026-02-20 after roadmap creation (phases 6-9 assigned)*
