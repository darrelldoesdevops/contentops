use contentops::silence::{
    build_concat_filter, parse_silencedetect, silence_to_speech, total_silence_removed,
    SilenceInterval, SpeechInterval,
};

// ── parse_silencedetect ──────────────────────────────────────────

#[test]
fn parse_empty_input_returns_empty() {
    let result = parse_silencedetect("", 60.0);
    assert!(result.is_empty());
}

#[test]
fn parse_no_silence_lines_returns_empty() {
    let stderr = "frame=  120 fps=0.0 q=0.0 size=    0kB time=00:00:04.00\n\
                  video:0kB audio:64kB subtitle:0kB other:0kB global headers:0kB\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert!(result.is_empty());
}

#[test]
fn parse_single_silence_pair() {
    let stderr = "[silencedetect @ 0x1234] silence_start: 5.000\n\
                  [silencedetect @ 0x1234] silence_end: 8.000 | silence_duration: 3.000\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![SilenceInterval {
            start: 5.0,
            end: 8.0
        }]
    );
}

#[test]
fn parse_multiple_silence_pairs() {
    let stderr = "[silencedetect @ 0x1] silence_start: 2.0\n\
                  [silencedetect @ 0x1] silence_end: 4.0 | silence_duration: 2.0\n\
                  [silencedetect @ 0x1] silence_start: 10.0\n\
                  [silencedetect @ 0x1] silence_end: 15.0 | silence_duration: 5.0\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![
            SilenceInterval {
                start: 2.0,
                end: 4.0
            },
            SilenceInterval {
                start: 10.0,
                end: 15.0
            },
        ]
    );
}

#[test]
fn parse_trailing_silence_uses_duration() {
    let stderr = "[silencedetect @ 0xab] silence_start: 55.0\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![SilenceInterval {
            start: 55.0,
            end: 60.0
        }]
    );
}

#[test]
fn parse_leading_silence() {
    let stderr = "[silencedetect @ 0x1] silence_end: 2.5 | silence_duration: 2.5\n\
                  [silencedetect @ 0x1] silence_start: 50.0\n\
                  [silencedetect @ 0x1] silence_end: 53.0 | silence_duration: 3.0\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![
            SilenceInterval {
                start: 0.0,
                end: 2.5
            },
            SilenceInterval {
                start: 50.0,
                end: 53.0
            },
        ]
    );
}

#[test]
fn parse_mixed_ffmpeg_output() {
    let stderr = "Input #0, mov,mp4,m4a:\n\
                  Duration: 00:01:00.00, start: 0.000000\n\
                  [silencedetect @ 0xf] silence_start: 5.0\n\
                  frame=  120 fps=0.0 q=0.0 size=    0kB\n\
                  [silencedetect @ 0xf] silence_end: 8.0 | silence_duration: 3.0\n\
                  video:0kB audio:64kB\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![SilenceInterval {
            start: 5.0,
            end: 8.0
        }]
    );
}

#[test]
fn parse_trailing_with_preceding_pair() {
    let stderr = "[silencedetect @ 0x1] silence_start: 2.0\n\
                  [silencedetect @ 0x1] silence_end: 4.0 | silence_duration: 2.0\n\
                  [silencedetect @ 0x1] silence_start: 55.0\n";
    let result = parse_silencedetect(stderr, 60.0);
    assert_eq!(
        result,
        vec![
            SilenceInterval {
                start: 2.0,
                end: 4.0
            },
            SilenceInterval {
                start: 55.0,
                end: 60.0
            },
        ]
    );
}

// ── silence_to_speech ────────────────────────────────────────────

#[test]
fn speech_no_silence_returns_full_duration() {
    let result = silence_to_speech(&[], 60.0, 0.2);
    assert_eq!(
        result,
        vec![SpeechInterval {
            start: 0.0,
            end: 60.0
        }]
    );
}

