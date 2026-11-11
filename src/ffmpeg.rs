use std::io::BufRead;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::{VideoCodec, parse_positive_bitrate};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessConfig {
    pub video_codec: VideoCodec,
    pub video_bitrate: String,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            video_codec: VideoCodec::X264,
            video_bitrate: "40M".to_string(),
        }
    }
}

static PROCESS_CONFIG: OnceLock<ProcessConfig> = OnceLock::new();

pub fn resolve_process_config(
    cli_codec: Option<VideoCodec>,
    cli_bitrate: Option<&str>,
    env_codec: Option<&str>,
    env_bitrate: Option<&str>,
) -> Result<ProcessConfig, String> {
    let video_codec = match cli_codec {
        Some(codec) => codec,
        None => match env_codec {
            Some("x264") => VideoCodec::X264,
            Some("videotoolbox") => VideoCodec::Videotoolbox,
            Some(value) => return Err(format!("invalid CONTENTOPS_VCODEC value: {value}")),
            None => VideoCodec::X264,
        },
    };
    let video_bitrate = parse_positive_bitrate(cli_bitrate.or(env_bitrate).unwrap_or("40M"))?;

    Ok(ProcessConfig {
        video_codec,
        video_bitrate,
    })
}

pub fn install_process_config(config: ProcessConfig) -> Result<(), ProcessConfig> {
    PROCESS_CONFIG.set(config)
}

fn process_config() -> &'static ProcessConfig {
    PROCESS_CONFIG.get_or_init(ProcessConfig::default)
}

pub struct FfmpegOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: Vec<u8>,
}

pub fn probe_duration(input: &str) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<f64>().ok()
}

pub fn probe_dimensions(input: &str) -> Option<(u32, u32)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            input,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().ok()?;
        let h = parts[1].parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

pub(crate) fn transform_encoder_args(args: &[&str], config: &ProcessConfig) -> Vec<String> {
    let mut transformed: Vec<String> = args.iter().map(|value| value.to_string()).collect();
    if config.video_codec == VideoCodec::X264 {
        return transformed;
    }

    let x264 = ["-c:v", "libx264", "-crf", "14", "-preset", "slow"];
    if let Some(index) = transformed
        .windows(x264.len())
        .position(|window| window.iter().map(String::as_str).eq(x264))
    {
        let videotoolbox = [
            "-c:v",
            "h264_videotoolbox",
            "-b:v",
            config.video_bitrate.as_str(),
            "-profile:v",
            "high",
        ];
        transformed.splice(
            index..index + x264.len(),
            videotoolbox.iter().map(|value| value.to_string()),
        );
    }
    transformed
}

fn encoder_args(args: &[&str]) -> Vec<String> {
    transform_encoder_args(args, process_config())
}

pub fn run_ffmpeg(args: &[&str]) -> Result<FfmpegOutput, std::io::Error> {
    let output = Command::new("ffmpeg")
        .args(encoder_args(args))
        .arg("-y")
        .arg("-nostdin")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;

    Ok(FfmpegOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stderr: output.stderr,
    })
}

pub fn run_ffmpeg_verbose(args: &[&str]) -> Result<FfmpegOutput, std::io::Error> {
    let output = Command::new("ffmpeg")
        .args(encoder_args(args))
        .arg("-y")
        .arg("-nostdin")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .output()?;

    Ok(FfmpegOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stderr: Vec::new(),
    })
}

pub fn extract_16k_wav(input: &str, dest: &Path, verbose: bool) -> Result<(), std::io::Error> {
    let dest_str = dest.to_string_lossy();
    let args = [
        "-i", input, "-ar", "16000", "-ac", "1", "-f", "wav", &dest_str,
    ];
    let output = if verbose {
        run_ffmpeg_verbose(&args)?
    } else {
        run_ffmpeg(&args)?
    };
    if !output.success {
        return Err(std::io::Error::other(format!(
            "ffmpeg WAV extraction failed with code {:?}",
            output.exit_code
        )));
    }
    Ok(())
}

pub fn scale_to_tiktok(
    input: &str,
    output: &str,
    verbose: bool,
) -> Result<FfmpegOutput, std::io::Error> {
    let vf = format!(
        "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
        crate::tiktok::OUTPUT_WIDTH,
        crate::tiktok::OUTPUT_HEIGHT,
        crate::tiktok::OUTPUT_WIDTH,
        crate::tiktok::OUTPUT_HEIGHT,
    );
    let args = [
        "-i", input, "-vf", &vf, "-c:v", "libx264", "-crf", "14", "-preset", "slow", "-pix_fmt",
        "yuv420p", "-c:a", "copy", output,
    ];
    if verbose {
        eprintln!("Running: ffmpeg {}", args.join(" "));
        run_ffmpeg_verbose(&args)
    } else {
        run_ffmpeg(&args)
    }
}

