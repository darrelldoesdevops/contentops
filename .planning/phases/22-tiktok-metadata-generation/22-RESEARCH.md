# Phase 22: TikTok Metadata Generation - Research

**Researched:** 2026-02-25

## Current Pipeline Flow

### pipeline.rs: finish_stages()
- Stage 6: Burn captions
- Stage 7: Title approval + overlay
- Returns `Ok(())` after overlay completes
- The `caption_json` temp file with word timestamps is available in `temp_dir`
- The approved `overlay_text` string is available (from Phase 21 approval)
- The `output` path is the final video location

### Key Variables Available in finish_stages()
- `words: &[caption::Word]` -- transcript with timestamps
- `overlay_text: String` -- the approved title (from Phase 21)
- `output: &Path` -- final video output path (for deriving sidecar path)
- `verbose: bool`

## Sidecar File Design

### Path Derivation
```rust
// output = /path/to/video_pipeline.mp4
// sidecar = /path/to/video_pipeline_tiktok.json
let sidecar_path = output.with_file_name(format!(
    "{}_tiktok.json",
    output.file_stem().unwrap_or_default().to_string_lossy()
));
```

### JSON Structure
```json
{
  "title": "THE TRUTH\nABOUT DEVOPS",
  "description": "Ever wondered why your deploys keep failing? Here's what nobody tells you about...",
  "hashtags": ["devops", "kubernetes", "cicd", "infrastructure", "cloudengineering"]
}
```

## Claude Description Generation

### Prompt Strategy
Single Claude call to generate both description and hashtags:
- Input: full transcript text (joined words)
- Output: JSON with description and hashtags fields
- Constraint: description must be under 4,000 characters
- Constraint: 3-5 hashtags, topic-specific (not generic)

### Claude Call Pattern (from overlay.rs)
```rust
let output = Command::new("claude")
    .arg("-p")
    .arg(&prompt)
    .arg("--model")
    .arg("haiku")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()?;
```

### Error Handling
- Description generation is non-critical -- video is already produced
- On failure: warn to stderr, skip sidecar file
- Never fail the pipeline because of metadata generation

## Files Modified

| File | Changes |
|------|---------|
| `src/commands/pipeline.rs` | Add `generate_metadata()` call after Stage 7, write sidecar JSON |

Only pipeline.rs needs changes. The metadata generation function can live in pipeline.rs directly (it's pipeline-specific, not reusable like overlay title generation).

## Test Strategy

- Unit test: sidecar path derivation from output path
- Unit test: JSON structure serialization
- Integration: Claude call requires external dependency, manual testing only

---

## RESEARCH COMPLETE
