# Plan 18-01 Summary: Tuning Flags & Dead Code Removal

**Status:** Complete
**Commit:** d651824

## What Changed

- Added `--vad-threshold` (f32, 0.0-1.0, default 0.5) and `--min-silence-ms` (u32, min 1, default 400) to CutArgs and PipelineArgs
- Removed `--breaths` flag from both commands
- Updated `vad::run_vad()` to accept threshold and min_silence_ms parameters
- Added min-silence merge pass in vad.rs (merges speech segments separated by gaps shorter than min_silence_ms)
- Deleted all dead amplitude code: SilenceInterval, parse_silencedetect, silence_to_speech, filter_silences_by_words, words_to_speech_intervals, total_silence_removed from silence.rs
- Deleted run_silencedetect from ffmpeg.rs
- Removed all DEPRECATED comments from cut.rs and pipeline.rs
- Updated tests to remove dead code tests, kept build_concat_filter tests

## Files Modified

- src/cli.rs -- new flags, removed breaths, added parse_vad_threshold validator
- src/vad.rs -- parameterized threshold/min_silence_ms, added merge pass
- src/silence.rs -- removed 6 dead items, kept SpeechInterval + 3 functions
- src/ffmpeg.rs -- removed run_silencedetect
- src/commands/cut.rs -- threaded new args, removed deprecated comments
- src/commands/pipeline.rs -- threaded new args, removed deprecated comments
- tests/silence_tests.rs -- kept only build_concat_filter tests

## Decisions

- Used custom parse function for f32 range validation (clap value_parser!().range() only supports integers)
- min_silence_ms=0 rejected by clap range validator (min 1), matching "no clamping" user decision
