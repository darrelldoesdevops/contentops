#[derive(Debug, Clone, PartialEq)]
pub struct SpeechInterval {
    pub start: f64,
    pub end: f64,
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
        while speech_idx + 1 < speeches.len() && speeches[speech_idx].end < *start {
            speech_idx += 1;
        }
        let offset = gap_before[speech_idx];
        adjusted.push((start - offset, end - offset, word.clone()));
    }

    // Enforce monotonicity: words at speech interval boundaries can get
    // different offsets that reverse their order. Clamp so starts always
    // increase and no timestamps go negative.
    if !adjusted.is_empty() {
        if adjusted[0].0 < 0.0 {
            adjusted[0].0 = 0.0;
        }
        if adjusted[0].1 < adjusted[0].0 {
            adjusted[0].1 = adjusted[0].0;
        }
        for i in 1..adjusted.len() {
            if adjusted[i].0 < adjusted[i - 1].0 {
                adjusted[i].0 = adjusted[i - 1].0;
            }
            if adjusted[i].1 < adjusted[i].0 {
                adjusted[i].1 = adjusted[i].0;
            }
        }
    }

    adjusted
}

pub fn total_silence_from_speeches(speeches: &[SpeechInterval], video_duration: f64) -> f64 {
    let speech_total: f64 = speeches.iter().map(|s| s.end - s.start).sum();
    (video_duration - speech_total).max(0.0)
}
