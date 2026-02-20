---
phase: 07-doctor-subcommand
status: gaps_found
verified: 2026-02-20
---

# Phase 7: Doctor Subcommand - Verification Report

## Phase Goal
Users can verify their environment is ready to run any contentops command before attempting video processing.

## Success Criteria Results

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `contentops doctor` prints colored [ok]/[warn]/[fail] status for ffmpeg, ffprobe, whisper-cli, and claude | PASS | doctor.rs:20-26 Status enum with green/yellow/red via owo_colors |
| 2 | Doctor output includes a per-subcommand readiness summary | PASS | doctor.rs:104-126 SUBCOMMANDS const with cut/caption/overlay/overlay --auto readiness |
| 3 | Doctor checks that ffmpeg version is >= 6.0 and reports if it is not | PASS | doctor.rs:44-81 check_ffmpeg_version with version_at_least(6, 0) |
| 4 | `contentops doctor` exits 0 by default; exits 1 only with `--strict` | PASS | doctor.rs:201-205 returns 0 unless strict && (failures || warnings) |
| 5 | Running `contentops overlay --auto` without claude on PATH shows error suggesting `contentops doctor` | PASS | overlay.rs:193-195 calls require_claude(); error.rs:44 ClaudeNotFound mentions doctor |

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| DOCT-01 | Complete | doctor.rs:128 `pub fn run()` wired into CLI |
| DOCT-02 | Complete | Checks ffmpeg, ffprobe, whisper-cli, claude with colored output |
| DOCT-03 | Complete | Per-subcommand readiness table in doctor output |
| DOCT-04 | Complete | ffmpeg version >= 6.0 check with version parsing |
| DOCT-05 | Complete | Exit 0 default, exit 1 with --strict |
| DOCT-06 | GAP | Only ClaudeNotFound suggests `contentops doctor`; FfmpegNotFound and WhisperNotFound suggest brew install without mentioning doctor |
| DOCT-07 | Complete | require_claude() in error.rs:69, enforced in overlay.rs:194 |

## Gap Details

**DOCT-06 partial:** The requirement says "Commands auto-suggest `contentops doctor` when failing due to missing prerequisite." Currently:
- `ClaudeNotFound` → "run `contentops doctor`" (correct)
- `FfmpegNotFound` → "brew install ffmpeg" (missing doctor suggestion)
- `WhisperNotFound` → "brew install whisper-cli" (missing doctor suggestion)

Fix: Append `, then run \`contentops doctor\`` to FfmpegNotFound and WhisperNotFound hint messages.

**Score:** 6/7 requirements verified, 1 gap

## Verdict

**GAPS FOUND** -- DOCT-06 partially implemented. Minor fix needed to error hint messages.
