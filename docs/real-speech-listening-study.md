# Local real-speech listening study

## Purpose and boundary

`voice-listen` creates a local, blinded A/B package from user-owned WAV files. It
compares the existing deterministic Signalsmith DSP renderer with the
experimental offline WORLD reference renderer by reusing the evaluator's
existing rendering code.

Package preparation does not perform human listening, produce ratings, declare
a winner, or establish product readiness. Automated synthetic tests do not
replace real-speech ratings. WORLD remains evaluator-only and is not available
in Voice Lab, Use, Test, presets, routing, or the live audio engine.

The tool runs without Tauri startup, audio devices, Python, ML runtimes, network
access, telemetry, uploads, downloads, or application-managed storage.

## Corpus preparation

Use only recordings that you own or are authorized to evaluate. Keep the corpus
and package in a local, non-synchronized directory. Short, dry recordings around
3–10 seconds are recommended; the existing WAV importer enforces a 15-second
maximum.

A useful corpus can include:

- lower-F0 and higher-F0 conversational speech;
- fricative-heavy and plosive-heavy phrases;
- vowels with clear resonances;
- breathy, quiet, louder-below-clipping, emotional, or creaky speech;
- mild room noise;
- held vowels or sung notes.

These are coverage suggestions, not required demographic categories. Do not add
identity, age, sex, ethnicity, health, or accent labels. User-provided tags are
organizational labels only. The tool does not infer any speaker attribute.

Supported inputs follow the existing offline policy: PCM 16/24/32-bit or
32-bit-float WAV, mono or stereo, at 44.1 or 48 kHz. The tool performs no
automatic resampling.

## Manifest

The listening manifest is strict JSON schema version 1. Unknown fields,
unsupported versions, duplicate or unsafe IDs, empty clip lists, invalid
parameters, and unsafe paths are rejected.

```json
{
  "schemaVersion": 1,
  "corpusRoot": ".",
  "study": {
    "id": "world-vs-signalsmith-real-speech",
    "title": "Offline renderer comparison",
    "seed": 20260727
  },
  "clips": [
    {
      "id": "speaker-owned-phrase-01",
      "input": "speech/phrase-01.wav",
      "description": "Fricative-heavy phrase",
      "tags": ["fricative", "quiet-room"],
      "transform": {
        "pitchSemitones": 7.0,
        "formantShiftSemitones": 0.0,
        "consonantPreservation": 1.0,
        "dryWet": 1.0
      }
    }
  ]
}
```

`corpusRoot` and `input` must be normalized relative paths using `/`. Absolute
paths, backslashes, and traversal are rejected. Canonicalized inputs must remain
below the corpus root.

The four shown transformation fields are required. The manifest parser also
recognizes the remaining authoritative DSP fields so it can return a clear
error if a non-neutral parameter unsupported by WORLD is requested. It never
silently ignores a parameter. Use transformations supported by both renderers;
WORLD currently supports pitch, formant shift, consonant preservation, and
dry/wet while the remaining DSP controls must remain neutral.

## Prepare a blinded package

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-listen -- prepare `
  --manifest path/to/listening-manifest.json `
  --output path/to/listening-package `
  --seed 20260727
```

The CLI seed overrides `study.seed`. With the same seed, manifest, and unchanged
input files, trial order and A/B assignments are deterministic. Assignments are
balanced as closely as possible, clip order is shuffled, and the first-played
A/B label is randomized. Different seeds generally change assignment or order.
Repeated packages have identical assignments and audio hashes; measured render
RTF is timing-only and may vary.

Each source is imported once, then rendered independently by each existing
renderer with the same source, parameters, sample rate, channels, and requested
frame count. One renderer is never fed through the other.

Raw renderer outputs are retained under `administrator/raw-rendered`. If either
valid output exceeds the conservative 0.95 peak, one identical gain is applied
to both participant A/B copies, preserving their relative loudness. Full-scale
or non-finite output is rejected instead of hidden. Raw hashes, participant
hashes, and the common gain are recorded.

