# Voice Dataset Capture Phase 2 implementation note

## Implemented boundary

Phase 2 adds `src-tauri/src/voice_dataset/` and a dedicated session-thread
`VoiceDatasetController`. Persistent profiles use application data under
`voice-datasets/`, independent schema-v1 `profiles.json`, per-profile manifests,
separate consent JSON, canonical raw PCM24 mono 48 kHz WAV files, derived trimmed
WAV files, and deletion tombstones. Profile names, prompts, and imported filenames
never become managed paths.

The frontend lives in `src/components/voice-dataset/`, `useVoiceDataset`, typed DTOs,
and narrow progress/filter/quality utilities. `App.tsx` still selects the top-level
page and passes shared device/engine dependencies; it does not own Dataset state.

## Capture and review

Prompted capture has an independent 20-second constant and remains dry. The CPAL
callback converts the current physical-input frame to finite mono, updates atomics,
pushes a bounded preallocated ring, and non-blockingly wakes a worker. It performs
no DSP, file access, logging, blocking, or frontend block events. Finalization
resamples offline when needed, analyzes quality, writes a canonical master, and
adds a pending take. The existing Voice Lab retains its separate 15-second limit.

PCM16, PCM24, PCM32, and float32 mono/stereo WAV at 44.1/48 kHz are imported without
modifying the source, mixed to mono, linearly resampled to 48 kHz, hashed with
SHA-256, analyzed, and kept pending. Automatic classification never accepts audio.
Failed acceptance requires acknowledgement and records a manual override.

Raw files are never overwritten. Trimming writes a derived file with frame
boundaries and recalculated quality/waveform metadata. Export creates an explicit
versioned directory package and includes accepted, non-excluded selected takes by
default. There are no network calls or model features.

## Validation boundary

Automated Rust tests use generated files/audio and no CPAL hardware. Frontend tests
server-render metadata-only states without hardware. These establish deterministic
logic and compatibility with existing tests, not microphone capture, audible
quality, device removal, long-session collection, physical preview, dialog behavior,
or Windows filesystem-lock behavior. Those remain in the manual plan.
