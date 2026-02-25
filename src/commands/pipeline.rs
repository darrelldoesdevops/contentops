use std::path::Path;

use humansize::{DECIMAL, format_size};
use owo_colors::OwoColorize;

use crate::cli::{OverlayArgs, PipelineArgs};
use crate::commands::{caption, cut, normalize, overlay};
use crate::error::AppError;
use crate::ffmpeg;
use crate::silence;
use crate::temp::{TempFileRegistry, make_temp_file};
use crate::tiktok;
use crate::vad;

pub fn run(args: PipelineArgs, verbose: bool, registry: &TempFileRegistry) -> anyhow::Result<()> {
    if !args.input.exists() {
        return Err(AppError::InputNotFound(args.input).into());
    }

    if !args.model.exists() {
        return Err(AppError::ModelNotFound(args.model).into());
    }

    let output = args
        .output
        .unwrap_or_else(|| cut::derive_output_path(&args.input, "pipeline"));

    if args.dry_run {
        eprintln!(
            "{} Pipeline stages for {}:",
            "plan:".bold(),
            args.input.display()
        );
        eprintln!("  1. scale      \u{2192} Scale to 1080x1920 (skip if already correct)");
        eprintln!("  2. normalize  \u{2192} Audio normalization (loudnorm)");
        eprintln!(
            "  3. transcribe \u{2192} Whisper transcription (model: {})",
            args.model.display()
        );
        eprintln!("  4. fix        \u{2192} LLM transcription correction");
        eprintln!("  5. cut        \u{2192} VAD-based silence removal");
        eprintln!("  6. caption    \u{2192} Burn captions onto cut video");
        eprintln!("  7. overlay    \u{2192} Title approval + overlay");
        eprintln!();
        eprintln!("Output: {}", output.display());
        return Ok(());
    }

    let temp_dir = tempfile::tempdir().map_err(|e| AppError::StageIo {
        stage: "pipeline-setup".to_string(),
        source: e,
    })?;

    let result = run_stages(
        temp_dir.path(),
        &args.input,
        &args.model,
        &output,
        args.text.as_deref(),
        args.font_size,
        args.vad_threshold,
        args.min_silence_ms,
        args.no_interactive,
        verbose,
        registry,
    );

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            let preserved = temp_dir.keep();
            eprintln!();
            eprintln!(
                "  {}: intermediate files preserved at {}",
                "hint".bold(),
                preserved.display()
            );
            eprintln!(
                "  {}: inspect or retry individual stages from that directory",
                "hint".bold()
            );
            Err(err)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_stages(
    temp_dir: &Path,
    input: &Path,
    model: &Path,
    output: &Path,
    text: Option<&str>,
    font_size: Option<u32>,
    vad_threshold: f32,
    min_silence_ms: u32,
    no_interactive: bool,
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<()> {
    let parent_dir = input.parent().unwrap_or(Path::new("."));

    // Stage 1: Scale to TikTok resolution (skip if already 1080x1920)
    let input_str_for_scale = input.to_string_lossy().to_string();
    let dims = ffmpeg::probe_dimensions(&input_str_for_scale);
    let scaled_input = if dims != Some((tiktok::OUTPUT_WIDTH, tiktok::OUTPUT_HEIGHT)) {
        eprintln!("\n{}", "Stage 1/7: scale".bold());
        let scaled_path = temp_dir.join("scaled.mp4");
        let scaled_str = scaled_path.to_string_lossy().to_string();
        let result = ffmpeg::scale_to_tiktok(&input_str_for_scale, &scaled_str, verbose);
        match result {
            Ok(ref o) if o.success => {
                if let Some((w, h)) = dims {
                    eprintln!("\u{2713} Scaled {}x{} -> 1080x1920", w, h);
                } else {
                    eprintln!("\u{2713} Scaled to 1080x1920");
                }
                scaled_path
            }
            Ok(o) => {
                let truncated = crate::error::last_n_lines(&o.stderr, 20);
                return Err(AppError::FfmpegFailed {
                    stage: "scale".to_string(),
                    code: o.exit_code.unwrap_or(-1),
                    stderr: truncated,
                }.into());
            }
            Err(io_err) => {
                return Err(AppError::StageIo {
                    stage: "scale".to_string(),
                    source: io_err,
                }.into());
            }
        }
    } else {
        eprintln!("\n{}", "Stage 1/7: scale (skipped, already 1080x1920)".bold());
        input.to_path_buf()
    };

    // Stage 2: Normalize audio so all downstream timestamps are on the same timeline
    eprintln!("\n{}", "Stage 2/7: normalize".bold());
    let normalized = normalize::normalize_to_temp(&scaled_input, verbose, registry)?;
    let normalized_str = normalized.to_string_lossy().to_string();

    // Clean up scaled temp file if it was created
    if scaled_input != input {
        let _ = std::fs::remove_file(&scaled_input);
    }

    // Extract shared 16kHz WAV from NORMALIZED video (not original)
    // This ensures Whisper timestamps, VAD intervals, and concat filter
    // all operate on the same audio timeline
    let wav_temp = make_temp_file(parent_dir, ".wav")?;
    let wav_path = wav_temp.path().to_path_buf();
    registry.register(wav_path.clone());

    ffmpeg::extract_16k_wav(&normalized_str, &wav_path, verbose).map_err(|e| AppError::StageIo {
        stage: "wav-extraction".to_string(),
        source: e,
    })?;

    // Stage 3: Transcribe normalized video (reuses shared WAV)
    eprintln!("\n{}", "Stage 3/7: transcribe".bold());
    let mut words = caption::transcribe(input, model, "en", verbose, registry, Some(&wav_path))?;

    if words.is_empty() {
        eprintln!("Warning: No speech detected");
        return Ok(());
    }

    // Stage 4: LLM fix
    eprintln!("\n{}", "Stage 4/7: fix".bold());
    caption::fix_transcription(&mut words, verbose)?;

    // Stage 5: VAD-based silence removal
    eprintln!("\n{}", "Stage 5/7: cut".bold());

    let video_duration =
        ffmpeg::probe_duration_strict(&normalized_str).map_err(|e| AppError::StageIo {
            stage: "probe-duration".to_string(),
            source: e,
        })?;

    // Run VAD on shared WAV (same timeline as normalized video)
    let speeches = vad::run_vad(&wav_path, video_duration, vad_threshold, min_silence_ms)?;

    // Clean up shared WAV (no longer needed)
    let _ = std::fs::remove_file(&wav_path);
    registry.remove(&wav_path);

    if speeches.is_empty() {
        eprintln!("No speech detected -- skipping cut stage");
        let cut_output = temp_dir.join("cut.mp4");
        std::fs::copy(&normalized, &cut_output).map_err(|e| AppError::StageIo {
            stage: "copy-normalized".to_string(),
            source: e,
        })?;
        let adjusted_words = words.clone();

        return finish_stages(
            temp_dir,
            &cut_output,
            &adjusted_words,
            output,
            text,
            font_size,
            no_interactive,
            verbose,
            registry,
        );
    }

    let total_silence = silence::total_silence_from_speeches(&speeches, video_duration);
    eprintln!(
        "Found {} speech segments, removing {:.1}s of silence",
        speeches.len(),
        total_silence
    );

    // 3c: Build filter and cut
    let cut_output = temp_dir.join("cut.mp4");
    let concat_filter = silence::build_concat_filter(&speeches);
    let cut_str = cut_output.to_string_lossy().to_string();

    let ffmpeg_args = vec![
        "-i",
        &normalized_str,
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
        &cut_str,
    ];

    let filename = input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let message = format!("Cutting silence from {}...", filename);

    let result = if verbose {
        eprintln!("Running: ffmpeg {}", ffmpeg_args.join(" "));
        ffmpeg::run_ffmpeg_verbose(&ffmpeg_args)
    } else {
        let duration = ffmpeg::probe_duration(&normalized_str);
        ffmpeg::run_ffmpeg_with_progress(&ffmpeg_args, duration, &message)
    };

    // Clean up normalized temp file
    let _ = std::fs::remove_file(&normalized);
    registry.remove(&normalized);

    match result {
        Ok(ref o) if o.success => {
            eprintln!(
                "\u{2713} Removed {:.1}s of silence ({} regions)",
                total_silence,
                speeches.len()
            );
        }
        Ok(o) => {
            let truncated = crate::error::last_n_lines(&o.stderr, 20);
            return Err(AppError::FfmpegFailed {
                stage: "cut".to_string(),
                code: o.exit_code.unwrap_or(-1),
                stderr: truncated,
            }
            .into());
        }
        Err(io_err) => {
            return Err(AppError::StageIo {
                stage: "cut".to_string(),
                source: io_err,
            }
            .into());
        }
    }

    // 3d: Adjust word timestamps to match the cut video
    let word_data: Vec<(f64, f64, String)> = words
        .iter()
        .map(|w| (w.start, w.end, w.word.clone()))
        .collect();
    let adjusted = silence::adjust_timestamps(&word_data, &speeches);
    let adjusted_words: Vec<caption::Word> = adjusted
        .into_iter()
        .map(|(start, end, word)| caption::Word { word, start, end })
        .collect();



    finish_stages(
        temp_dir,
        &cut_output,
        &adjusted_words,
        output,
        text,
        font_size,
        no_interactive,
        verbose,
        registry,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_stages(
    temp_dir: &Path,
    cut_video: &Path,
    words: &[caption::Word],
    output: &Path,
    text: Option<&str>,
    font_size: Option<u32>,
    no_interactive: bool,
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<()> {
    // Write JSON sidecar (for overlay auto-title)
    let caption_json = temp_dir.join("captioned.json");
    let json_content = serde_json::to_string_pretty(words).map_err(|e| AppError::ParseFailed {
        stage: "caption-json".into(),
        message: e.to_string(),
    })?;
    std::fs::write(&caption_json, json_content).map_err(|e| AppError::StageIo {
        stage: "write-json".to_string(),
        source: e,
    })?;

    // Stage 6: Burn captions onto cut video
    eprintln!("\n{}", "Stage 6/7: caption".bold());
    let captioned_video = temp_dir.join("captioned.mp4");
    caption::burn_captions(cut_video, words, &captioned_video, verbose, registry)?;

    let cap_size = std::fs::metadata(&captioned_video)
        .map(|m| format_size(m.len(), DECIMAL))
        .unwrap_or_else(|_| "unknown size".to_string());
    eprintln!("\u{2713} Created captioned video ({})", cap_size);

    // Stage 7: Overlay (with title approval)
    eprintln!("\n{}", "Stage 7/7: overlay".bold());
    let overlay_text = if let Some(t) = text {
        t.to_string()
    } else {
        let options = overlay::generate_title_options(&caption_json, verbose)?;
        overlay::approve_title(&options, no_interactive, verbose)?
    };
    overlay::run(
        OverlayArgs {
            input: captioned_video,
            text: Some(overlay_text),
            auto: None,
            output: Some(output.to_path_buf()),
            font: None,
            font_size: font_size.unwrap_or(144),
            color: "black".to_string(),
            position: "top".to_string(),
            start: 0.3,
            duration: 3.5,
            no_interactive,
        },
        verbose,
        registry,
    )?;

    Ok(())
}
