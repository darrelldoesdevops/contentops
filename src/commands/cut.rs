use std::path::{Path, PathBuf};

use humansize::{format_size, DECIMAL};

use crate::cli::CutArgs;
use crate::commands::normalize;
use crate::error::{last_n_lines, require_ffmpeg, AppError};
use crate::ffmpeg;
use crate::silence;
use crate::temp::{make_temp_file, TempFileRegistry};
use crate::ui;

const SILENCE_THRESHOLD_DB: f64 = -30.0;
const SILENCE_MIN_DURATION: f64 = 0.5;
const BREATH_THRESHOLD_DB: f64 = -24.0;
const BREATH_MIN_DURATION: f64 = 0.15;
const SPEECH_PADDING: f64 = 0.075;
pub fn derive_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    parent.join(format!("{}_{}.mp4", stem, suffix))
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
    let filename = args
        .input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // --- Phase 1: Normalize audio ---
    let normalized_path = normalize::normalize_to_temp(&args.input, verbose, registry)?;
    let input_str = normalized_path.to_string_lossy().to_string();

    // --- Phase 2: Detect silence ---
    let (threshold, min_duration) = if args.breaths {
        (BREATH_THRESHOLD_DB, BREATH_MIN_DURATION)
    } else {
        (SILENCE_THRESHOLD_DB, SILENCE_MIN_DURATION)
    };
    let detect_label = if args.breaths {
        "silence and breaths"
    } else {
        "silence"
    };

    let spinner = if !verbose {
        Some(ui::make_spinner(format!("Detecting {} in {}...", detect_label, filename)))
    } else {
        None
    };

    let video_duration = ffmpeg::probe_duration_strict(&input_str).map_err(|e| AppError::StageIo {
        stage: "probe-duration".to_string(),
        source: e,
    })?;

    let stderr =
        ffmpeg::run_silencedetect(&input_str, threshold, min_duration)
            .map_err(|e| AppError::StageIo {
                stage: "silence-detect".to_string(),
                source: e,
            })?;

    let silences = silence::parse_silencedetect(&stderr, video_duration);

    if silences.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        eprintln!("No {} detected in {}", detect_label, filename);
        return Ok(());
    }

    // --- Phase 3: Dry-run branch ---
    if args.dry_run {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        let total = silence::total_silence_removed(&silences, SPEECH_PADDING);

        eprintln!("{} detected in {}:", if args.breaths { "Silence and breaths" } else { "Silence" }, filename);
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

    // --- Phase 4: Remove silence ---
    if let Some(pb) = &spinner {
        pb.set_message(format!("Removing {}...", detect_label));
    }

    let speeches = silence::silence_to_speech(&silences, video_duration, SPEECH_PADDING);
    if speeches.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        return Err(AppError::NoSpeechDetected(args.input.clone()).into());
    }

    let concat_filter = silence::build_concat_filter(&speeches);

    let temp_file = make_temp_file(parent_dir, ".mp4")?;
    let temp_path = temp_file.path().to_path_buf();
    registry.register(temp_path.clone());
    let temp_str = temp_path.to_string_lossy().to_string();

    let ffmpeg_args = vec![
        "-i",
        &input_str,
        "-filter_complex",
        &concat_filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
        "-c:v",
        "libx264",
        "-crf",
        "14",
        "-preset",
        "slow",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        &temp_str,
    ];

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    let message = format!("Removing {} from {}...", detect_label, filename);

    let result = if verbose {
        eprintln!("Running: ffmpeg {}", ffmpeg_args.join(" "));
        ffmpeg::run_ffmpeg_verbose(&ffmpeg_args)
    } else {
        let duration = ffmpeg::probe_duration(&input_str);
        ffmpeg::run_ffmpeg_with_progress(&ffmpeg_args, duration, &message)
    };

    match result {
        Ok(ffmpeg_output) if ffmpeg_output.success => {
            std::fs::copy(&temp_path, &output).map_err(|e| AppError::StageIo {
                stage: "copy-output".to_string(),
                source: e,
            })?;

            let _ = std::fs::remove_file(&temp_path);
            registry.remove(&temp_path);
            let _ = std::fs::remove_file(&normalized_path);
            registry.remove(&normalized_path);

            let size = std::fs::metadata(&output)
                .map(|m| format_size(m.len(), DECIMAL))
                .unwrap_or_else(|_| "unknown size".to_string());

            let silence_total = silence::total_silence_removed(&silences, SPEECH_PADDING);
            eprintln!("\u{2713} Created {} ({})", output.display(), size);
            eprintln!("Removed {:.1}s of {}", silence_total, detect_label);
        }
        Ok(ffmpeg_output) => {
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
            return Err(AppError::StageIo {
                stage: "silence-remove".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    Ok(())
}
