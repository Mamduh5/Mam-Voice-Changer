# Voice Model Phase 4 design

## Scope

Phase 4 qualifies and hardens the optional local offline model backend introduced
in Phase 3. It does not add realtime neural conversion, streaming inference,
dependency installation, repository mutation, downloads, cloud work, model
sharing, or public publishing. Mam Voice Changer remains fully usable when no ML
backend is configured. Use, Test, VB-CABLE/external routing, the realtime DSP
worker, and CPAL callbacks remain outside this subsystem.

## Phase 3 inspection and adapter status

The Phase 3 Rust validator and Python adapter currently assume a checkout contains
`train.py`, `inference.py`, and `modules/commons.py`. The adapter constructs fixed
CLI flags for those scripts, but those names and arguments have not been verified
against an identified, pinned Seed-VC commit. Phase 4 therefore treats the existing
adapter contract as experimental and unqualified. A successful import or compile
is never presented as proof of backend qualification.

## Compatibility-profile model

`BackendCompatibilityProfileV1` is a strict, versioned adapter contract. It records
the profile/backend identities, repository identity, explicitly supported commit
SHAs, adapter and worker-protocol versions, Python and package requirements,
expected/configuration files, checkpoint roles, supported devices and precisions,
declared capabilities, support status, and licensing notices. Core fields are
typed rather than arbitrary JSON maps.

The built-in Seed-VC profile is `experimental`: its supported commit list is empty
until a real checkout is inspected and a revision is deliberately pinned. Empty or
unknown revision/checkpoint identity prevents a reproducible or highest-level
qualification. Test-only mock profiles are separate and cannot qualify a real
backend.

## Environment fingerprint

`ModelEnvironmentFingerprintV1` records only qualification-relevant facts:
operating system and architecture; Python implementation/version; worker adapter
and protocol versions; sanitized repository identity/revision/dirty state; selected
package versions; accelerator details; configuration/checkpoint SHA-256 identities;
and a deterministic aggregate hash. It excludes environment variables, usernames,
home directories, complete PATH values, tokens, shell history, credentials, and a
full package dump.

Fingerprints are persisted with qualification reports and copied into training-job,
artifact, and inference provenance. Comparison classifies environments as
`identical`, `compatible`, `changedWithWarning`, `incompatible`, or `unknown`.
Backend revision, adapter/protocol version, important packages, configuration
hashes, checkpoint hashes, device/precision support, and CUDA compatibility are
material. Volatile free disk/RAM measurements are warnings rather than identity.

## Qualification state machine and depth

Qualification is independent from training and follows explicit transitions through
`notStarted`, identity/file/checkpoint collection, worker/protocol/package/device
inspection, import/audio/inference smoke tests, evaluation, and terminal
`qualified`, `qualifiedWithWarnings`, `failed`, `cancelled`, or `interrupted`
states. Invalid transitions are rejected.

Qualification depth is not a boolean. Reports distinguish
`configurationValidated`, `backendLoaded`, `inferenceGenerated`,
`manuallyListened`, and `trainingCompleted`. Worker startup alone cannot exceed the
configuration layer. The highest audible level requires the explicit manual
listening checklist and timestamp; training completion remains a separate fact.

## Adapter entry-point strategy

Rust communicates only with `mam_voice_worker` using bounded JSON Lines and typed
commands. The worker owns fixed operations: `inspect_seed_vc`,
`preprocess_snapshot`, `fine_tune_seed_vc`, `convert_with_seed_vc`, and
`inspect_checkpoint`. Each compatibility profile selects an adapter implementation
and fixed files/arguments; the frontend never supplies a command, module, script,
or argument string. No Gradio interface or terminal-progress scraping is used.

The initial Seed-VC entry points remain disabled for qualified use until a pinned
revision is listed by the profile. Experimental execution, when explicitly
acknowledged, uses only fixed direct-process arguments and adapter-controlled result
files. `shell=True` and shell invocation are forbidden.

## Repository and checkpoint identity

Repository inspection uses fixed read-only direct Git operations only:
`rev-parse HEAD`, `status --porcelain`, and `remote get-url origin`. The checkout is
never modified. Remote URLs are stripped of user info, credentials, and query or
fragment data before storage/display. Missing Git leaves revision identity unknown
without breaking the rest of the application.

Every configuration and checkpoint role records a sanitized/relative path label,
size, SHA-256, optional expected hash, validation result, and check time. Hashing
runs outside realtime/UI threads with bounded progress and cancellation. Rust never
deserializes model contents. An unspecified expected hash permits only explicitly
experimental work and prevents reproducible qualification.

## Qualification smoke-test layers

1. Static checks validate configured paths/files, repository identity/cleanliness,
   hashes, writable output, and disk estimates.
2. Worker checks validate Python startup, package import, protocol/adapter versions,
   and declared capabilities.
3. Framework checks exercise CPU tensors and, only when selected, CUDA initialize,
   tensor, synchronize, device identity, and precision support.
4. Backend checks import only profile-declared modules, parse configuration, resolve
   checkpoint roles, and discover the model-construction path without downloads.
5. Audio checks use project-generated non-personal PCM WAV data, decode/resample it,
   validate finite bounded output, and clean temporary files.
6. Optional inference uses bounded project-owned speech plus an explicitly selected
   consent-active reference where necessary. WAV structure/level/duration are
   automatic checks; audible quality remains pending manual listening.

## Offline enforcement and trust boundary