#[test]
fn speech_single_silence_mid_video() {
    let silences = vec![SilenceInterval {
        start: 5.0,
        end: 8.0,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    assert_eq!(
        result,
        vec![
            SpeechInterval {
                start: 0.0,
                end: 5.2
            },
            SpeechInterval {
                start: 7.8,
                end: 60.0
            },
        ]
    );
}

#[test]
fn speech_padding_clamps_to_zero() {
    let silences = vec![SilenceInterval {
        start: 0.0,
        end: 3.0,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    // Speech starts at max(0, 3.0 - 0.2) = 2.8
    assert_eq!(
        result,
        vec![SpeechInterval {
            start: 2.8,
            end: 60.0
        }]
    );
}

#[test]
fn speech_padding_clamps_to_duration() {
    let silences = vec![SilenceInterval {
        start: 57.0,
        end: 60.0,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    // Speech ends at min(60.0, 57.0 + 0.2) = 57.2
    assert_eq!(
        result,
        vec![SpeechInterval {
            start: 0.0,
            end: 57.2
        }]
    );
}

#[test]
fn speech_overlapping_padding_merges() {
    // Short silence gap: 5.0 to 5.3 (only 0.3s)
    // With 0.2s padding on each side, the speech segments would overlap
    let silences = vec![SilenceInterval {
        start: 5.0,
        end: 5.3,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    // Padding would create [0, 5.2] and [5.1, 60] -- overlap -> merge
    assert_eq!(
        result,
        vec![SpeechInterval {
            start: 0.0,
            end: 60.0
        }]
    );
}

#[test]
fn speech_entire_video_silent() {
    let silences = vec![SilenceInterval {
        start: 0.0,
        end: 60.0,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    assert!(result.is_empty());
}

#[test]
fn speech_zero_padding() {
    let silences = vec![SilenceInterval {
        start: 5.0,
        end: 8.0,
    }];
    let result = silence_to_speech(&silences, 60.0, 0.0);
    assert_eq!(
        result,
        vec![
            SpeechInterval {
                start: 0.0,
                end: 5.0
            },
            SpeechInterval {
                start: 8.0,
                end: 60.0
            },
        ]
    );
}

#[test]
fn speech_multiple_silences() {
    let silences = vec![
        SilenceInterval {
            start: 2.0,
            end: 4.0,
        },
        SilenceInterval {
            start: 10.0,
            end: 15.0,
        },
    ];
    let result = silence_to_speech(&silences, 60.0, 0.2);
    assert_eq!(
        result,
        vec![
            SpeechInterval {
                start: 0.0,
                end: 2.2
            },
            SpeechInterval {
                start: 3.8,
                end: 10.2
            },
            SpeechInterval {
                start: 14.8,
                end: 60.0
            },
        ]
    );
}

// ── build_concat_filter ──────────────────────────────────────────

#[test]
fn concat_filter_single_segment() {
    let speeches = vec![SpeechInterval {
        start: 0.0,
        end: 5.2,
    }];
    let result = build_concat_filter(&speeches);
    assert_eq!(
        result,
        "[0:v]trim=start=0.000:end=5.200,setpts=PTS-STARTPTS[v0];\
         [0:a]atrim=start=0.000:end=5.200,asetpts=PTS-STARTPTS[a0];\
         [v0][a0]concat=n=1:v=1:a=1[outv][outa]"
    );
}

#[test]
fn concat_filter_multiple_segments() {
    let speeches = vec![
        SpeechInterval {
            start: 0.0,
            end: 5.2,
        },
        SpeechInterval {
            start: 7.8,
            end: 60.0,
        },
    ];
    let result = build_concat_filter(&speeches);
    assert_eq!(
        result,
        "[0:v]trim=start=0.000:end=5.200,setpts=PTS-STARTPTS[v0];\
         [0:a]atrim=start=0.000:end=5.200,asetpts=PTS-STARTPTS[a0];\
         [0:v]trim=start=7.800:end=60.000,setpts=PTS-STARTPTS[v1];\
         [0:a]atrim=start=7.800:end=60.000,asetpts=PTS-STARTPTS[a1];\
         [v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"
    );
}

#[test]
fn concat_filter_empty_speeches() {
    let result = build_concat_filter(&[]);
    assert_eq!(result, "");
}

// ── total_silence_removed ────────────────────────────────────────

#[test]
fn total_removed_single_silence() {
    let silences = vec![SilenceInterval {
        start: 5.0,
        end: 8.0,
    }];
    // 3.0s silence, minus 0.2 padding from each side = 2.6s removed
    let result = total_silence_removed(&silences, 0.2);
    assert!((result - 2.6).abs() < 1e-9);
}

#[test]
fn total_removed_multiple_silences() {
    let silences = vec![
        SilenceInterval {
            start: 2.0,
            end: 4.0,
        },
        SilenceInterval {
            start: 10.0,
            end: 15.0,
        },
    ];
    // (2.0 - 0.4) + (5.0 - 0.4) = 1.6 + 4.6 = 6.2
    let result = total_silence_removed(&silences, 0.2);
    assert!((result - 6.2).abs() < 1e-9);
}

#[test]
fn total_removed_no_silence() {
    let result = total_silence_removed(&[], 0.2);
    assert!((result - 0.0).abs() < 1e-9);
}

#[test]
fn total_removed_short_silence_under_padding() {
    // Silence is only 0.3s, padding would eat 0.4s -- clamp to 0
    let silences = vec![SilenceInterval {
        start: 5.0,
        end: 5.3,
    }];
    let result = total_silence_removed(&silences, 0.2);
    assert!((result - 0.0).abs() < 1e-9);
}
