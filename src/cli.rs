use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "contentops", about = "Video processing pipeline")]
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

    /// Also detect and remove breaths (raises threshold, lowers min duration)
    #[arg(long)]
    pub breaths: bool,
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

    /// Font size in pixels
    #[arg(long, default_value = "44")]
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
