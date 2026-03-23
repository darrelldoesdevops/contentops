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

pub fn total_silence_from_speeches(speeches: &[SpeechInterval], video_duration: f64) -> f64 {
    let speech_total: f64 = speeches.iter().map(|s| s.end - s.start).sum();
    (video_duration - speech_total).max(0.0)
}
