# Backend qualification guide

Backend qualification is an explicit Models workflow. It does not install Python,
PyTorch, CUDA, Seed-VC, packages, configurations, or checkpoints and never changes
the configured checkout.

1. Select the experimental compatibility profile.
2. Select the Python executable, Mam Voice Worker directory, Seed-VC checkout,
   configuration, checkpoints, output directory, device, and precision.
3. Optionally enter expected SHA-256 identities. Missing expected hashes permit only
   experimental work and prevent reproducibility.
4. Save settings and run the worker-handshake check. This is not qualification.
5. Run layered qualification. The app inspects fixed read-only Git identity, hashes
   files, starts the versioned worker, inspects relevant packages/resources,
   exercises the configured framework/device, imports profile-declared backend
   modules, and runs a project-generated WAV preprocessing smoke test.
6. Review every check, warning, failure, resource estimate, revision, dirty state,
   package version, checkpoint/configuration hash, and aggregate environment
   fingerprint.
7. Copy or save the sanitized JSON/text report. Reports omit absolute private paths,
   environment variables, usernames, tokens, profile audio, and Dataset contents.
8. Run optional real inference only with local files and a deliberately selected
   consent-active reference. WAV validity is not audible-quality evidence.
9. Complete manual listening only after an inference smoke result was generated.
10. Re-run qualification after material Python, PyTorch, CUDA, backend revision,
    adapter, configuration, or checkpoint changes.

Qualification levels are `configurationValidated`, `backendLoaded`,
`inferenceGenerated`, `manuallyListened`, and `trainingCompleted`. Training and
listening are separate facts. A backend is never considered qualified merely
because the adapter compiles or the worker starts.

No automatic downloads are permitted. The configured third-party Python code may
still be capable of network access outside Mam Voice Changer's control. Offline
flags and a filtered child-process environment are defense in depth, not a network
firewall or sandbox.

