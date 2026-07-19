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
- Versioned `presets.json` persistence in Tauri's application-data directory
- Built-in `Natural`, `Warm tone`, and `Bright tone` presets plus user presets
- Preset apply, save, rename, duplicate, delete, and reset workflows
- Complete DSP snapshot validation on preset save and load
- Persisted selected-preset restoration before audio starts

Built-in presets can be applied or duplicated but cannot be renamed or deleted.
Saving creates and selects a user preset. Deleting the selected user preset, or
using Reset, selects the built-in `Natural` preset and applies its parameters.

## Automated validation coverage

Device-independent Rust tests cover the versioned preset document, invalid schema
and parameter rejection, persistence and selection restoration,
duplicate/delete/reset consistency, and corrupt-file preservation. Frontend tests
cover the existing device-selection fallback. This section describes test
coverage; it does not claim that a particular checkout has passed the suites.

## Manual validation completed

The 2026-07-18 session established that a Tauri debug executable could launch,
the React UI could render, and the present Realtek endpoints could be enumerated.
It did not exercise live passthrough or preset workflows. See
`docs/manual-test-plan.md` for the exact record.

## Manual validation still required

- Preset save, apply, rename, duplicate, delete, reset, and restart persistence
- Continuous monitored audio and repeated start/stop behavior
- Device disconnection and recovery
- VB-CABLE routing
- Discord, OBS, TikTok Live Studio, browser, and Facebook Live compatibility
- Long-duration stability and subjective listening quality

Planned compatibility testing is a manual acceptance milestone, not a current
implementation failure.

## Deferred functionality

- Recording
- Resampling devices without a common rate
- AI voice conversion, cloning, or neural inference
- Custom virtual audio drivers
- Cloud processing, accounts, or telemetry
- macOS, Linux, or mobile support

## Validation boundary

Compile-time success does not establish audible quality, safe listening volume,
VB-CABLE routing, Discord/OBS/TikTok compatibility, or long-duration stability.
Those require deliberately low-level manual monitoring. Automated checks establish
implementation safety and consistency only; they do not establish audible quality
or third-party application compatibility.

