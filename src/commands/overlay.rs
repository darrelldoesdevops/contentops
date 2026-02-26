use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

use humansize::{DECIMAL, format_size};
use serde::Deserialize;

use crate::cli::OverlayArgs;
use crate::commands::cut::derive_output_path;
use crate::error::{AppError, last_n_lines, require_claude, require_ffmpeg};
use crate::ffmpeg;
use crate::temp::{TempFileRegistry, make_temp_file};
use crate::tiktok;
use crate::ui;

#[derive(Deserialize)]
struct TranscriptWord {
    word: String,
}

fn parse_title_options(response: &str) -> Vec<String> {
    let options: Vec<String> = response
        .split("---")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if options.len() < 2 {
        vec![response.trim().to_string()]
    } else {
        options
    }
}

pub fn generate_title_options(
    transcript_path: &Path,
    verbose: bool,
) -> anyhow::Result<Vec<String>> {
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
        "Generate exactly 3 different short, punchy title options (3-8 words each, max 3 lines each) for this talking head video. \
         Each title should be a hook that grabs attention. \
         Split each title across 2-3 lines for visual impact (use newlines within each title). \
         Keep each line to 2-4 words max. \
         Separate each title option with --- on its own line. \
         Return ONLY the title options separated by ---, nothing else. No quotes, no explanation, no numbering.\n\n\
         Transcript: {}",
        transcript
    );

    let spinner = if !verbose {
        Some(ui::make_spinner("Generating title options with Claude..."))
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

    let response = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if response.is_empty() {
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
        pb.finish_with_message("Title options generated");
    }

    Ok(parse_title_options(&response))
}

pub fn approve_title(
    options: &[String],
    no_interactive: bool,
    _verbose: bool,
) -> anyhow::Result<String> {
    if options.is_empty() {
        return Err(AppError::ClaudeFailed {
            stage: "title-approval".into(),
            code: 0,
            stderr: "no title options to approve".into(),
        }
        .into());
    }

    if no_interactive || !std::io::stdin().is_terminal() {
        let first = &options[0];
        eprintln!("Auto-selected title: \"{}\"", first.replace('\n', " / "));
        return Ok(first.clone());
    }

    let mut items: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let preview = opt.replace('\n', " / ");
            format!("{}. {}", i + 1, preview)
        })
        .collect();
    items.push("Custom...".to_string());

    let selection = dialoguer::Select::new()
        .with_prompt("Select a title")
        .items(&items)
        .default(0)
        .interact()?;

    let chosen = if selection < options.len() {
        options[selection].clone()
    } else {
        let custom: String = dialoguer::Input::new()
            .with_prompt("Enter custom title")
            .interact_text()?;
        custom
    };

    eprintln!("\u{2713} Title: \"{}\"", chosen.replace('\n', " / "));
    Ok(chosen)
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

const REF_HEIGHT: f64 = 1920.0;

fn scale(value: u32, video_height: u32) -> u32 {
    ((value as f64 * video_height as f64 / REF_HEIGHT) + 0.5) as u32
}

fn scale_i32(value: i32, video_height: u32) -> i32 {
    (value as f64 * video_height as f64 / REF_HEIGHT + 0.5) as i32
}

