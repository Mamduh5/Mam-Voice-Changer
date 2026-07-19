# Phase 2: Old Lady vocal-aging design

## Existing capabilities and missing character

Signalsmith already supplies the moderate upward pitch and independent formant
shift needed to move a typical adult source toward a lighter female register.
The pitch-latency-aligned dry/wet path can retain articulation, while the existing
warmth and brightness shelves can reduce weight and avoid excess treble. The gate,
output gain, linked lookahead limiter, bypass crossfade, and mute ramp already
provide the required routing and safety behavior.

Those processors do not create elderly vocal instability, irregularity, aspiration,
or a dedicated thin/nasal spectral character. Phase 2 therefore adds one cohesive
`VocalAgingProcessor` rather than treating Old Lady as an EQ/pitch preset.

## Processor and placement

The processor owns:

- a 4.8 Hz stereo-linked tremor LFO for conservative pitch and amplitude movement;
- deterministic, interpolated pitch jitter and amplitude shimmer with bounded
  targets and fixed reset seeds;
- a linked speech envelope with separate 8 ms attack and 90 ms release;
- deterministic aspiration noise shaped by a 1.6 kHz high-pass and 7.5 kHz
  low-pass before envelope-controlled mixing;
- stable one-pole low-weight reduction, broad 0.9-2.4 kHz presence coloration,
  and gentle high-frequency restraint.

Pitch tremor and jitter are averaged over the worker block and added to the base
transpose sent to the existing Signalsmith instance. This avoids a second pitch
engine and avoids expensive per-sample backend reconfiguration. Amplitude movement,
aspiration, and spectral aging run after pitch-aligned dry/wet and before the
existing tone EQ.

The resulting chain is input gain, 20 Hz high-pass, gate, Signalsmith pitch/formant
with dynamic aging offset, pitch-aligned dry/wet, vocal-aging amplitude movement,
envelope-shaped aspiration, vocal-aging spectral shaping, warmth/brightness,
pitch-aligned bypass crossfade, output gain, linked lookahead limiter, and mute ramp.

## Latency, CPU, and real-time behavior

The aging processor adds zero frames of latency. Existing pitch, dry/wet, bypass,
worker-block, and limiter latency reporting is unchanged. Its cost is a small fixed
number of scalar operations per frame plus four one-pole states per channel; state
and channel buffers are allocated only during `prepare`. Random generators, target
queues, envelopes, phases, and filters are bounded. Processing performs no I/O,
locking, logging, device work, frontend calls, or heap allocation.

## Controls and internal tuning

The public controls are `ageCharacter`, `breathiness`, and `tremor`, each from 0 to
1. `ageCharacter` uses a smoothstep perceptual curve and coordinates jitter,
shimmer, aspiration, thinning, presence, and high-frequency restraint. Breathiness
and tremor scale their respective families without exposing scientific tuning.

At full internal strength, pitch tremor is bounded to +/-18 cents, pitch jitter to
+/-9 cents, amplitude tremor to +/-3.5%, shimmer to +/-1.8%, and aspiration gain to
0.045 before spectral coloring and the master limiter. The Old Lady preset uses
conservative fractions of these maxima.

## Bypass and persistence

The bypass tap remains after input gain and the safety high-pass. Bypass skips the
gate, Signalsmith transformation, dry/wet mix, all vocal-aging stages, aspiration,
and tone EQ. Output gain, the linked limiter, and mute remain after the crossfade.
The aging state continues advancing while bypassed, preventing phase resets when
the wet path returns.

Preset schema version 2 stores the three new values. Loading version 1 uses an
explicit legacy structure, validates it, migrates missing values to zero, preserves
the selected preset, and atomically rewrites the current schema. Corrupt documents
and unsupported future versions remain errors; existing built-in IDs are unchanged.
