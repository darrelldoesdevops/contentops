use std::io::BufRead;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

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

pub fn extract_16k_wav(input: &str, dest: &Path, verbose: bool) -> Result<(), std::io::Error> {
    let dest_str = dest.to_string_lossy();
    let args = ["-i", input, "-ar", "16000", "-ac", "1", "-f", "wav", &dest_str];
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

// DEPRECATED: Phase 18 removes
#[allow(dead_code)]
pub fn run_silencedetect(
    input: &str,
    threshold_db: f64,
    min_duration: f64,
) -> Result<String, std::io::Error> {
    let af = format!("silencedetect=noise={}dB:d={}", threshold_db, min_duration);
    let output = Command::new("ffmpeg")
        .arg("-nostdin")
        .arg("-i")
        .arg(input)
        .arg("-af")
        .arg(&af)
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;

    Ok(String::from_utf8_lossy(&output.stderr).to_string())
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
        .args(args)
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
