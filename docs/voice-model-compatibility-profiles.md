# Voice-model compatibility profiles

A compatibility profile is a narrow, versioned contract between Mam Voice Worker
and one explicitly listed backend revision. It is not a general claim that every
Seed-VC checkout, fork, checkpoint, Python version, PyTorch build, CUDA runtime, or
GPU is supported.

The built-in `seed-vc-experimental-v1` profile is intentionally experimental and
has no supported commit SHA until a real checkout is inspected and pinned. It
declares the current fixed adapter entry-point expectations (`train.py`,
`inference.py`, and `modules/commons.py`), required package roles, devices,
precisions, checkpoint roles, adapter version, and protocol version. Those script
and argument contracts remain unqualified until tested against an exact revision.

Profile support states are `unknown`, `experimental`, `candidate`, `qualified`,
`deprecated`, and `blocked`. A profile definition is not proof that an environment
is qualified. Qualification also requires the local repository identity, clean or
explicitly acknowledged dirty state, file/checkpoint hashes, worker and framework
checks, backend import, audio smoke testing, and any desired inference/listening
gates.

To pin a future profile safely:

1. Inspect the separately installed checkout without modifying it.
2. Record the canonical repository identity and exact 40-character commit SHA.
3. Verify every adapter-owned entry point and typed argument against that commit.
4. Record expected configuration and required checkpoint roles/hashes.
5. Run the deterministic worker/Rust tests and an opt-in real-backend qualification.
6. Record real CPU/CUDA, training, inference, cancellation, and listening evidence
   separately; do not infer it from compilation.

Profiles never run Git mutation, installers, package managers, or downloads. A
dirty checkout or unknown revision prevents reproducibility. Git unavailability is
reported without disabling non-ML application work.

