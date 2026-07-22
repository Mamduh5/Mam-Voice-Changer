# Privacy and consent boundary

Voice Dataset Capture is local, explicit, and consent-gated. A profile cannot be
created until the user confirms that the target speaker consented to visible,
deliberate recording and private use of the profile. Consent metadata is a product
safeguard, not legal verification; the application does not infer consent from a
relationship or claim legal authorization.

The application does not record automatically, record while hidden, upload audio,
send prompt/profile telemetry, train a model, download a model, create embeddings,
run neural conversion, or submit a dataset. Dataset collection does not create a
cloned voice and must not be described as capable of reproducing the speaker. A
future generated voice must not be represented as an authentic recording of the
target speaker.

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