The output layout is:

```text
listening-package/
  participant/
    instructions.md
    ratings.csv
    trials.csv
    audio/
      trial-001-reference.wav
      trial-001-a.wav
      trial-001-b.wav
  administrator/
    key.json
    manifest-resolved.json
    render-metrics.csv
    hashes.json
    raw-rendered/
  package-summary.json
```

Participant filenames and documents contain no renderer identity, source path,
study description, tag, private username, absolute path, Git detail, or
implementation detail. Tags and descriptions remain administrator-side.

Do not share `administrator/key.json` or other administrator files with
listeners before ratings are complete.

## Listening and ratings

Use a safe listening volume and the same headphones, playback volume, room, and
device for every trial. For each trial:

1. Listen to the original reference.
2. Listen to A and B in the order listed in `trials.csv`.
3. Replay as needed.
4. Rate A and B independently.
5. Select A, B, or tie and provide confidence.

Each output receives an integer score from 1–7 for naturalness,
intelligibility, consonant clarity, pitch plausibility, vocal-character
plausibility, artifact absence, and overall quality. Confidence is 1–5.
Artifact flags are semicolon-separated values from:

`metallic`, `buzzy`, `phasey`, `muffled`, `harsh`, `unstable pitch`,
`distorted consonants`, `excessive breath/noise`, `timing artifact`, `other`.

Notes are optional and limited to 512 characters. Do not try to identify the
renderer or infer speaker attributes.

## Validate completed ratings

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-listen -- validate-ratings `
  --package path/to/listening-package `
  --ratings path/to/ratings.csv
```

Validation remains blinded. It verifies the exact header, one row for every
expected trial, no duplicates or unknown trials, complete 1–7 scores, valid
preference and confidence, known artifact flags, bounded notes, and absence of
renderer labels in participant materials. Errors identify the row and field.

## Unblind and summarize

Only after ratings are complete:

```powershell
cargo run --release --manifest-path src-tauri/Cargo.toml --bin voice-listen -- summarize `
  --package path/to/listening-package `
  --ratings path/to/ratings.csv `
  --output path/to/listening-results
```

This produces `summary.json`, `summary.csv`, `summary.md`, and
`trial-results.csv`. It reports per-renderer counts, means, medians,
wins/losses/ties, artifact counts, mean confidence, user-tag groups,
transformation groups, and paired `WORLD - existing DSP` score differences.
Incomplete ratings, fewer than three trials in a category, one-source
categories, and missing tags are explicitly flagged.

`administrator/render-metrics.csv` links every trial and renderer to existing
evaluator metrics where available. Pitch-analysis v2 records its estimator and
metric versions, paired source/output medians, median per-frame ratio and error,
absolute/P10/P90 errors, paired coverage, confidence exclusions, octave
ambiguity, and the shared-source-track fingerprint. The previous
normalized-autocorrelation error, source/output medians, and paired-frame count
remain available only in explicitly named `legacy_*` diagnostic columns. It
also includes formant error, V/UV disagreement, unvoiced HF LSD, waveform
correlation, clipping, non-finite samples, duration delta, and render RTF.
Real-speech formant error can remain
unavailable because the listening manifest does not invent synthetic formant
bands. Objective and subjective values are linked descriptively; the tool does
not claim correlation or causation.

## Privacy and interpretation

All processing stays local. Recordings and ratings are never uploaded,
downloaded, registered as Dataset data, used for training, backed up, or copied
to application-managed storage. Package outputs are created only in the
explicit output directory. Deleting a package does not delete the original
corpus.

The 1–7 scores are local study ratings, not standardized MOS. One listener is
anecdotal evidence; multiple clips and listeners are preferable. No
statistical significance is claimed. WORLD vocoder artifacts, corpus selection,
listener expectations, and playback setup remain limitations.

No Voice Lab integration decision should be made until actual human ratings
have been collected and reviewed. Even favorable ratings do not authorize live
Use/Test integration.
