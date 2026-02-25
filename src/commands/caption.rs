use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use humansize::{DECIMAL, format_size};
use serde::Deserialize;

use crate::cli::CaptionArgs;
use crate::error::{
    AppError, last_n_lines, require_claude, require_ffmpeg, require_ffmpeg_libass, require_whisper,
};
use crate::ffmpeg;
use crate::temp::{TempFileRegistry, make_temp_file};
use crate::tiktok;
use crate::ui;

#[derive(serde::Serialize, Deserialize, Clone)]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

pub struct SrtEntry {
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

pub fn group_words_into_srt(words: &[Word]) -> Vec<SrtEntry> {
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

struct AssGroup {
    words: Vec<AssWord>,
}

const HIGHLIGHT_COLOR: &str = "&HFFFF00&";

struct AssWord {
    text: String,
    start: f64,
    end: f64,
}

fn group_words_for_ass(words: &[Word]) -> Vec<AssGroup> {
    let mut groups = Vec::new();
    let mut current: Vec<&Word> = Vec::new();

    for word in words {
        current.push(word);

        let ends_with_punctuation = word.word.ends_with('.')
            || word.word.ends_with('!')
            || word.word.ends_with('?')
            || word.word.ends_with(',');

        if current.len() >= 5 || (current.len() >= 3 && ends_with_punctuation) {
            let ass_words = current
                .iter()
                .map(|w| AssWord {
                    text: w.word.clone(),
                    start: w.start,
                    end: w.end,
                })
                .collect();
            groups.push(AssGroup { words: ass_words });
            current.clear();
        }
    }

    if !current.is_empty() {
        let ass_words = current
            .iter()
            .map(|w| AssWord {
                text: w.word.clone(),
                start: w.start,
                end: w.end,
            })
            .collect();
        groups.push(AssGroup { words: ass_words });
    }

    groups
}

fn format_ass_time(seconds: f64) -> String {
    let total_cs = (seconds * 100.0).round() as u64;
    let hours = total_cs / 360_000;
    let minutes = (total_cs % 360_000) / 6_000;
    let secs = (total_cs % 6_000) / 100;
    let cs = total_cs % 100;
    format!("{}:{:02}:{:02}.{:02}", hours, minutes, secs, cs)
}

pub fn generate_ass(words: &[Word]) -> String {
    let mut output = String::new();

    // Script Info
    output.push_str("[Script Info]\n");
    output.push_str("ScriptType: v4.00+\n");
    output.push_str("PlayResX: 1080\n");
    output.push_str("PlayResY: 1920\n");
    output.push_str("WrapStyle: 0\n");
    output.push('\n');

    // V4+ Styles
    output.push_str("[V4+ Styles]\n");
    output.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");
    output.push_str(&format!(
        "Style: Default,Arial,58,&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,4,2,2,{},{},{},1\n",
        tiktok::SAFE_MARGIN_LEFT,
        tiktok::SAFE_MARGIN_RIGHT,
        tiktok::SAFE_MARGIN_BOTTOM,
    ));
    output.push('\n');

    // Events
    output.push_str("[Events]\n");
    output.push_str(
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );

    let groups = group_words_for_ass(words);
    for (group_idx, group) in groups.iter().enumerate() {
        let next_group_start = if group_idx + 1 < groups.len() {
            Some(groups[group_idx + 1].words[0].start)
        } else {
            None
        };

        for (active_idx, _) in group.words.iter().enumerate() {
            let start = format_ass_time(group.words[active_idx].start);
            let end = if active_idx + 1 < group.words.len() {
                format_ass_time(group.words[active_idx + 1].start)
            } else {
                let raw_end = group.words[active_idx].end;
                let clamped = match next_group_start {
                    Some(next_start) => raw_end.min(next_start),
                    None => raw_end,
                };
                format_ass_time(clamped)
            };

            let mut text = String::new();
            for (i, w) in group.words.iter().enumerate() {
                if i == active_idx {
                    text.push_str(&format!("{{\\c{}}}", HIGHLIGHT_COLOR));
                    text.push_str(&w.text.to_uppercase());
                    text.push_str("{\\c&H00FFFFFF&}");
                } else {
                    text.push_str(&w.text.to_uppercase());
                }
                if i < group.words.len() - 1 {
                    text.push(' ');
                }
            }

            output.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
                start, end, text
            ));
        }
    }

    output
}

