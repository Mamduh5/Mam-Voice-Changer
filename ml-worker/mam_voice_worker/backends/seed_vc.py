from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
import sys
import time
import platform
import tempfile
import wave
import struct
import hashlib
from importlib import metadata
from pathlib import Path
from typing import Any

from ..audio import validate_wav
from ..errors import WorkerError
from ..protocol import PROTOCOL_VERSION, WORKER_VERSION
from .base import Backend, require_path


class SeedVcBackend(Backend):
    backend_id = "seed-vc-local"

    adapter_version = "mam-seed-vc-adapter-v2-experimental"

    def validate(self, payload: dict[str, Any]) -> dict[str, Any]:
        return self.inspect_seed_vc(payload)

    def inspect_seed_vc(self, payload: dict[str, Any]) -> dict[str, Any]:
        root = require_path(payload.get("seedVcDirectory"), "Seed-VC directory", directory=True)
        require_path(str(root / "train.py"), "Seed-VC train.py")
        require_path(str(root / "inference.py"), "Seed-VC inference.py")
        require_path(str(root / "modules"), "Seed-VC modules", directory=True)
        configuration = require_path(payload.get("modelConfigurationPath"), "Model configuration")
        _validate_configuration_text(configuration)
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
            "workerVersion": WORKER_VERSION,
            "protocolVersion": PROTOCOL_VERSION,
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
        return self.fine_tune_seed_vc(request_id, payload, context)

    def fine_tune_seed_vc(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
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
        return self.convert_with_seed_vc(request_id, payload, context)

    def convert_with_seed_vc(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
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

    def preprocess_snapshot(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        snapshot = require_path(payload.get("snapshotDirectory"), "Snapshot directory", directory=True)
        audio_dir = require_path(str(snapshot / "audio"), "Snapshot audio", directory=True)
        files = sorted(audio_dir.glob("*.wav"))
        if not files:
            raise WorkerError("emptySnapshot", "Snapshot contains no WAV files.")
        for index, path in enumerate(files):
            if context.cancel.is_set():
                raise WorkerError("cancelled", "Snapshot preprocessing was cancelled.")
            validate_wav(path)
            context.emitter.emit(
                request_id,
                "progress",
                {"progress": (index + 1) / len(files), "step": index + 1},
            )
        return {"validated": True, "takeCount": len(files)}

    def inspect_checkpoint(self, _request_id: str, payload: dict[str, Any], _context: Any) -> dict[str, Any]:
        checkpoint = require_path(payload.get("checkpointPath"), "Checkpoint")
        if checkpoint.suffix.lower() not in (".pth", ".pt", ".safetensors"):
            raise WorkerError("checkpointUnsupported", "Checkpoint file type is unsupported.")
        if checkpoint.stat().st_size <= 0:
            raise WorkerError("checkpointInvalid", "Checkpoint file is empty.")
        expected = payload.get("expectedSha256")
        content_hash = _sha256_file(checkpoint)
        if expected is not None and (not isinstance(expected, str) or expected.lower() != content_hash):
            raise WorkerError("checkpointMismatch", "Checkpoint SHA-256 does not match the expected identity.")
        return {
            "structurallyUsable": True,
            "fileName": checkpoint.name,
            "sizeBytes": checkpoint.stat().st_size,
            "deserialized": False,
            "sha256": content_hash,
        }

    def qualify(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        checks: list[dict[str, Any]] = []
        packages = self._inspect_packages(payload, checks)
        context.emitter.emit(request_id, "packageReport", {"packages": packages})
        capability = self.inspect_seed_vc(payload)
        accelerator, framework_checks = self._inspect_framework(payload)
        checks.extend(framework_checks)
        context.emitter.emit(request_id, "acceleratorReport", accelerator)
        checks.append(_check("backendImport", "Seed-VC adapter import", "backendImport", "passed", "Profile-declared backend modules imported without downloads."))
        context.emitter.emit(request_id, "backendImportReport", {"status": "passed"})
        audio_check = self._audio_smoke_test(context)
        checks.append(audio_check)
        context.emitter.emit(request_id, "audioSmokeReport", {"status": audio_check["status"]})
        if payload.get("runInferenceSmokeTest") is True:
            inference_check = self._inference_smoke_test(request_id, payload, context)
            checks.append(inference_check)
            context.emitter.emit(request_id, "inferenceSmokeReport", {"status": inference_check["status"]})
        output = require_path(payload.get("outputDirectory"), "Output directory", directory=True)
        usage = shutil.disk_usage(output)
        checkpoint_size = sum(Path(item).stat().st_size for item in payload.get("pretrainedCheckpointPaths", []))
        reasons: list[str] = []
        risk = "low"
        if payload.get("requestedDevice") == "cpu":
            reasons.append("cpuOnlyTraining")
            risk = "high"
        if accelerator["availableVramBytes"] is None and payload.get("requestedDevice") == "cuda":
            reasons.append("unavailableVramMeasurement")
            risk = "unknown"
        resources = {
            "logicalCpuCount": os.cpu_count(),
            "totalMemoryBytes": _memory_info()[0],
            "availableMemoryBytes": _memory_info()[1],
            "processMemoryBytes": _process_memory(),
            "freeDiskBytes": usage.free,
            "snapshotSizeBytes": None,
            "checkpointSizeBytes": checkpoint_size,
            "estimatedTemporaryBytes": checkpoint_size * 2,
            "totalVramBytes": accelerator["totalVramBytes"],
            "availableVramBytes": accelerator["availableVramBytes"],
            "riskLevel": risk,
            "reasons": reasons,
        }
        return {
            "python": {
                "implementation": platform.python_implementation(),
                "version": platform.python_version(),
                "executableLabel": Path(sys.executable).name,
            },
            "worker": {
                "workerVersion": WORKER_VERSION,
                "adapterVersion": self.adapter_version,
                "protocolVersion": PROTOCOL_VERSION,
            },
            "packages": packages,
            "accelerator": accelerator,
            "resources": resources,
            "checks": checks,
            "capabilityReport": capability,
        }

    def _inspect_packages(self, payload: dict[str, Any], checks: list[dict[str, Any]]) -> list[dict[str, Any]]:
        requirements = payload.get("packageRequirements")
        if not isinstance(requirements, list):
            raise WorkerError("invalidConfiguration", "Typed package requirements are missing.")
        packages: list[dict[str, Any]] = []
        for requirement in requirements:
            if not isinstance(requirement, dict) or not isinstance(requirement.get("package"), str):
                raise WorkerError("invalidConfiguration", "A package requirement is malformed.")
            name = requirement["package"]
            required = bool(requirement.get("required"))
            try:
                version: str | None = metadata.version(name)
            except metadata.PackageNotFoundError:
                version = None
            requested = str(requirement.get("requirement", ""))
            compatible = None if version is None else _version_matches(version, requested)
            status = "failed" if required and (version is None or compatible is False) else "passed"
            checks.append(_check(f"package:{name}", f"Python package {name}", "worker", status, f"{name} version is {version or 'missing'}."))
            packages.append({"package": name, "version": version, "required": required, "compatible": compatible})
        return packages

    def _inspect_framework(self, payload: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
        checks: list[dict[str, Any]] = []
        try:
            import torch  # type: ignore

            tensor = torch.tensor([1.0, 2.0], device="cpu")
            cpu_ok = bool(float(tensor.sum().item()) == 3.0)
            checks.append(_check("cpuTensor", "CPU tensor operation", "framework", "passed" if cpu_ok else "failed", "A bounded CPU tensor operation completed."))
            cuda_available = bool(torch.cuda.is_available())
            gpu_count = int(torch.cuda.device_count()) if cuda_available else 0
            gpu_name = str(torch.cuda.get_device_name(0)) if cuda_available else None
            cuda_runtime = str(getattr(torch.version, "cuda", None)) if cuda_available else None
            total_vram = None
            available_vram = None
            if cuda_available:
                properties = torch.cuda.get_device_properties(0)
                total_vram = int(getattr(properties, "total_memory", 0)) or None
                try:
                    available_vram, _ = (int(value) for value in torch.cuda.mem_get_info(0))
                except (AttributeError, RuntimeError, TypeError, ValueError):
                    available_vram = None
                if payload.get("requestedDevice") == "cuda":
                    probe = torch.tensor([1.0], device="cuda") + 1.0
                    _ = float(probe.item())
                    torch.cuda.synchronize()
                    checks.append(_check("cudaTensor", "CUDA tensor and synchronization", "framework", "passed", "CUDA initialization, tensor operation, and synchronization completed."))
        except Exception as error:
            raise WorkerError("frameworkFailure", "PyTorch framework smoke test failed.") from error
        accelerator = {
            "cudaAvailable": cuda_available,
            "cudaRuntimeVersion": cuda_runtime,
            "gpuName": gpu_name,
            "gpuCount": gpu_count,
            "totalVramBytes": total_vram,
            "availableVramBytes": available_vram,
            "selectedDevice": payload.get("requestedDevice"),
            "selectedPrecision": payload.get("requestedPrecision"),
        }
        if payload.get("requestedDevice") == "cuda" and not cuda_available:
            checks.append(_check("cudaAvailable", "CUDA availability", "framework", "failed", "CUDA was selected but is unavailable."))
        return accelerator, checks

    @staticmethod
    def _audio_smoke_test(context: Any) -> dict[str, Any]:
        with tempfile.TemporaryDirectory(prefix="mam-voice-audio-smoke-") as temporary:
            path = Path(temporary) / "fixture.wav"
            with wave.open(str(path), "wb") as writer:
                writer.setnchannels(1)
                writer.setsampwidth(2)
                writer.setframerate(48000)
                frames = bytearray()
                for index in range(2400):
                    if context.cancel.is_set():
                        raise WorkerError("cancelled", "Audio smoke test was cancelled.")
                    value = int(1000 * ((index % 48) / 48.0 - 0.5))
                    frames.extend(struct.pack("<h", value))
                writer.writeframes(bytes(frames))
            summary = validate_wav(path)
            finite = all(isinstance(value, (int, float)) for value in summary.values())
            return _check("audioPreprocess", "Project fixture WAV preprocessing", "audio", "passed" if finite else "failed", "Project-generated WAV decoded with finite bounded metadata and temporary files were cleaned.")

    def _inference_smoke_test(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        root = require_path(payload.get("seedVcDirectory"), "Seed-VC directory", directory=True)
        configuration = require_path(payload.get("modelConfigurationPath"), "Model configuration")
        source = require_path(payload.get("inferenceSmokeSourcePath"), "Project smoke source")
        reference = require_path(payload.get("inferenceSmokeReferencePath"), "Consent-active smoke reference")
        checkpoints = payload.get("pretrainedCheckpointPaths")
        if not isinstance(checkpoints, list) or not checkpoints:
            raise WorkerError("checkpointMissing", "Inference smoke test requires a local checkpoint.")
        checkpoint = require_path(checkpoints[0], "Inference smoke checkpoint")
        output = Path(_required_string(payload, "inferenceSmokeOutputPath")).resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        validate_wav(source)
        validate_wav(reference)
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
            "2",
            "--length-adjust",
            "1.0",
            "--inference-cfg-rate",
            "0.7",
            "--f0-condition",
            "False",
            "--auto-f0-adjust",
            "False",
            "--semi-tone-shift",
            "0",
            "--checkpoint",
            str(checkpoint),
            "--config",
            str(configuration),
            "--fp16",
            str(payload.get("requestedPrecision") == "float16"),
        ]
        before = {item.resolve() for item in output.parent.glob("*.wav")}
        context.run_process(request_id, command, output.parent)
        if not output.is_file():
            created = [item for item in output.parent.glob("*.wav") if item.resolve() not in before]
            if len(created) != 1:
                raise WorkerError("unexpectedOutput", "Inference smoke test did not create exactly one expected WAV.")
            shutil.move(str(created[0]), output)
        validate_wav(output)
        return _check("inferenceSmoke", "Bounded offline inference smoke test", "inference", "passed", "A bounded synthetic WAV was generated and structurally validated; audible quality remains pending.")


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


def _check(code: str, label: str, layer: str, status: str, message: str) -> dict[str, Any]:
    return {"code": code, "label": label, "layer": layer, "status": status, "message": message}


def _memory_info() -> tuple[int | None, int | None]:
    try:
        import psutil  # type: ignore

        memory = psutil.virtual_memory()
        return int(memory.total), int(memory.available)
    except (ImportError, AttributeError, OSError):
        pass
    if os.name == "nt":
        try:
            import ctypes

            class MemoryStatus(ctypes.Structure):
                _fields_ = [
                    ("length", ctypes.c_ulong),
                    ("memory_load", ctypes.c_ulong),
                    ("total_physical", ctypes.c_ulonglong),
                    ("available_physical", ctypes.c_ulonglong),
                    ("total_page", ctypes.c_ulonglong),
                    ("available_page", ctypes.c_ulonglong),
                    ("total_virtual", ctypes.c_ulonglong),
                    ("available_virtual", ctypes.c_ulonglong),
                    ("available_extended_virtual", ctypes.c_ulonglong),
                ]

            status = MemoryStatus()
            status.length = ctypes.sizeof(status)
            if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
                return int(status.total_physical), int(status.available_physical)
        except (AttributeError, OSError, ValueError):
            pass
    return None, None


def _process_memory() -> int | None:
    try:
        import psutil  # type: ignore

        return int(psutil.Process().memory_info().rss)
    except (ImportError, AttributeError, OSError):
        return None


def _validate_configuration_text(path: Path) -> None:
    try:
        if path.stat().st_size > 2 * 1024 * 1024:
            raise WorkerError("configurationInvalid", "Model configuration exceeds its size limit.")
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise WorkerError("configurationInvalid", "Model configuration is not bounded UTF-8 text.") from error
    if not text.strip() or "\x00" in text or (path.suffix.lower() in (".yaml", ".yml") and ":" not in text):
        raise WorkerError("configurationInvalid", "Model configuration could not be parsed safely.")


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(64 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def _version_matches(version: str, requirement: str) -> bool | None:
    def parsed(value: str) -> tuple[int, ...] | None:
        pieces: list[int] = []
        for part in value.split("."):
            digits = "".join(character for character in part if character.isdigit())
            if not digits:
                break
            pieces.append(int(digits))
        return tuple(pieces) if pieces else None

    current = parsed(version)
    if current is None or not requirement:
        return None
    for clause in requirement.split(","):
        clause = clause.strip()
        operator = next((item for item in (">=", "<=", "==", ">", "<") if clause.startswith(item)), None)
        if operator is None:
            return None
        expected = parsed(clause[len(operator) :])
        if expected is None:
            return None
        width = max(len(current), len(expected))
        left = current + (0,) * (width - len(current))
        right = expected + (0,) * (width - len(expected))
        valid = {
            ">=": left >= right,
            "<=": left <= right,
            "==": left == right,
            ">": left > right,
            "<": left < right,
        }[operator]
        if not valid:
            return False
    return True
