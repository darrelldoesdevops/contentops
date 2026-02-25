# Requirements: contentops v1.5

**Defined:** 2026-02-25
**Core Value:** Take a raw video file and remove dead air automatically

## v1.5 Requirements

### Safe Zones

- [ ] **SZ-01**: Title overlay stays within TikTok safe area (top/bottom/side margins)
- [ ] **SZ-02**: ASS subtitle margins respect TikTok right-side icon column
- [ ] **SZ-03**: Long overlay titles clamp width to avoid overflow into icon area

### Title Approval

- [ ] **TTL-01**: Pipeline presents 2-3 Claude-generated title options for user selection
- [ ] **TTL-02**: User can edit selected title before it burns into overlay
- [ ] **TTL-03**: Non-TTY environments skip approval and use first option automatically

### Metadata

- [ ] **META-01**: Claude generates TikTok description from transcript after pipeline completes
- [ ] **META-02**: Sidecar file written next to output video with title and description
- [ ] **META-03**: Transcript fix prompt enforces exact word count to prevent timing corruption

## v2 Requirements

### Upload Automation

- **UPL-01**: Direct TikTok API upload from CLI
- **UPL-02**: Hashtag generation from transcript topics

## Out of Scope

| Feature | Reason |
|---------|--------|
| TikTok API upload | OAuth complexity, personal tool -- copy-paste is sufficient |
| Hashtag generation | Description field handles discovery; separate hashtag logic adds complexity |
| Cover image selection | TikTok auto-generates from video; manual selection not needed |
| Multiple aspect ratios | 9:16 only; TikTok standard is the only target |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SZ-01 | Phase 19 | Pending |
| SZ-02 | Phase 19 | Pending |
| SZ-03 | Phase 19 | Pending |
| TTL-01 | Phase 21 | Pending |
| TTL-02 | Phase 21 | Pending |
| TTL-03 | Phase 21 | Pending |
| META-01 | Phase 22 | Pending |
| META-02 | Phase 22 | Pending |
| META-03 | Phase 20 | Pending |

**Coverage:**
- v1.5 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-02-25*
*Last updated: 2026-02-25 after roadmap creation (phases 19-22)*
