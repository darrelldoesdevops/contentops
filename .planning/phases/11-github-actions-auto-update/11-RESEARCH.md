# Phase 11: GitHub Actions Auto-Update - Research

**Researched:** 2026-02-21
**Domain:** GitHub Actions cross-repo workflow_dispatch, sed-based formula patching
**Confidence:** HIGH

## Summary

Phase 11 adds an `update-tap` job to `release.yml` in contentops that triggers a `workflow_dispatch` in `darrelldoesdevops/homebrew-tap` after the release job completes. The tap workflow checks out the formula, patches all 5 sentinel-marked fields using sed, commits, and pushes. No third-party actions are needed — `gh workflow run`, `sed`, and the GitHub API provide everything required.

The sentinel pattern from Phase 10 is already in place and fully tested. All 5 sed patterns (`VERSION`, `ARM-URL`, `ARM-SHA256`, `INTEL-URL`, `INTEL-SHA256`) have been verified against the live formula. SHA256 values can be fetched directly from the `asset.digest` field in the GitHub API release response (confirmed available — no sidecar file download needed).

The locked prior decision requires a classic PAT (`TAP_UPDATE_TOKEN`) with `repo` + `workflow` scopes. The GitHub REST API docs confirm that fine-grained PATs with `Actions: write` also support `workflow_dispatch`, but that is noted for information only — the classic PAT decision is locked.

**Primary recommendation:** Two-workflow pattern. Add `update-tap` job to `release.yml` (needs: release) that calls `gh workflow run update-tap.yml -f version=... -R darrelldoesdevops/homebrew-tap`. In homebrew-tap, create `.github/workflows/update-tap.yml` that accepts `version` input, fetches SHA256 from `asset.digest`, runs 5 sed commands, commits, and pushes.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AUTO-01 | Pushing a version tag auto-updates the tap formula version and SHA256 within minutes | `update-tap` job in release.yml with `needs: release` triggers on tag push; tap workflow patches formula via sed and pushes commit within ~2 min of release job completing |
| AUTO-02 | Auto-update uses cross-repo `workflow_dispatch` with PAT stored as `TAP_UPDATE_TOKEN` | `gh workflow run update-tap.yml -R darrelldoesdevops/homebrew-tap` with `GH_TOKEN: ${{ secrets.TAP_UPDATE_TOKEN }}`; classic PAT with `repo` + `workflow` scopes required by locked prior decision |
</phase_requirements>

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| `gh workflow run` | gh CLI (pre-installed on GitHub runners) | Cross-repo workflow_dispatch trigger | Official GitHub CLI; no extra installation needed on ubuntu-latest |
| `sed` (GNU) | pre-installed on ubuntu-latest | In-place formula patching | GNU sed on ubuntu-latest uses `sed -i` without `''` suffix; all 5 patterns verified |
| `gh api` | gh CLI | Fetch SHA256 from release `asset.digest` | Single API call returns `sha256:...` for each asset; avoids downloading sidecar files |
| `actions/checkout@v4` | v4 | Clone homebrew-tap in tap workflow | Standard; use `token:` parameter with TAP_UPDATE_TOKEN for push permission |
| `softprops/action-gh-release@v2` | v2 | Already in release.yml | No change needed; assets uploaded before update-tap job runs |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `git config user.name/email` | n/a | Bot identity for formula commit | Required in tap workflow before `git commit`; use generic bot identity |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `gh workflow run` | `curl -X POST api.github.com/.../dispatches` | gh CLI is simpler, already authenticated via GH_TOKEN env var |
| `asset.digest` field | Download `.sha256` sidecar file | Sidecar download required redirect handling in Phase 10; `asset.digest` is simpler and already confirmed working |
| 5 separate `sed -i` commands | Python/Ruby script | sed chosen in prior decision as simpler and readable |

## Architecture Patterns

### Recommended Project Structure

