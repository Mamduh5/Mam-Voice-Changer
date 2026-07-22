# Voice Dataset Capture Phase 2 design

## Scope and product boundary

Voice Dataset Capture collects quality-controlled, consented recordings for a possible future local trainer. It does not clone a voice, train or run a model, create speaker embeddings, download models, upload audio, or claim that a dataset can reproduce a speaker. A future generated voice must not be represented as an authentic recording of the target speaker.

Consent metadata is a product safeguard, not legal verification. The target speaker must consent; recording is visible and deliberate; managed data stays local until explicit export; and consent can be revoked by deleting the profile. Exported copies are outside application management and must be deleted separately.

## Profile ownership

`VoiceDatasetController` and `DatasetStorage` own persistent dataset operations. `AudioController`, `VoiceLabController`, DSP processors, CPAL callbacks, React components, and application settings do not own profile files. The controller keeps only operation state, the active profile/prompt, an in-progress bounded capture, and one active preview. Manifests and WAV files are loaded through storage as needed rather than retained permanently in memory.

Voice Lab comparison remains a temporary, memory-only system. Dataset files never become Voice Lab session clips unless a user explicitly imports an exported WAV into Voice Lab.

## Consent model

A profile creation request includes a checked explicit-consent confirmation, consent version, confirmation time, and `confirmedByUser`. Creation fails without confirmation. The notice states that the speaker consents to visible recording and private local collection, collection does not clone a voice, export is explicit, and profile deletion revokes consent inside managed storage.

Consent is stored in `consent/consent.json`, separate from take quality. An optional recorded-consent take uses source `recordedConsent`, is excluded from future training by default, is not required, and can be deleted separately. Consent cannot be unchecked while accepted takes exist. Revocation uses the explicit full-profile deletion flow; it does not claim legal verification.

## Dataset directory structure

The application-data root contains:

```text
voice-datasets/
  profiles.json
  <opaque-profile-id>/
    manifest.json
    consent/
      consent.json
      recorded-consent.wav        # optional
    raw/
      <opaque-take-id>.wav
    derived/
      <opaque-take-id>-trimmed.wav
    deletion.json                 # only during interrupted deletion
```

Exports are written only to a user-selected external directory. Profile names, prompt text, and imported filenames never form managed paths. Manifest paths are normalized relative paths containing only generated identifiers. The storage layer rejects absolute paths, parent traversal, Windows path prefixes, separators in identifiers, duplicate IDs, and paths that escape the profile root after canonicalization. Managed directories are not followed through symlinks/reparse points when validating existing files.

## Versioned schemas

`profiles.json` and each `manifest.json` use schema version 1, independent of application-settings schema v4. A manifest contains profile metadata, consent summary, canonical recording format, prompt-pack reference, take metadata, rebuilt statistics, and millisecond Unix timestamps. Raw samples are never serialized into JSON.

Each take records its opaque ID, optional prompt ID/text, prompt category, source, relative raw/derived file, selected version, original import format when applicable, canonical sample rate/channels/frame count/duration, waveform envelope, deterministic quality report, review state, exclusion/notes/override metadata, capture metrics, creation time, and SHA-256 content hash. Future schema versions are reported as unsupported without modifying source files.

## Recording lifecycle

Recording is an explicit foreground action:

```text
selected physical microphone
  -> CPAL input callback
  -> bounded preallocated ring
  -> dataset capture worker
  -> bounded mono f32 take buffer
  -> stop/20-second automatic finalization
  -> canonical PCM24 WAV + manifest transaction
```

The callback allocates no memory, accesses no files, logs nothing, performs no DSP, emits no frontend block events, and never blocks. It only converts the current frame to dry finite mono, records atomics for level/gaps/non-finite/drop/overflow, pushes to the ring, and sends a non-blocking wake. The worker owns the preallocated bounded take buffer. The minimum prompted take is one second and `DATASET_MAX_TAKE_SECONDS` is a separate 20-second Rust constant; Voice Lab keeps its 15-second limit.

The timer, meter, clipping state, remaining duration, and finalization state are polled as a compact status DTO. Closing Dataset stops and discards only an unfinished buffer; already finalized takes remain persistent.

## Take lifecycle

Recorded or imported audio is canonicalized, persisted, analyzed, and added as `pending`. Automatic classification never accepts a take. Review explicitly chooses accept, reject, redo, exclusion, notes, and raw or trimmed selection. Accepting a failed take requires warning acknowledgement and records a manual override. Redo never overwrites a prior take; the user explicitly chooses whether to retain or delete it.

Status/statistics changes are manifest transactions. Rejected audio remains until explicit deletion. Recorded-consent audio is separate and excluded by default.

## Quality measurements and classification

`analyze_take(samples, sample_rate, capture_metrics)` is a deterministic offline function. Objective measurements include duration, peak, RMS, clipped count/ratio, DC offset, leading/trailing silence, low-energy ratio, consecutive-zero/dropout regions, overflow/dropped frames, non-finite input count, sample rate, and channels. Heuristics are explicitly labeled estimates: active-speech ratio, background-noise floor, and signal-to-noise ratio.

