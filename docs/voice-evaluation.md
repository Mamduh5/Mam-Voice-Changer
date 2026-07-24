# Deterministic voice-transformation evaluation

## Purpose and boundary

`voice-eval` establishes a repeatable objective baseline for the existing
deterministic DSP chain. It imports local WAV files, renders them through the same
`ExistingDspOfflineProcessor` used by Voice Lab, performs offline analysis, checks
declared expectations, and writes JSON, CSV, Markdown, and optional rendered WAV
outputs.

The evaluator does not start Tauri, open an audio device, access the network, use
Python, load a model, or change live settings. Analysis is isolated under
`voice_evaluation`; none of it runs in the live callback or DSP worker.

Objective metrics do not replace listening tests. Synthetic fixtures do not prove
naturalness or product quality.

## CLI

Generate the deterministic example corpus:

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-eval -- `
  --generate-example target/voice-eval-example
```

Evaluate it and fail CI if a declared expectation fails:

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-eval -- `
  --manifest target/voice-eval-example/evaluation-manifest.json `
  --output target/voice-eval-example/report `
  --fail-on-expectation
```

Optional flags:

- `--no-rendered-audio` omits `rendered/<case-id>.wav` while retaining metrics.
- `--baseline <report.json>` compares deterministic quality metrics with an older
  schema-v1 report.
- Without `--fail-on-expectation`, failed expectations are reported but return
  exit code 0.

Exit code 1 means expectations failed with enforcement enabled. Exit code 2 means
the CLI, manifest, input, rendering, baseline, or report operation was invalid.

## Manifest schema

The schema version is 1. Unknown fields are rejected at every level. A manifest
contains 1–128 cases:

```json
{
  "schemaVersion": 1,
  "corpusRoot": ".",
  "cases": [
    {
      "id": "pitch-up-seven",
      "description": "220 Hz harmonic fixture shifted upward seven semitones",
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
      },
      "expectedPitchRatio": 1.4983070769,
      "segments": [
        {
          "label": "voiced",
          "startMs": 250,
          "endMs": 1750,
          "kind": "voiced"
        }
      ],
      "formantBands": [],
      "expectations": {
        "maximumDurationDeltaFrames": 0,
        "maximumNonFiniteSamples": 0,
        "maximumPitchErrorCents": 45.0
      },
      "tags": ["synthetic", "pitch"]
    }
  ]
}
```

Every case uses the complete current `DspParameters` snapshot and its authoritative
Rust validator. Segment kinds are `voiced`, `unvoiced`, `silence`, and `all`.
Optional formant bands contain `label`, `minimumHz`, and `maximumHz`.

IDs contain only ASCII letters, digits, `-`, or `_` and are safe rendered-audio
filenames. IDs must be unique. Strings, tags, segments, bands, and cases have
explicit count/length limits.

`corpusRoot` and `input` must be normalized relative paths. Absolute paths,
backslashes, root components, and `..` traversal are rejected. Inputs are
canonicalized and must remain beneath the canonical corpus root, including when a
symlink is involved. Reports store the manifest filename and relative input labels,
not private absolute paths.

Segments must have `startMs < endMs`, be ordered by start time, and remain inside
the imported clip.

## WAV and rendering policy

The evaluator reuses Voice Lab WAV and clip validation:

- PCM 16-, 24-, and 32-bit integer WAV;
- IEEE float32 WAV;
- mono or stereo;
- 44.1 or 48 kHz;
- non-empty complete frames;
- maximum 15 seconds.

No resampling occurs. Source sample rate and channels are preserved. Unsupported,
malformed, empty, excessive, or non-finite input is rejected. The current
`AudioClip` policy clamps finite samples to normalized `[-1, 1]`; it does not
silently replace NaN or infinity. `inputSanitized` is therefore false for accepted
inputs, and non-finite counts are still recorded by the defensive analysis layer.

Rendering creates a fresh `DspChain`, prepares it at the source format, processes
512-frame offline blocks, appends sufficient zero input, removes the chain's
reported latency, and returns exactly the source frame count. This is the existing
Voice Lab behavior.

## Analysis configuration

Every report records:

- 40 ms Hann analysis window;
- 10 ms hop;
- 2,048-point deterministic radix-2 FFT;
- 50–1,000 Hz F0 search;
- normalized-autocorrelation voicing threshold 0.55;
- RMS voicing floor 0.0001;
- zero-crossing rejection above 0.35;
- clipping threshold `abs(sample) >= 0.995`;
- spectral epsilon `1e-9`;
- general spectral range 50 Hz–10 kHz, limited by Nyquist;
- high-frequency range 3–10 kHz, limited by Nyquist.

The FFT is project-authored, offline-only code; no new dependency or runtime was
added.

F0 uses normalized autocorrelation, the first sufficiently strong local peak, and
parabolic lag interpolation. A frame is voiced only when periodicity, RMS, and
zero-crossing checks agree. At least three paired voiced frames are required for a
pitch result. Otherwise the report uses `notEnoughVoicedFrames`; it never invents
zero Hz or a zero error.

## Metrics

Structural metrics:

- input/output sample rate and channels;
- input/output frame count;
- signed duration delta;
- reported DSP latency in frames and milliseconds;
- input sanitization policy.

Numerical metrics:

- input/output non-finite sample count;
- absolute peak;
- RMS;
- `20 * log10(output RMS / input RMS)`;
- arithmetic DC offset;
- fraction of samples at or above the 0.995 clipping threshold.

