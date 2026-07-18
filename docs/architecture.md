# Architecture

## Runtime ownership

```text
React controls
  -> typed Tauri service
  -> Tauri commands
  -> atomic parameter state / bounded engine commands
  -> engine thread owning CPAL streams

input callback -> normalized/channel-mapped input ring
  -> fixed-block DSP worker
  -> processed output ring -> output callback
```

The CPAL callbacks only convert/map samples, access bounded rings, signal the DSP
worker, and update atomics. Device discovery, frontend calls, logging, locks,
allocation, and DSP algorithms stay out of the callbacks.

## DSP ownership

- `dsp/chain.rs`: validated parameters and exact processor order
- `dsp/gain.rs`: smoothed input/output gain
- `dsp/high_pass.rs`: per-channel DC-blocking filter
- `dsp/noise_gate.rs`: linked detector, hysteresis, and gain envelope
- `dsp/pitch.rs`: fixed-chunk formant-aware pitch frontend
- `dsp/signalsmith.rs`: owned Rust boundary for the static C ABI
- `dsp/dry_wet.rs`: pitch-latency alignment for dry and bypass signals
- `dsp/tone.rs`: smoothed 200 Hz low shelf and 4 kHz high shelf
- `dsp/master_limiter.rs`: linked lookahead ceiling limiter
- `dsp/smoothing.rs`: allocation-free live parameter ramps

## Pitch backend and packaging

Pitch and formant processing use Signalsmith Stretch 1.3.2. The upstream C++11
header and its Signalsmith Linear dependency are vendored with their MIT license
texts. A small C ABI wrapper is compiled by `cc` with the existing Windows MSVC
toolchain and linked statically into the Tauri binary. There is no external DSP
DLL.

The backend is configured with a 2,048-frame analysis block and 512-frame
interval. Pitch and formant values ramp over 20 ms and are applied at fixed
512-frame processing boundaries. Formant compensation remains enabled during
pitch shifts; the independent formant control adds -6 to +6 semitones of spectral
envelope movement.

The published Rust crate wrapper was evaluated but is not used because its build
script requires `libclang` to regenerate a stable C ABI at every build. The local
fixed declarations remove that packaging dependency while using the same upstream
implementation.

## Signal order

```text
normalized input
  -> input gain
  -> 20 Hz high-pass
  -> noise gate
  -> formant-aware pitch
  -> pitch-aligned dry/wet
  -> warmth low shelf
  -> brightness high shelf
  -> pitch-aligned bypass crossfade
  -> output gain
  -> linked lookahead master limiter
  -> final mute ramp
  -> processed output ring
```

Bypass skips gate, pitch, dry/wet, warmth, and brightness. Input gain and the
high-pass filter remain before the bypass tap. Output gain and the master limiter
remain active after bypass. Mute is the final authority.

## Latency

Signalsmith reports separate input and output latency. Their sum is the pitch-path
latency used by both dry/wet and bypass delay lines. The chain then adds the
master limiter's 5 ms lookahead. The DSP metric adds one fixed worker block:

```text
DSP latency frames = Signalsmith input latency
                   + Signalsmith output latency
                   + limiter lookahead
                   + worker block frames
```

The device estimate adds negotiated input/output buffers and output-ring prefill.
These values are configuration-derived estimates, not measured round-trip delay.

## Live updates and allocation boundary

The frontend submits complete validated parameter snapshots. `ParameterState`
stores scalar fields atomically; the DSP worker reads one snapshot per fixed block.
Gain, mix, pitch, formant, tone coefficients, bypass, and mute transition without
hard parameter jumps. All scratch buffers, delay lines, filter states, limiter
lookahead storage, and backend capacity are prepared before block processing.


## Preset persistence

`config/presets.rs` owns the versioned JSON document, built-in definitions, strict
name/id/timestamp/parameter validation, and atomic file replacement. The file is
stored as `presets.json` in Tauri's application-data directory. A completed file
is never replaced by invalid JSON; temporary and backup files support recovery
from an interrupted write.

Preset commands run on the application side, outside CPAL callbacks and the DSP
worker. Applying, duplicating, deleting the active preset, or resetting publishes
one complete validated `DspParameters` snapshot through the existing atomic
parameter state. The selected preset id is committed with the document and
restored before audio starts.

## Master limiter boundary

The limiter detects the linked peak across all channels, applies immediate gain
reduction to a delayed signal, holds reduction through the lookahead window, and
releases over 80 ms. Non-finite input is replaced with silence. A final ceiling
clamp covers numerical and live-ceiling edge cases while the limiter is enabled.

This is a digital peak boundary only. It does not guarantee safe acoustic volume
or prevent feedback elsewhere in the physical monitoring path.

