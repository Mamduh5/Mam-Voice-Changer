# Architecture

## Current milestone

The current implementation includes Milestone 1 audio routing, Milestone 2 basic DSP,
and tested Milestone 3 pitch, dry/wet, and noise-gate processing. Presets are not
connected.

```text
React components
      |
typed Tauri service
      |
Tauri command handlers
      |
bounded engine command channel
      |
dedicated engine worker (owns both CPAL streams)
      |
input callback -> normalized f32 -> channel map -> bounded input ring
      |
dedicated DSP worker -> parameter snapshot -> fixed-size DSP block
      |
bounded processed-output ring -> output callback -> device samples
```

## Module ownership

### Frontend

- `src/components`: visual selectors, controls, meters, and diagnostics only
- `src/hooks`: device refresh/selection and engine polling/commands
- `src/services/tauriAudioApi.ts`: the complete typed Tauri boundary
- `src/types`: serializable frontend contracts
- `src/utils/deviceSelection.ts`: missing-device selection fallback

### Rust application boundary

- `commands`: input validation and frontend-safe command results
- `state/app_state.rs`: owns the thread-safe engine controller
- `state/engine_state.rs`: explicit lifecycle states and transition rules
- `state/parameter_state.rs`: atomic live DSP parameter values
- `error.rs`: precise user-facing audio errors

### DSP

- `dsp/processor.rs`: device-independent processor interface
- `dsp/chain.rs`: ordered processors plus mute and bypass behavior
- `dsp/gain.rs`: input/output decibel gain
- `dsp/high_pass.rs`: per-channel DC-blocking filter state
- `dsp/pitch.rs`: stateful STFT phase-vocoder pitch transformation
- `dsp/dry_wet.rs`: pitch-latency-aligned dry delay and mixing
- `dsp/smoothing.rs`: allocation-free parameter ramps
- `dsp/limiter.rs`: bounded soft limiting
- `dsp/noise_gate.rs`: coherent detector, hysteresis, and smoothed gate gain

### Rust audio infrastructure

- `audio/device.rs`: discovery, stable fingerprints, and device resolution
- `audio/stream_config.rs`: common-rate, channel, format, and buffer negotiation
- `audio/sample_format.rs`: normalized `f32` conversion for `f32`, `i16`, and `u16`
- `audio/channel_mapper.rs`: allocation-free mono/stereo mapping
- `audio/ring_buffer.rs`: bounded SPSC buffering and explicit under/overflow policy
- `audio/input_stream.rs`: typed CPAL input callbacks
- `audio/worker.rs`: fixed-block processing, parameter snapshots, and processed-ring writes
- `audio/output_stream.rs`: processed-ring reads and typed output conversion
- `audio/controller.rs`: stream-owning worker and lifecycle commands
- `audio/metrics.rs`: atomics for callback metrics and locks used only outside callbacks

## Stream lifecycle

CPAL stream handles are intentionally created, played, held, and dropped on the
dedicated audio worker. They never enter Tauri shared state. Start and stop commands use
a bounded synchronous channel and one-shot bounded replies. Repeated start requests drop
the previous pair before constructing a new pair.

CPAL error callbacks send a fixed enum into a bounded non-blocking channel. The worker
converts that event into a descriptive error, drops both streams, clears the active
format, and moves the engine to `error`. Callback code does not format or log errors.

## Format negotiation

Only `f32`, `i16`, and `u16` stream formats are considered. Every input/output supported
configuration pair is checked for a sample-rate intersection. The scorer prefers:

1. 48 kHz
2. 44.1 kHz
3. another common rate closest to 48 kHz
4. `f32`, then `i16`, then `u16`
5. stereo, then mono, then wider layouts

The engine does not resample. A pair with no common rate is rejected before playback
with a corrective user-facing message.

## Buffering and latency

The SPSC ring holds at most 250 ms of output-channel-interleaved normalized samples.
It is prefilled with two negotiated callback buffers of silence to reduce startup
underruns. If full, input drops the newest sample and records one overrun for that input
callback. If empty, output writes silence and records one underrun for that output
callback.

Estimated latency is computed from requested input frames, output frames, and prefill
frames. It is not a measured round-trip value.

## Real-time invariants

Audio data callbacks do not:

- allocate
- acquire an ordinary lock
- log or format diagnostics
- perform file or network I/O
- enumerate devices
- sleep
- panic on recoverable errors

They only convert/map samples, access a lock-free ring, signal the worker through a
bounded non-blocking channel, and update atomics. FFT processing and parameter snapshot
application run only on the dedicated DSP worker.

## Pitch algorithm and dependency

Pitch shifting uses a native Rust short-time Fourier transform phase vocoder with a
2,048-frame Hann window, 4x overlap, and persistent per-channel phase state. Spectral
bins and instantaneous frequencies are shifted while the synthesis hop remains fixed,
so output sample count and stream duration remain continuous. This is not amplitude
scaling and is not a playback-rate-only resampler.

FFT operations use RustFFT 6.4.1, a pure-Rust dependency licensed under MIT OR
Apache-2.0. It compiles into the application executable, supports Windows x64, and does
not require a separately installed DLL or native build tool beyond the existing Rust
toolchain.

The processor does not claim formant preservation or studio-quality artifacts. Larger
shifts can produce phase-vocoder smearing and shifted vocal formants.

## Processing latency

The pitch algorithm reports 1,536 frames of algorithmic latency (FFT size minus hop
size). A preallocated dry delay line uses the same frame count before dry/wet mixing.
DSP processing latency also includes one fixed worker block. The UI reports this DSP
estimate separately and adds it to the existing device-buffer/prefill estimate. This is
not a measured round-trip latency.

## Signal order and transitions

The worker applies this order to each fixed block:

```text
input gain -> 20 Hz high-pass -> noise gate -> pitch -> aligned dry/wet
           -> mute/bypass crossfade -> output gain -> soft limiter
```

Mute always fades to silence over 10 ms. Bypass crossfades over 10 ms to a separate
latency-aligned signal taken after input gain and high-pass but before gate, pitch, and
dry/wet. Output gain and the safety limiter remain active during bypass. The pitch and
gate state stay warm during bypass so returning to processed audio does not introduce a
fresh algorithmic-latency gap.

Input gain, output gain, and dry/wet changes use 10 ms ramps. Pitch changes use a 15 ms
semitone ramp before new phase-vocoder frames consume the value.

## Noise gate

The gate uses one peak detector across all channels so stereo and multichannel gains
remain coherent. The detector uses 5 ms attack and 50 ms release. Opening occurs at the
configured threshold; closing occurs 6 dB below it. The applied gain uses 10 ms attack
and 120 ms release, preventing per-sample hard toggles. The default is enabled at a
conservative -50 dBFS threshold; valid thresholds are -80 to -10 dBFS.

## Device identity limitation

CPAL 0.15 does not expose WASAPI endpoint GUIDs through its public device API. The app
therefore uses a deterministic FNV-1a fingerprint of direction plus normalized Windows
friendly name. This removes enumeration-index instability, but renaming a device changes
its ID and duplicate friendly names cannot be uniquely distinguished. Native endpoint-ID
lookup is a focused future hardening task if this limitation is observed in testing.
