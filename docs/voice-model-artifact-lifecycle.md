# Voice-model artifact lifecycle

```text
accepted consent-active Dataset
  -> immutable copied snapshot + deterministic split
  -> explicit isolated local training job
  -> schema-v1 hash-validated synthetic model artifact
  -> offline synthetic evaluation clips
  -> manual ratings and listening confirmation
  -> explicit approval for local offline Voice Lab use
```

Snapshots freeze profile, Dataset schema, prompt/quality data, selected take hashes,
consent version/time, copied canonical audio, duration, and split membership. They
never contain rejected, pending, excluded, or recorded-consent takes and never
mutate the Dataset.

Jobs persist the exact backend/protocol versions, snapshot/hash, consent provenance,
typed configuration, state, progress, backend-reported metrics, checkpoint, bounded
log path, and terminal/error information. Startup marks abandoned running states
`interrupted`; resume is explicit and requires an existing checkpoint, compatible
backend capability, the same intact snapshot, and still-active consent.

Artifacts store only relative model-file paths plus file and aggregate hashes.
Rust never deserializes model pickle/checkpoint contents. New artifacts are
`unevaluated`. Approval requires a successful offline conversion, completed manual
ratings/listening confirmation, active matching consent, valid files/hashes, and an
explicit approval action. Approval means **approved for local offline conversion**,
not verified identity, objective similarity, or realtime readiness.

Revoking/deleting the Dataset profile requests cancellation of active dependent
model work and marks managed artifacts `disabledByConsent`. Disabled, rejected,
invalid, missing-file, or unevaluated artifacts cannot run ordinary offline
conversion. Externally exported audio/model copies cannot be found or deleted by
the application.

Deleting an artifact removes its managed artifact directory, evaluation data, and
previews but never Dataset audio. A snapshot referenced by an artifact is blocked
from deletion. Active jobs must be cancelled before job deletion. Temporary
inference output is cleared explicitly; Voice Lab Clear drops the loaded in-memory
comparison. Synthetic WAV export uses a `synthetic` filename and writes adjacent
JSON provenance with no absolute machine paths.

Phase 4 adds explicit bounded model-package export/import. Packages contain artifact,
evaluation, qualification/environment/checkpoint provenance, synthetic-use, hash
inventory, README, and licensing notices. They exclude Dataset/consent/snapshot
audio, temporary sources, environments, checkouts, absolute paths, usernames,
secrets, and pretrained checkpoints by default. Portability is reported as
`localOnly`, `portableWithExternalDependencies`, `portable`, `incompatible`, or
`unknown`; ZIP creation alone never proves portability.

Imports reject unsafe paths, duplicates, unsupported archive features, file-count
and size overflow, unsupported schemas, missing/unexpected files, and CRC/SHA-256
mismatch. Rust never executes or deserializes model files. The user explicitly
selects a consent-active profile by opaque ID. Imported artifacts retain original
package provenance, remain unevaluated/unapproved, and require local backend,
environment, dependency, hash, evaluation, and consent validation before inference.