Central conservative thresholds produce `pass`, `warning`, or `fail` plus stable reason codes and guidance. Reasons cover too short/long, clipping, low/high level, excessive edge/total silence, low estimated SNR, possible dropout, capture overflow, non-finite input, unsupported format, and manual review. Classification never claims studio quality, identity, transcript/phoneme accuracy, model readiness, or guaranteed future quality.

## Non-destructive trimming

Auto trim searches only the leading and trailing low-energy regions with conservative padding. Manual start/end boundaries are frame indices and must leave a valid non-empty interval. Applying trim reads the raw canonical WAV, writes a separate derived PCM24 WAV, recalculates waveform and quality, and atomically selects the derived version. Internal pauses are untouched. Reset selects raw and removes the managed derived file only after the manifest can no longer reference it.

## WAV import

The frontend supplies one explicit dialog selection or a bounded batch. Import accepts uncompressed PCM 16/24/32-bit and IEEE float32 mono/stereo WAV at 44.1 or 48 kHz. It rejects empty, compressed, excessive-channel/rate, or over-limit files. Samples are counted for non-finite values, sanitized, safely mixed to mono, linearly resampled offline to 48 kHz, and written once as canonical PCM24 without gain normalization, denoising, compression, pitch/formant changes, or source modification. SHA-256 detects exact duplicate canonical content. Imported takes remain pending and do not infer prompts from filenames.

## Export behavior

Export creates a versioned directory package at a user-selected destination. By default it contains an export manifest, consent metadata without recorded-consent audio, prompt metadata, quality reports, accepted non-excluded selected WAV files, and a README. Rejected/pending/consent audio, application settings, presets, usernames, absolute paths, and external routing details are excluded. Advanced options may include rejected takes or raw masters. Export is local and explicit; cancellation/failure removes only the incomplete package created for that operation.

## Deletion behavior and recovery

Take deletion first commits a manifest tombstone/removes active references, then deletes derived and raw files, then commits the final manifest/statistics. Partial failures remain visible and retryable without a manifest pointing at a successfully deleted file. Profile deletion stops its capture/preview, removes it from the selectable index, writes a deletion tombstone, deletes managed files, clears controller state, and reports any remaining managed identifiers. It is idempotent where practical and never touches exports.

Startup recovers valid `.bak`/`.tmp` atomic writes conservatively, validates the index and manifests, and reports `healthy`, `needsRepair`, `missingFiles`, `orphanedFiles`, `unsupportedSchema`, or `corruptManifest`. It does not silently delete recoverable data. Repair can restore a backup, rebuild statistics, remove a confirmed missing derived reference, and report orphans; it never guesses consent or accepts takes.

## Crash-safety strategy

JSON changes use same-directory `.tmp` files, flush and `sync_all`, rename the prior valid target to `.bak`, replace the target, and remove the backup only after success. Recovery prefers a valid target, otherwise a valid backup, and treats a lone temporary file as recoverable state rather than silently overwriting data. WAV files are finalized before their manifest reference is committed. Failed creation removes the unreferenced new WAV when safe; otherwise startup reports it as orphaned.

## Audio ownership

The existing backend ownership boundary is extended so exactly one hardware operation owns CPAL streams: Use, Test, Voice Lab capture/preview, or Dataset capture/preview. Every start command takes the shared operation lock and checks live engine, Voice Lab, and Dataset active flags. Dataset navigation cleanup stops its own streams. Frontend disabled buttons are advisory only; typed backend `audioOperationAlreadyActive` errors enforce the rule.

## Privacy boundary

There are no network calls, uploads, telemetry, hidden copies, background recording, automatic exports, model downloads, or training. Profile and prompt text stay in local manifests and explicit exports. The UI shows managed storage size and the delete action from the main Dataset page. Deletion does not claim cryptographic erasure.

## Future trainer interface

`VoiceDatasetSource` is a read-only iterator-based insertion point. It returns profile metadata, rebuilt statistics, and only accepted, non-excluded canonical selected takes by default, including prompt and quality metadata. It exposes file descriptors/paths one take at a time, not mutable storage internals or complete decoded datasets. Phase 2 includes no trainer.

## Known limitations

- Quality and SNR values are deterministic heuristics, not speech recognition or acoustic certification.
- Linear 44.1-to-48 kHz resampling is isolated and suitable for Phase 2 ingestion, but a future trainer may justify a higher-order offline resampler.
- CPAL 0.15 exposes a stable friendly-name fingerprint rather than the Windows endpoint GUID; ambiguous restored devices require reselection.
- Managed files are local plaintext. The product does not add custom encryption or claim secure erasure.
- Automated tests use generated audio and no hardware. Microphone behavior, audible playback, device removal, partial filesystem failures, and long-session comfort require the manual acceptance plan.
