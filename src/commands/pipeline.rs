use std::path::Path;

use humansize::{DECIMAL, format_size};
use owo_colors::OwoColorize;

use crate::cli::{OverlayArgs, PipelineArgs};
use crate::commands::{caption, cut, normalize, overlay};
use crate::error::AppError;
use crate::ffmpeg;
use crate::silence;
use crate::temp::{TempFileRegistry, make_temp_file};
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
        eprintln!("  1. normalize  → Audio normalization (loudnorm)");
        eprintln!(
            "  2. transcribe → Whisper transcription (model: {})",
            args.model.display()
        );
        eprintln!("  3. fix        → LLM transcription correction");
        eprintln!("  4. cut        → VAD-based silence removal");
        eprintln!("  5. caption    → Burn captions onto cut video");
        eprintln!("  6. overlay    → Add title overlay");
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
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<()> {
    let parent_dir = input.parent().unwrap_or(Path::new("."));

    // Normalize audio FIRST so all downstream timestamps are on the same timeline
    eprintln!("\n{}", "Stage 1/6: normalize".bold());
    let normalized = normalize::normalize_to_temp(input, verbose, registry)?;
    let normalized_str = normalized.to_string_lossy().to_string();

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

    // Stage 2: Transcribe normalized video (reuses shared WAV)
    eprintln!("\n{}", "Stage 2/6: transcribe".bold());
    let mut words = caption::transcribe(input, model, "en", verbose, registry, Some(&wav_path))?;

    if words.is_empty() {
        eprintln!("Warning: No speech detected");
        return Ok(());
    }

    // Stage 3: LLM fix
    eprintln!("\n{}", "Stage 3/6: fix".bold());
    caption::fix_transcription(&mut words, verbose)?;

    // Stage 4: VAD-based silence removal
    eprintln!("\n{}", "Stage 4/6: cut".bold());

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

    // Stage 5: Burn captions onto cut video
    eprintln!("\n{}", "Stage 5/6: caption".bold());
    let captioned_video = temp_dir.join("captioned.mp4");
    caption::burn_captions(cut_video, words, &captioned_video, verbose, registry)?;

    let cap_size = std::fs::metadata(&captioned_video)
        .map(|m| format_size(m.len(), DECIMAL))
        .unwrap_or_else(|_| "unknown size".to_string());
    eprintln!("\u{2713} Created captioned video ({})", cap_size);

    // Stage 6: Overlay
    eprintln!("\n{}", "Stage 6/6: overlay".bold());
    let (overlay_text, overlay_auto) = if let Some(t) = text {
        (Some(t.to_string()), None)
    } else {
        (None, Some(caption_json))
    };
    overlay::run(
        OverlayArgs {
            input: captioned_video,
            text: overlay_text,
            auto: overlay_auto,
            output: Some(output.to_path_buf()),
            font: None,
            font_size: font_size.unwrap_or(144),
            color: "black".to_string(),
            position: "top".to_string(),
            start: 0.3,
            duration: 3.5,
        },
        verbose,
        registry,
    )?;

    Ok(())
}
