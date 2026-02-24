use std::path::PathBuf;

use owo_colors::OwoColorize;
use thiserror::Error;

fn ffmpeg_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install ffmpeg"
    } else if cfg!(target_os = "windows") {
        "choco install ffmpeg"
    } else {
        "apt install ffmpeg"
    }
}

fn whisper_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install whisper-cli"
    } else {
        "build from source: https://github.com/ggerganov/whisper.cpp"
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("ffmpeg not found on PATH")]
    FfmpegNotFound,

    #[error("error in stage '{stage}': ffmpeg exited with code {code}\n{stderr}")]
    FfmpegFailed {
        stage: String,
        code: i32,
        stderr: String,
    },

    #[error("error in stage '{stage}': {source}")]
    StageIo {
        stage: String,
        #[source]
        source: std::io::Error,
    },

    #[error("input file not found: {0}")]
    InputNotFound(PathBuf),

    #[error("whisper-cli not found on PATH")]
    WhisperNotFound,

    #[error("error in stage '{stage}': whisper-cli exited with code {code}\n{stderr}")]
    WhisperFailed {
        stage: String,
        code: i32,
        stderr: String,
    },

    #[error(
        "whisper model not found: {0}\n  hint: download from https://huggingface.co/ggerganov/whisper.cpp"
    )]
    ModelNotFound(PathBuf),

    #[error("no speech detected in {0}: entire video is silence")]
    NoSpeechDetected(PathBuf),

    #[error(
        "claude not found on PATH\n  hint: install Claude Code CLI, then run `contentops doctor`"
    )]
    ClaudeNotFound,

    #[error("error in stage '{stage}': claude CLI exited with code {code}\n{stderr}")]
    ClaudeFailed {
        stage: String,
        code: i32,
        stderr: String,
    },

    #[error("error in stage '{stage}': failed to parse output\n{message}")]
    ParseFailed { stage: String, message: String },

    #[error("ffmpeg was built without libass (no 'ass' filter)")]
    LibassNotFound,
}

pub fn require_ffmpeg() -> Result<PathBuf, AppError> {
    which::which("ffmpeg").map_err(|_| AppError::FfmpegNotFound)
}

pub fn require_ffmpeg_libass() -> Result<(), AppError> {
    let output = std::process::Command::new("ffmpeg")
        .args(["-filters"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.lines().any(|l| {
                let trimmed = l.trim();
                trimmed.contains("ass") && trimmed.contains("->") && trimmed.contains("Render ASS")
            }) {
                Ok(())
            } else {
                Err(AppError::LibassNotFound)
            }
        }
        Err(_) => Err(AppError::FfmpegNotFound),
    }
}

pub fn require_whisper() -> Result<PathBuf, AppError> {
    which::which("whisper-cli").map_err(|_| AppError::WhisperNotFound)
}

pub fn require_claude() -> Result<PathBuf, AppError> {
    which::which("claude").map_err(|_| AppError::ClaudeNotFound)
}

pub fn last_n_lines(stderr: &[u8], n: usize) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

pub fn format_error(err: &AppError) -> String {
    match err {
        AppError::FfmpegNotFound => {
            format!(
                "{} ffmpeg not found on PATH\n  {}: {}, then run `contentops doctor`",
                "error:".red().bold(),
                "hint".bold(),
                ffmpeg_install_hint()
            )
        }
        AppError::FfmpegFailed {
            stage,
            code,
            stderr,
        } => {
            let indented: String = stderr
                .lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{} in stage '{}': ffmpeg exited with code {}\n{}",
                "error:".red().bold(),
                stage.bold(),
                code,
                indented
            )
        }
        AppError::StageIo { stage, source } => {
            format!(
                "{} in stage '{}': {}",
                "error:".red().bold(),
                stage.bold(),
                source
            )
        }
        AppError::InputNotFound(path) => {
            format!(
                "{} input file not found: {}",
                "error:".red().bold(),
                path.display()
            )
        }
        AppError::WhisperNotFound => {
            format!(
                "{} whisper-cli not found on PATH\n  {}: {}, then run `contentops doctor`",
                "error:".red().bold(),
                "hint".bold(),
                whisper_install_hint()
            )
        }
        AppError::WhisperFailed {
            stage,
            code,
            stderr,
        } => {
            let indented: String = stderr
                .lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{} in stage '{}': whisper-cli exited with code {}\n{}",
                "error:".red().bold(),
                stage.bold(),
                code,
                indented
            )
        }
        AppError::ModelNotFound(path) => {
            format!(
                "{} whisper model not found: {}\n  {}: download from https://huggingface.co/ggerganov/whisper.cpp",
                "error:".red().bold(),
                path.display(),
                "hint".bold()
            )
        }
        AppError::NoSpeechDetected(path) => {
            format!(
                "{} no speech detected in {}: entire video is silence",
                "error:".red().bold(),
                path.display()
            )
        }
        AppError::ClaudeNotFound => {
            format!(
                "{} claude not found on PATH\n  {}: install Claude Code CLI, then run `contentops doctor`",
                "error:".red().bold(),
                "hint".bold()
            )
        }
        AppError::ClaudeFailed {
            stage,
            code,
            stderr,
        } => {
            let indented: String = stderr
                .lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{} in stage '{}': claude CLI exited with code {}\n{}",
                "error:".red().bold(),
                stage.bold(),
                code,
                indented
            )
        }
        AppError::ParseFailed { stage, message } => {
            format!(
                "{} in stage '{}': failed to parse output\n  {}",
                "error:".red().bold(),
                stage.bold(),
                message
            )
        }
        AppError::LibassNotFound => {
            let reinstall_hint = if cfg!(target_os = "macos") {
                "brew install libass && brew reinstall ffmpeg"
            } else {
                "apt install libass-dev && apt install ffmpeg"
            };
            format!(
                "{} ffmpeg was built without libass (caption burn requires the 'ass' filter)\n  {}: {}",
                "error:".red().bold(),
                "hint".bold(),
                reinstall_hint
            )
        }
    }
}
