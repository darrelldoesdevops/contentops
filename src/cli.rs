use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum VideoCodec {
    X264,
    Videotoolbox,
}

#[derive(Parser)]
#[command(name = "contentops", about = "Video processing pipeline", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Show verbose FFmpeg output
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Video encoder for re-encoding operations
    #[arg(long, global = true, value_enum)]
    pub vcodec: Option<VideoCodec>,

    /// Video bitrate used by the VideoToolbox encoder
    #[arg(long, global = true, value_parser = parse_positive_bitrate)]
    pub vbitrate: Option<String>,
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

    /// Print the dry-run result as JSON to stdout
    #[arg(long, requires = "dry_run")]
    pub json: bool,

    /// VAD speech probability threshold (0.0-1.0, lower = more sensitive)
    #[arg(long, default_value = "0.5", value_parser = parse_vad_threshold)]
    pub vad_threshold: f32,

    /// Minimum silence duration in milliseconds to cut
    #[arg(long, default_value = "300", value_parser = clap::value_parser!(u32).range(1..))]
    pub min_silence_ms: u32,

    /// Extra seconds to keep before the first speech segment
    #[arg(long, default_value = "0.5")]
    pub start_pad: f64,
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("caption_source")
        .required(true)
        .multiple(false)
        .args(["model", "words_json"])
))]
pub struct CaptionArgs {
    /// Input video file
    pub input: PathBuf,

    /// Output SRT path (default: input_captioned.srt)
    #[arg(short = 'o')]
    pub output: Option<PathBuf>,

    /// Path to whisper model file (required unless --words-json is used)
    #[arg(long, conflicts_with = "words_json")]
    pub model: Option<PathBuf>,

    /// Existing word timings as a JSON array, in seconds relative to the input video
    #[arg(long, value_name = "PATH", conflicts_with_all = ["model", "fix"])]
    pub words_json: Option<PathBuf>,

    /// Language code for transcription
    #[arg(long, default_value = "en")]
    pub lang: String,

    /// Burn captions into video
    #[arg(long)]
    pub burn: bool,

    /// Burned video output path (default: input_captioned.mp4)
    #[arg(long, value_name = "PATH", requires = "burn")]
    pub video_out: Option<PathBuf>,

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

    /// Skip interactive prompts (auto-select defaults)
    #[arg(long)]
    pub no_interactive: bool,
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
    #[arg(long, default_value = "300", value_parser = clap::value_parser!(u32).range(1..))]
    pub min_silence_ms: u32,

    /// Extra seconds to keep before the first speech segment
    #[arg(long, default_value = "0.5")]
    pub start_pad: f64,

    /// Title text for overlay (skips Claude auto-generation)
    #[arg(long)]
    pub text: Option<String>,

    /// Font size in pixels for title overlay
    #[arg(long)]
    pub font_size: Option<u32>,

    /// Preview planned stages without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip interactive prompts (auto-select defaults)
    #[arg(long)]
    pub no_interactive: bool,
}

fn parse_vad_threshold(s: &str) -> Result<f32, String> {
    let val: f32 = s.parse().map_err(|_| format!("invalid float: {}", s))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!("{} is not in 0.0..=1.0", val));
    }
    Ok(val)
}

pub(crate) fn parse_positive_bitrate(value: &str) -> Result<String, String> {
    let number = value
        .strip_suffix(['k', 'K', 'm', 'M', 'g', 'G'])
        .unwrap_or(value);
    let parsed: f64 = number
        .parse()
        .map_err(|_| format!("invalid bitrate: {value}"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("bitrate must be positive: {value}"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn caption_accepts_model_input() {
        assert!(
            Cli::try_parse_from(["contentops", "caption", "--model", "model.bin", "video.mp4"])
                .is_ok()
        );
    }

    #[test]
    fn caption_accepts_words_json_input() {
        assert!(
            Cli::try_parse_from([
                "contentops",
                "caption",
                "--words-json",
                "words.json",
                "video.mp4",
            ])
            .is_ok()
        );
    }

    #[test]
    fn caption_requires_exactly_one_caption_source() {
        let error = Cli::try_parse_from(["contentops", "caption", "video.mp4"])
            .err()
            .expect("caption source should be required")
            .to_string();

        assert!(error.contains("<--model <MODEL>|--words-json <PATH>>"));
        assert!(!error.contains("--model <MODEL> --words-json <PATH>"));
    }

    #[test]
    fn caption_rejects_model_with_words_json() {
        assert!(
            Cli::try_parse_from([
                "contentops",
                "caption",
                "--model",
                "model.bin",
                "--words-json",
                "words.json",
                "video.mp4",
            ])
            .is_err()
        );
    }

    #[test]
    fn caption_rejects_fix_with_words_json() {
        assert!(
            Cli::try_parse_from([
                "contentops",
                "caption",
                "--words-json",
                "words.json",
                "--fix",
                "video.mp4",
            ])
            .is_err()
        );
    }

    #[test]
    fn caption_video_out_requires_burn() {
        assert!(
            Cli::try_parse_from([
                "contentops",
                "caption",
                "--words-json",
                "words.json",
                "--video-out",
                "burned.mp4",
                "video.mp4",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "contentops",
                "caption",
                "--words-json",
                "words.json",
                "--burn",
                "--video-out",
                "burned.mp4",
                "video.mp4",
            ])
            .is_ok()
        );
    }

    #[test]
    fn cut_json_requires_dry_run() {
        assert!(Cli::try_parse_from(["contentops", "cut", "--json", "video.mp4"]).is_err());
        assert!(
            Cli::try_parse_from(["contentops", "cut", "--dry-run", "--json", "video.mp4"]).is_ok()
        );
    }

    #[test]
    fn global_encoder_flags_accept_valid_values() {
        assert!(
            Cli::try_parse_from([
                "contentops",
                "--vcodec",
                "videotoolbox",
                "--vbitrate",
                "25M",
                "cut",
                "video.mp4",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "contentops",
                "cut",
                "video.mp4",
                "--vcodec",
                "x264",
                "--vbitrate",
                "0",
            ])
            .is_err()
        );
    }
}
