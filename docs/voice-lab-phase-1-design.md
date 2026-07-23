# Voice Lab Phase 1 design

## Inspected baseline

- `DspChain` can be created independently with `DspChain::default()`. It is
  prepared with an explicit sample rate, channel count, and maximum block size;
  receives one validated `DspParameters` snapshot; and is reset through the
  existing `AudioProcessor::reset` contract.
- The chain reports latency after `prepare`. That value includes the Signalsmith
  input/output latency, the aligned dry/bypass delay, zero-latency vocal aging,
  and the limiter lookahead. The live worker adds one scheduling block to its
  latency metric; an offline renderer does not need that worker-only block.
- The live worker uses fixed profile-selected blocks. Signalsmith internally
  processes chunks of up to 512 frames, while `DspChain` accepts any prepared
  block length that fits its scratch buffers. Voice Lab will use 512-frame
  offline blocks for deterministic processing.
- `reset` clears every processor and delay state and resets Signalsmith. There is
  no separate chain flush API. Offline processing must append at least the
  reported latency as zero input, round up to a full offline block, then discard
  the initial latency and retain the original frame count for aligned A/B clips.
- Live input conversion supports CPAL `f32`, `i16`, and `u16`, normalizes to
  finite `f32`, and maps complete frames through the existing channel mapper.
  Output conversion clamps normalized `f32` back to those three CPAL formats.
- Use and Test are mutually exclusive because `AudioController` retains one
  active request/stream bundle/recovery plan and rejects another start until the
  first route stops. Voice Lab will not use that controller for capture,
  rendering, or preview.
- Presets are native-backed, versioned, and atomically persisted. Existing save
  selects the new preset and updates live DSP, so Voice Lab needs a separate
  non-selecting save operation to preserve isolation.
- No WAV crate, Tauri dialog plugin, or frontend filesystem package is currently
  installed. Phase 1 will add `hound` for WAV encoding/decoding and the Tauri 2
  dialog plugin for explicit open/save selection. File bytes remain in Rust;
  temporary audio is never written unless the user explicitly exports it.

## Isolation boundary

Voice Lab is a new Rust-owned subsystem under `src-tauri/src/voice_lab/`. It owns:

- one bounded dry clip and one derived processed clip;
- one independent CPAL capture stream plus bounded capture worker;
- one independent CPAL preview stream;
- one offline renderer backend;
- clip metadata, waveform summaries, preview position, and lifecycle state.

The subsystem may read device metadata and the live engine state, but it does not
reuse `AudioController`, its rings, metrics, recovery plan, or output routes. Lab
capture, rendering, and preview require the live engine to be stopped. Conversely,
the existing start command gains only a guard that rejects Use/Test start while a
Lab capture or preview stream exists. Route creation and recovery remain unchanged.

Voice Lab navigation is UI-local. It is deliberately not added to application
settings, so schema v4 and its persisted `lastPage` enum remain unchanged.

## Bounded audio model

- Maximum clip duration: 15 seconds.
- Supported rates: 44.1 kHz and 48 kHz.
- Supported clip channels: mono or stereo.
- Internal representation: interleaved finite `f32` samples in `[-1, 1]`.
- At most one original and one processed clip are retained (about 11.5 MiB total
  at the maximum stereo/48 kHz case).
- Capture callbacks write complete mapped frames into an existing bounded ring.
  A dedicated Lab worker drains into a preallocated vector; callbacks do not lock,
  allocate, perform DSP, or access the live engine.
- Clear stops Lab streams, joins capture work, and drops every clip buffer.

Imported WAV support is intentionally narrow and actionable: mono/stereo PCM
16/24/32-bit or 32-bit float at 44.1/48 kHz, no more than 15 seconds. Exports are
16-bit PCM WAV and are created only at a user-selected path.

## Offline backend insertion point

`OfflineVoiceProcessor` accepts an immutable `AudioClip` and validated
`DspParameters`, returning a new clip plus render metadata. Phase 1 installs only
`ExistingDspOfflineProcessor`, which creates and resets its own `DspChain` for
every render. A future offline conversion backend can implement this interface
without changing capture, preview, WAV, UI session, or live routing. Phase 1 has
no model files, embeddings, training, neural inference, cloning, or realtime AI.

## Frontend session behavior

The Voice Lab hook keeps a local parameter snapshot initialized from live DSP.
Sliders and Lab preset selection modify only that snapshot and mark an existing
render stale. **Render processed** sends the snapshot to the offline backend.

Explicit actions are separate:

- **Apply preset to Lab** copies preset parameters without selecting it live.
- **Save as new preset** persists the Lab snapshot without changing the selected
  preset or live parameters.
- **Apply to live settings** publishes the complete Lab snapshot through the
  existing serialized parameter synchronizer and waits for confirmation.
- **Export original/processed** opens a save dialog and writes the chosen buffer.
- **Clear temporary audio** stops Lab audio and releases both buffers.

Original and processed previews use the selected local output independently of
Test. Looping is fixed for the lifetime of a preview start; replaying replaces the
prior Lab preview. Leaving Voice Lab stops capture/preview but retains the clips
until Clear or application exit.

## Focused verification

Rust tests will cover WAV formats/limits, bounded clip validation, latency-aligned
offline rendering, finite output, deterministic reset, capture frame mapping,
preview cursor/loop behavior, clear semantics, non-selecting preset persistence,
and live/Lab mutual-exclusion helpers. Frontend tests will cover navigation,
local-only parameter editing, preset copying, stale render state, record/import,
A/B/loop controls, explicit live apply/save/export, disabled states, and clear.

Automated tests cannot establish microphone capture, audible quality, feedback
safety, Windows dialog behavior, or real endpoint compatibility; those remain
manual acceptance items.

## Phase 4.1 preview compatibility addendum

Preview playback no longer requests the clip or active DSP sample rate from the
output device. The selected device is inspected independently, with 48 kHz
preferred, followed by its default rate and then another supported rate. A
`PreparedPreview` owns bounded normalized samples converted offline to that rate;
the source original/processed clip is not mutated.

Both A/B sources use this policy. A source cursor is converted by elapsed time
when switching between prepared buffers with different rates, and loop restart
uses the prepared rate without accumulating a fractional-frame error. The
existing short edge fade remains in the callback. Preview diagnostics report clip
rate, output rate, whether conversion is active, output channels, and sample
format.

This addendum changes only Voice Lab preview construction. It does not change
Voice Lab rendering, live Use/Test negotiation, external routing, or DSP order.
