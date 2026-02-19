use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use humansize::{format_size, DECIMAL};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

use crate::cli::CaptionArgs;
use crate::error::{last_n_lines, require_ffmpeg, require_whisper, AppError};
use crate::ffmpeg;
use crate::temp::{make_temp_file, TempFileRegistry};

#[derive(serde::Serialize)]
struct Word {
    word: String,
    start: f64,
    end: f64,
}

struct SrtEntry {
    index: usize,
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize)]
struct WhisperJson {
    transcription: Vec<WhisperSegment>,
}

#[derive(Deserialize)]
struct WhisperSegment {
    timestamps: WhisperTimestamps,
    text: String,
}

#[derive(Deserialize)]
struct WhisperTimestamps {
    from: String,
    to: String,
}

fn parse_timestamp(ts: &str) -> f64 {
    // Format: "HH:MM:SS.mmm" or "HH:MM:SS,mmm"
    let ts = ts.replace(',', ".");
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 3 {
        return 0.0;
    }
    let hours: f64 = parts[0].trim().parse().unwrap_or(0.0);
    let minutes: f64 = parts[1].trim().parse().unwrap_or(0.0);
    let seconds: f64 = parts[2].trim().parse().unwrap_or(0.0);
    hours * 3600.0 + minutes * 60.0 + seconds
}

fn format_srt_time(seconds: f64) -> String {
    let total_ms = (seconds * 1000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let secs = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, ms)
}

fn derive_caption_output(input: &Path, suffix: &str, ext: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    parent.join(format!("{}_{}.{}", stem, suffix, ext))
}

