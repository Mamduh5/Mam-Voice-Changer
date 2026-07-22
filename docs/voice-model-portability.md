# Voice-model portability

Mam Voice Changer exports a bounded ZIP model package containing project manifests,
hash-validated artifact files, evaluation and environment/checkpoint provenance,
synthetic-use notices, and licensing notices. Export excludes raw/trimmed Dataset
recordings, recorded consent audio, snapshot audio, temporary inference sources,
Python environments, third-party checkouts, environment variables, usernames,
tokens, and secrets. Pretrained checkpoints remain excluded by default.

Portability is factual:

- `localOnly`: the artifact is tied to its managed installation.
- `portableWithExternalDependencies`: the package requires separately configured
  checkpoints/profile/runtime.
- `portable`: every technically and legally includable artifact file is present,
  except the separately configured worker/runtime.
- `incompatible`: current profile or environment does not satisfy provenance.
- `unknown`: portability has not been established.

ZIP creation alone does not make a package portable. Backend source licenses,
checkpoint licenses, configuration licenses, adapter code, and user-trained output
are separate roles. When status is unknown the UI states: "Redistribution
permission has not been verified for this file." This is not legal advice.

Import treats every package as untrusted. The reader accepts only bounded,
unencrypted stored ZIP entries; rejects absolute/traversal paths, duplicate files,
unsupported flags/compression, excess file count or size, missing/unexpected
inventory entries, CRC/SHA-256 mismatch, and unsupported schemas; never executes or
deserializes model content in Rust; and installs atomically into managed storage.
The user must select a consent-active profile by opaque ID and explicitly confirm
the association. Original package provenance remains in the import index. Imported
artifacts are unevaluated, unapproved, and require local compatibility/profile/hash
validation before inference.

Consent revocation disables imported managed artifacts exactly like locally trained
artifacts. External package copies remain outside application management.

