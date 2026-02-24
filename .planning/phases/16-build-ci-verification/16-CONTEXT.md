# Phase 16: Build & CI Verification - Context

**Gathered:** 2026-02-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Add `voice_activity_detector` 0.2.1 to Cargo.toml and ensure it compiles on all four CI targets (macOS ARM64, macOS Intel, Linux x86_64, Windows x86_64) with ONNX Runtime cached. No VAD logic is wired in — this phase only proves the dependency builds everywhere.

</domain>

<decisions>
## Implementation Decisions

### Verification scope
- Full CI pass required: build + tests + clippy + tree check — ensure nothing regresses from adding the dependency
- Single PR: add voice_activity_detector dependency and CI cache changes together in one PR
- Any regression caused by adding the dependency gets fixed in this phase, not deferred

### Workflow structure
- Modify the existing CI workflow — no separate workflow file
- Update both CI (PR/push) and release workflows so both benefit from cached ORT
- All 4 platform builds run in parallel via matrix strategy (existing pattern)

### Failure handling
- All 4 platforms must pass for CI to be green — any platform failure = red
- Exception: if Windows ORT cache path is problematic, ship with 3 platforms passing and file a GitHub issue for Windows specifically
- Fix any test/build regressions in this phase before moving on

### Claude's Discretion
- ORT cache strategy: whether to include ORT path in existing Cargo cache or use a separate cache step
- ort version conflict checking: whether to add a cargo tree CI step or verify manually
- ORT download retry behavior in CI
- Cache key design and invalidation strategy

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 16-build-ci-verification*
*Context gathered: 2026-02-24*
