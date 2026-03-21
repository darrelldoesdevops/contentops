use std::path::{Path, PathBuf};

use humansize::{DECIMAL, format_size};

use crate::cli::CutArgs;
use crate::commands::normalize;
use crate::error::{AppError, last_n_lines, require_ffmpeg};
use crate::ffmpeg;
use crate::silence;
use crate::temp::{TempFileRegistry, make_temp_file};
use crate::ui;
use crate::vad;

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

    // --- Phase 2: Detect speech via VAD ---
    let spinner = if !verbose {
        Some(ui::make_spinner(format!(
            "Detecting speech in {}...",
            filename
        )))
    } else {
        None
    };

    let video_duration =
        ffmpeg::probe_duration_strict(&input_str).map_err(|e| AppError::StageIo {
            stage: "probe-duration".to_string(),
            source: e,
        })?;

    let wav_temp = make_temp_file(parent_dir, ".wav")?;
    let wav_path = wav_temp.path().to_path_buf();
    registry.register(wav_path.clone());

    ffmpeg::extract_16k_wav(&input_str, &wav_path, verbose).map_err(|e| AppError::StageIo {
        stage: "wav-extraction".to_string(),
        source: e,
    })?;

    let speeches = vad::run_vad(
        &wav_path,
        video_duration,
        args.vad_threshold,
        args.min_silence_ms,
        args.start_pad,
    )?;

    let _ = std::fs::remove_file(&wav_path);
    registry.remove(&wav_path);

    if speeches.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        return Err(AppError::NoSpeechDetected(args.input.clone()).into());
    }

    let total_silence = silence::total_silence_from_speeches(&speeches, video_duration);
    eprintln!(
        "Found {} speech segments, removing {:.1}s of silence",
        speeches.len(),
        total_silence
    );

    // --- Phase 3: Dry-run branch ---
    if args.dry_run {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        eprintln!("Speech segments detected in {}:", filename);
        for s in &speeches {
            let dur = s.end - s.start;
            eprintln!("  {:.1}s - {:.1}s ({:.1}s)", s.start, s.end, dur);
        }
        eprintln!();
        eprintln!("Total silence: {:.1}s", total_silence);
        eprintln!(
            "Would remove {:.1}s from {:.1}s video",
            total_silence, video_duration
        );
        return Ok(());
    }

    // --- Phase 4: Remove silence ---
    if let Some(pb) = &spinner {
        pb.set_message("Removing silence...".to_string());
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

    let message = format!("Removing silence from {}...", filename);

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

            eprintln!("\u{2713} Created {} ({})", output.display(), size);
            eprintln!("Removed {:.1}s of silence", total_silence);
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
