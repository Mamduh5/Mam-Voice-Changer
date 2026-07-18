# Prototype scope

## Delivered in this change

This change restores a valid Tauri 2 project and implements Milestone 1 clean audio
passthrough:

- Windows input/output discovery
- User-selected input and output
- Common-rate and buffer negotiation
- `f32`, `i16`, and `u16` conversion
- Mono/stereo mapping
- Bounded non-blocking buffering
- Dedicated stream lifecycle ownership
- Runtime state, meters, counters, format, latency estimate, and recoverable errors
- Typed React-to-Tauri service boundary

## Gated follow-up work

The following milestones remain gated on audible VB-CABLE passthrough validation:

- Basic DSP: high-pass filter, gain, mute, bypass, and limiter
- Voice processing: genuine pitch transformation, dry/wet mix, and noise gate
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
