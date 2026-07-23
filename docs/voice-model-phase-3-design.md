# Voice Model Phase 3 design

> Phase 4 extends this design with compatibility profiles, environment/checkpoint
> identity, layered qualification, recovery indexes, preflight, and safe package
> portability. See `voice-model-phase-4-design.md`. Phase 3 adapter entry-point
> assumptions remain experimental until pinned and qualified.

## Scope and trust boundary

Phase 3 adds consent-dependent, local, offline model training and conversion. Model output is synthetic speech and must not be represented as an authentic recording of the target speaker. No realtime inference, live-route integration, communication-app routing, automatic downloads, package installation, checkpoint bundling, or cloud service is part of this phase.

Mam Voice Changer trusts its own versioned manifests and managed roots after validation. The configured Python executable, worker environment, Seed-VC checkout, checkpoints, configuration files, and generated model files are untrusted local inputs. “Local” does not make third-party ML code safe. The user explicitly selects these paths and is responsible for the external environment.

## Worker process boundary

Rust owns orchestration and persistence; Python owns ML imports, backend preprocessing, training, and inference. Rust starts Python directly, never through a shell, as `python -m mam_voice_worker`. The worker may start only the fixed, typed Seed-VC entry points selected by its adapter. It receives only the snapshot, job, reference, source, checkpoint, configuration, and output paths needed for the active request. It does not receive the application environment wholesale.

No model code runs in `AudioController`, `VoiceLabController`, `VoiceDatasetController`, a CPAL callback, the DSP worker, a Tauri command handler, React rendering, or the main Tauri event loop. Tauri commands make bounded controller requests; the controller thread owns worker lifetime and job state.

## Versioned backend protocol

Protocol version 1 is newline-delimited JSON over piped stdin/stdout. Every request has `protocolVersion`, `requestId`, `command`, and a typed `payload`. Every event has the matching version and request ID plus a typed `event` and payload. Supported commands are `hello`, `validateBackend`, `inspectCapabilities`, `preprocessSnapshot`, `startTraining`, `resumeTraining`, `cancelJob`, `inspectArtifact`, `runInference`, and `shutdown`. Supported events are `ready`, `capabilityReport`, `phaseStarted`, `progress`, `metric`, `checkpointSaved`, `warning`, `completed`, `cancelled`, `failed`, and `log`.

Protocol lines are bounded before JSON decoding. Unknown versions, commands, and events are rejected. Malformed stdout is a protocol error; stderr is captured separately and never interpreted as status. Request IDs correlate events. Log records and UI history are bounded. Process exit is not completion: a successful job requires an explicit `completed` event with typed output metadata.

## Backend configuration

`model-backends.json` is a separate schema-versioned app-data file. It stores typed Seed-VC settings only: Python executable, worker package directory, Seed-VC checkout, model configuration, required pretrained checkpoints, managed output directory, preferred device, and precision. It does not store process IDs, arbitrary arguments, tokens, environment dumps, or running jobs.

Readiness is one of `notConfigured`, `pythonMissing`, `workerMissing`, `backendMissing`, `checkpointMissing`, `configurationInvalid`, `protocolMismatch`, `unsupportedHardware`, or `ready`. Validation checks paths, direct Python startup, worker import and handshake, protocol compatibility, required Seed-VC files, configured checkpoints/configuration, writable output, and reported device/precision capabilities. It never starts training and never downloads anything.

The reference adapter targets the configured Seed-VC checkout's documented `train.py` and `inference.py` interfaces with fixed structured arguments. Seed-VC is optional and its archived upstream checkout is not vendored or modified. The adapter forces offline dependency behavior where supported and requires explicit checkpoint/config paths so upstream automatic checkpoint downloads are not relied upon.

## Immutable training snapshots