The worker receives a filtered environment and known-library offline flags. All
declared checkpoint/configuration files must already exist. Adapter-owned output
directories are inventoried so unexpected files and attempted remote resolution
can fail qualification. This is defense in depth, not a network sandbox:

> No automatic downloads are permitted. The configured third-party Python code
> may still be capable of network access outside Mam Voice Changer's control.

The Python executable, installed packages, Seed-VC checkout, configurations, and
checkpoints are third-party local code/data and potentially untrusted. A
qualification run treats the checkout as immutable and never invokes Git mutation,
package managers, installers, or download tools.

## Resource reporting and training preflight

Resource diagnostics record logical CPUs, total/available physical memory, process
memory, free disk, snapshot/checkpoint sizes, estimated temporary space, CUDA/runtime
and GPU/VRAM details where reliable, and selected device/precision. Typed risk
levels (`low`, `moderate`, `high`, `unsupported`, `unknown`) use typed reason codes
such as CPU-only training, insufficient disk/RAM/VRAM, unknown VRAM, unsupported
precision, oversized batch/workers/steps, and tiny Dataset. Estimates never promise
that a run will fit.

The training preflight combines immutable snapshot counts/durations/bytes, profile
and fingerprint status, device/precision, typed configuration, estimated checkpoint
count/disk range, consent, qualification depth, fatal findings, and warnings.
Experimental profiles, unspecified checkpoint identity, dirty checkouts,
qualification warnings, tiny data, CPU-only selection, and close disk margins
require explicit acknowledgement. Fatal findings keep Start disabled.

## Persistent indexes, recovery, and repair

Versioned indexes cover qualification runs, snapshots, jobs, artifacts, temporary
inference results, and imported packages. They contain opaque IDs and managed
relative paths, are written atomically, reject duplicates/unsupported schemas, and
can be rebuilt from authoritative valid manifests.

Startup marks abandoned qualification, training, and inference work interrupted;
detects incomplete temporary directories, partial imports/exports, hashing files,
orphans, missing/unexpected files, hash mismatch, unsupported schemas, and deletion
tombstones; and preserves recoverable content. Repair is explicit.

Resume never occurs automatically. A checkpoint must be declared by the job,
hash-valid, structurally accepted by the fixed adapter, supported by the profile,
and tied to the original snapshot/split/configuration/seed/profile/fingerprint.
Material environment changes block resume unless the adapter explicitly supports
the change and the user acknowledges it.

## Artifact hardening and portability

The Phase 4 artifact schema records compatibility profile, environment fingerprint,
backend revision, adapter version, typed file roles and hashes, checkpoint
identities, expected sample rate/controls, qualification level, portability, license
notices, and the synthetic-use notice. Imported or migrated artifacts are never
automatically approved.

Export creates a bounded project-defined ZIP package containing manifests,
artifact files, evaluation, qualification/provenance, README, and license notices.
It excludes Dataset/consent/snapshot audio, temporary sources, environments,
checkouts, absolute paths, usernames, secrets, and pretrained checkpoints unless
redistribution is separately verified and explicitly selected. Successful ZIP
creation does not imply portability.

Import extracts only into a temporary managed directory; rejects absolute and
traversal paths, links/reparse points, unsupported special files, excess file count
or uncompressed size, unsupported schemas, missing/extra files, and hash mismatch;
then atomically installs an unapproved artifact. Package content is never executed
or deserialized in Rust. A consent-active profile is selected by opaque ID, original
provenance is retained, and local dependencies/hashes are revalidated before use.

Portability is `localOnly`, `portableWithExternalDependencies`, `portable`,
`incompatible`, or `unknown`. `portable` requires all legally and technically
required artifact files except the separately configured worker/runtime.

## Licensing boundary

Manifests distinguish Mam Voice Changer code, adapter code, third-party backend
code, user-trained artifacts, base/auxiliary checkpoints, and configurations.
Backend-source and weight licenses are not assumed to be the same. Unknown status
is factual: **Redistribution permission has not been verified for this file.**
Export requires acknowledgement for unknown licensing but does not provide legal
advice.

## Qualification reports

Versioned JSON and human-readable reports include application/qualification/profile
identity, sanitized revision and dirty state, worker/protocol/adapter versions,
environment/package/device/checkpoint/configuration fingerprints, layered checks,
smoke results, resources, warnings/failures, manual-listening state, and final depth.
Copy/save surfaces exclude private absolute paths, environment variables, tokens,
usernames, profile audio, and raw Dataset content.

## Manual qualification gate

Automated validation can prove deterministic protocol, file, framework, and WAV
properties only. Real Seed-VC, PyTorch/CUDA, training, cancellation behavior,
subjective listening, cross-machine portability, and third-party compatibility are
reported pending until performed. Manual listening separately records playback,
intelligibility, clipping, truncation, source/target mix-up, synthetic-label review,
notes, and confirmation time. It is not biometric verification and does not claim
perfect similarity.

## Known limitations

- No verified Seed-VC commit is bundled in the initial experimental profile.
- Offline flags are not a firewall or operating-system sandbox.
- Resource estimates cannot guarantee successful training.
- Rust validates hashes and containers but never validates model semantics by
  deserializing checkpoints.
- Compatibility profiles are intentionally narrow and do not support arbitrary
  Seed-VC forks.
- Real backend, CUDA, hardware, listening, and another-machine portability tests
  remain manual and opt-in.

## Realtime exclusion

Phase 4 does not add a neural option to Use, Test, VB-CABLE/external routing,
Discord/communication applications, the realtime DSP worker, or CPAL callbacks.
Model training and conversion remain explicit offline actions in Voice Lab Models.
