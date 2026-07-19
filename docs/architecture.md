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
  -> independent bounded destination ring -> destination callback
  -> independent bounded monitor ring -> optional monitor callback
```

The CPAL callbacks only convert/map samples, access bounded rings, signal the DSP
worker, and update atomics. Device discovery, frontend calls, logging, locks,
allocation, and DSP algorithms stay out of the callbacks.

## DSP ownership

- `dsp/chain.rs`: validated parameters and exact processor order
- `dsp/gain.rs`: smoothed input/output gain
- `dsp/high_pass.rs`: per-channel DC-blocking filter
- `dsp/noise_gate.rs`: linked soft speech expander with hysteresis, hold, and an attenuation floor
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
  -> soft speech expander (when Gate is enabled)
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

The device estimate adds negotiated input/output buffers and profile-specific output-ring prefill.
These values are configuration-derived estimates, not measured round-trip delay.

## Reliability and recovery

Low latency requests 128 callback frames, uses 80 ms input/output rings, 256 prefill frames, no worker underrun tolerance, and up to 3 ms concealment. Balanced (default) uses 256 frames, 250 ms rings, 1,024 prefill frames, one-block tolerance, and 6 ms concealment. Reliable uses 512 frames, 500 ms rings, 2,048 prefill frames, two-block tolerance, and 10 ms concealment. Actual callback sizes remain subject to CPAL/WASAPI negotiation.

Input starts first. Output streams are constructed and started only after all configured processed-output rings reach the prefill target; 500/1,000/1,500 ms profile-specific timeouts prevent an indefinite startup wait. Very short underruns continue the last valid frame with linear decay, then crossfade back over 2 ms. Longer gaps become silence.

Input or main-destination errors are retained verbatim and trigger at most three staged restart attempts with 100, 300, and 900 ms delays. Endpoints are re-enumerated by stable identifier, then by a unique matching friendly name. An optional monitor failure leaves the main destination running in `degraded`; a Test monitor-only failure is treated as the active route failure. Stop cancels any queued recovery.

Ring-fill trends and a correction ratio/min/max of 1.0 are exposed for clock-drift observation. Adaptive resampling is intentionally pending until long-session evidence shows persistent drift.

## Live updates and allocation boundary

The frontend submits complete validated parameter snapshots. `ParameterState`
stores scalar fields atomically; the DSP worker reads one snapshot per fixed block.
Gain, mix, pitch, formant, tone coefficients, bypass, and mute transition without
hard parameter jumps. All scratch buffers, delay lines, filter states, limiter
lookahead storage, and backend capacity are prepared before block processing.


## Preset persistence

`config/presets.rs` owns the versioned JSON document, built-in definitions, strict
name/id/timestamp/parameter validation, and atomic file replacement. The file is
stored as `presets.json` in Tauri's application-data directory. It stores user
presets and the selected preset id; the three built-ins (`Natural`, `Warm tone`,
and `Bright tone`) are defined by the application and merged into the catalog at
read time. A completed file is never replaced by invalid JSON; temporary and
backup files support recovery from an interrupted write.

Preset commands run on the application side, outside CPAL callbacks and the DSP
worker. Save creates and selects a user preset from one complete validated
`DspParameters` snapshot. Apply, duplicate, deletion of the selected user preset,
and reset publish the resulting complete snapshot through the existing parameter
state; rename changes metadata only. Reset selects `Natural`, and deleting the
selected user preset has the same fallback. Built-ins can be applied or duplicated
but cannot be renamed or deleted. The selected preset id is committed with the
document and restored before audio starts.

Preset persistence is not an application-compatibility claim. Its storage and
state transitions are device-independent; audible behavior and compatibility with
VB-CABLE or receiving applications remain separate manual validation work.

## Master limiter boundary

The limiter detects the linked peak across all channels, applies immediate gain
reduction to a delayed signal, holds reduction through the lookahead window, and
releases over 80 ms. Non-finite input is replaced with silence. A final ceiling
clamp covers numerical and live-ceiling edge cases while the limiter is enabled.

This is a digital peak boundary only. It does not guarantee safe acoustic volume
or prevent feedback elsewhere in the physical monitoring path.

