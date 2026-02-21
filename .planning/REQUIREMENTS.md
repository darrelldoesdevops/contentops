# Requirements: contentops

**Defined:** 2026-02-20
**Core Value:** Take a raw video file and remove dead air automatically

## v1.2 Requirements

Requirements for milestone v1.2 Distribution & Docs. Each maps to roadmap phases.

### Homebrew Tap

- [x] **BREW-01**: User can run `brew install darrelltang/tap/contentops` to install the correct architecture binary on ARM Mac
- [x] **BREW-02**: User can run `brew install darrelltang/tap/contentops` to install the correct architecture binary on Intel Mac
- [x] **BREW-03**: Formula passes `brew audit` and `brew test contentops`
- [x] **BREW-04**: Formula includes `caveats` block documenting whisper model and prerequisites

### Auto-Update

- [x] **AUTO-01**: Pushing a version tag auto-updates the tap formula version and SHA256 within minutes
- [x] **AUTO-02**: Auto-update uses cross-repo `workflow_dispatch` with PAT stored as `TAP_UPDATE_TOKEN`

### Documentation

- [x] **DOCS-01**: README includes prerequisites section (FFmpeg, whisper-cli, Claude CLI optional)
- [x] **DOCS-02**: README includes Homebrew install and direct download install paths
- [x] **DOCS-03**: README includes pipeline-first usage with copy-paste example
- [x] **DOCS-04**: README includes flag reference table for each subcommand
- [x] **DOCS-05**: README includes troubleshooting section derived from `doctor` and error hints
- [x] **DOCS-06**: All flags in README match `contentops <subcommand> --help` output exactly

## Future Requirements

### Distribution Enhancements

- **BREW-05**: Homebrew bottle distribution (requires Homebrew CI infrastructure)
- **DOCS-07**: README auto-generation from `clap` output

## Out of Scope

| Feature | Reason |
|---------|--------|
| homebrew-core submission | Requires cross-platform support; contentops is macOS-only |
| Homebrew bottles | Requires Homebrew CI infrastructure; overkill for personal tap |
| README auto-generation | Low priority for 5-subcommand tool; manual is sufficient |
| crates.io publish | Application binary, not a library; pre-built binaries have better UX |
| Linux/Windows builds | Hard-coded macOS font paths; no current users on other platforms |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BREW-01 | Phase 10 | Complete |
| BREW-02 | Phase 10 | Complete |
| BREW-03 | Phase 10 | Complete |
| BREW-04 | Phase 10 | Complete |
| AUTO-01 | Phase 11 | Complete |
| AUTO-02 | Phase 11 | Complete |
| DOCS-01 | Phase 12 | Complete |
| DOCS-02 | Phase 12 | Complete |
| DOCS-03 | Phase 12 | Complete |
| DOCS-04 | Phase 12 | Complete |
| DOCS-05 | Phase 12 | Complete |
| DOCS-06 | Phase 12 | Complete |

**Coverage:**
- v1.2 requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0

---
*Requirements defined: 2026-02-20*
*Last updated: 2026-02-20 after roadmap creation*