```
contentops (main repo)
└── .github/workflows/
    └── release.yml           # ADD: update-tap job (needs: release)

darrelldoesdevops/homebrew-tap
└── .github/workflows/
    └── update-tap.yml        # NEW: accepts version input, patches formula, pushes
```

### Pattern 1: Two-Job Cross-Repo Dispatch

**What:** `release.yml` gets a new `update-tap` job that depends on `release` and dispatches to the tap repo.

**When to use:** Whenever a release completes and the formula must be updated automatically.

**Example — addition to release.yml:**
```yaml
  update-tap:
    name: Update Homebrew Tap
    needs: release
    runs-on: ubuntu-latest
    steps:
      - name: Trigger tap update
        run: |
          gh workflow run update-tap.yml \
            -f version="${GITHUB_REF_NAME#v}" \
            -R darrelldoesdevops/homebrew-tap
        env:
          GH_TOKEN: ${{ secrets.TAP_UPDATE_TOKEN }}
```

Notes:
- `GITHUB_REF_NAME` is the tag name (e.g. `v1.2.0`); `${GITHUB_REF_NAME#v}` strips the `v` prefix to produce `1.2.0`
- The formula stores version without `v` (e.g. `1.2.0`), matching this strip
- `GH_TOKEN` (not `GITHUB_TOKEN`) is used intentionally — `GITHUB_TOKEN` cannot trigger workflows in other repos

### Pattern 2: Tap Workflow with sed Patching

**What:** `update-tap.yml` in homebrew-tap accepts `version` input, fetches SHA256 via API, patches formula with sed, commits, pushes.

**Example — full update-tap.yml:**
```yaml
name: Update Formula

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version (without v prefix, e.g. 1.2.0)'
        required: true
        type: string

jobs:
  update-formula:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

      - name: Fetch SHA256 from release
        id: sha256
        run: |
          ARM_SHA=$(gh api repos/darrelldoesdevops/contentops/releases/tags/v${{ inputs.version }} \
            --jq '.assets[] | select(.name == "contentops-aarch64-apple-darwin") | .digest | ltrimstr("sha256:")')
          INTEL_SHA=$(gh api repos/darrelldoesdevops/contentops/releases/tags/v${{ inputs.version }} \
            --jq '.assets[] | select(.name == "contentops-x86_64-apple-darwin") | .digest | ltrimstr("sha256:")')
          echo "arm_sha=$ARM_SHA" >> "$GITHUB_OUTPUT"
          echo "intel_sha=$INTEL_SHA" >> "$GITHUB_OUTPUT"
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Patch formula
        run: |
          VERSION="${{ inputs.version }}"
          ARM_SHA="${{ steps.sha256.outputs.arm_sha }}"
          INTEL_SHA="${{ steps.sha256.outputs.intel_sha }}"
          F="Formula/contentops.rb"

          sed -i "s|\(# current version: \)[^ ]* \(=== AUTO-UPDATE: VERSION ===\)|\1${VERSION} \2|" "$F"
          sed -i "s|\(url \"https://github\.com/darrelldoesdevops/contentops/releases/download/v\)[^/]*\(/contentops-aarch64-apple-darwin\" # === AUTO-UPDATE: ARM-URL ===\)|\1${VERSION}\2|" "$F"
          sed -i "s|\(sha256 \"\)[^\"]*\(\" # === AUTO-UPDATE: ARM-SHA256 ===\)|\1${ARM_SHA}\2|" "$F"
          sed -i "s|\(url \"https://github\.com/darrelldoesdevops/contentops/releases/download/v\)[^/]*\(/contentops-x86_64-apple-darwin\" # === AUTO-UPDATE: INTEL-URL ===\)|\1${VERSION}\2|" "$F"
          sed -i "s|\(sha256 \"\)[^\"]*\(\" # === AUTO-UPDATE: INTEL-SHA256 ===\)|\1${INTEL_SHA}\2|" "$F"

      - name: Commit and push
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/contentops.rb
          git commit -m "chore: update contentops formula to v${{ inputs.version }}"
          git push
```