fn group_words_into_srt(words: &[Word]) -> Vec<SrtEntry> {
    let mut entries = Vec::new();
    let mut current_words: Vec<&Word> = Vec::new();
    let mut index = 1;

    for word in words {
        current_words.push(word);

        let text = &word.word;
        let ends_with_punctuation = text.ends_with('.')
            || text.ends_with('!')
            || text.ends_with('?')
            || text.ends_with(',');

        if current_words.len() >= 5 || (current_words.len() >= 3 && ends_with_punctuation) {
            let start = current_words[0].start;
            let end = current_words.last().unwrap().end;
            let text = current_words
                .iter()
                .map(|w| w.word.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            entries.push(SrtEntry {
                index,
                start,
                end,
                text,
            });
            index += 1;
            current_words.clear();
        }
    }

    if !current_words.is_empty() {
        let start = current_words[0].start;
        let end = current_words.last().unwrap().end;
        let text = current_words
            .iter()
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        entries.push(SrtEntry {
            index,
            start,
            end,
            text,
        });
    }

    entries
}

fn format_srt(entries: &[SrtEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}\n{} --> {}\n{}\n",
                entry.index,
                format_srt_time(entry.start),
                format_srt_time(entry.end),
                entry.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn make_spinner(message: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "\u{2800}", "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{283b}",
                "\u{2839}", "\u{2838}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2713}",
            ]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(message);
    pb
}

pub fn run(args: CaptionArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    // 1. Validation
    require_ffmpeg()?;
    require_whisper()?;

    if !args.input.exists() {
        return Err(AppError::InputNotFound(args.input).into());
    }

    if !args.model.exists() {
        return Err(AppError::ModelNotFound(args.model).into());
    }

    // 2. Derive output paths
    let srt_output = args
        .output
        .clone()
        .unwrap_or_else(|| derive_caption_output(&args.input, "captioned", "srt"));
    let json_output = if args.output.is_some() {
        srt_output.with_extension("json")
    } else {
        derive_caption_output(&args.input, "captioned", "json")
    };

    let parent_dir = args.input.parent().unwrap_or(Path::new("."));

    // 3. Audio extraction
    let temp_file = make_temp_file(parent_dir, ".wav")?;
    let temp_wav = temp_file.path().to_path_buf();
    registry.register(temp_wav.clone());

    let input_str = args.input.to_string_lossy();
    let wav_str = temp_wav.to_string_lossy();

    let ffmpeg_args = [
        "-i",
        &input_str,
        "-ar",
        "16000",
        "-ac",
        "1",
        "-f",
        "wav",
        &wav_str,
    ];

    let spinner = if !verbose {
        let filename = args
            .input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        Some(make_spinner(format!("Extracting audio from {}...", filename)))
    } else {
        eprintln!("Running: ffmpeg {}", ffmpeg_args.join(" "));
        None
    };

    let result = if verbose {
        ffmpeg::run_ffmpeg_verbose(&ffmpeg_args)
    } else {
        ffmpeg::run_ffmpeg(&ffmpeg_args)
    };

    match result {
        Ok(ref output) if output.success => {
            if let Some(ref pb) = spinner {
                pb.finish_with_message("Audio extracted");
            }
        }
        Ok(output) => {
            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }
            let truncated = last_n_lines(&output.stderr, 20);
            let code = output.exit_code.unwrap_or(-1);
            return Err(AppError::FfmpegFailed {
                stage: "audio-extraction".to_string(),
                code,
                stderr: truncated,
            }
            .into());
        }
        Err(io_err) => {
            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }
            return Err(AppError::StageIo {
                stage: "audio-extraction".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    // 4. Transcription via whisper-cpp
    let spinner = if !verbose {
        Some(make_spinner("Transcribing with Whisper...".to_string()))
    } else {
        eprintln!(
            "Running: whisper-cpp -m {} -f {} --output-json --max-len 1 -l {}",
            args.model.display(),
            wav_str,
            args.lang
        );
        None
    };

    let whisper_output = Command::new("whisper-cpp")
        .arg("-m")
        .arg(&args.model)
        .arg("-f")
        .arg(&temp_wav)
        .arg("--output-json")
        .arg("--max-len")
        .arg("1")
        .arg("-l")
        .arg(&args.lang)
        .stdin(Stdio::null())
        .stdout(if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .output()
        .map_err(|e| AppError::StageIo {
            stage: "transcription".to_string(),
            source: e,
        })?;

    if !whisper_output.status.success() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        let code = whisper_output.status.code().unwrap_or(-1);
        let stderr = last_n_lines(&whisper_output.stderr, 20);
        return Err(AppError::WhisperFailed {
            stage: "transcription".to_string(),
            code,
            stderr,
        }
        .into());
    }

    if let Some(ref pb) = spinner {
        pb.finish_with_message("Transcription complete");
    }

    // 5. Parse whisper-cpp JSON output
    let whisper_json_path = PathBuf::from(format!("{}.json", temp_wav.display()));
    registry.register(whisper_json_path.clone());

    let json_content = std::fs::read_to_string(&whisper_json_path).map_err(|e| AppError::StageIo {
        stage: "read-whisper-json".to_string(),
        source: e,
    })?;

    let whisper_data: WhisperJson =
        serde_json::from_str(&json_content).map_err(|e| anyhow::anyhow!("parsing whisper JSON: {}", e))?;

    let words: Vec<Word> = whisper_data
        .transcription
        .iter()
        .map(|seg| {
            let text = seg.text.trim().to_string();
            let start = parse_timestamp(&seg.timestamps.from);
            let end = parse_timestamp(&seg.timestamps.to);
            Word {
                word: text,
                start,
                end,
            }
        })
        .filter(|w| !w.word.is_empty())
        .collect();

    if words.is_empty() {
        eprintln!("Warning: No speech detected in video");
        std::fs::write(&srt_output, "").map_err(|e| AppError::StageIo {
            stage: "write-srt".to_string(),
            source: e,
        })?;
        std::fs::write(&json_output, "[]").map_err(|e| AppError::StageIo {
            stage: "write-json".to_string(),
            source: e,
        })?;
    } else {
        // 6. Generate SRT
        let entries = group_words_into_srt(&words);
        let srt_content = format_srt(&entries);
        std::fs::write(&srt_output, srt_content).map_err(|e| AppError::StageIo {
            stage: "write-srt".to_string(),
            source: e,
        })?;

        // 7. Generate JSON sidecar
        let json_content =
            serde_json::to_string_pretty(&words).map_err(|e| anyhow::anyhow!("serializing JSON: {}", e))?;
        std::fs::write(&json_output, json_content).map_err(|e| AppError::StageIo {
            stage: "write-json".to_string(),
            source: e,
        })?;
    }

    // 8. Cleanup temp files
    let _ = std::fs::remove_file(&temp_wav);
    registry.remove(&temp_wav);
    let _ = std::fs::remove_file(&whisper_json_path);
    registry.remove(&whisper_json_path);

    // Print success
    let srt_size = std::fs::metadata(&srt_output)
        .map(|m| format_size(m.len(), DECIMAL))
        .unwrap_or_else(|_| "unknown size".to_string());
    let json_size = std::fs::metadata(&json_output)
        .map(|m| format_size(m.len(), DECIMAL))
        .unwrap_or_else(|_| "unknown size".to_string());

    eprintln!(
        "\u{2713} Created {} ({})",
        srt_output.display(),
        srt_size
    );
    eprintln!(
        "\u{2713} Created {} ({})",
        json_output.display(),
        json_size
    );

    Ok(())
}
