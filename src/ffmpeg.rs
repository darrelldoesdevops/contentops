use std::process::{Command, Stdio};

pub struct FfmpegOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: Vec<u8>,
}

pub fn run_ffmpeg(args: &[&str]) -> Result<FfmpegOutput, std::io::Error> {
    let output = Command::new("ffmpeg")
        .args(args)
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
        .args(args)
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
