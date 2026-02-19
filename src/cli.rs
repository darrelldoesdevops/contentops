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
}
