# Requirements: contentops

**Defined:** 2026-03-22
**Core Value:** Take a raw video file and remove dead air automatically

## v1.7 Requirements

Requirements for Pipeline Reorder milestone.

### Pipeline

- [x] **PIPE-01**: Pipeline runs cut (silence removal) before transcription so Whisper timestamps match the cut video timeline
- [x] **PIPE-02**: `adjust_timestamps` logic removed from pipeline — no longer needed when transcription runs on cut audio
- [x] **PIPE-03**: Caption highlight tracks spoken words accurately without boundary drift or timestamp clamping artifacts

### Cleanup

- [ ] **CLN-01**: Dead `adjust_timestamps` function and monotonicity clamping code removed from `silence.rs`

## Out of Scope

| Feature | Reason |
|---------|--------|
| Caption highlight style changes | Deferred to next milestone per user request |
| New pipeline stages | Reorder only, no new functionality |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PIPE-01 | Phase 23 | Complete |
| PIPE-02 | Phase 23 | Complete |
| PIPE-03 | Phase 23 | Complete |
| CLN-01 | Phase 24 | Pending |

**Coverage:**
- v1.7 requirements: 4 total
- Mapped to phases: 4
- Unmapped: 0

---
*Requirements defined: 2026-03-22*
*Last updated: 2026-03-22 after roadmap creation*