Snapshots live under `voice-models/snapshots/<snapshot-id>/`. Creation requires an existing healthy profile with active consent and uses `VoiceDatasetSource` to enumerate only accepted, non-excluded, non-consent takes. It validates the canonical raw master against the Dataset manifest hash, hashes the selected raw or trimmed canonical file independently, then copies it to `audio/<opaque-id>.wav`. Missing, mismatched, rejected, pending, excluded, or recorded-consent files abort or are excluded as specified; the Dataset is never modified.

`snapshot.json` records schema version, source profile and Dataset schema, consent version/time, selected take IDs, raw and selected hashes, prompt/category metadata, quality metadata, canonical format, duration, warnings, deterministic split membership, and a content hash over stable snapshot content. It is written into a temporary sibling directory, flushed, atomically renamed, and never mutated after completion.

The split is deterministic from snapshot ID and configured seed. It groups by prompt category where practical, prevents overlap, and reserves at least one validation take when size permits. Tiny datasets may run training-only with a prominent evaluation limitation. Counts, durations, and split seed are persisted and reused on resume.

## Training lifecycle and state machine

Typed configurations provide `quickExperiment`, `balancedFineTune`, and `extendedFineTune` defaults while keeping step count, save interval, batch size, worker count, device, precision, resume behavior, and random seed validated and editable. There is no arbitrary command field. Validation checks bounds, capabilities, disk/resource warnings, snapshot health, and active consent. More steps are not presented as guaranteed quality improvement.

Jobs use explicit states: `idle`, `validating`, `snapshotting`, `preparing`, `preprocessing`, `training`, `savingCheckpoint`, `evaluatingCheckpoint`, `cancelling`, `cancelled`, `completed`, `failed`, `interrupted`, and `needsRecovery`. Invalid transitions fail. A job manifest records backend/version, protocol, snapshot/hash/profile/consent provenance, exact configuration, progress, backend-reported metrics, timestamps, checkpoint, bounded logs, error summary, and cancellation state.

## Progress, cancellation, and crash recovery

The UI labels loss, learning rate, memory, and other metrics as backend-reported and does not equate lower loss with better similarity. Remaining time appears only after enough samples exist. Technical logs remain in managed job storage; the UI exposes a bounded sanitized tail.

Cancellation first sends `cancelJob`, waits a bounded grace period, then terminates the worker and, on Windows, its process tree where safely possible. Valid checkpoints and artifacts remain; incomplete temporary outputs are removed. No job remains marked running. Shutdown requires confirmation while training is active, then follows the same bounded cancellation and marks an unfinished job interrupted.

Startup changes abandoned running states to `interrupted` and never auto-resumes.
Artifact listing verifies schema/files/hashes and surfaces missing or invalid entries;
snapshot listing verifies content before exposing it. Explicit resume requires a
managed checkpoint, intact original snapshot/split/configuration, compatible backend,
and current consent. Broader index rebuilding, tombstone repair, orphan-checkpoint
classification, and completed-checkpoint artifact recovery remain future hardening.

## Model artifacts and consent dependency

Artifacts live under `voice-models/profiles/<profile-id>/artifacts/<artifact-id>/` with a schema-v1 `artifact.json`, relative model-file paths, hashes, backend and protocol versions, snapshot/consent provenance, exact training configuration and summary, evaluation, timestamps, and approval state. Future schemas are rejected without modification. Model files are never deserialized by Rust.

Approval states are `unevaluated`, `evaluationInProgress`, `approvedForOfflineUse`, `rejected`, `disabledByConsent`, `invalid`, and `missingFiles`. Active consent is rechecked before training, resume, inference, evaluation approval, selection, and export. Deleting the existing Dataset profile is the current consent-revocation action. Dependent jobs are stopped and managed artifacts are disabled before the Dataset is removed. Exported copies remain outside managed deletion.

## Offline inference lifecycle

Inference accepts the current Voice Lab original clip, which may be newly recorded or
explicitly imported, materialized as a managed temporary WAV. The controller assigns
an opaque source ID/path and never sends a sample array over IPC; the worker bounds
input format/duration and Rust validates decoded output format, duration, finiteness,
levels, and waveform. A separately persisted rich source-object manifest is not part
of this first implementation.

