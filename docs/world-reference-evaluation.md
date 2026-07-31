# Experimental WORLD offline reference evaluation

## Status and boundary

WORLD is available only as an explicitly selected `voice-eval` renderer. It is
an experimental, noncausal, offline reference path for objective comparison with
the existing deterministic Signalsmith DSP. It is not active in Use or Test, the
CPAL callbacks, the real-time DSP worker, presets, the Voice Lab UI, or Voice
Lab's default offline renderer.

WORLD is a classical source-filter vocoder, not a neural model. WORLD output is
not automatically superior to Signalsmith, and this experiment makes no product
quality claim. Objective measurements do not prove naturalness.

## Official source and license

The algorithm source is the official
[`mmorise/World`](https://github.com/mmorise/World) C++ implementation:

- official release: `v1.0.1`;
- pinned commit: `d625e7608ca23a870018f01e7c562ac683d9847f`;
- license: upstream modified BSD;
- provenance: `src-tauri/vendor/world/PROVENANCE.md`;
- license text:
  `src-tauri/vendor/world/licenses/WORLD-modified-BSD.txt`.

Only Harvest, CheapTrick, D4C, synthesis, and their shared FFT/math dependencies
are vendored. No upstream algorithm file is modified. The project-owned C ABI is
`src-tauri/native/world_wrapper.cpp`.

The existing `cc` build in `src-tauri/build.rs` compiles the selected upstream
sources and wrapper as C++14. This uses the same static native-build approach as
the Signalsmith wrapper and supports the Windows MSVC toolchain.

## Analysis and synthesis

The recorded default configuration is:

- frame period: 5.0 ms;
- Harvest F0 floor: 50 Hz;
- Harvest F0 ceiling: 1,000 Hz;
- supported sample rates: 44.1 kHz and 48 kHz.

The offline lifecycle is:

1. Harvest estimates the time axis and F0 contour.
2. CheapTrick estimates a positive power spectral envelope.
3. D4C estimates aperiodicity.
4. Transformations edit features owned by the native result handle.
5. WORLD synthesis generates a waveform from the transformed features.

Unvoiced frames retain `F0 = 0`. Feature dimensions, values, pointers, and
allocation products are validated at the C ABI and again in safe Rust. C++
exceptions are contained and converted to bounded native errors. Rust owns the
opaque result and releases it exactly once through `Drop`.

Official WORLD synthesis calls `randn_reseed` for each synthesis operation with
the same internal state. Repeated renders are tested for waveform equality, but
the report describes this measured implementation behavior instead of promising
determinism for arbitrary future revisions.

## Transformations

For pitch semitones `p`:

```text
pitch_ratio = 2^(p / 12)
voiced_output_f0 = source_f0 * pitch_ratio
unvoiced_output_f0 = 0
```

Pitch accepts the existing -12 to +12 semitone evaluator range. Non-finite
values are rejected. Positive transformed F0 is bounded away from invalid low
values and from a conservative sample-rate-dependent upper limit; clamp counts
appear in the report. The time axis and duration are unchanged.

For formant semitones `q`:

```text
formant_ratio = 2^(q / 12)
output_envelope(f) = input_envelope(f / formant_ratio)
```

Interpolation is linear in log power. Source positions are clamped to DC or
Nyquist at the boundaries; no invalid extrapolation is performed. Formant
warping never changes F0 or applies the pitch ratio again. The first experiment
does not frequency-warp D4C aperiodicity, and this policy is recorded.

WORLD consonant preservation is an offline waveform blend:

```text
amount = consonantPreservation * smoothed_unvoiced_probability
preserved = world_output * (1 - amount) + aligned_source * amount
```

The primary mask is the WORLD voiced/unvoiced contour, linearly lifted to the
sample rate and smoothed with bounded 5 ms attack and 20 ms release. This
preserves original unvoiced waveform content and is separate from WORLD
aperiodic synthesis.

Dry/wet is applied afterward:

```text
output = aligned_source * (1 - dryWet) + preserved * dryWet
```

WORLD resynthesis is not phase-identical to the source, so intermediate dry/wet
values can produce coloration and remain experimental.

## Channels, duration, and alignment

Mono is analyzed directly. Stereo uses a single linked analysis signal:

```text
mono[n] = 0.5 * (left[n] + right[n])
```

One WORLD signal is synthesized and the final mono result is duplicated to both
output channels. Independent left/right pitch tracking and invented spatial
information are intentionally absent.

WORLD's natural length is derived from its frame count and frame period. A
deterministic excess is trimmed; a shortage is zero-padded. When adjustment is
needed, a 5 ms end fade protects the boundary. Reports retain the raw and final
frame counts and adjustment method. Sample rate is never changed.

Source and WORLD use the common time origin. No unconstrained cross-correlation
or hidden bulk correction is applied; the current diagnostic alignment offset
is recorded as zero. This is an offline, noncausal renderer and does not claim
zero real-time algorithmic latency.

## Supported parameters

WORLD cases support only:

- `pitchSemitones`;
- `formantShiftSemitones`;
- `consonantPreservation`;
- `dryWet`.

The evaluation-neutral values are zero age, breathiness, tremor, warmth,
brightness, input gain, and output gain; gate off at its default threshold;
limiter off with the default ceiling; bypass off; and mute off. Any unsupported
non-neutral value fails the case with the parameter names. Nothing is silently
ignored and Signalsmith is not re-entered as a postprocessor.

## Manifest and comparisons

Manifest schema 2 adds case-level renderer and comparison identity:

```json
{
  "schemaVersion": 2,
  "corpusRoot": ".",
  "cases": [
    {
      "id": "pitch-up-seven-world",
      "comparisonGroup": "pitch-up-seven",
      "renderer": "worldReference",
      "description": "Seven-semitone WORLD reference",
      "input": "fixtures/harmonic-220.wav",
      "parameters": {
        "pitchSemitones": 7.0,
        "formantShiftSemitones": 0.0,
        "consonantPreservation": 1.0,
        "dryWet": 1.0,
        "ageCharacter": 0.0,
        "breathiness": 0.0,
        "tremor": 0.0,
        "gateEnabled": false,
        "gateThresholdDb": -50.0,
        "inputGainDb": 0.0,
        "outputGainDb": 0.0,
        "masterCeilingDb": -3.0,
        "warmthDb": 0.0,
        "brightnessDb": 0.0,
        "limiterEnabled": false,
        "bypass": false,
        "muted": false
      }
    }
  ]
}
```

`renderer` is optional and always defaults to `existingDsp`; WORLD must be
explicit. Schema 1 remains supported and is interpreted only as `existingDsp`.
Schema 1 cannot contain renderer or comparison-group fields. The schema was
advanced because the old strict reader rejected unknown fields, so adding them
under version 1 would not have been reader-compatible.

Every report case records renderer identity. Baselines match the stable case ID
and renderer together, preventing an implicit Signalsmith/WORLD comparison.
Cases with the same explicit `comparisonGroup` also appear in a descriptive
cross-renderer section covering pitch and formant error, voicing disagreement,
unvoiced high-frequency LSD and correlation, clipping, RMS change, duration,
and rendering real-time factor. The evaluator does not compute MOS or declare a
winner.

## Running the experiment

From the repository root:

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-eval -- `
  --generate-example target/world-reference-example

cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-eval -- `
  --manifest target/world-reference-example/evaluation-manifest.json `
  --output target/world-reference-example/report

cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-eval -- `
  --manifest target/world-reference-example/evaluation-manifest.json `
  --output target/world-reference-example/enforced-report `
  --baseline target/world-reference-example/report/report.json `
  --fail-on-expectation
```

The generated matrix pairs Signalsmith and WORLD for neutral, +12, -12, and +7
pitch, +4 and -4 formants, three consonant-preservation settings, silence, and a
half-dry structural case. It also covers 44.1 kHz mono and 48 kHz linked stereo.

## Interpretation and limitations

Formant estimates are approximate synthetic-envelope peaks. WORLD may sound
vocoded or buzzy for some voices. Breathy speech, vocal fry, whispering,
irregular phonation, singing, and noise can reduce analysis quality.
Intermediate dry/wet or consonant blends can color transitions. Synthetic
fixtures cannot establish intelligibility, preference, identity preservation,
or product quality.

Release-mode RTF is machine-dependent and descriptive, not a default CI
threshold. Failed quality expectations are retained as experimental results.
Real-speech listening tests remain required before considering any later Voice
Lab integration.
