# Mam Voice Worker

This optional Python package is the isolated machine-learning boundary for Mam Voice Changer Phases 3-4. The Tauri process does not import Python, PyTorch, Seed-VC, or model files.

The package does not install Python, CUDA, PyTorch, Seed-VC, or checkpoints. It does not clone repositories or download weights. Prepare a dedicated local Python 3.10 environment manually, install the dependencies required by your chosen Seed-VC checkout yourself, and configure every path in Voice Lab → Models. Different GPU, driver, CUDA, PyTorch, and Seed-VC combinations are not guaranteed to work.

Run the protocol worker from this directory with:

```powershell
python -m mam_voice_worker
```

The app starts that command directly and communicates using versioned JSON Lines on stdin/stdout. Human/backend logs are wrapped as `log` events; stderr remains separate. Do not run the worker behind a shell wrapper that injects arbitrary arguments.

The experimental reference adapter owns fixed internal operations:
`inspect_seed_vc`, `preprocess_snapshot`, `fine_tune_seed_vc`,
`convert_with_seed_vc`, and `inspect_checkpoint`. It currently expects the
configured checkout to expose `train.py`, `inference.py`, and `modules/commons.py`
and supplies fixed typed arguments. Those assumptions have not yet been qualified
against a pinned commit; compilation or import is not a support claim. Training
runs with the managed job directory as its working directory so it does not create
`runs/` inside the configured checkout. Inference supplies explicit artifact,
checkpoint, configuration, reference, source, and output paths.

Phase 4 qualification reports only relevant package versions, Python/worker/device
identity, framework/backend/audio smoke results, and resource diagnostics. It never
dumps the complete environment. `MockQualificationBackend` exercises deterministic
handshake, fingerprint, preprocessing, progress/checkpoint, interruption/resume,
artifact, inference, and failure paths without ML dependencies or hardware.

The worker sets common offline flags and filters its child environment. No automatic
downloads are permitted. Configured third-party Python code may still be capable of
network access outside Mam Voice Changer's control; this is not a firewall or
sandbox.

Normal tests require no Seed-VC, PyTorch, GPU, checkpoint, network, or audio hardware:

```powershell
python -m unittest discover -s tests -v
```

Real adapter tests are intentionally opt-in and should use a disposable, manually prepared environment. No realtime conversion or communication-app routing exists in this worker.
