use std::path::Path;

use humansize::{format_size, DECIMAL};

use crate::cli::OverlayArgs;
use crate::commands::cut::derive_output_path;
use crate::error::{last_n_lines, require_ffmpeg, AppError};
use crate::ffmpeg;
use crate::temp::{make_temp_file, TempFileRegistry};

fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\'', "'\\\\\\''")
        .replace(':', "\\:")
        .replace(';', "\\;")
}

fn build_drawtext_filter(args: &OverlayArgs) -> String {
    let escaped_text = escape_drawtext(&args.text);

    let y_expr = match args.position.as_str() {
        "top" => "260".to_string(),
        "bottom" => "(h-330)".to_string(),
        _ => "((h-text_h)/2)".to_string(),
    };

    let mut filter = format!(
        "drawtext=text='{}':fontsize={}:fontcolor={}:x=(w-text_w)/2:y={}",
        escaped_text, args.font_size, args.color, y_expr
    );

    if let Some(ref font_path) = args.font {
        filter.push_str(&format!(":fontfile='{}'", font_path.display()));
    }

    if args.duration > 0.0 {
        let end = args.start + args.duration;
        filter.push_str(&format!(":enable='between(t,{},{})'", args.start, end));
    } else if args.start > 0.0 {
        filter.push_str(&format!(":enable='gte(t,{})'", args.start));
    }

    filter
}

pub fn run(args: OverlayArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    require_ffmpeg()?;

    if !args.input.exists() {
        return Err(AppError::InputNotFound(args.input).into());
    }

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| derive_output_path(&args.input, "overlay"));

    let parent_dir = args.input.parent().unwrap_or(Path::new("."));

    let temp_file = make_temp_file(parent_dir, ".mp4")?;
    let temp_path = temp_file.path().to_path_buf();
    registry.register(temp_path.clone());

    let input_str = args.input.to_string_lossy();
    let temp_str = temp_path.to_string_lossy();

    let drawtext_filter = build_drawtext_filter(&args);

    let ffmpeg_args = [
        "-i",
        &input_str,
        "-vf",
        &drawtext_filter,
        "-c:a",
        "copy",
        &temp_str,
    ];

    let filename = args
        .input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let message = format!("Adding overlay to {}...", filename);

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

            let size = std::fs::metadata(&output)
                .map(|m| format_size(m.len(), DECIMAL))
                .unwrap_or_else(|_| "unknown size".to_string());

            eprintln!("\u{2713} Created {} ({})", output.display(), size);
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
                stage: "overlay".to_string(),
                code,
                stderr: truncated_stderr,
            }
            .into());
        }
        Err(io_err) => {
            return Err(AppError::StageIo {
                stage: "overlay".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    Ok(())
}
