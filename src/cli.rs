use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "contentops", about = "Video processing pipeline", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Show verbose FFmpeg output
    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Remove silence from video
    Cut(CutArgs),

    /// Generate captions from video
    Caption(CaptionArgs),

    /// Add text overlay to video
    Overlay(OverlayArgs),

    /// Check prerequisites and environment readiness
    Doctor(DoctorArgs),

    /// Run full pipeline: cut → caption → overlay
    Pipeline(PipelineArgs),
}

#[derive(Args)]
pub struct DoctorArgs {
    /// Exit with code 1 if any check fails or warns
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args)]
pub struct CutArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output path (default: input_cut.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Preview what would be cut without producing output
    #[arg(long)]
    pub dry_run: bool,

    /// VAD speech probability threshold (0.0-1.0, lower = more sensitive)
    #[arg(long, default_value = "0.5", value_parser = parse_vad_threshold)]
    pub vad_threshold: f32,

    /// Minimum silence duration in milliseconds to cut
    #[arg(long, default_value = "400", value_parser = clap::value_parser!(u32).range(1..))]
    pub min_silence_ms: u32,
}

#[derive(Args)]
pub struct CaptionArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output SRT path (default: input_captioned.srt)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Path to whisper model file
    #[arg(long)]
    pub model: PathBuf,

    /// Language code for transcription
    #[arg(long, default_value = "en")]
    pub lang: String,

    /// Burn captions into video
    #[arg(long)]
    pub burn: bool,

    /// Use Claude to fix transcription errors
    #[arg(long)]
    pub fix: bool,
}

#[derive(Args)]
pub struct OverlayArgs {
    /// Input video file
    pub input: PathBuf,

    /// Text to overlay (or use --auto to generate from transcription)
    #[arg(long, required_unless_present = "auto")]
    pub text: Option<String>,

    /// Auto-generate title from transcription JSON file using Claude
    #[arg(long, value_name = "TRANSCRIPTION_JSON")]
    pub auto: Option<PathBuf>,

    /// Output path (default: input_overlay.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Path to .ttf font file
    #[arg(long)]
    pub font: Option<PathBuf>,

    /// Font size in pixels (scaled to video resolution)
    #[arg(long, default_value = "144")]
    pub font_size: u32,

    /// Font color (FFmpeg color name or hex)
    #[arg(long, default_value = "black")]
    pub color: String,

    /// Position preset: top, center, bottom
    #[arg(long, default_value = "top")]
    pub position: String,

    /// When overlay appears (seconds)
    #[arg(long, default_value = "0.3")]
    pub start: f64,

    /// How long overlay is visible (seconds, 0 = entire video)
    #[arg(long, default_value = "3.5")]
    pub duration: f64,
}

#[derive(Args)]
pub struct PipelineArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output path (default: input_pipeline.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Path to whisper model file
    #[arg(long)]
    pub model: PathBuf,

    /// VAD speech probability threshold (0.0-1.0, lower = more sensitive)
    #[arg(long, default_value = "0.5", value_parser = parse_vad_threshold)]
    pub vad_threshold: f32,

    /// Minimum silence duration in milliseconds to cut
    #[arg(long, default_value = "400", value_parser = clap::value_parser!(u32).range(1..))]
    pub min_silence_ms: u32,

    /// Title text for overlay (skips Claude auto-generation)
    #[arg(long)]
    pub text: Option<String>,

    /// Font size in pixels for title overlay
    #[arg(long)]
    pub font_size: Option<u32>,

    /// Preview planned stages without executing
    #[arg(long)]
    pub dry_run: bool,
}

fn parse_vad_threshold(s: &str) -> Result<f32, String> {
    let val: f32 = s.parse().map_err(|_| format!("invalid float: {}", s))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!("{} is not in 0.0..=1.0", val));
    }
    Ok(val)
}
