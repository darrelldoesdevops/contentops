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
    /// Add text overlay to video
    Overlay(OverlayArgs),
    /// Normalize audio loudness
    Normalize(NormalizeArgs),
}

#[derive(Args)]
pub struct CutArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output path (default: input_cut.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct OverlayArgs {
    /// Input video file
    pub input: PathBuf,

    /// Text to overlay on video
    #[arg(long)]
    pub text: String,

    /// Output path (default: input_overlay.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Path to .ttf font file
    #[arg(long)]
    pub font: Option<PathBuf>,

    /// Font size in pixels
    #[arg(long, default_value = "48")]
    pub font_size: u32,

    /// Font color (FFmpeg color name or hex)
    #[arg(long, default_value = "white")]
    pub color: String,

    /// Position preset: top, center, bottom
    #[arg(long, default_value = "center")]
    pub position: String,

    /// When overlay appears (seconds)
    #[arg(long, default_value = "0.0")]
    pub start: f64,

    /// How long overlay is visible (seconds, 0 = entire video)
    #[arg(long, default_value = "0.0")]
    pub duration: f64,
}

#[derive(Args)]
pub struct NormalizeArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output path (default: input_normalized.mp4)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,
}
