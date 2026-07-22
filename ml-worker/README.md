# Mam Voice Worker

This optional Python package is the isolated machine-learning boundary for Mam Voice Changer Phase 3. The Tauri process does not import Python, PyTorch, Seed-VC, or model files.

The package does not install Python, CUDA, PyTorch, Seed-VC, or checkpoints. It does not clone repositories or download weights. Prepare a dedicated local Python 3.10 environment manually, install the dependencies required by your chosen Seed-VC checkout yourself, and configure every path in Voice Lab → Models. Different GPU, driver, CUDA, PyTorch, and Seed-VC combinations are not guaranteed to work.

Run the protocol worker from this directory with:

```powershell
python -m mam_voice_worker
```

The app starts that command directly and communicates using versioned JSON Lines on stdin/stdout. Human/backend logs are wrapped as `log` events; stderr remains separate. Do not run the worker behind a shell wrapper that injects arbitrary arguments.

The reference adapter expects the configured checkout to contain the archived upstream Seed-VC v1 entry points `train.py` and `inference.py`. It supplies fixed typed arguments corresponding to those documented scripts. Training runs with the managed job directory as its working directory so it does not create `runs/` inside the configured Seed-VC checkout. Inference always supplies explicit artifact checkpoint and configuration paths. The worker sets common Hugging Face/Transformers offline flags, but third-party code remains untrusted and may have behavior outside this package's control.

Normal tests require no Seed-VC, PyTorch, GPU, checkpoint, network, or audio hardware:

```powershell
python -m unittest discover -s tests -v
```

Real adapter tests are intentionally opt-in and should use a disposable, manually prepared environment. No realtime conversion or communication-app routing exists in this worker.

