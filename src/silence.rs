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

pub fn parse_silencedetect(_stderr: &str, _duration: f64) -> Vec<SilenceInterval> {
    todo!()
}

pub fn silence_to_speech(
    _silences: &[SilenceInterval],
    _duration: f64,
    _padding: f64,
) -> Vec<SpeechInterval> {
    todo!()
}

pub fn build_select_filter(_speeches: &[SpeechInterval], _frame_rate: f64) -> String {
    todo!()
}

pub fn build_aselect_filter(_speeches: &[SpeechInterval]) -> String {
    todo!()
}

pub fn total_silence_removed(_silences: &[SilenceInterval], _padding: f64) -> f64 {
    todo!()
}
