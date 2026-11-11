# contentops

Video post-production CLI for TikTok/short-form content. Removes silence, generates word-by-word captions, adds title overlays -- replacing CapCut's repetitive editing steps with one command.

```bash
contentops pipeline --model ~/models/ggml-base.en.bin raw-video.mp4
# Runs: cut → caption → overlay
# Output: raw-video_pipeline.mp4
```

## Prerequisites

| Tool | macOS | Linux | Windows | Used by |
|------|-------|-------|---------|---------|
| FFmpeg | `brew install ffmpeg` | `apt install ffmpeg` | `choco install ffmpeg` | All commands |
| whisper-cli | `brew install whisper-cli` | [Build from source](https://github.com/ggerganov/whisper.cpp) | [Build from source](https://github.com/ggerganov/whisper.cpp) | `caption`, `pipeline` |
| Claude CLI | [claude.com/claude-code](https://claude.com/claude-code) | [claude.com/claude-code](https://claude.com/claude-code) | [claude.com/claude-code](https://claude.com/claude-code) | `overlay --auto` (optional) |

Download a Whisper model for transcription:

```bash
curl -L -o ~/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/models/ggml-base.en.bin
```

## Install

**Homebrew (recommended):**

```bash
brew install darrelldoesdevops/tap/contentops
```

**Direct download (Apple Silicon):**

```bash
curl -L -o /usr/local/bin/contentops \
  https://github.com/darrelldoesdevops/contentops/releases/latest/download/contentops-aarch64-apple-darwin \
  && chmod +x /usr/local/bin/contentops
```

**Direct download (Intel Mac):**

```bash
curl -L -o /usr/local/bin/contentops \
  https://github.com/darrelldoesdevops/contentops/releases/latest/download/contentops-x86_64-apple-darwin \
  && chmod +x /usr/local/bin/contentops
```

**Direct download (Linux x86_64):**

```bash
curl -L -o /usr/local/bin/contentops \
  https://github.com/darrelldoesdevops/contentops/releases/latest/download/contentops-x86_64-unknown-linux-gnu \
  && chmod +x /usr/local/bin/contentops
```

**Direct download (Windows x86_64):**

```powershell
Invoke-WebRequest -Uri https://github.com/darrelldoesdevops/contentops/releases/latest/download/contentops-x86_64-pc-windows-msvc.exe -OutFile contentops.exe
```

Move `contentops.exe` to a directory on your PATH.

Verify: `contentops doctor`

## Usage

### `pipeline` -- Full workflow

Runs cut, caption, and overlay in sequence. The built-in pipeline approves the title before applying captions and title in a single final video encode.

```bash
contentops pipeline --model ~/models/ggml-base.en.bin input.mp4
```

| Flag | Description | Default |
|------|-------------|---------|
| `<INPUT>` | Input video file | required |
| `-o <OUTPUT>` | Output path | `input_pipeline.mp4` |
| `--model <MODEL>` | Path to whisper model file | required |
| `--vad-threshold <THRESHOLD>` | VAD speech probability threshold (0.0-1.0) | `0.5` |
| `--min-silence-ms <MS>` | Minimum silence duration to cut (ms) | `300` |
| `--dry-run` | Preview planned stages without executing | off |
| `--verbose` | Show verbose FFmpeg output | off |

### `cut` -- Silence removal

Detects and removes silent segments, normalizes audio loudness.

```bash
contentops cut input.mp4
```

| Flag | Description | Default |
|------|-------------|---------|
| `<INPUT>` | Input video file | required |
| `-o <OUTPUT>` | Output path | `input_cut.mp4` |
| `--dry-run` | Preview what would be cut without producing output | off |
| `--json` | With `--dry-run`, write the versioned result JSON to stdout | off |
| `--vad-threshold <THRESHOLD>` | VAD speech probability threshold (0.0-1.0) | `0.5` |
| `--min-silence-ms <MS>` | Minimum silence duration to cut (ms) | `300` |
| `--verbose` | Show verbose FFmpeg output | off |

`contentops cut --dry-run --json input.mp4` writes only this schema to stdout; progress and diagnostics remain on stderr:

```json
{"schema_version":1,"duration_seconds":12.5,"total_silence_seconds":8.7,"speech_segments":[{"start":0.4,"end":4.2}]}
```

Errors return a nonzero exit status and do not emit a JSON report. No speech is an error with the same nonzero/no-JSON behavior. The human-readable `--dry-run` output is unchanged when `--json` is omitted.

### `caption` -- Generate captions

Transcribes audio with Whisper and optionally burns word-by-word captions into video.

```bash
contentops caption --model ~/models/ggml-base.en.bin --burn input.mp4
```

Existing corrected word timings can be rendered without Whisper:

```bash
contentops caption --words-json corrected-words.json --burn input.mp4
```

`--words-json` accepts a nonempty JSON array. Times are seconds relative to the supplied input video's output timeline, not the source video before cuts:

```json
[
  {"word": "Corrected", "start": 0.42, "end": 0.91},
  {"word": "caption", "start": 0.91, "end": 1.36}
]
```

Words must have nonempty text and finite, nonnegative, positive-duration, monotonic, nonoverlapping intervals within the video duration (with a 0.05-second final-frame tolerance). Output collision checks recognize symlinks on all targets and hard links on Unix. On other targets, both imported-word and Whisper captions require unused output paths: existing outputs are rejected because file identity cannot be safely verified with stable standard-library APIs.

| Flag | Description | Default |
|------|-------------|---------|
| `<INPUT>` | Input video file | required |
| `-o <OUTPUT>` | Output SRT path | `input_captioned.srt` |
| `--model <MODEL>` | Path to whisper model file; conflicts with `--words-json` | required unless `--words-json` is used |
| `--words-json <PATH>` | Existing `{word,start,end}` array in input-video seconds; conflicts with `--model` and `--fix` | required unless `--model` is used |
| `--lang <LANG>` | Language code for transcription | `en` |
| `--burn` | Burn captions into video | off |
| `--video-out <PATH>` | Burned video path; requires `--burn` | `input_captioned.mp4` |
| `--verbose` | Show verbose FFmpeg output | off |

### `overlay` -- Title cards

Adds animated text overlay to video. Use `--text` for manual or `--auto` for Claude-generated titles.

```bash
contentops overlay --text "My Video Title" input.mp4
```

| Flag | Description | Default |
|------|-------------|---------|
| `<INPUT>` | Input video file | required |
| `--text <TEXT>` | Text to overlay | -- |
| `--auto <TRANSCRIPTION_JSON>` | Auto-generate title from transcription JSON using Claude | -- |
| `-o <OUTPUT>` | Output path | `input_overlay.mp4` |
| `--font <FONT>` | Path to .ttf font file | system default |
| `--font-size <FONT_SIZE>` | Font size in pixels | `44` |
| `--color <COLOR>` | Font color (FFmpeg color name or hex) | `black` |
| `--position <POSITION>` | Position preset: top, center, bottom | `top` |
| `--start <START>` | When overlay appears (seconds) | `0.3` |
| `--duration <DURATION>` | How long overlay is visible (seconds, 0 = entire video) | `3.5` |
| `--verbose` | Show verbose FFmpeg output | off |

### Global encoding options

Video re-encoding commands accept `--vcodec x264|videotoolbox` and a positive `--vbitrate <BITRATE>`. CLI values override `CONTENTOPS_VCODEC` and `CONTENTOPS_VBITRATE`; defaults are `x264` and `40M`. The bitrate is used only when the selected encoder is VideoToolbox. Audio-only and stream-copy operations are unchanged.

### `doctor` -- Check prerequisites

Verifies all tools are installed and each subcommand is ready to use.

```bash
contentops doctor
```

| Flag | Description | Default |
|------|-------------|---------|
| `--strict` | Exit with code 1 if any check fails or warns | off |
| `--verbose` | Show verbose FFmpeg output | off |

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `ffmpeg not found on PATH` | FFmpeg not installed | `brew install ffmpeg` (macOS), `apt install ffmpeg` (Linux), `choco install ffmpeg` (Windows) |
| `whisper-cli not found on PATH` | whisper-cli not installed | `brew install whisper-cli` (macOS), [build from source](https://github.com/ggerganov/whisper.cpp) (Linux/Windows) |
| `whisper model not found: <path>` | Model file missing or wrong path | Download from [huggingface.co/ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) |
| `claude not found on PATH` | Claude CLI not installed (only needed for `overlay --auto`) | Install [Claude Code CLI](https://claude.com/claude-code) |
| `input file not found: <path>` | File path is wrong | Check the path and try again |
| `no speech detected in <file>` | Video contains no audible speech | Video must have spoken content for silence removal |
| `ffmpeg exited with code 1` | Codec or format issue | Run with `--verbose` to see FFmpeg stderr |

Run `contentops doctor` after installing prerequisites to verify everything works.