pub fn format_srt(entries: &[SrtEntry]) -> String {
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

fn print_mismatch_context(original: &[Word], corrected: &[Word]) {
    eprintln!(
        "Warning: LLM returned {} words, expected {}",
        corrected.len(),
        original.len()
    );
    let min_len = original.len().min(corrected.len());
    let diffs: Vec<_> = original[..min_len]
        .iter()
        .zip(corrected[..min_len].iter())
        .filter(|(o, c)| o.word != c.word)
        .take(5)
        .collect();
    for (orig, fixed) in &diffs {
        eprintln!("  \"{}\" -> \"{}\"", orig.word, fixed.word);
    }
    let diff_count = original.len().abs_diff(corrected.len());
    if corrected.len() > original.len() {
        eprintln!("  ({} word(s) added)", diff_count);
    } else {
        eprintln!("  ({} word(s) removed)", diff_count);
    }
}

fn invoke_claude_fix(
    words: &[Word],
    verbose: bool,
    enforce_count: bool,
) -> anyhow::Result<Option<Vec<Word>>> {
    require_claude()?;

    let words_json = serde_json::to_string_pretty(words).unwrap_or_default();

    let mut prompt = format!(
        "This is speech-to-text output as JSON. Each object has word, start, end (seconds).\n\
         Fix transcription errors: misspellings, words that don't fit context, \
         phonetic mishearings (e.g. \"suit\" \u{2192} \"soon\", \"school\" \u{2192} \"skool\" if that's a brand/community name).\n\
         Use surrounding words and timing to infer what was actually said.\n\
         Return the SAME JSON array structure with ONLY the \"word\" fields corrected.\n\
         Do NOT change start/end times. Do NOT add or remove entries.\n\
         Do NOT add punctuation or change capitalization.\n\
         Return raw JSON only, no markdown fences.\n\n{}",
        words_json
    );

    if enforce_count {
        prompt.push_str(&format!(
            "\n\nCRITICAL: You MUST return EXACTLY {} entries. Do NOT add or remove words.",
            words.len()
        ));
    }

    let spinner = if !verbose {
        let msg = if enforce_count {
            "Retrying transcription fix with Claude..."
        } else {
            "Fixing transcription with Claude..."
        };
        Some(crate::ui::make_spinner(msg))
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
            stage: "fix-transcription".to_string(),
            source: e,
        })?;

    if !output.status.success() {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        eprintln!("Warning: transcription fix failed, using original words");
        return Ok(None);
    }

    let raw_stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stdout = if raw_stdout.starts_with("```") {
        raw_stdout
            .strip_prefix("```json")
            .or_else(|| raw_stdout.strip_prefix("```"))
            .unwrap_or(&raw_stdout)
            .strip_suffix("```")
            .unwrap_or(&raw_stdout)
            .trim()
    } else {
        &raw_stdout
    };

    let corrected: Vec<Word> = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(e) => {
            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }
            if verbose {
                eprintln!("LLM response parse error: {}", e);
                eprintln!(
                    "Raw response:\n{}",
                    &raw_stdout[..raw_stdout.len().min(500)]
                );
            }
            eprintln!("Warning: could not parse LLM response, using original words");
            return Ok(None);
        }
    };

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    Ok(Some(corrected))
}

fn apply_fixes(words: &mut [Word], corrected: &[Word], verbose: bool) -> usize {
    let mut fixes = 0;
    let min_len = words.len().min(corrected.len());
    for i in 0..min_len {
        if words[i].word != corrected[i].word {
            if verbose {
                eprintln!(
                    "  fix: \"{}\" \u{2192} \"{}\"",
                    words[i].word, corrected[i].word
                );
            }
            words[i].word = corrected[i].word.clone();
            fixes += 1;
        }
    }
    fixes
}

pub fn fix_transcription(words: &mut [Word], verbose: bool) -> anyhow::Result<()> {
    let corrected = match invoke_claude_fix(words, verbose, false)? {
        Some(c) => c,
        None => return Ok(()),
    };

    if corrected.len() == words.len() {
        let fixes = apply_fixes(words, &corrected, verbose);
        if fixes > 0 {
            eprintln!("Transcription fixed ({} corrections)", fixes);
        }
        return Ok(());
    }

    print_mismatch_context(words, &corrected);

    if !std::io::stdin().is_terminal() {
        return Err(AppError::TranscriptMismatch {
            expected: words.len(),
            actual: corrected.len(),
        }
        .into());
    }

    let choices = &[
        "Use originals (keep timing safe)",
        "Use fixed (accept word count change)",
        "Retry (re-run with word count constraint)",
    ];
    let selection = dialoguer::Select::new()
        .with_prompt("How to handle mismatch?")
        .items(choices)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            eprintln!("Using original words");
            Ok(())
        }
        1 => {
            let fixes = apply_fixes(words, &corrected, verbose);
            eprintln!("Applied {} corrections from fixed version", fixes);
            Ok(())
        }
        2 => handle_retry(words, &corrected, verbose),
        _ => unreachable!(),
    }
}