fn wrap_title_lines(text: &str, font_size: u32, max_width: u32) -> Vec<String> {
    let avg_char_px = font_size as f64 * 0.55;
    let max_chars = (max_width as f64 / avg_char_px).floor() as usize;

    let mut result = Vec::new();
    for input_line in text.split('\n') {
        let words: Vec<&str> = input_line.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        let mut current_line = String::new();

        for word in words {
            let candidate = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            if candidate.len() <= max_chars || current_line.is_empty() {
                current_line = candidate;
            } else {
                result.push(current_line.clone());
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            result.push(current_line);
        }
    }
    result
}

fn build_title_filter(text: &str, args: &OverlayArgs, video_height: u32) -> String {
    let t_start = args.start;
    let t_end = if args.duration > 0.0 {
        args.start + args.duration
    } else {
        f64::MAX
    };

    let y_base: u32 = match args.position.as_str() {
        "bottom" => scale(1400, video_height),
        "center" => scale(760, video_height),
        _ => scale(200, video_height),
    };

    let font_path = args
        .font
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                DEFAULT_FONT.to_string()
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                resolve_default_font()
            }
        });

    let font_size = scale(args.font_size, video_height);
    let final_x: i32 = scale_i32(80, video_height);
    let box_pad: u32 = scale(10, video_height);
    let accent_w: u32 = scale(8, video_height);
    let accent_x: i32 = final_x - accent_w as i32 - scale_i32(4, video_height);

    let wrapped = wrap_title_lines(text, font_size, tiktok::SAFE_WIDTH);
    let lines: Vec<&str> = wrapped.iter().map(|s| s.as_str()).collect();
    let line_count = lines.len();
    let line_height = font_size + box_pad * 2 + scale(4, video_height);

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
            txt = escaped, fs = font_size, y = y, bp = box_pad, font = font_path,
            line_s = line_start, te = t_end
        ));

        // orange accent bar on the left (appears once text lands, hides on exit)
        let bar_h = font_size + box_pad * 2;
        let bar_y = y as i32 - box_pad as i32 + scale_i32(2, video_height);
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
        let options = generate_title_options(transcript_path, verbose)?;
        approve_title(&options, args.no_interactive, verbose)?
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

    let video_height = ffmpeg::probe_dimensions(&input_str)
        .map(|(_, h)| h)
        .unwrap_or(1920);
    let drawtext_filter = build_title_filter(&title_text, &args, video_height);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_title_stays_single_line() {
        // "HELLO" at font_size=144, max_width=900
        // avg_char_px = 144 * 0.55 = 79.2, max_chars = floor(900/79.2) = 11
        let lines = wrap_title_lines("HELLO", 144, 900);
        assert_eq!(lines, vec!["HELLO"]);
    }

    #[test]
    fn two_word_title_fits_single_line() {
        // "HELLO WORLD" = 11 chars, max_chars = 11 → fits
        let lines = wrap_title_lines("HELLO WORLD", 144, 900);
        assert_eq!(lines, vec!["HELLO WORLD"]);
    }

    #[test]
    fn long_title_wraps_to_multiple_lines() {
        // "THIS IS A VERY LONG TITLE THAT SHOULD WRAP" at font_size=144
        // max_chars = 11, so each line can hold ~11 chars
        let lines = wrap_title_lines("THIS IS A VERY LONG TITLE THAT SHOULD WRAP", 144, 900);
        assert!(lines.len() > 1, "expected multiple lines, got: {:?}", lines);
        for line in &lines {
            // Each line should be at most ~11 chars (except single long words)
            assert!(
                line.len() <= 15,
                "line too long: '{}' ({} chars)",
                line,
                line.len()
            );
        }
    }

    #[test]
    fn multiline_input_preserves_newlines() {
        let lines = wrap_title_lines("LINE ONE\nLINE TWO", 144, 900);
        assert_eq!(lines, vec!["LINE ONE", "LINE TWO"]);
    }

    #[test]
    fn single_long_word_not_split() {
        // A single word longer than max_chars should stay on one line
        let lines = wrap_title_lines("SUPERLONGWORD", 144, 900);
        assert_eq!(lines, vec!["SUPERLONGWORD"]);
    }

    #[test]
    fn empty_input_returns_empty() {
        let lines = wrap_title_lines("", 144, 900);
        assert!(lines.is_empty());
    }

    #[test]
    fn smaller_font_allows_more_chars_per_line() {
        // font_size=72, avg_char_px = 39.6, max_chars = floor(900/39.6) = 22
        let text = "THIS IS A LONGER LINE THAT FITS";
        let lines = wrap_title_lines(text, 72, 900);
        // 30 chars including spaces — should wrap at ~22
        assert!(lines.len() >= 1);
        assert!(
            lines.len() <= 2,
            "expected 1-2 lines at font_size=72, got: {:?}",
            lines
        );
    }

    #[test]
    fn parse_title_options_three_options() {
        let response =
            "STOP DOING\nTHIS NOW\n---\nYOU NEED TO\nHEAR THIS\n---\nTHE TRUTH\nABOUT DEVOPS";
        let options = parse_title_options(response);
        assert_eq!(options.len(), 3);
        assert!(options[0].contains("STOP DOING"));
        assert!(options[1].contains("YOU NEED TO"));
        assert!(options[2].contains("THE TRUTH"));
    }

    #[test]
    fn parse_title_options_no_delimiter() {
        let response = "STOP DOING\nTHIS NOW";
        let options = parse_title_options(response);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0], "STOP DOING\nTHIS NOW");
    }

    #[test]
    fn parse_title_options_empty_sections() {
        let response = "---\nTITLE ONE\n---\n---\nTITLE TWO\n---";
        let options = parse_title_options(response);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0], "TITLE ONE");
        assert_eq!(options[1], "TITLE TWO");
    }
}