The approved, hash-valid artifact and active consent are checked before a dedicated worker job starts. Reference selection uses accepted non-excluded canonical Dataset takes. Automatic selection excludes manual overrides of failed quality and ranks pass quality, active duration, estimated SNR, clipping, and prompt diversity; explicit accepted-take selection is supported. Result provenance records reference take IDs and hashes.

The worker writes the generated WAV only to managed temporary model storage. Rust
validates/summarizes it and inserts that managed path into the existing processed
comparison slot. Original/model A-B preview and explicit WAV plus adjacent JSON
provenance export reuse Voice Lab ownership rules. The Models **Clear result** action
removes managed temporary output; Voice Lab Clear drops its in-memory comparison.
No live parameter, Use, Test, external-route, or CPAL path is changed.

## Evaluation and approval workflow

Evaluation uses project-authored source phrases plus optional user recordings; the target Dataset is not the sole source. It covers neutral, long, question, number, plosive, sibilant, sustained-vowel, and varied-pitch material where practical. Each result carries source/converted playback summaries, conversion/reference metadata, peak/clipping information, and manual 1–5 ratings for intelligibility, target similarity, naturalness, stability, and noise/artifacts plus notes.

Ratings are subjective descriptions, not biometrics. Approval requires at least one successful conversion, completed manual listening confirmation, active consent, valid hashes/files, and no fatal evaluation errors. Approval is explicit and means only “Approved for local offline conversion.” It never enables realtime conversion.

## Deletion, storage, and resource limits

Managed snapshot, terminal job/checkpoint, artifact/evaluation, and temporary-result
deletion validates opaque IDs and managed roots. Active jobs must be cancelled first;
a snapshot referenced by an artifact is blocked. Profile deletion requests dependent
cancellation and consent-disables artifacts. Model cleanup never deletes Dataset
takes or external exports. Dedicated deletion indexes/tombstones and a one-action
"all models for profile" cleanup remain future hardening.

Backend capability validation exposes configured device/precision, output location,
and worker-reported system/GPU/disk/checkpoint fields when available. Training records
the managed snapshot byte estimate and warnings cover CPU execution, batch/workers,
small datasets, and large step counts. These are non-guaranteed estimates. Mam Voice
Changer never changes Windows power, GPU, or virtual-memory settings.

## Privacy, security, and Windows behavior

Managed IDs—not display names—form paths. Relative manifest paths reject traversal. Canonicalization and managed-root checks occur before worker access. Rust invokes executables with structured arguments and does not use command-string concatenation. Worker messages, stderr, logs, source duration, output duration, and expected files are bounded. Unexpected output is rejected. Sensitive environment variables are not forwarded and absolute paths are sanitized from ordinary user-facing errors.

Windows paths are passed as individual `Command` arguments, so spaces and quoting are handled by the process API rather than shell escaping. Worker shutdown uses a graceful protocol request followed by bounded direct process termination; process-tree cleanup is a fixed platform operation, never frontend-provided command text. Tauri sidecars are not configured because the worker and Seed-VC environment are explicitly user-managed and optional.

## Known limitations and realtime exclusion

Seed-VC dependency, GPU, driver, precision, and checkpoint compatibility vary by the user's manually prepared environment. CPU training may be extremely slow. Dataset duration and training steps do not guarantee quality or similarity. The reference adapter supports only the documented configured entry points and cannot make arbitrary forks compatible. Manual listening, Windows hardware, real Seed-VC training, GPU, cancellation under every backend, and disk-full behavior require environment-specific acceptance.

Phase 3 contains no realtime model inference. Neural output is never routed to Discord, VB-CABLE, Use, Test, external routes, or live callbacks. Existing DSP Voice Lab remains the default and unchanged.
