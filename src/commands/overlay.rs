use std::path::Path;
use std::process::{Command, Stdio};

use humansize::{DECIMAL, format_size};
use serde::Deserialize;

use crate::cli::OverlayArgs;
use crate::commands::cut::derive_output_path;
use crate::error::{AppError, last_n_lines, require_claude, require_ffmpeg};
use crate::ffmpeg;
use crate::temp::{TempFileRegistry, make_temp_file};
use crate::ui;

#[derive(Deserialize)]
struct TranscriptWord {
    word: String,
}

fn generate_title(transcript_path: &Path, verbose: bool) -> anyhow::Result<String> {
    let json_content = std::fs::read_to_string(transcript_path).map_err(|e| AppError::StageIo {
        stage: "read-transcription".to_string(),
        source: e,
    })?;

    let words: Vec<TranscriptWord> =
        serde_json::from_str(&json_content).map_err(|e| AppError::ParseFailed {
            stage: "overlay-json".into(),
            message: e.to_string(),
        })?;

    let transcript: String = words
        .iter()
        .map(|w| w.word.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let prompt = format!(
        "Generate a short, punchy title (3-8 words, max 3 lines) for this talking head video. \
         The title should be a hook that grabs attention. \
         Split across 2-3 lines for visual impact (use newlines). \
         Keep each line to 2-4 words max. \
         Return ONLY the title text with newlines, nothing else. No quotes, no explanation.\n\n\
         Transcript: {}",
        transcript
    );

    let spinner = if !verbose {
        Some(ui::make_spinner("Generating title with Claude..."))
    } else {
        eprintln!("Running: claude -p <prompt> --model haiku");
        None
    };

    let output = Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--model")
        .arg("haiku")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(if verbose {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .output()
        .map_err(|e| AppError::StageIo {
            stage: "title-generation".to_string(),
            source: e,
        })?;

    if !output.status.success() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::ClaudeFailed {
            stage: "title-generation".into(),
            code: output.status.code().unwrap_or(-1),
            stderr,
        }
        .into());
    }

    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if title.is_empty() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        return Err(AppError::ClaudeFailed {
            stage: "title-generation".into(),
            code: 0,
            stderr: "claude returned empty title".into(),
        }
        .into());
    }

    if let Some(pb) = spinner {
        pb.finish_with_message("Title generated");
    }

    Ok(title)
}

fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\'', "'\\\\\\''")
        .replace(':', "\\:")
        .replace(';', "\\;")
}

#[cfg(target_os = "macos")]
const DEFAULT_FONT: &str = "/System/Library/Fonts/Supplemental/Impact.ttf";

#[cfg(target_os = "windows")]
const DEFAULT_FONT: &str = "C:\\Windows\\Fonts\\impact.ttf";

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/msttcorefonts/Impact.ttf",
    "/usr/share/fonts/msttcorefonts/Impact.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
];

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn resolve_default_font() -> String {
    for path in FONT_CANDIDATES {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    FONT_CANDIDATES[0].to_string()
}

const SLIDE_DURATION: f64 = 0.25;
const STAGGER_DELAY: f64 = 0.08;

fn build_title_filter(text: &str, args: &OverlayArgs) -> String {
    let t_start = args.start;
    let t_end = if args.duration > 0.0 {
        args.start + args.duration
    } else {
        f64::MAX
    };

    let y_base: u32 = match args.position.as_str() {
        "bottom" => 1400,
        "center" => 760,
        _ => 60,
    };

    let font_path = args
        .font
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            { DEFAULT_FONT.to_string() }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            { resolve_default_font() }
        });

    let final_x: i32 = 30;
    let box_pad: u32 = 10;
    let accent_w: u32 = 8;
    let accent_x: i32 = final_x - accent_w as i32 - 4;

    let lines: Vec<&str> = text.split('\n').collect();
    let line_count = lines.len();
    let line_height = args.font_size + box_pad * 2 + 4;

    let mut parts: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let escaped = escape_drawtext(line.trim());
        let y = y_base + i as u32 * line_height;

        // entrance: stagger by line index
        let line_start = t_start + i as f64 * STAGGER_DELAY;
        let slide_in_end = line_start + SLIDE_DURATION;

        // exit: reverse stagger (last line exits first)
        let exit_offset = (line_count - 1 - i) as f64 * STAGGER_DELAY;
        let exit_start = t_end - SLIDE_DURATION - exit_offset;

        // x expression: slide in from left, hold, slide out to right
        // phase 1: off-screen left
        // phase 2: sliding in
        // phase 3: stationary
        // phase 4: sliding out to right
        let x_expr = format!(
            "if(lt(t\\,{in_s})\\, -text_w-{bp}\\, if(lt(t\\,{in_e})\\, -text_w-{bp}+(text_w+{bp}+{fx})*(t-{in_s})/{dur}\\, if(lt(t\\,{out_s})\\, {fx}\\, {fx}+(w-{fx})*(t-{out_s})/{dur})))",
            in_s = line_start,
            in_e = slide_in_end,
            fx = final_x,
            bp = box_pad,
            dur = SLIDE_DURATION,
            out_s = exit_start
        );

        // white box with black text
        parts.push(format!(
            "drawtext=text='{txt}':fontsize={fs}:fontcolor=black:x='{x_expr}':y={y}:box=1:boxcolor=white@0.95:boxborderw={bp}:fontfile='{font}':enable='between(t,{line_s},{te})'",
            txt = escaped, fs = args.font_size, y = y, bp = box_pad, font = font_path,
            line_s = line_start, te = t_end
        ));

        // orange accent bar on the left (appears once text lands, hides on exit)
        let bar_h = args.font_size + box_pad * 2;
        let bar_y = y as i32 - box_pad as i32 + 2;
        parts.push(format!(
            "drawbox=x={ax}:y={bar_y}:w={aw}:h={bh}:color=#FF6B00:t=fill:enable='between(t,{in_e},{out_s})'",
            ax = accent_x, bar_y = bar_y, aw = accent_w, bh = bar_h,
            in_e = slide_in_end, out_s = exit_start
        ));
    }

    parts.join(",")
}

pub fn run(args: OverlayArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    require_ffmpeg()?;

    if !args.input.exists() {
        return Err(AppError::InputNotFound(args.input).into());
    }

    if args.auto.is_some() {
        require_claude()?;
    }

    let title_text = if let Some(ref transcript_path) = args.auto {
        if !transcript_path.exists() {
            return Err(AppError::InputNotFound(transcript_path.clone()).into());
        }
        let title = generate_title(transcript_path, verbose)?;
        eprintln!("\u{2713} Generated title: \"{}\"", title);
        title
    } else {
        args.text.clone().unwrap_or_default()
    };

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

    let drawtext_filter = build_title_filter(&title_text, &args);

    let ffmpeg_args = [
        "-i",
        &input_str,
        "-vf",
        &drawtext_filter,
        "-c:v",
        "libx264",
        "-crf",
        "14",
        "-preset",
        "slow",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "copy",
        &temp_str,
    ];

    let filename = args.input.file_name().unwrap_or_default().to_string_lossy();
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
