from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from ..audio import validate_wav
from ..errors import WorkerError
from .base import Backend, require_path


class SeedVcBackend(Backend):
    backend_id = "seed-vc-local"

    def validate(self, payload: dict[str, Any]) -> dict[str, Any]:
        root = require_path(payload.get("seedVcDirectory"), "Seed-VC directory", directory=True)
        require_path(str(root / "train.py"), "Seed-VC train.py")
        require_path(str(root / "inference.py"), "Seed-VC inference.py")
        require_path(str(root / "modules"), "Seed-VC modules", directory=True)
        require_path(payload.get("modelConfigurationPath"), "Model configuration")
        checkpoints = payload.get("pretrainedCheckpointPaths")
        if not isinstance(checkpoints, list) or not checkpoints or len(checkpoints) > 16:
            raise WorkerError("checkpointMissing", "Required pretrained checkpoints are not configured.")
        for checkpoint in checkpoints:
            require_path(checkpoint, "Pretrained checkpoint")
        output = Path(_required_string(payload, "outputDirectory"))
        output.mkdir(parents=True, exist_ok=True)
        probe = output / f".mam-worker-write-{os.getpid()}"
        try:
            probe.write_bytes(b"probe")
            probe.unlink()
        except OSError as error:
            raise WorkerError("outputNotWritable", "Configured output directory is not writable.") from error

        devices = ["cpu"]
        precisions = ["float32"]
        warnings: list[str] = []
        torch_version = "not-imported"
        if importlib.util.find_spec("torch") is not None:
            try:
                import torch  # type: ignore

                torch_version = str(torch.__version__)
                if torch.cuda.is_available():
                    devices.append("cuda")
                    precisions.extend(["float16", "bfloat16"])
            except Exception as error:  # third-party import boundary
                raise WorkerError("backendImportFailure", "PyTorch import failed in the configured environment.") from error
        else:
            warnings.append("PyTorch is not installed in the configured worker environment.")
        probe_environment = {
            key: value
            for key, value in os.environ.items()
            if key.upper() in {"SYSTEMROOT", "WINDIR", "PATH", "TEMP", "TMP"}
        }
        probe_environment.update(
            {
                "PYTHONNOUSERSITE": "1",
                "HF_HUB_OFFLINE": "1",
                "TRANSFORMERS_OFFLINE": "1",
            }
        )
        try:
            probe = subprocess.run(
                [
                    sys.executable,
                    "-c",
                    "import sys; sys.path.insert(0, sys.argv[1]); import modules.commons",
                    str(root),
                ],
                cwd=output,
                env=probe_environment,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=30,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise WorkerError("backendImportFailure", "Seed-VC import probe could not run.") from error
        if probe.returncode != 0:
            raise WorkerError("backendImportFailure", "Seed-VC modules failed to import in the configured environment.")
        if importlib.util.find_spec("torch_directml") is not None:
            devices.append("directMl")
        requested_device = payload.get("requestedDevice")
        requested_precision = payload.get("requestedPrecision")
        return {
            "backendId": self.backend_id,
            "backendVersion": f"configured-seed-vc;torch={torch_version}",
            "workerVersion": "0.1.0",
            "protocolVersion": 1,
            "devices": devices,
            "precisions": list(dict.fromkeys(precisions)),
            "supportsResume": True,
            "supportsMultipleReferences": False,
            "resources": {
                "systemMemoryBytes": None,
                "gpuMemoryBytes": None,
                "availableDiskBytes": shutil.disk_usage(output).free,
                "snapshotSizeBytes": None,
                "checkpointSizeBytes": sum(Path(item).stat().st_size for item in checkpoints),
                "riskLevel": "high" if requested_device == "cpu" else "unknown",
            },
            "warnings": warnings
            + (["Requested device is not available."] if requested_device not in devices else [])
            + (["Requested precision is not available."] if requested_precision not in precisions else []),
        }

    def train(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        root = require_path(payload.get("seedVcDirectory"), "Seed-VC directory", directory=True)
        config = require_path(payload.get("modelConfigurationPath"), "Model configuration")
        snapshot = require_path(payload.get("snapshotDirectory"), "Snapshot directory", directory=True)
        job = Path(_required_string(payload, "jobDirectory")).resolve()
        job.mkdir(parents=True, exist_ok=True)
        snapshot_manifest = require_path(str(snapshot / "snapshot.json"), "Snapshot manifest")
        _ = snapshot_manifest
        audio_dir = require_path(str(snapshot / "audio"), "Snapshot audio", directory=True)
        wavs = list(audio_dir.glob("*.wav"))
        if not wavs:
            raise WorkerError("emptySnapshot", "Snapshot contains no WAV files.")
        for wav in wavs:
            validate_wav(wav)
        training = payload.get("trainingConfiguration")
        if not isinstance(training, dict):
            raise WorkerError("invalidTrainingConfiguration", "Training configuration is missing.")
        maximum_steps = _bounded_int(training, "maximumSteps", 10, 100000)
        save_interval = _bounded_int(training, "saveInterval", 1, maximum_steps)
        batch_size = _bounded_int(training, "batchSize", 1, 64)
        worker_count = _bounded_int(training, "workerCount", 0, 16)
        run_name = job.name
        context.emitter.emit(request_id, "phaseStarted", {"phase": "preprocessing", "message": "Validated immutable snapshot audio."})
        context.emitter.emit(request_id, "progress", {"progress": 0.05, "step": 0})
        command = [
            sys.executable,
            str(root / "train.py"),
            "--config",
            str(config),
            "--dataset-dir",
            str(audio_dir),
            "--run-name",
            run_name,
            "--batch-size",
            str(batch_size),
            "--max-steps",
            str(maximum_steps),
            "--max-epochs",
            str(maximum_steps),
            "--save-every",
            str(save_interval),
            "--num-workers",
            str(worker_count),
        ]
        context.emitter.emit(request_id, "phaseStarted", {"phase": "training", "message": "Seed-VC fine-tuning started in the isolated worker."})
        started = time.monotonic()
        context.run_process(request_id, command, job)
        run_dir = job / "runs" / run_name
        checkpoint = run_dir / "ft_model.pth"
        if not checkpoint.is_file():
            candidates = sorted(run_dir.glob("*.pth"), key=lambda item: item.stat().st_mtime)
            if not candidates:
                raise WorkerError("artifactMissing", "Training exited without an expected checkpoint.")
            checkpoint = candidates[-1]
        copied_config = run_dir / config.name
        if not copied_config.exists():
            shutil.copy2(config, copied_config)
        artifact_files = [
            checkpoint.relative_to(job).as_posix(),
            copied_config.relative_to(job).as_posix(),
        ]
        context.emitter.emit(request_id, "checkpointSaved", {"relativePath": artifact_files[0], "message": "Final checkpoint saved."})
        context.emitter.emit(request_id, "progress", {"progress": 1.0, "step": maximum_steps})
        return {
            "backendVersion": "configured-seed-vc-v1",
            "artifactFiles": artifact_files,
            "trainingSummary": {
                "completedSteps": maximum_steps,
                "finalTrainingLoss": None,
                "finalValidationLoss": None,
                "checkpointCount": 1,
                "durationMs": int((time.monotonic() - started) * 1000),
                "warnings": ["Seed-VC console output was retained as logs and was not interpreted as metrics."],
            },
        }

    def infer(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        root = require_path(payload.get("seedVcDirectory"), "Seed-VC directory", directory=True)
        artifact = require_path(payload.get("artifactDirectory"), "Artifact directory", directory=True)
        source = require_path(payload.get("sourcePath"), "Source WAV")
        validate_wav(source)
        references = payload.get("referencePaths")
        if not isinstance(references, list) or not references:
            raise WorkerError("referenceMissing", "At least one target reference WAV is required.")
        reference = require_path(references[0], "Reference WAV")
        validate_wav(reference)
        output = Path(_required_string(payload, "outputPath")).resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        files = payload.get("modelFiles")
        if not isinstance(files, list) or not files:
            raise WorkerError("artifactMissing", "Artifact model files are missing.")
        resolved: list[Path] = []
        for item in files:
            if not isinstance(item, dict) or not isinstance(item.get("relativePath"), str):
                raise WorkerError("artifactInvalid", "Artifact file metadata is invalid.")
            relative = Path(item["relativePath"])
            if relative.is_absolute() or ".." in relative.parts:
                raise WorkerError("pathTraversal", "Artifact relative path is unsafe.")
            path = (artifact / relative).resolve()
            if artifact not in path.parents or not path.is_file():
                raise WorkerError("artifactMissing", "Artifact file is outside managed storage or missing.")
            resolved.append(path)
        checkpoint = next((item for item in resolved if item.suffix.lower() in (".pth", ".pt", ".safetensors")), None)
        config = next((item for item in resolved if item.suffix.lower() in (".yml", ".yaml")), None)
        if checkpoint is None or config is None:
            raise WorkerError("artifactInvalid", "Artifact requires a checkpoint and YAML configuration.")
        inference = payload.get("inferenceConfiguration")
        if not isinstance(inference, dict):
            raise WorkerError("invalidInferenceConfiguration", "Inference configuration is missing.")
        steps = _bounded_int(inference, "diffusionSteps", 1, 100)
        pitch = _bounded_int(inference, "pitchAdjustmentSemitones", -24, 24)
        length = inference.get("lengthAdjustment")
        if not isinstance(length, (int, float)) or not 0.5 <= float(length) <= 2.0:
            raise WorkerError("invalidInferenceConfiguration", "Length adjustment is invalid.")
        f0 = inference.get("f0Conditioning")
        if not isinstance(f0, bool):
            raise WorkerError("invalidInferenceConfiguration", "F0 conditioning must be boolean.")
        command = [
            sys.executable,
            str(root / "inference.py"),
            "--source",
            str(source),
            "--target",
            str(reference),
            "--output",
            str(output.parent),
            "--diffusion-steps",
            str(steps),
            "--length-adjust",
            str(float(length)),
            "--inference-cfg-rate",
            "0.7",
            "--f0-condition",
            str(f0),
            "--auto-f0-adjust",
            "False",
            "--semi-tone-shift",
            str(pitch),
            "--checkpoint",
            str(checkpoint),
            "--config",
            str(config),
            "--fp16",
            str(inference.get("precision") == "float16"),
        ]
        context.emitter.emit(request_id, "phaseStarted", {"phase": "inference", "message": "Offline synthetic conversion started."})
        context.emitter.emit(request_id, "progress", {"progress": 0.05})
        before = {item.resolve() for item in output.parent.glob("*.wav")}
        context.run_process(request_id, command, output.parent)
        if not output.is_file():
            created = [item for item in output.parent.glob("*.wav") if item.resolve() not in before]
            if len(created) != 1:
                raise WorkerError("unexpectedOutput", "Inference did not create exactly one expected WAV result.")
            shutil.move(str(created[0]), output)
        validate_wav(output)
        context.emitter.emit(request_id, "progress", {"progress": 1.0})
        return {"outputFile": output.name, "synthetic": True}


def _required_string(payload: dict[str, Any], key: str) -> str:
    value = payload.get(key)
    if not isinstance(value, str) or not value or len(value) > 2000:
        raise WorkerError("invalidConfiguration", f"{key} is required.")
    return value


def _bounded_int(payload: dict[str, Any], key: str, minimum: int, maximum: int) -> int:
    value = payload.get(key)
    if not isinstance(value, int) or isinstance(value, bool) or not minimum <= value <= maximum:
        raise WorkerError("invalidConfiguration", f"{key} is outside its supported bounds.")
    return value