Notes:
- `GITHUB_TOKEN` in the tap workflow has `contents: write` via `permissions:` — sufficient to push to homebrew-tap main branch
- `GH_TOKEN` in the SHA256 fetch step uses `GITHUB_TOKEN` because contentops is a public repo; no cross-repo PAT needed for reading public release data
- The tap workflow does NOT need `TAP_UPDATE_TOKEN` — only the contentops `release.yml` needs it to trigger the dispatch

### Pattern 3: SHA256 via asset.digest

**What:** GitHub API release response includes `digest` field (`sha256:<hex>`) for each asset. Strip prefix with `ltrimstr`.

**Verified working (live test against v1.1.0):**
```bash
gh api repos/darrelldoesdevops/contentops/releases/tags/v1.1.0 \
  --jq '.assets[] | select(.name == "contentops-aarch64-apple-darwin") | .digest | ltrimstr("sha256:")'
# Output: ec58e2d8106c84de25ae20641a060cbf85a91bb7cab4f0f60f27577f8333f0ba
```

Confidence: HIGH — confirmed via live `gh api` call. The `digest` field was absent in older releases but is present for v1.1.0 onward. New releases will have it automatically.

### Anti-Patterns to Avoid

- **Using `GITHUB_TOKEN` for cross-repo dispatch:** `GITHUB_TOKEN` is scoped to the current repo; it cannot trigger workflows in `homebrew-tap`. Must use `TAP_UPDATE_TOKEN`.
- **Running update-tap on ubuntu-latest without GNU sed awareness:** macOS `sed` needs `-i ''`; ubuntu-latest (GNU sed) needs `-i` only. Since tap workflow runs on ubuntu-latest, use `sed -i` without the empty string.
- **Triggering update-tap before release job completes:** The `needs: release` dependency ensures assets are uploaded before SHA256 fetch.
- **Stripping `v` in the wrong place:** `GITHUB_REF_NAME` = `v1.2.0`; the formula stores `1.2.0`. Strip once in release.yml when passing input; tap workflow uses the version as-is.
- **Using `sed -i` on an expression that modifies the wrong line:** The sentinel comments are unique per line — but both `sha256` lines use the same field name. The sed pattern must include the sentinel text to avoid matching the wrong line.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SHA256 computation | Download binary and run `shasum` | `asset.digest` from GitHub API | Digest is computed at upload time by GitHub; no download needed |
| Formula version parsing | regex on formula to read current version | Pass version as workflow input | Source of truth is the git tag, not the formula |
| Cross-repo authentication | Custom curl POST to dispatches endpoint | `gh workflow run` with `GH_TOKEN` env var | gh CLI handles auth, JSON, and error codes; simpler |

**Key insight:** The GitHub API already has the SHA256. Don't download assets to compute it.

## Common Pitfalls

### Pitfall 1: `GITHUB_TOKEN` Cannot Trigger Cross-Repo Workflows

**What goes wrong:** Setting `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}` in the `update-tap` job fails silently or with 403 because `GITHUB_TOKEN` is scoped to the current repo.

**Why it happens:** `GITHUB_TOKEN` has no permissions on `darrelldoesdevops/homebrew-tap`.

**How to avoid:** Use `GH_TOKEN: ${{ secrets.TAP_UPDATE_TOKEN }}` (the classic PAT). The secret must be stored in the contentops repo settings.

**Warning signs:** `gh workflow run` exits with "HTTP 422" or "Resource not accessible by integration".

### Pitfall 2: Missing `workflow` Scope on Classic PAT

**What goes wrong:** Classic PAT with only `repo` scope cannot trigger workflow_dispatch in another repo.

**Why it happens:** The `workflow` scope is required in addition to `repo` for classic PATs to dispatch workflow events.

**How to avoid:** Create classic PAT with both `repo` and `workflow` scopes. The `workflow` scope is separate from `repo` in the classic PAT UI.

