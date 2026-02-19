use std::path::{Path, PathBuf};
use std::time::Duration;

use humansize::{format_size, DECIMAL};
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::CutArgs;
use crate::error::{last_n_lines, require_ffmpeg, AppError};
use crate::ffmpeg;
use crate::temp::{make_temp_file, TempFileRegistry};

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

    let parent_dir = args
        .input
        .parent()
        .unwrap_or(Path::new("."));

    let temp_file = make_temp_file(parent_dir, ".mp4")?;
    let temp_path = temp_file.path().to_path_buf();
    registry.register(temp_path.clone());

    let input_str = args.input.to_string_lossy();
    let temp_str = temp_path.to_string_lossy();

    let ffmpeg_args = [
        "-i",
        &input_str,
        "-c:v",
        "libx264",
        "-c:a",
        "aac",
        &temp_str,
    ];

    let spinner = if !verbose {
        let filename = args
            .input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
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
        pb.set_message(format!("Processing {}...", filename));
        Some(pb)
    } else {
        eprintln!(
            "Running: ffmpeg {}",
            ffmpeg_args.join(" ")
        );
        None
    };

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
                stage: "re-encode".to_string(),
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
                stage: "re-encode".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    Ok(())
}
