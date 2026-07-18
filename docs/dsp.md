# DSP design and controls

## Default parameter set

| Parameter      |  Default |     Valid range |
| -------------- | -------: | --------------: |
| Pitch          |     0 st |   -12 to +12 st |
| Formant shift  |     0 st |     -6 to +6 st |
| Dry/wet        |      35% |       0 to 100% |
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

Dry/wet combines the transformed signal with a dry signal delayed by the backend's
reported pitch latency. At 0% the result is latency-aligned dry audio; at 100% it
is fully transformed.

## Tone controls

Warmth is an RBJ-style low shelf centered at 200 Hz. Brightness is a high shelf at
4 kHz. Each channel has independent biquad history, while coefficient ramps use a
shared 20 ms transition so channel timing remains coherent. Both controls are
limited to +/-6 dB.

## Gate, bypass, limiter, and mute

The gate uses one linked peak detector for every channel with hysteresis and
smoothed attack/release. It is disabled by default.

Bypass crossfades to a pitch-latency-aligned tap taken after input gain and the
high-pass filter. It skips the gate, pitch, dry/wet, and tone controls while still
passing through output gain and the limiter.

The master limiter uses linked detection, 5 ms lookahead, an 80 ms release, and a
configurable ceiling. Its delay remains in the path while disabled so toggling it
does not change alignment. Mute is applied last with a 10 ms ramp.

The limiter controls digital sample peaks. Acoustic level depends on later output
gain stages, Windows volume, amplifiers, headphones/speakers, microphone coupling,
and listening duration.


## Preset parameter scope

Presets serialize the complete \`DspParameters\` snapshot shown above: pitch,
formant shift, dry/wet, gate state and threshold, input/output gain, warmth,
brightness, master ceiling, limiter state, bypass, and mute. Built-in presets only
adjust processors that exist in the native chain. Preset JSON is validated before
storage and again when loaded; applying a preset publishes the same live atomic
snapshot as direct control changes.

## Real-time constraints

The processing worker owns every stateful processor. It allocates scratch buffers,
delay storage, filter states, and backend capacity during preparation. Per-block
processing reads atomics, mutates owned buffers, and does not acquire application
locks or call frontend/device APIs.

