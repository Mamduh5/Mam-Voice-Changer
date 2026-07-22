# Local voice-model backend setup

Mam Voice Changer does not bundle or install Python, PyTorch, CUDA, Seed-VC,
configuration files, or model checkpoints. Model training and conversion are
optional; every other application workspace continues to function when this setup
is absent.

## Trust warning

Python packages, model checkpoints, configurations, and the configured Seed-VC
checkout are third-party local code/data. Local execution is not a sandbox and does
not make them safe. Review the exact sources and licenses, prefer an isolated Python
environment, pin a revision you have assessed, keep backups, and do not place
secrets in its environment. The official Seed-VC repository is archived, so future
runtime/dependency compatibility cannot be assumed.

## Manual preparation

1. Obtain Python and create a dedicated environment yourself. Python 3.10 is the
   conservative target documented by this worker; the app does not install it.
2. Obtain and review a Seed-VC checkout yourself. Do not place it inside Mam Voice
   Changer's managed `voice-models` storage.
3. Follow that pinned checkout's own dependency instructions manually. GPU/CUDA,
   DirectML, PyTorch, driver, and precision combinations are environment-specific.
4. Obtain configuration and all required pretrained checkpoints yourself, review
   their licenses, and keep them at explicit stable paths. The app never downloads
   missing weights.
5. From this repository's `ml-worker` directory, verify the protocol package with:

   ```powershell
   python -m mam_voice_worker
   ```

   It waits for JSON Lines input; use Ctrl+C when running it by hand.
6. Open **Voice Lab → Models → Configure local model backend** and select, through
   the path dialogs, the Python executable, this repository's `ml-worker`
   directory, Seed-VC checkout, model configuration, every checkpoint, and an
   output directory.
7. Select CPU or CUDA and a precision actually supported by that environment, save,
   then choose **Check worker handshake**. A handshake is not qualification.
8. Run **Backend Qualification** and review the repository revision/dirty state,
   relevant packages, configuration/checkpoint SHA-256 values, CPU/CUDA resources,
   layered smoke checks, warnings, failures, and environment fingerprint.
9. Save the sanitized report. Expected checkpoint hashes that remain unspecified
   prevent a reproducible result and require explicit experimental acknowledgement.

Readiness distinguishes missing Python, worker, backend, checkpoint, invalid
configuration, protocol mismatch, unsupported hardware, and ready states. Do not
start training until the capability report matches the selected device/precision.
The training panel additionally requires a completed backend-load qualification and
an explicit preflight review. Experimental profile, dirty checkout, unknown hashes,
warnings, tiny Dataset, CPU-only selection, and tight disk estimates require
acknowledgement; fatal findings keep Start disabled.

The initial profile supports no verified Seed-VC commit and is deliberately marked
experimental. Its `train.py`/`inference.py` argument contract is fixed and typed but
remains unqualified until inspected and tested against an exact pinned revision.

## Runtime behavior

Rust starts the selected Python executable directly with `-m mam_voice_worker`; no
shell or frontend-provided argument string is used. The worker then calls only the
adapter's fixed `train.py` or `inference.py` interface. It sets common dependency
offline flags and supplies explicit configuration/checkpoint/output paths. This is
defense in depth, not a guarantee that arbitrary third-party code cannot access the
network or machine.

No automatic downloads are permitted. The configured third-party Python code may
still be capable of network access outside Mam Voice Changer's control. These flags
and the filtered process environment are not a firewall or sandbox.

CPU fine-tuning may be extremely slow. Reported disk/RAM/VRAM values are estimates,
not a promise that a run will fit. Begin with **Quick experiment**, keep unrelated
work closed, and preserve checkpoints before testing cancellation/recovery.

## Deliberate exclusions

There is no automatic clone, package install, weight download, cloud training,
realtime inference, Discord/VB-CABLE output, or neural Use/Test route. A validated
model remains synthetic and must complete manual evaluation before offline approval.