Pitch metrics:

- median input and output F0 over paired reliable voiced frames;
- `measuredPitchRatio = medianOutputF0 / medianInputF0`;
- declared expected ratio;
- `pitchErrorCents = 1200 * log2(measuredRatio / expectedRatio)`;
- paired voiced-frame count and coverage.

Voiced/unvoiced metrics:

- source and output voiced-frame ratios;
- mask disagreement ratio;
- voiced-to-unvoiced and unvoiced-to-voiced error counts.

Spectral metrics use aligned FFT frames. For each selected bin:

```text
LSD(frame) = sqrt(mean((20*log10(source + epsilon)
                       - 20*log10(output + epsilon))^2))
```

Reports contain mean and median LSD, source-voiced LSD, source-unvoiced LSD, and
3–10 kHz source-unvoiced LSD. Spectral distance is descriptive: lower is not
universally better when a transformation intentionally changes voiced spectra.

Consonant metrics use source-unvoiced frames:

- mean source and output 3–10 kHz energy;
- output/source high-frequency energy ratio;
- direct aligned waveform correlation for explicitly labeled unvoiced segments;
- unvoiced and high-frequency LSD from the spectral section.

Together, waveform correlation and high-frequency LSD provide stable comparisons
for preservation 0.0, 0.5, and 1.0. Raw energy ratio is also reported, but phase
interaction can make intermediate energy non-monotonic even when similarity
improves.

Formant-oriented metrics average voiced-frame spectra, smooth each envelope by
approximately 13 FFT bins, and locate the dominant peak inside each manifest band.
They report input/output peak frequency, measured ratio, expected
`2^(formantSemitones/12)` ratio, and error in cents. Ambiguous, silent, out-of-range,
or missing peaks are unavailable. These synthetic-fixture envelope peaks are not
clinical or physiological formant measurements. Arbitrary speech should treat them
as descriptive unless its bands are independently trustworthy.

Performance metrics:

- DSP render wall time;
- audio duration;
- `realTimeFactor = renderSeconds / audioSeconds`;
- processing milliseconds per audio second;
- debug or release build mode.

Analysis and report-writing time are excluded from RTF. Performance varies by
machine, load, toolchain, and build mode and is not a deterministic default gate.

## Expectations

Cases can declare:

- `maximumDurationDeltaFrames`;
- `maximumNonFiniteSamples`;
- `maximumOutputClippingRatio`;
- `maximumPitchErrorCents`;
- `maximumVoicedUnvoicedDisagreement`;
- `minimumVoicedFrameCoverage`;
- `maximumNeutralF0DriftCents`;
- `maximumNeutralRmsChangeDb`;
- `minimumFormantRatio`;
- `maximumFormantRatio`;
- `minimumUnvoicedHighFrequencyEnergyRatio`;
- `maximumUnvoicedLogSpectralDistanceDb`;
- `maximumRealTimeFactor`.

Thresholds must be finite and non-negative; ratios constrained to fractions cannot
exceed 1. Every result records metric, comparator, threshold, measured value,
pass/fail, and an unavailable explanation. If a required metric is unavailable,
the expectation fails.

The generated example's pitch tolerance is 45 cents, voiced-frame coverage is at
least 0.7, voiced/unvoiced disagreement is at most 0.2, clipping is at most 0.001,
duration delta and non-finite count are zero, and neutral F0/RMS tolerances are 35
cents/3 dB. These tolerate established deterministic Signalsmith behavior without
requiring waveform identity.

## Reports and baseline comparison

The output directory contains:

- `report.json`: schema version 1, tool version, timestamp, build mode, analysis
  configuration, case metrics, expectation results, summary, warnings, and optional
  baseline comparison;
- `cases.csv`: one deterministic ID-sorted row per case with important scalar
  metrics;
- `report.md`: expectation totals and a compact pitch, voicing, consonant,
  formant, numerical-safety, and performance table;
- `rendered/<case-id>.wav`, unless disabled.

A baseline must be a valid schema-v1 evaluator report with unique case IDs.
Comparison matches stable IDs and reports added/missing cases plus improvement,
regression, or unchanged classification for pitch error, voicing disagreement,
unvoiced LSD, high-frequency preservation error relative to 1.0, formant-ratio
error, clipping, non-finite output, and absolute duration delta. Timestamp,
filesystem paths, and timing are excluded. One deterministic comparison does not
establish statistical significance.

## Local real-speech cases and privacy

Place recordings in a user-controlled corpus and reference them with relative
manifest paths. Nothing is uploaded, downloaded, registered as a Dataset take, or
retained in application-managed storage. Only explicitly requested rendered WAVs
are copied into the selected output directory. Tags are organizational labels and
must not be treated as inferred identity or personal attributes.

A useful user-owned listening corpus includes:

- adult lower-F0 speech;
- adult higher-F0 speech;
- fricative-heavy and plosive-heavy phrases;
- breathy speech;
- vocal fry or creaky speech;
- quiet and noisy-room speech;
- emotional speech;
- a held vowel or sung note.

Do not add copyrighted third-party recordings to the repository.

## Limitations

F0 and voicing estimates can fail on vocal fry, strong breathiness, whispering,
irregular phonation, simultaneous voices, and noise. Synthetic results do not
establish naturalness, intelligibility, or listener preference. No PESQ, STOI,
MOSNet, neural quality predictor, or automatic Mean Opinion Score is included.
A controlled human listening protocol and real device/routing tests remain
required before product-quality claims.