**Warning signs:** `gh workflow run` returns 403 despite correct `GH_TOKEN`.

### Pitfall 3: `asset.digest` Null for Old Releases

**What goes wrong:** `.digest` is `null` for assets uploaded before GitHub added this field (~mid-2025 rollout).

**Why it happens:** GitHub only computes digests for newly uploaded assets.

**How to avoid:** v1.1.0 and all future releases will have the field. If a release predates the feature, fall back to the `.sha256` sidecar file via API asset download. For Phase 11, all new releases will have `asset.digest`.

**Warning signs:** `ltrimstr("sha256:")` returns empty string; ARM_SHA or INTEL_SHA is blank.

### Pitfall 4: Version Prefix Mismatch

**What goes wrong:** Formula has `1.2.0` but sed gets `v1.2.0`, leaving the sentinel unpatched.

**Why it happens:** `GITHUB_REF_NAME` = `v1.2.0`; formula stores without `v`.

**How to avoid:** Strip the `v` prefix once: `${GITHUB_REF_NAME#v}`. Pass the stripped value as the `version` input. Do not strip again in the tap workflow.

**Warning signs:** Formula diff shows no changes after the update-tap job completes.

### Pitfall 5: Push Fails Due to Missing `contents: write` Permission

**What goes wrong:** `git push` in the tap workflow returns 403.

**Why it happens:** Default workflow token permissions may be `read-only` depending on org/repo settings.

**How to avoid:** Add `permissions: contents: write` to the `update-formula` job in `update-tap.yml`.

**Warning signs:** `remote: Permission to darrelldoesdevops/homebrew-tap.git denied`.

## Code Examples

Verified patterns from live testing:

### Extract SHA256 from asset.digest (verified against v1.1.0)

```bash
# ARM
gh api repos/darrelldoesdevops/contentops/releases/tags/v1.1.0 \
  --jq '.assets[] | select(.name == "contentops-aarch64-apple-darwin") | .digest | ltrimstr("sha256:")'

# Intel
gh api repos/darrelldoesdevops/contentops/releases/tags/v1.1.0 \
  --jq '.assets[] | select(.name == "contentops-x86_64-apple-darwin") | .digest | ltrimstr("sha256:")'
```

### All 5 sed Patches (verified against live formula)

```bash
F="Formula/contentops.rb"
VERSION="1.2.0"
ARM_SHA="<64-char hex>"
INTEL_SHA="<64-char hex>"

# VERSION sentinel: "# current version: 1.1.0 === AUTO-UPDATE: VERSION ==="
sed -i "s|\(# current version: \)[^ ]* \(=== AUTO-UPDATE: VERSION ===\)|\1${VERSION} \2|" "$F"

# ARM-URL sentinel: url "...v1.1.0/contentops-aarch64-apple-darwin" # === AUTO-UPDATE: ARM-URL ===
sed -i "s|\(url \"https://github\.com/darrelldoesdevops/contentops/releases/download/v\)[^/]*\(/contentops-aarch64-apple-darwin\" # === AUTO-UPDATE: ARM-URL ===\)|\1${VERSION}\2|" "$F"

# ARM-SHA256 sentinel: sha256 "ec58..." # === AUTO-UPDATE: ARM-SHA256 ===
sed -i "s|\(sha256 \"\)[^\"]*\(\" # === AUTO-UPDATE: ARM-SHA256 ===\)|\1${ARM_SHA}\2|" "$F"

# INTEL-URL sentinel: url "...v1.1.0/contentops-x86_64-apple-darwin" # === AUTO-UPDATE: INTEL-URL ===
sed -i "s|\(url \"https://github\.com/darrelldoesdevops/contentops/releases/download/v\)[^/]*\(/contentops-x86_64-apple-darwin\" # === AUTO-UPDATE: INTEL-URL ===\)|\1${VERSION}\2|" "$F"

# INTEL-SHA256 sentinel: sha256 "7b45..." # === AUTO-UPDATE: INTEL-SHA256 ===
sed -i "s|\(sha256 \"\)[^\"]*\(\" # === AUTO-UPDATE: INTEL-SHA256 ===\)|\1${INTEL_SHA}\2|" "$F"
```

