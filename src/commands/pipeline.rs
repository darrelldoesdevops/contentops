use std::path::Path;

use owo_colors::OwoColorize;

use crate::cli::{CaptionArgs, CutArgs, OverlayArgs, PipelineArgs};
use crate::commands::{caption, cut, overlay};
use crate::error::AppError;
use crate::temp::TempFileRegistry;

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
        eprintln!("  1. cut     → Remove silence");
        eprintln!(
            "  2. caption → Transcribe and burn captions (model: {})",
            args.model.display()
        );
        eprintln!("  3. overlay → Auto-generate and add title overlay");
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
        args.breaths,
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

fn run_stages(
    temp_dir: &Path,
    input: &Path,
    model: &Path,
    output: &Path,
    text: Option<&str>,
    font_size: Option<u32>,
    breaths: bool,
    verbose: bool,
    registry: &TempFileRegistry,
) -> anyhow::Result<()> {
    // Stage 1: Cut
    let cut_output = temp_dir.join("cut.mp4");
    eprintln!("\n{}", "Stage 1/3: cut".bold());
    cut::run(
        CutArgs {
            input: input.to_path_buf(),
            output: Some(cut_output.clone()),
            dry_run: false,
            breaths,
        },
        verbose,
        registry,
    )?;

    // Stage 2: Caption (with burn)
    let caption_srt = temp_dir.join("captioned.srt");
    eprintln!("\n{}", "Stage 2/3: caption".bold());
    caption::run(
        CaptionArgs {
            input: cut_output,
            output: Some(caption_srt),
            model: model.to_path_buf(),
            lang: "en".to_string(),
            burn: true,
        },
        verbose,
        registry,
    )?;

    // caption with burn produces: temp_dir/cut_captioned.mp4
    // caption with explicit output produces: temp_dir/captioned.json
    let captioned_video = temp_dir.join("cut_captioned.mp4");
    let caption_json = temp_dir.join("captioned.json");

    // Stage 3: Overlay
    eprintln!("\n{}", "Stage 3/3: overlay".bold());
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