pub fn probe_duration_strict(input: &str) -> Result<f64, std::io::Error> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(input)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .trim()
        .parse::<f64>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn run_ffmpeg_with_progress(
    args: &[&str],
    total_duration_secs: Option<f64>,
    message: &str,
) -> Result<FfmpegOutput, std::io::Error> {
    let total_duration = match total_duration_secs {
        Some(d) if d > 0.0 => d,
        _ => return run_ffmpeg_with_spinner(args, message),
    };

    let mut child = Command::new("ffmpeg")
        .args(encoder_args(args))
        .arg("-y")
        .arg("-nostdin")
        .arg("-progress")
        .arg("pipe:2")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr_pipe = child.stderr.take().expect("stderr piped");
    let reader = std::io::BufReader::new(stderr_pipe);

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} {msg} [{bar:30.cyan/dim}] {pos}% ({elapsed})",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(message.to_string());

    let mut all_stderr = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        all_stderr.extend_from_slice(line.as_bytes());
        all_stderr.push(b'\n');

        if let Some(time_us) = line.strip_prefix("out_time_us=")
            && let Ok(us) = time_us.trim().parse::<i64>()
        {
            let secs = us as f64 / 1_000_000.0;
            let pct = ((secs / total_duration) * 100.0).min(100.0) as u64;
            pb.set_position(pct);
        }
    }

    let status = child.wait()?;

    if status.success() {
        pb.set_position(100);
        pb.finish_and_clear();
    } else {
        pb.finish_and_clear();
    }

    Ok(FfmpegOutput {
        success: status.success(),
        exit_code: status.code(),
        stderr: all_stderr,
    })
}

fn run_ffmpeg_with_spinner(args: &[&str], message: &str) -> Result<FfmpegOutput, std::io::Error> {
    let pb = crate::ui::make_spinner(message);

    let result = run_ffmpeg(args);

    pb.finish_and_clear();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_config_precedence_is_cli_then_env_then_defaults() {
        assert_eq!(
            resolve_process_config(None, None, None, None).unwrap(),
            ProcessConfig::default()
        );
        assert_eq!(
            resolve_process_config(None, None, Some("videotoolbox"), Some("30M")).unwrap(),
            ProcessConfig {
                video_codec: VideoCodec::Videotoolbox,
                video_bitrate: "30M".to_string(),
            }
        );
        assert_eq!(
            resolve_process_config(
                Some(VideoCodec::X264),
                Some("12M"),
                Some("invalid-env-codec"),
                Some("0M"),
            )
            .unwrap(),
            ProcessConfig {
                video_codec: VideoCodec::X264,
                video_bitrate: "12M".to_string(),
            }
        );
    }

    #[test]
    fn process_config_rejects_invalid_environment_values() {
        assert!(resolve_process_config(None, None, Some("nvenc"), None).is_err());
        assert!(resolve_process_config(None, None, None, Some("0M")).is_err());
    }

    #[test]
    fn x264_transform_preserves_existing_arguments() {
        let args = [
            "-i",
            "input.mp4",
            "-c:v",
            "libx264",
            "-crf",
            "14",
            "-preset",
            "slow",
            "output.mp4",
        ];
        assert_eq!(
            transform_encoder_args(&args, &ProcessConfig::default()),
            args
        );
    }

    #[test]
    fn videotoolbox_transform_replaces_only_the_known_x264_sequence() {
        let args = [
            "-i",
            "input.mp4",
            "-c:v",
            "libx264",
            "-crf",
            "14",
            "-preset",
            "slow",
            "-pix_fmt",
            "yuv420p",
            "output.mp4",
        ];
        let config = ProcessConfig {
            video_codec: VideoCodec::Videotoolbox,
            video_bitrate: "24M".to_string(),
        };

        assert_eq!(
            transform_encoder_args(&args, &config),
            [
                "-i",
                "input.mp4",
                "-c:v",
                "h264_videotoolbox",
                "-b:v",
                "24M",
                "-profile:v",
                "high",
                "-pix_fmt",
                "yuv420p",
                "output.mp4",
            ]
        );
    }

    #[test]
    fn videotoolbox_transform_preserves_copy_and_audio_only_invocations() {
        let config = ProcessConfig {
            video_codec: VideoCodec::Videotoolbox,
            video_bitrate: "24M".to_string(),
        };
        let copy_args = ["-i", "input.mp4", "-c:v", "copy", "output.mp4"];
        let audio_args = ["-i", "input.mp4", "-vn", "-c:a", "aac", "output.m4a"];

        assert_eq!(transform_encoder_args(&copy_args, &config), copy_args);
        assert_eq!(transform_encoder_args(&audio_args, &config), audio_args);
    }
}
