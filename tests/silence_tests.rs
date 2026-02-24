use contentops::silence::{SpeechInterval, build_concat_filter};

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
