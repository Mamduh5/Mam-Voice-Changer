# DSP design and controls

## Default parameter set

| Parameter      |  Default |     Valid range |
| -------------- | -------: | --------------: |
| Pitch          |     0 st |   -12 to +12 st |
| Formant shift  |     0 st |     -6 to +6 st |
| Consonant preservation | 100% |       0 to 100% |
| Dry/wet        |      35% |       0 to 100% |
| Age Character  |       0% |       0 to 100% |
| Breathiness    |       0% |       0 to 100% |
| Tremor         |       0% |       0 to 100% |
| Noise gate     |      Off |          On/off |
| Gate threshold | -50 dBFS | -80 to -10 dBFS |
| Input gain     |     0 dB |   -24 to +24 dB |
| Output gain    |    -6 dB |   -24 to +12 dB |
| Warmth         |     0 dB |     -6 to +6 dB |
| Brightness     |     0 dB |     -6 to +6 dB |
| Master ceiling |  -3 dBFS |  -12 to -1 dBFS |
| Master limiter |       On |          On/off |
| Bypass         |      Off |          On/off |
| Mute           |      Off |          On/off |

## Voice transformation

Signalsmith Stretch performs pitch transformation while preserving stream length.
Formant compensation is active during pitch changes, and the formant slider moves
the estimated spectral envelope independently. This is genuine native processing;
the frontend does not synthesize results or provide a fallback device response.
Pitch follows the equal-tempered ratio `2^(semitones / 12)`. Formant shift changes
the spectral envelope without intentionally changing F0.

Before the user dry/wet stage, a deterministic voiced/unvoiced detector protects
consonants and fricatives from the pitch/formant transformation. It is a causal DSP
classifier, not a neural model or speech/non-speech VAD. A trailing 25 ms window,
updated every 10 ms, combines normalized autocorrelation periodicity over an
approximate 60-500 Hz F0 range with RMS energy, zero-crossing rate, and normalized
first-difference energy. Hysteresis plus 20 ms attack and 35 ms release smoothing
produce one continuous, stereo-linked voicing probability. Startup is unvoiced
until one complete trailing window is available.

The pre-transform signal uses the existing dry delay, while the probability is
delayed by the same reported Signalsmith latency. For each aligned frame:

```text
preserved amount = consonant preservation * (1 - voicing probability)
effect wet = transformed * (1 - preserved amount)
           + aligned preserved * preserved amount
```

The preservation control ramps over 20 ms. Signalsmith continues processing every
sample and is never reset when classification changes. Dry/wet then independently
combines this effect-wet signal with the same aligned dry signal: 0% remains
latency-aligned dry audio and 100% uses the consonant-preserved effect result.

The configured 2,048-frame analysis backend reports 2,048 total algorithmic
latency frames (Signalsmith input latency plus output latency). The application
converts that frame count using the active sample rate: about 42.7 ms at 48 kHz
or 46.4 ms at 44.1 kHz. The live DSP estimate additionally includes limiter
lookahead and one worker block. The preserved signal, dry mix, and bypass tap delay by the
Signalsmith total before crossfading, so they do not combine an immediate dry
signal with delayed transformed audio.

Pitch and formant targets use a 20 ms ramp and are published to Signalsmith at
bounded 64-frame control intervals without reconstructing the backend.

The detector uses a trailing window and therefore adds zero lookahead frames. Its
classification naturally reacts over the window, hop, and attack/release times;
that response time is not hidden as audio-path latency.

Extreme pitch/formant combinations can sound synthetic. Classification is not
guaranteed for whispered or strongly breathy speech, vocal fry, creaky or
irregular elderly voices, simultaneous speakers, music, heavy environmental
noise, singing with unusual phonation, or consonants that strongly overlap voiced
energy.
Deterministic DSP tests establish frequency, envelope, duration, transition,
latency, and finite-output behavior; they do not establish subjective voice
quality or naturalness.

## Vocal aging

The dedicated zero-latency vocal-aging processor adds bounded 4.8 Hz pitch and
amplitude tremor, deterministic interpolated pitch jitter and amplitude shimmer,
speech-envelope-followed shaped aspiration, low-frequency weight reduction, mild
upper-mid presence, and gentle high-frequency restraint. Pitch movement is added
to the existing Signalsmith transpose control at worker-block rate; there is no
second pitch shifter.

At full internal strength the bounds are +/-18 cents tremor, +/-9 cents jitter,
+/-3.5% amplitude tremor, +/-1.8% shimmer, and 0.045 aspiration gain before
spectral shaping and limiting. `Age Character` coordinates the full processor with
a perceptual curve. `Breathiness` and `Tremor` scale the two most useful character
families. All three default to zero, so migrated and existing presets remain neutral.

## Tone controls

Warmth is an RBJ-style low shelf centered at 200 Hz. Brightness is a high shelf at
4 kHz. Each channel has independent biquad history, while coefficient ramps use a
shared 20 ms transition so channel timing remains coherent. Both controls are
limited to +/-6 dB.

## Gate, bypass, limiter, and mute

The gate uses one linked peak detector for every channel with hysteresis and
smoothed attack/release. It is disabled by default.

Bypass crossfades to a pitch-latency-aligned tap taken after input gain and the
high-pass filter. It skips the gate, pitch, dry/wet, vocal aging, aspiration, and
tone controls while still passing through output gain and the limiter.

The master limiter uses linked detection, 5 ms lookahead, an 80 ms release, and a
configurable ceiling. Its delay remains in the path while disabled so toggling it
does not change alignment. Mute is applied last with a 10 ms ramp.

The limiter controls digital sample peaks. Acoustic level depends on later output
gain stages, Windows volume, amplifiers, headphones/speakers, microphone coupling,
and listening duration.


## Preset parameter scope

Presets serialize the complete `DspParameters` snapshot shown above: pitch,
formant shift, consonant preservation, dry/wet, vocal-aging controls, gate state and threshold, input/output gain, warmth,
brightness, master ceiling, limiter state, bypass, and mute. Built-in presets only
adjust processors that exist in the native chain. Preset JSON is validated before
storage and again when loaded; applying a preset publishes the same live atomic
snapshot as direct control changes.

Schema version 3 adds consonant preservation. Version-2 user presets are migrated
with preservation at 100%; version-1 presets also receive zero aging values.
Parameters, preset identity, and selection are preserved during the atomic rewrite;
future schema versions and corrupt documents are still rejected.

## Real-time constraints

The processing worker owns every stateful processor. It allocates scratch buffers,
analysis storage, delay storage, filter states, and backend capacity during preparation. Per-block
processing reads atomics, mutates owned buffers, and does not acquire application
locks or call frontend/device APIs.

