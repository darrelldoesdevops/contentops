use std::process::{Command, Stdio};

use owo_colors::OwoColorize;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    Ok,
    Warn,
    Fail,
}

struct CheckResult {
    name: String,
    status: Status,
    detail: String,
}

impl CheckResult {
    fn print(&self) {
        let tag = match self.status {
            Status::Ok => format!("{}", "[ok]".green()),
            Status::Warn => format!("{}", "[warn]".yellow()),
            Status::Fail => format!("{}", "[fail]".red()),
        };
        eprintln!("  {} {} {}", tag, self.name, self.detail.dimmed());
    }
}

fn check_on_path(name: &str) -> CheckResult {
    match which::which(name) {
        Ok(path) => CheckResult {
            name: name.to_string(),
            status: Status::Ok,
            detail: format!("({})", path.display()),
        },
        Err(_) => CheckResult {
            name: name.to_string(),
            status: Status::Fail,
            detail: "not found on PATH".to_string(),
        },
    }
}

fn check_ffmpeg_version() -> CheckResult {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => {
            return CheckResult {
                name: "ffmpeg version".to_string(),
                status: Status::Fail,
                detail: "could not run ffmpeg -version".to_string(),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    match parse_ffmpeg_version(&stdout) {
        Some(version) if version_at_least(&version, 6, 0) => CheckResult {
            name: "ffmpeg version".to_string(),
            status: Status::Ok,
            detail: format!("({})", version),
        },
        Some(version) => CheckResult {
            name: "ffmpeg version".to_string(),
            status: Status::Warn,
            detail: format!("({}, requires >= 6.0)", version),
        },
        None => CheckResult {
            name: "ffmpeg version".to_string(),
            status: Status::Warn,
            detail: "could not parse version".to_string(),
        },
    }
}

fn parse_ffmpeg_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    // "ffmpeg version 7.1 ..." or "ffmpeg version N-12345-g..."
    let version_str = first_line.strip_prefix("ffmpeg version ")?;
    let version = version_str.split_whitespace().next()?;
    Some(version.to_string())
}

fn version_at_least(version: &str, major: u32, minor: u32) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    let parsed_major = parts.first().and_then(|s| s.parse::<u32>().ok());
    let parsed_minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());

    match (parsed_major, parsed_minor) {
        (Some(maj), _) if maj > major => true,
        (Some(maj), Some(min)) if maj == major && min >= minor => true,
        (Some(maj), None) if maj >= major => true,
        _ => false,
    }
}

struct SubcommandReadiness {
    name: &'static str,
    required: &'static [&'static str],
}

const SUBCOMMANDS: &[SubcommandReadiness] = &[
    SubcommandReadiness {
        name: "cut",
        required: &["ffmpeg", "ffprobe"],
    },
    SubcommandReadiness {
        name: "caption",
        required: &["ffmpeg", "whisper-cli"],
    },
    SubcommandReadiness {
        name: "overlay",
        required: &["ffmpeg"],
    },
    SubcommandReadiness {
        name: "overlay --auto",
        required: &["ffmpeg", "claude"],
    },
];

pub fn run(strict: bool) -> i32 {
    eprintln!("{}", "contentops doctor".bold());
    eprintln!();

    let checks = vec![
        check_on_path("ffmpeg"),
        check_ffmpeg_version(),
        check_on_path("ffprobe"),
        check_on_path("whisper-cli"),
        check_on_path("claude"),
    ];

    eprintln!("{}:", "Prerequisites".bold());
    for check in &checks {
        check.print();
    }

    let failed_tools: Vec<&str> = checks
        .iter()
        .filter(|c| c.status == Status::Fail)
        .map(|c| c.name.as_str())
        .collect();

    eprintln!();
    eprintln!("{}:", "Subcommand readiness".bold());

    let mut any_not_ready = false;
    for sub in SUBCOMMANDS {
        let missing: Vec<&str> = sub
            .required
            .iter()
            .filter(|tool| failed_tools.contains(tool))
            .copied()
            .collect();

        if missing.is_empty() {
            eprintln!(
                "  {} {}",
                sub.name,
                "ready".green()
            );
        } else {
            any_not_ready = true;
            eprintln!(
                "  {} {} (missing {})",
                sub.name,
                "not ready".red(),
                missing.join(", ")
            );
        }
    }

    let has_failures = checks.iter().any(|c| c.status == Status::Fail);
    let has_warnings = checks.iter().any(|c| c.status == Status::Warn);

    eprintln!();
    if has_failures || any_not_ready {
        eprintln!(
            "{} some checks failed",
            "!".yellow().bold()
        );
    } else if has_warnings {
        eprintln!(
            "{} all tools found (with warnings)",
            "!".yellow().bold()
        );
    } else {
        eprintln!(
            "{} all checks passed",
            "ok:".green().bold()
        );
    }

    if strict && (has_failures || has_warnings) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ffmpeg_version_standard() {
        let output = "ffmpeg version 7.1 Copyright (c) 2000-2024";
        assert_eq!(parse_ffmpeg_version(output), Some("7.1".to_string()));
    }

    #[test]
    fn parse_ffmpeg_version_nightly() {
        let output = "ffmpeg version N-12345-gabcdef Copyright";
        assert_eq!(
            parse_ffmpeg_version(output),
            Some("N-12345-gabcdef".to_string())
        );
    }

    #[test]
    fn version_at_least_passes() {
        assert!(version_at_least("7.1", 6, 0));
        assert!(version_at_least("6.0", 6, 0));
        assert!(version_at_least("6.1", 6, 0));
    }

    #[test]
    fn version_at_least_fails() {
        assert!(!version_at_least("5.1", 6, 0));
        assert!(!version_at_least("4.4", 6, 0));
    }

    #[test]
    fn version_at_least_major_only() {
        assert!(version_at_least("7", 6, 0));
        assert!(version_at_least("6", 6, 0));
        assert!(!version_at_least("5", 6, 0));
    }

    #[test]
    fn version_at_least_nightly_not_numeric() {
        assert!(!version_at_least("N-12345-gabcdef", 6, 0));
    }
}
