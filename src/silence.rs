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

pub fn words_to_speech_intervals(
    word_times: &[(f64, f64)],
    gap_threshold: f64,
    padding: f64,
    duration: f64,
) -> Vec<SpeechInterval> {
    if word_times.is_empty() {
        return Vec::new();
    }

    let mut segments: Vec<SpeechInterval> = Vec::new();
    let mut seg_start = word_times[0].0;
    let mut seg_end = word_times[0].1;

    for &(start, end) in &word_times[1..] {
        if start - seg_end > gap_threshold {
            segments.push(SpeechInterval {
                start: (seg_start - padding).max(0.0),
                end: (seg_end + padding).min(duration),
            });
            seg_start = start;
        }
        seg_end = end;
    }
    segments.push(SpeechInterval {
        start: (seg_start - padding).max(0.0),
        end: (seg_end + padding).min(duration),
    });

    // Merge overlapping after padding
    let mut merged = Vec::new();
    let mut current = segments.remove(0);
    for next in segments {
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

pub fn filter_silences_by_words(
    silences: &[SilenceInterval],
    words: &[(f64, f64)],
) -> Vec<SilenceInterval> {
    let midpoints: Vec<f64> = words.iter().map(|(s, e)| (s + e) / 2.0).collect();

    silences
        .iter()
        .filter(|silence| {
            !midpoints
                .iter()
                .any(|&mid| mid >= silence.start && mid <= silence.end)
        })
        .cloned()
        .collect()
}

pub fn adjust_timestamps(
    word_times: &[(f64, f64, String)],
    speeches: &[SpeechInterval],
) -> Vec<(f64, f64, String)> {
    let mut cumulative_removed = 0.0;
    let mut prev_speech_end = 0.0;

    let mut gap_before: Vec<f64> = Vec::with_capacity(speeches.len());
    for speech in speeches {
        cumulative_removed += (speech.start - prev_speech_end).max(0.0);
        gap_before.push(cumulative_removed);
        prev_speech_end = speech.end;
    }

    let mut adjusted = Vec::new();
    let mut speech_idx = 0;

    for (start, end, word) in word_times {
        // Find which speech interval contains this word
        while speech_idx + 1 < speeches.len() && speeches[speech_idx].end < *start {
            speech_idx += 1;
        }
        let offset = gap_before[speech_idx];
        adjusted.push((start - offset, end - offset, word.clone()));
    }

    adjusted
}

pub fn total_silence_from_speeches(speeches: &[SpeechInterval], video_duration: f64) -> f64 {
    let speech_total: f64 = speeches.iter().map(|s| s.end - s.start).sum();
    (video_duration - speech_total).max(0.0)
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