All patterns tested against the live formula at `darrelldoesdevops/homebrew-tap` — all 5 produce correct output.

### Trigger cross-repo workflow_dispatch (in release.yml)

```yaml
- name: Trigger tap update
  run: |
    gh workflow run update-tap.yml \
      -f version="${GITHUB_REF_NAME#v}" \
      -R darrelldoesdevops/homebrew-tap
  env:
    GH_TOKEN: ${{ secrets.TAP_UPDATE_TOKEN }}
```

### Validate patches before commit (optional safety check)

```bash
# Verify no sentinel is still showing the old version
grep "AUTO-UPDATE" Formula/contentops.rb | grep -v "${VERSION}" && echo "PATCH INCOMPLETE" && exit 1 || true
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Download `.sha256` sidecar via API redirect | Use `asset.digest` field directly | ~mid-2025 | No binary download needed; simpler workflow |
| Classic PAT required for workflow_dispatch | Fine-grained PAT with `Actions: write` also works | 2024 | Fine-grained PAT is an option — but prior decision locks classic PAT |
| `mislav/bump-homebrew-formula-action` | Custom sed script | Phase 10 decision | The action cannot handle `Hardware::CPU` conditionals; sed with sentinels is the chosen approach |

## Open Questions

1. **TAP_UPDATE_TOKEN creation**
   - What we know: Must be a classic PAT with `repo` + `workflow` scopes; stored as `TAP_UPDATE_TOKEN` secret in contentops repo
   - What's unclear: Secret has not been created yet (no secrets visible in `gh secret list`)
   - Recommendation: Plan must include a human task to create the PAT and store it as a secret; cannot be automated

2. **Brew auto-update after tap commit**
   - What we know: `brew update` fetches new formula from tap; `brew upgrade contentops` installs new version
   - What's unclear: No action needed in the workflow — Homebrew handles this automatically when users run `brew update`
   - Recommendation: Success criterion 3 is validated by a user running `brew update && brew upgrade contentops` after the tap commit lands

## Sources

### Primary (HIGH confidence)
- Live `gh api` calls against `darrelldoesdevops/contentops` and `darrelldoesdevops/homebrew-tap` — confirmed `asset.digest`, sentinel format, sed patterns
- `docs.github.com/en/rest/actions/workflows` — workflow_dispatch token requirements (classic PAT: `repo` scope; fine-grained: `Actions: write`)
- `cli.github.com/manual/gh_workflow_run` — `gh workflow run` syntax with `-R` and `-f` flags
- Phase 10 artifacts: `10-01-PLAN.md`, `10-01-SUMMARY.md`, `10-VERIFICATION.md` — sentinel format, URL patterns, SHA256 values

### Secondary (MEDIUM confidence)
- builtfast.dev/blog/automating-homebrew-tap-updates-with-github-actions — two-workflow pattern with `gh workflow run -R`, PAT secret storage
- josh.fail/2023/automate-updating-custom-homebrew-formulae-with-github-actions — tap workflow structure, `workflow_dispatch` inputs

### Tertiary (LOW confidence)
- Community discussions on fine-grained PAT limitations for workflow_dispatch — conflicting reports; official docs are authoritative

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — gh CLI, sed, GitHub API all verified via live calls
- Architecture: HIGH — two-workflow pattern validated against working examples; all sed patterns tested
- Pitfalls: HIGH — `GITHUB_TOKEN` scope issue, `workflow` scope requirement, `v` prefix strip verified from actual data

**Research date:** 2026-02-21
**Valid until:** 2026-03-21 (GitHub Actions API stable; gh CLI flags stable)
