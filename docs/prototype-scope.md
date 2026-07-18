# Prototype scope

## Delivered in this change

This change restores a valid Tauri 2 project and implements Milestones 1 and 2:

- Windows input/output discovery
- User-selected input and output
- Common-rate and buffer negotiation
- `f32`, `i16`, and `u16` conversion
- Mono/stereo mapping
- Bounded non-blocking buffering
- Dedicated stream lifecycle ownership
- Runtime state, meters, counters, format, latency estimate, and recoverable errors
- Typed React-to-Tauri service boundary
- Input and output gain
- Mute and bypass
- Per-channel 20 Hz high-pass filtering
- Soft limiting
- Atomic live parameter updates with no callback allocation or ordinary locking
- Dedicated bounded DSP processing worker
- Stateful -12 to +12 semitone phase-vocoder pitch shifting
- Pitch-latency-aligned dry/wet mixing
- Smoothed gain, mix, pitch, mute, and bypass transitions
- Stateful coherent noise gate with attack, release, and hysteresis

## Gated follow-up work

The following work is not implemented:

- Persisted JSON presets and parameter reset
- Compatibility testing in Discord and OBS
- TikTok Live Studio routing validation

The fake amplitude-based pitch implementation and hardcoded UI presets were removed.
They were not converted into UI simulations.

## Explicitly out of scope

- Custom Windows virtual audio drivers
- AI voice conversion or cloning
- Neural inference
- Cloud processing
- Recording
- Accounts or telemetry
- macOS, Linux, or mobile support
