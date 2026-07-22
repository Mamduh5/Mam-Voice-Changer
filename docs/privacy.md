# Privacy and consent boundary

Voice Dataset Capture is local, explicit, and consent-gated. A profile cannot be
created until the user confirms that the target speaker consented to visible,
deliberate recording and private use of the profile. Consent metadata is a product
safeguard, not legal verification; the application does not infer consent from a
relationship or claim legal authorization.

The application does not record automatically, record while hidden, upload audio,
send prompt/profile telemetry, download models, install ML software, scrape audio,
share output, or submit a dataset. Optional Phase 3 training and conversion happen
only after explicit local actions, through a separately configured local child
process. Generated speech is synthetic and must not be represented as an authentic
recording of the target speaker.

Managed recordings, manifests, and consent metadata are stored as local plaintext
inside the application's `voice-datasets` application-data directory only after an
explicit create, record, or import action. An optional recorded-consent take is
identified separately and excluded from future training/export by default. The UI
shows managed storage size and provides take and full-profile deletion.

Deleting a profile revokes its consent inside application-managed storage and
removes raw audio, derived audio, manifest, and consent metadata where the operating
system permits. Partial deletion is reported and retryable. The product does not
claim cryptographic erasure. Explicit exported copies are outside application
management and must be deleted separately.

Every managed snapshot, training job, artifact, and inference result retains the
profile and consent version that authorized it. Training, resume, conversion, and
approval re-check active consent. Profile deletion requests cancellation for active
dependent work and marks managed artifacts `disabledByConsent`; disabled artifacts
cannot be selected for conversion. Managed model deletion never deletes Dataset
takes. Exported synthetic WAV/provenance or model copies outside application storage
cannot be revoked or deleted automatically.

The optional Seed-VC checkout, Python runtime, PyTorch packages, configurations, and
checkpoints are third-party/user-controlled code and files. Local execution is not
a sandbox and does not make them trusted. The worker receives only the managed
snapshot/job/artifact/source paths needed for the requested operation, starts fixed
entrypoints without a shell, filters its environment, and performs no automatic
network/download/install action. Users should review and isolate that environment.
