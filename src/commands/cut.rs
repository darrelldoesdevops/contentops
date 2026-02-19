use std::path::{Path, PathBuf};
use std::time::Duration;

use humansize::{format_size, DECIMAL};
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::CutArgs;
use crate::error::{last_n_lines, require_ffmpeg, AppError};
use crate::ffmpeg;
use crate::silence;
use crate::temp::{make_temp_file, TempFileRegistry};

const SILENCE_THRESHOLD_DB: f64 = -30.0;
const SILENCE_MIN_DURATION: f64 = 0.5;
const SPEECH_PADDING: f64 = 0.2;
const DEFAULT_FRAME_RATE: f64 = 30.0;

pub fn derive_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    parent.join(format!("{}_{}.mp4", stem, suffix))
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

pub fn run(args: CutArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    require_ffmpeg()?;

    if !args.input.exists() {
        return Err(AppError::InputNotFound(args.input).into());
    }

    let output = args
        .output
        .unwrap_or_else(|| derive_output_path(&args.input, "cut"));

    let parent_dir = args.input.parent().unwrap_or(Path::new("."));
    let input_str = args.input.to_string_lossy().to_string();
    let filename = args
        .input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // --- Phase 1: Detect silence ---
    let spinner = if !verbose {
        Some(make_spinner(format!("Detecting silence in {}...", filename)))
    } else {
        None
    };

    let video_duration = ffmpeg::probe_duration(&input_str).map_err(|e| AppError::StageIo {
        stage: "probe-duration".to_string(),
        source: e,
    })?;

    let stderr =
        ffmpeg::run_silencedetect(&input_str, SILENCE_THRESHOLD_DB, SILENCE_MIN_DURATION)
            .map_err(|e| AppError::StageIo {
                stage: "silence-detect".to_string(),
                source: e,
            })?;

    let silences = silence::parse_silencedetect(&stderr, video_duration);

    if silences.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        eprintln!("No silence detected in {}", filename);
        return Ok(());
    }

    // --- Phase 2: Dry-run branch ---
    if args.dry_run {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        let total = silence::total_silence_removed(&silences, SPEECH_PADDING);

        eprintln!("Silence detected in {}:", filename);
        for s in &silences {
            let dur = s.end - s.start;
            eprintln!("  {:.1}s - {:.1}s ({:.1}s)", s.start, s.end, dur);
        }
        eprintln!();
        eprintln!("Total silence: {:.1}s", total);
        eprintln!(
            "Would remove {:.1}s from {:.1}s video",
            total, video_duration
        );
        return Ok(());
    }

    // --- Phase 3: Remove silence ---
    if let Some(pb) = &spinner {
        pb.set_message("Removing silence...".to_string());
    }

    let speeches = silence::silence_to_speech(&silences, video_duration, SPEECH_PADDING);
    if speeches.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        anyhow::bail!("No speech detected -- entire video is silence");
    }

    let select_filter = silence::build_select_filter(&speeches, DEFAULT_FRAME_RATE);
    let aselect_filter = silence::build_aselect_filter(&speeches);

    let temp_file = make_temp_file(parent_dir, ".mp4")?;
    let temp_path = temp_file.path().to_path_buf();
    registry.register(temp_path.clone());
    let temp_str = temp_path.to_string_lossy().to_string();

    let ffmpeg_args = vec![
        "-i",
        &input_str,
        "-vf",
        &select_filter,
        "-af",
        &aselect_filter,
        "-c:v",
        "libx264",
        "-crf",
        "23",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        &temp_str,
    ];

    if verbose {
        eprintln!("Running: ffmpeg {}", ffmpeg_args.join(" "));
    }

    let result = if verbose {
        ffmpeg::run_ffmpeg_verbose(&ffmpeg_args)
    } else {
        ffmpeg::run_ffmpeg(&ffmpeg_args)
    };

    match result {
        Ok(ffmpeg_output) if ffmpeg_output.success => {
            std::fs::copy(&temp_path, &output).map_err(|e| AppError::StageIo {
                stage: "copy-output".to_string(),
                source: e,
            })?;

            let _ = std::fs::remove_file(&temp_path);
            registry.remove(&temp_path);

            let size = std::fs::metadata(&output)
                .map(|m| format_size(m.len(), DECIMAL))
                .unwrap_or_else(|_| "unknown size".to_string());

            let silence_total = silence::total_silence_removed(&silences, SPEECH_PADDING);

            if let Some(pb) = spinner {
                pb.finish_with_message(format!(
                    "\u{2713} Created {} ({})",
                    output.display(),
                    size
                ));
            } else {
                eprintln!(
                    "\u{2713} Created {} ({})",
                    output.display(),
                    size
                );
            }
            eprintln!("Removed {:.1}s of silence", silence_total);
        }
        Ok(ffmpeg_output) => {
            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }

            let truncated_stderr = last_n_lines(&ffmpeg_output.stderr, 20);
            let code = ffmpeg_output.exit_code.unwrap_or(-1);

            let log_path = parent_dir.join(".contentops_error.log");
            let _ = std::fs::write(
                &log_path,
                String::from_utf8_lossy(&ffmpeg_output.stderr).as_ref(),
            );

            return Err(AppError::FfmpegFailed {
                stage: "silence-remove".to_string(),
                code,
                stderr: truncated_stderr,
            }
            .into());
        }
        Err(io_err) => {
            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }

            return Err(AppError::StageIo {
                stage: "silence-remove".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    Ok(())
}
