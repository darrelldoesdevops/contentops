use std::path::{Path, PathBuf};

use crate::error::{last_n_lines, AppError};
use crate::ffmpeg;
use crate::temp::{make_temp_file, TempFileRegistry};
use crate::ui;

struct LoudnormMeasurement {
    input_i: String,
    input_tp: String,
    input_lra: String,
    input_thresh: String,
    target_offset: String,
}

fn parse_loudnorm_stats(stderr: &[u8]) -> Option<LoudnormMeasurement> {
    let text = String::from_utf8_lossy(stderr);

    let json_start = text.rfind('{')?;
    let json_end = text.rfind('}')?;
    if json_end <= json_start {
        return None;
    }
    let json_block = &text[json_start..=json_end];

    let extract = |key: &str| -> Option<String> {
        let pattern = format!("\"{}\"", key);
        let idx = json_block.find(&pattern)?;
        let after_key = &json_block[idx + pattern.len()..];
        let colon = after_key.find(':')?;
        let after_colon = after_key[colon + 1..].trim_start();
        let value_start = after_colon.find('"')? + 1;
        let rest = &after_colon[value_start..];
        let value_end = rest.find('"')?;
        Some(rest[..value_end].to_string())
    };

    Some(LoudnormMeasurement {
        input_i: extract("input_i")?,
        input_tp: extract("input_tp")?,
        input_lra: extract("input_lra")?,
        input_thresh: extract("input_thresh")?,
        target_offset: extract("target_offset")?,
    })
}

pub fn normalize_to_temp(
    input: &Path,
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<PathBuf> {
    let parent_dir = input.parent().unwrap_or(Path::new("."));
    let input_str = input.to_string_lossy();
    let filename = input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    // Pass 1: Measure loudness
    let measure_spinner = if !verbose {
        Some(ui::make_spinner(format!("Analyzing loudness in {}...", filename)))
    } else {
        None
    };

    let measure_args = [
        "-i",
        &input_str,
        "-af",
        "loudnorm=I=-14:TP=-1.5:LRA=11:print_format=json",
        "-f",
        "null",
        "/dev/null",
    ];

    let measure_result = ffmpeg::run_ffmpeg(&measure_args);

    let measurement = match measure_result {
        Ok(ref ffmpeg_output) if ffmpeg_output.success => {
            if let Some(pb) = measure_spinner {
                pb.finish_and_clear();
            }

            parse_loudnorm_stats(&ffmpeg_output.stderr).ok_or_else(|| {
                anyhow::anyhow!("failed to parse loudnorm measurement from FFmpeg output")
            })?
        }
        Ok(ffmpeg_output) => {
            if let Some(pb) = measure_spinner {
                pb.finish_and_clear();
            }

            let truncated_stderr = last_n_lines(&ffmpeg_output.stderr, 20);
            let code = ffmpeg_output.exit_code.unwrap_or(-1);

            return Err(AppError::FfmpegFailed {
                stage: "loudness-analysis".to_string(),
                code,
                stderr: truncated_stderr,
            }
            .into());
        }
        Err(io_err) => {
            if let Some(pb) = measure_spinner {
                pb.finish_and_clear();
            }

            return Err(AppError::StageIo {
                stage: "loudness-analysis".to_string(),
                source: io_err,
            }
            .into());
        }
    };

    // Pass 2: Apply normalization
    let temp_file = make_temp_file(parent_dir, ".mp4")?;
    let temp_path = temp_file.path().to_path_buf();
    temp_file.keep().map_err(|e| e.error)?;
    registry.register(temp_path.clone());

    let temp_str = temp_path.to_string_lossy().to_string();

    let loudnorm_filter = format!(
        "loudnorm=I=-14:TP=-1.5:LRA=11:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true",
        measurement.input_i,
        measurement.input_tp,
        measurement.input_lra,
        measurement.input_thresh,
        measurement.target_offset,
    );

    let normalize_args = vec![
        "-i",
        &input_str,
        "-af",
        &loudnorm_filter,
        "-c:v",
        "copy",
        &temp_str,
    ];

    let message = format!("Normalizing audio in {}...", filename);

    let result = if verbose {
        eprintln!("Running: ffmpeg {}", normalize_args.join(" "));
        ffmpeg::run_ffmpeg_verbose(&normalize_args)
    } else {
        let duration = ffmpeg::probe_duration(&input_str);
        ffmpeg::run_ffmpeg_with_progress(&normalize_args, duration, &message)
    };

    match result {
        Ok(ffmpeg_output) if ffmpeg_output.success => Ok(temp_path),
        Ok(ffmpeg_output) => {
            let truncated_stderr = last_n_lines(&ffmpeg_output.stderr, 20);
            let code = ffmpeg_output.exit_code.unwrap_or(-1);

            let log_path = parent_dir.join(".contentops_error.log");
            let _ = std::fs::write(
                &log_path,
                String::from_utf8_lossy(&ffmpeg_output.stderr).as_ref(),
            );

            Err(AppError::FfmpegFailed {
                stage: "normalize".to_string(),
                code,
                stderr: truncated_stderr,
            }
            .into())
        }
        Err(io_err) => Err(AppError::StageIo {
            stage: "normalize".to_string(),
            source: io_err,
        }
        .into()),
    }
}
