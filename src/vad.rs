use std::path::Path;

use voice_activity_detector::{IteratorExt, LabeledAudio, VoiceActivityDetector};

use crate::silence::SpeechInterval;

const VAD_THRESHOLD: f32 = 0.5;
const CHUNK_SIZE: usize = 512;
const SAMPLE_RATE: f64 = 16000.0;

pub fn run_vad(wav_path: &Path, video_duration: f64) -> anyhow::Result<Vec<SpeechInterval>> {
    let reader = hound::WavReader::open(wav_path)
        .map_err(|e| anyhow::anyhow!("WAV open failed: {}", e))?;

    let spec = reader.spec();
    if spec.sample_rate != 16000 {
        return Err(anyhow::anyhow!(
            "VAD requires 16kHz audio, got {}Hz",
            spec.sample_rate
        ));
    }
    if spec.channels != 1 {
        return Err(anyhow::anyhow!(
            "VAD requires mono audio, got {} channels",
            spec.channels
        ));
    }

    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(16000i64)
        .chunk_size(CHUNK_SIZE)
        .build()
        .map_err(|e| anyhow::anyhow!("ONNX Runtime failed to initialize: {}", e))?;

    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let chunk_secs = CHUNK_SIZE as f64 / SAMPLE_RATE;
    let mut speeches: Vec<SpeechInterval> = Vec::new();
    let mut chunk_idx: usize = 0;
    let mut in_speech = false;
    let mut speech_start = 0.0f64;

    for label in samples.into_iter().label(&mut vad, VAD_THRESHOLD, 0usize) {
        let start = chunk_idx as f64 * chunk_secs;
        match label {
            LabeledAudio::Speech(_) => {
                if !in_speech {
                    speech_start = start;
                    in_speech = true;
                }
            }
            LabeledAudio::NonSpeech(_) => {
                if in_speech {
                    speeches.push(SpeechInterval {
                        start: speech_start,
                        end: start,
                    });
                    in_speech = false;
                }
            }
        }
        chunk_idx += 1;
    }

    if in_speech {
        let end = (chunk_idx as f64 * chunk_secs).min(video_duration);
        speeches.push(SpeechInterval {
            start: speech_start,
            end,
        });
    }

    if let Some(last) = speeches.last_mut() {
        last.end = last.end.min(video_duration);
    }

    Ok(speeches)
}