fn handle_retry(
    words: &mut [Word],
    first_fix: &[Word],
    verbose: bool,
) -> anyhow::Result<()> {
    let retry_result = match invoke_claude_fix(words, verbose, true)? {
        Some(r) => r,
        None => {
            eprintln!("Retry failed to parse. Falling back to selection.");
            return offer_post_retry_menu(words, first_fix, None, verbose);
        }
    };

    if retry_result.len() == words.len() {
        let fixes = apply_fixes(words, &retry_result, verbose);
        eprintln!("Retry succeeded \u{2014} {} corrections applied", fixes);
        return Ok(());
    }

    eprintln!(
        "\nRetry also returned wrong count ({} words).",
        retry_result.len()
    );
    offer_post_retry_menu(words, first_fix, Some(&retry_result), verbose)
}

fn offer_post_retry_menu(
    words: &mut [Word],
    first_fix: &[Word],
    retry_fix: Option<&[Word]>,
    verbose: bool,
) -> anyhow::Result<()> {
    eprintln!("\nComparison (first 20 words):");
    eprintln!(
        "  Original:  {}",
        words
            .iter()
            .take(20)
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    eprintln!(
        "  First fix: {}",
        first_fix
            .iter()
            .take(20)
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    if let Some(retry) = retry_fix {
        eprintln!(
            "  Retry fix: {}",
            retry
                .iter()
                .take(20)
                .map(|w| w.word.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    let mut post_choices = vec![
        "Use originals (keep timing safe)",
        "Use first fix",
    ];
    if retry_fix.is_some() {
        post_choices.push("Use retry fix");
    }
    post_choices.push("Abort pipeline");

    let selection = dialoguer::Select::new()
        .with_prompt("Choose version")
        .items(&post_choices)
        .default(0)
        .interact()?;

    let abort_idx = post_choices.len() - 1;

    match selection {
        0 => {
            eprintln!("Using original words");
            Ok(())
        }
        1 => {
            let fixes = apply_fixes(words, first_fix, verbose);
            eprintln!("Applied {} corrections from first fix", fixes);
            Ok(())
        }
        idx if retry_fix.is_some() && idx == 2 => {
            let fixes = apply_fixes(words, retry_fix.unwrap(), verbose);
            eprintln!("Applied {} corrections from retry fix", fixes);
            Ok(())
        }
        idx if idx == abort_idx => {
            let actual = retry_fix
                .map(|r| r.len())
                .unwrap_or(first_fix.len());
            Err(AppError::TranscriptMismatch {
                expected: words.len(),
                actual,
            }
            .into())
        }
        _ => unreachable!(),
    }
}

pub fn transcribe(
    input: &Path,
    model: &Path,
    lang: &str,
    verbose: bool,
    registry: &TempFileRegistry,
    wav_path: Option<&Path>,
) -> anyhow::Result<Vec<Word>> {
    require_ffmpeg()?;
    require_whisper()?;

    let owns_wav = wav_path.is_none();
    let parent_dir = input.parent().unwrap_or(Path::new("."));

    let (temp_wav, _temp_file) = if let Some(provided) = wav_path {
        (provided.to_path_buf(), None)
    } else {
        let temp_file = make_temp_file(parent_dir, ".wav")?;
        let path = temp_file.path().to_path_buf();
        registry.register(path.clone());

        let input_str = input.to_string_lossy();

        let spinner = if !verbose {
            let filename = input.file_name().unwrap_or_default().to_string_lossy();
            Some(ui::make_spinner(format!("Extracting audio from {}...", filename)))
        } else {
            None
        };

        ffmpeg::extract_16k_wav(&input_str, &path, verbose).map_err(|e| {
            if let Some(pb) = &spinner {
                pb.finish_and_clear();
            }
            AppError::StageIo {
                stage: "audio-extraction".to_string(),
                source: e,
            }
        })?;

        if let Some(ref pb) = spinner {
            pb.finish_with_message("Audio extracted");
        }

        (path, Some(temp_file))
    };

    let wav_str = temp_wav.to_string_lossy();

    let spinner = if !verbose {
        Some(ui::make_spinner("Transcribing with Whisper..."))
    } else {
        eprintln!(
            "Running: whisper-cli -m {} -f {} --output-json --max-len 1 -l {}",
            model.display(),
            wav_str,
            lang
        );
        None
    };

    let whisper_output = Command::new("whisper-cli")
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&temp_wav)
        .arg("--output-json")
        .arg("--max-len")
        .arg("1")
        .arg("--split-on-word")
        .arg("-l")
        .arg(lang)
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

    let whisper_json_path = PathBuf::from(format!("{}.json", temp_wav.display()));
    registry.register(whisper_json_path.clone());

    let json_content = std::fs::read_to_string(&whisper_json_path)
        .map_err(|e| AppError::StageIo {
            stage: "read-whisper-json".to_string(),
            source: e,
        })?;

    let whisper_data: WhisperJson =
        serde_json::from_str(&json_content).map_err(|e| AppError::ParseFailed {
            stage: "whisper-json".into(),
            message: e.to_string(),
        })?;

    let raw_words: Vec<Word> = whisper_data
        .transcription
        .iter()
        .map(|seg| {
            let text = seg
                .text
                .trim()
                .replace(|c: char| c.is_ascii_punctuation() && c != '\'', "");
            Word {
                word: text,
                start: parse_timestamp(&seg.timestamps.from),
                end: parse_timestamp(&seg.timestamps.to),
            }
        })
        .filter(|w| !w.word.is_empty())
        .collect();

    let mut words: Vec<Word> = Vec::with_capacity(raw_words.len());
    for w in raw_words {
        if !words.is_empty() && w.word.starts_with('\'') {
            let prev = words.last_mut().unwrap();
            prev.word.push_str(&w.word);
            prev.end = w.end;
        } else {
            words.push(w);
        }
    }

    if owns_wav {
        let _ = std::fs::remove_file(&temp_wav);
        registry.remove(&temp_wav);
    }
    let _ = std::fs::remove_file(&whisper_json_path);
    registry.remove(&whisper_json_path);

    Ok(words)
}

pub fn burn_captions(
    input: &Path,
    words: &[Word],
    output: &Path,
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<()> {
    require_ffmpeg_libass()?;

    let parent_dir = input.parent().unwrap_or(Path::new("."));
    let ass_content = generate_ass(words);
    let ass_temp = make_temp_file(parent_dir, ".ass")?;
    let ass_path = ass_temp.path().to_path_buf();
    registry.register(ass_path.clone());
    std::fs::write(&ass_path, ass_content)
        .map_err(|e| AppError::StageIo { stage: "write-ass".to_string(), source: e })?;

    let input_str = input.to_string_lossy();
    let vf_string = format!("ass={}", ass_path.to_string_lossy());
    let output_str = output.to_string_lossy().to_string();

    let burn_args = [
        "-i", &input_str, "-vf", &vf_string,
        "-c:v", "libx264", "-crf", "14", "-preset", "slow", "-pix_fmt", "yuv420p",
        "-c:a", "copy", &output_str,
    ];

    let spinner = if !verbose {
        Some(ui::make_spinner("Burning captions..."))
    } else {
        eprintln!("Running: ffmpeg {}", burn_args.join(" "));
        None
    };

    let result = if verbose {
        ffmpeg::run_ffmpeg_verbose(&burn_args)
    } else {
        ffmpeg::run_ffmpeg(&burn_args)
    };

    let _ = std::fs::remove_file(&ass_path);
    registry.remove(&ass_path);

    match result {
        Ok(ref o) if o.success => {
            if let Some(ref pb) = spinner { pb.finish_with_message("Captions burned"); }
        }
        Ok(o) => {
            if let Some(pb) = spinner { pb.finish_and_clear(); }
            let truncated = last_n_lines(&o.stderr, 20);
            return Err(AppError::FfmpegFailed {
                stage: "caption-burn".to_string(), code: o.exit_code.unwrap_or(-1), stderr: truncated,
            }.into());
        }
        Err(io_err) => {
            if let Some(pb) = spinner { pb.finish_and_clear(); }
            return Err(AppError::StageIo { stage: "caption-burn".to_string(), source: io_err }.into());
        }
    }

    Ok(())
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
        "-i", &input_str, "-ar", "16000", "-ac", "1", "-f", "wav", &wav_str,
    ];

    let spinner = if !verbose {
        let filename = args.input.file_name().unwrap_or_default().to_string_lossy();
        Some(ui::make_spinner(format!(
            "Extracting audio from {}...",
            filename
        )))
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

    // 4. Transcription via whisper-cli
    let spinner = if !verbose {
        Some(ui::make_spinner("Transcribing with Whisper..."))
    } else {
        eprintln!(
            "Running: whisper-cli -m {} -f {} --output-json --max-len 1 -l {}",
            args.model.display(),
            wav_str,
            args.lang
        );
        None
    };

    let whisper_output = Command::new("whisper-cli")
        .arg("-m")
        .arg(&args.model)
        .arg("-f")
        .arg(&temp_wav)
        .arg("--output-json")
        .arg("--max-len")
        .arg("1")
        .arg("--split-on-word")
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

    // 5. Parse whisper-cli JSON output
    let whisper_json_path = PathBuf::from(format!("{}.json", temp_wav.display()));
    registry.register(whisper_json_path.clone());

    let json_content =
        std::fs::read_to_string(&whisper_json_path).map_err(|e| AppError::StageIo {
            stage: "read-whisper-json".to_string(),
            source: e,
        })?;

    let whisper_data: WhisperJson =
        serde_json::from_str(&json_content).map_err(|e| AppError::ParseFailed {
            stage: "whisper-json".into(),
            message: e.to_string(),
        })?;

    let raw_words: Vec<Word> = whisper_data
        .transcription
        .iter()
        .map(|seg| {
            let text = seg
                .text
                .trim()
                .replace(|c: char| c.is_ascii_punctuation() && c != '\'', "");
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

    let mut words: Vec<Word> = Vec::with_capacity(raw_words.len());
    for w in raw_words {
        let should_merge = !words.is_empty() && w.word.starts_with('\'');
        if should_merge {
            let prev = words.last_mut().unwrap();
            prev.word.push_str(&w.word);
            prev.end = w.end;
        } else {
            words.push(w);
        }
    }

    // 5b. Fix transcription with LLM (if --fix flag passed)
    if args.fix && !words.is_empty() {
        fix_transcription(&mut words, verbose)?;
    }

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
            serde_json::to_string_pretty(&words).map_err(|e| AppError::ParseFailed {
                stage: "caption-json".into(),
                message: e.to_string(),
            })?;
        std::fs::write(&json_output, json_content).map_err(|e| AppError::StageIo {
            stage: "write-json".to_string(),
            source: e,
        })?;
    }

    // 8. Burn captions into video (if --burn flag passed)
    if args.burn && !words.is_empty() {
        require_ffmpeg_libass()?;
        let video_output = derive_caption_output(&args.input, "captioned", "mp4");

        let ass_content = generate_ass(&words);
        let ass_temp = make_temp_file(parent_dir, ".ass")?;
        let ass_path = ass_temp.path().to_path_buf();
        registry.register(ass_path.clone());
        std::fs::write(&ass_path, ass_content).map_err(|e| AppError::StageIo {
            stage: "write-ass".to_string(),
            source: e,
        })?;

        let vf_string = format!("ass={}", ass_path.to_string_lossy());
        let video_output_str = video_output.to_string_lossy().to_string();

        let burn_args = [
            "-i",
            &input_str,
            "-vf",
            &vf_string,
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
            &video_output_str,
        ];

        let spinner = if !verbose {
            Some(ui::make_spinner("Burning captions..."))
        } else {
            eprintln!("Running: ffmpeg {}", burn_args.join(" "));
            None
        };

        let result = if verbose {
            ffmpeg::run_ffmpeg_verbose(&burn_args)
        } else {
            ffmpeg::run_ffmpeg(&burn_args)
        };

        let _ = std::fs::remove_file(&ass_path);
        registry.remove(&ass_path);

        match result {
            Ok(ref output) if output.success => {
                if let Some(ref pb) = spinner {
                    pb.finish_with_message("Captions burned");
                }
            }
            Ok(output) => {
                if let Some(pb) = spinner {
                    pb.finish_and_clear();
                }
                let truncated = last_n_lines(&output.stderr, 20);
                let code = output.exit_code.unwrap_or(-1);
                return Err(AppError::FfmpegFailed {
                    stage: "caption-burn".to_string(),
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
                    stage: "caption-burn".to_string(),
                    source: io_err,
                }
                .into());
            }
        }

        let video_size = std::fs::metadata(&video_output)
            .map(|m| format_size(m.len(), DECIMAL))
            .unwrap_or_else(|_| "unknown size".to_string());
        eprintln!(
            "\u{2713} Created {} ({})",
            video_output.display(),
            video_size
        );
    }

    // 9. Cleanup temp files
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

    eprintln!("\u{2713} Created {} ({})", srt_output.display(), srt_size);
    eprintln!("\u{2713} Created {} ({})", json_output.display(), json_size);

    Ok(())
}
