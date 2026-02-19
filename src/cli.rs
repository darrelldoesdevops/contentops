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
}
