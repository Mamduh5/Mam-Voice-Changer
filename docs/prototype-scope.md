# Prototype scope

## Implemented

- Windows input/output discovery, selection, and common-rate negotiation
- Normalized sample conversion and mono/stereo channel mapping
- Bounded non-blocking rings and dedicated DSP worker
- Runtime state, meters, counters, format, errors, and latency estimates
- Input/output gain, 20 Hz high-pass, bypass, and final mute
- Optional coherent noise gate
- Signalsmith pitch transformation with formant compensation
- Independent -6 to +6 semitone formant shift
- Pitch-aligned 0-100% dry/wet mixing
- Warmth and brightness shelf EQ
- Linked lookahead master limiter with -12 to -1 dBFS ceiling
- Atomic live parameter updates and smoothed transitions

## Not implemented

- Persisted presets or parameter reset workflows
- Recording
- Resampling devices without a common rate
- AI voice conversion, cloning, or neural inference
- Custom virtual audio drivers
- Cloud processing, accounts, or telemetry
- macOS, Linux, or mobile support

## Validation boundary

Compile-time success does not establish audible quality, safe listening volume,
VB-CABLE routing, Discord/OBS/TikTok compatibility, or long-duration stability.
Those require deliberately low-level manual monitoring and remain separate from
implementation work.

