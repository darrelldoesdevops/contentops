# Plan 18-02 Summary: Doctor VAD Health Check & README Update

**Status:** Complete
**Commit:** ec95ec0

## What Changed

- Added `check_vad()` function to doctor.rs that creates a VoiceActivityDetector instance to verify ONNX Runtime initializes
- Displays "VAD (Silero V5)" with [ok] or [fail] status
- Updated README.md: removed --breaths from cut flags table, added --vad-threshold and --min-silence-ms to both pipeline and cut flags tables

## Files Modified

- src/commands/doctor.rs -- added check_vad(), added to checks vec
- README.md -- updated flags tables
