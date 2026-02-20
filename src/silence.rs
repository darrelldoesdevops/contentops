#[derive(Debug, Clone, PartialEq)]
pub struct SilenceInterval {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeechInterval {
    pub start: f64,
    pub end: f64,
}

pub fn parse_silencedetect(stderr: &str, duration: f64) -> Vec<SilenceInterval> {
    let mut intervals = Vec::new();
    let mut pending_start: Option<f64> = None;
    let mut seen_start = false;

    for line in stderr.lines() {
        if let Some(pos) = line.find("silence_start:") {
            let value_str = line[pos + "silence_start:".len()..].trim();
            if let Ok(start) = value_str.parse::<f64>() {
                if let Some(prev_start) = pending_start {
                    intervals.push(SilenceInterval {
                        start: prev_start,
                        end: duration,
                    });
                }
                pending_start = Some(start);
                seen_start = true;
            }
        } else if let Some(pos) = line.find("silence_end:") {
            let after = &line[pos + "silence_end:".len()..];
            let value_str = after.split('|').next().unwrap_or("").trim();
            if let Ok(end) = value_str.parse::<f64>() {
                if let Some(start) = pending_start.take() {
                    intervals.push(SilenceInterval { start, end });
                } else if !seen_start {
                    intervals.push(SilenceInterval { start: 0.0, end });
                }
            }
        }
    }

    if let Some(start) = pending_start {
        intervals.push(SilenceInterval {
            start,
            end: duration,
        });
    }

    intervals
}

pub fn silence_to_speech(
    silences: &[SilenceInterval],
    duration: f64,
    padding: f64,
) -> Vec<SpeechInterval> {
    if silences.is_empty() {
        return vec![SpeechInterval {
            start: 0.0,
            end: duration,
        }];
    }

    let mut raw_speeches = Vec::new();
    let mut cursor = 0.0_f64;

    for silence in silences {
        if silence.start > cursor {
            raw_speeches.push(SpeechInterval {
                start: cursor,
                end: silence.start,
            });
        }
        cursor = silence.end;
    }

    if cursor < duration {
        raw_speeches.push(SpeechInterval {
            start: cursor,
            end: duration,
        });
    }

    if raw_speeches.is_empty() {
        return Vec::new();
    }

    let mut padded: Vec<SpeechInterval> = raw_speeches
        .iter()
        .map(|s| SpeechInterval {
            start: (s.start - padding).max(0.0),
            end: (s.end + padding).min(duration),
        })
        .collect();

    let mut merged = Vec::new();
    let mut current = padded.remove(0);

    for next in padded {
        if next.start <= current.end {
            current.end = current.end.max(next.end);
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);

    merged
}

pub fn build_concat_filter(speeches: &[SpeechInterval]) -> String {
    if speeches.is_empty() {
        return String::new();
    }

    let n = speeches.len();
    let mut parts = Vec::new();

    for (i, s) in speeches.iter().enumerate() {
        parts.push(format!(
            "[0:v]trim=start={:.3}:end={:.3},setpts=PTS-STARTPTS[v{i}]",
            s.start, s.end
        ));
        parts.push(format!(
            "[0:a]atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS[a{i}]",
            s.start, s.end
        ));
    }

    let stream_labels: String = (0..n).map(|i| format!("[v{i}][a{i}]")).collect();
    parts.push(format!("{stream_labels}concat=n={n}:v=1:a=1[outv][outa]"));

    parts.join(";")
}

pub fn total_silence_removed(silences: &[SilenceInterval], padding: f64) -> f64 {
    silences
        .iter()
        .map(|s| {
            let raw = s.end - s.start;
            let removed = raw - 2.0 * padding;
            removed.max(0.0)
        })
        .sum()
}
