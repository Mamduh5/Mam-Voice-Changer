# Architecture

## Current milestone

The current implementation is a clean microphone-to-output passthrough. Effects are
not connected. This keeps Milestone 1 behavior observable and prevents the removed fake
pitch algorithm from being mistaken for working DSP.

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
input callback -> normalized f32 -> channel map -> bounded ring -> output callback
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
- `error.rs`: precise user-facing audio errors

### Rust audio infrastructure

- `audio/device.rs`: discovery, stable fingerprints, and device resolution
- `audio/stream_config.rs`: common-rate, channel, format, and buffer negotiation
- `audio/sample_format.rs`: normalized `f32` conversion for `f32`, `i16`, and `u16`
- `audio/channel_mapper.rs`: allocation-free mono/stereo mapping
- `audio/ring_buffer.rs`: bounded SPSC buffering and explicit under/overflow policy
- `audio/input_stream.rs`: typed CPAL input callbacks
- `audio/output_stream.rs`: typed CPAL output callbacks
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

They only convert/map samples, access the lock-free ring, and update atomics.

## Device identity limitation

CPAL 0.15 does not expose WASAPI endpoint GUIDs through its public device API. The app
therefore uses a deterministic FNV-1a fingerprint of direction plus normalized Windows
friendly name. This removes enumeration-index instability, but renaming a device changes
its ID and duplicate friendly names cannot be uniquely distinguished. Native endpoint-ID
lookup is a focused future hardening task if this limitation is observed in testing.
