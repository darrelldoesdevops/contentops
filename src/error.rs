use std::path::PathBuf;

use owo_colors::OwoColorize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("ffmpeg not found on PATH\n  hint: brew install ffmpeg")]
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
}

pub fn require_ffmpeg() -> Result<PathBuf, AppError> {
    which::which("ffmpeg").map_err(|_| AppError::FfmpegNotFound)
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
                "{} ffmpeg not found on PATH\n  {}: brew install ffmpeg",
                "error:".red().bold(),
                "hint".bold()
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
    }
}
