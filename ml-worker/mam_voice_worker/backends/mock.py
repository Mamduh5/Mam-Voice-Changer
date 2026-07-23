from __future__ import annotations

import math
import struct
import time
import wave
from pathlib import Path
from typing import Any

from ..audio import validate_wav
from ..errors import WorkerError
from ..protocol import PROTOCOL_VERSION, WORKER_VERSION
from .base import Backend


class MockQualificationBackend(Backend):
    """Deterministic, dependency-free backend used only by automated tests."""

    backend_id = "mock-qualification"
    adapter_version = "mam-mock-adapter-v1"

    def inspect_seed_vc(self, _payload: dict[str, Any]) -> dict[str, Any]:
        return {
            "backendId": self.backend_id,
            "backendVersion": "mock-v1",
            "workerVersion": WORKER_VERSION,
            "protocolVersion": PROTOCOL_VERSION,
            "devices": ["cpu"],
            "precisions": ["float32"],
            "supportsResume": True,
            "supportsMultipleReferences": False,
            "resources": {
                "systemMemoryBytes": None,
                "gpuMemoryBytes": None,
                "availableDiskBytes": None,
                "snapshotSizeBytes": None,
                "checkpointSizeBytes": 16,
                "riskLevel": "low",
            },
            "warnings": [],
        }

    def preprocess_snapshot(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        self._failure(payload, context)
        context.emitter.emit(request_id, "phaseStarted", {"phase": "preprocessing"})
        context.emitter.emit(request_id, "progress", {"progress": 1.0, "step": 1})
        return {"validated": True, "takeCount": 1}

    def fine_tune_seed_vc(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        self._failure(payload, context)
        job = Path(str(payload.get("jobDirectory", "."))).resolve()
        output = job / "mock-output"
        output.mkdir(parents=True, exist_ok=True)
        resume = bool(payload.get("resume"))
        for step in (0, 25, 50, 75, 100):
            if context.cancel.is_set():
                checkpoint = output / "checkpoint-050.pth"
                checkpoint.write_bytes(b"mock-checkpoint-v1")
                raise WorkerError("cancelled", "Mock training interrupted after a valid checkpoint.")
            context.emitter.emit(request_id, "phaseStarted", {"phase": "training"})
            context.emitter.emit(request_id, "progress", {"progress": step / 100, "step": step})
            context.emitter.emit(request_id, "metric", {"trainingLoss": 1.0 - step / 200, "learningRate": 0.0001})
        checkpoint = output / ("resumed-model.pth" if resume else "model.pth")
        checkpoint.write_bytes(b"mock-trained-model-v1")
        configuration = output / "model.yaml"
        configuration.write_text("model: mock\n", encoding="utf-8")
        context.emitter.emit(request_id, "checkpointSaved", {"relativePath": checkpoint.relative_to(job).as_posix()})
        return {
            "backendVersion": "mock-v1",
            "artifactFiles": [checkpoint.relative_to(job).as_posix(), configuration.relative_to(job).as_posix()],
            "trainingSummary": {
                "completedSteps": 100,
                "finalTrainingLoss": 0.5,
                "finalValidationLoss": 0.6,
                "checkpointCount": 1,
                "durationMs": 1,
                "warnings": [],
            },
        }

    def convert_with_seed_vc(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        self._failure(payload, context)
        output = Path(str(payload.get("outputPath"))).resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        mode = payload.get("failureMode")
        if mode == "missingArtifactFile":
            raise WorkerError("artifactMissing", "Injected missing artifact file.")
        if mode == "emptyGeneratedWav":
            self._write_wav(output, [])
        elif mode == "invalidGeneratedWav":
            output.write_bytes(b"not-wave")
        elif mode == "nonFiniteGeneratedWav":
            output.write_bytes(b"RIFF invalid non-finite fixture")
        else:
            samples = [0.15 * math.sin(2 * math.pi * 220 * index / 48000) for index in range(2400)]
            self._write_wav(output, samples)
        if mode not in ("invalidGeneratedWav", "nonFiniteGeneratedWav"):
            validate_wav(output)
        context.emitter.emit(request_id, "progress", {"progress": 1.0})
        return {"outputFile": output.name, "synthetic": True}

    def inspect_checkpoint(self, _request_id: str, payload: dict[str, Any], _context: Any) -> dict[str, Any]:
        checkpoint = Path(str(payload.get("checkpointPath", "")))
        if not checkpoint.is_file() or checkpoint.stat().st_size == 0:
            raise WorkerError("checkpointInvalid", "Mock checkpoint is missing or empty.")
        if payload.get("failureMode") == "invalidCheckpointEvent":
            raise WorkerError("checkpointInvalid", "Injected invalid checkpoint event.")
        return {"structurallyUsable": True, "sizeBytes": checkpoint.stat().st_size, "deserialized": False}

    def qualify(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]:
        self._failure(payload, context)
        packages = [{"package": "mock-runtime", "version": "1.0.0", "required": True, "compatible": True}]
        accelerator = {
            "cudaAvailable": False,
            "cudaRuntimeVersion": None,
            "gpuName": None,
            "gpuCount": 0,
            "totalVramBytes": None,
            "availableVramBytes": None,
            "selectedDevice": "cpu",
            "selectedPrecision": "float32",
        }
        context.emitter.emit(request_id, "packageReport", {"packages": packages})
        context.emitter.emit(request_id, "acceleratorReport", accelerator)
        context.emitter.emit(request_id, "backendImportReport", {"status": "passed"})
        context.emitter.emit(request_id, "audioSmokeReport", {"status": "passed"})
        checks = [
            _check("mockPackage", "Mock package", "worker"),
            _check("mockCpu", "Mock CPU operation", "framework"),
            _check("mockImport", "Mock adapter import", "backendImport"),
            _check("mockAudio", "Mock audio preprocessing", "audio"),
        ]
        if payload.get("runInferenceSmokeTest") is True:
            output = Path(str(payload.get("inferenceSmokeOutputPath"))).resolve()
            output.parent.mkdir(parents=True, exist_ok=True)
            self._write_wav(output, [0.1 * math.sin(2 * math.pi * 220 * index / 48000) for index in range(2400)])
            checks.append(_check("mockInference", "Mock inference smoke test", "inference"))
            context.emitter.emit(request_id, "inferenceSmokeReport", {"status": "passed"})
        return {
            "python": {"implementation": "mock", "version": "3.10.0", "executableLabel": "mock-python"},
            "worker": {"workerVersion": WORKER_VERSION, "adapterVersion": self.adapter_version, "protocolVersion": PROTOCOL_VERSION},
            "packages": packages,
            "accelerator": accelerator,
            "resources": {
                "logicalCpuCount": 4,
                "totalMemoryBytes": 8 * 1024**3,
                "availableMemoryBytes": 4 * 1024**3,
                "processMemoryBytes": 32 * 1024**2,
                "freeDiskBytes": 20 * 1024**3,
                "snapshotSizeBytes": 4096,
                "checkpointSizeBytes": 16,
                "estimatedTemporaryBytes": 8192,
                "totalVramBytes": None,
                "availableVramBytes": None,
                "riskLevel": "low",
                "reasons": [],
            },
            "checks": checks,
        }

    @staticmethod
    def _failure(payload: dict[str, Any], context: Any) -> None:
        mode = payload.get("failureMode")
        if mode in {"workerCrash", "partialExport", "partialImport", "interruptedSnapshot", "interruptedQualification", "interruptedTraining", "interruptedInference"}:
            raise WorkerError(mode, f"Injected deterministic failure: {mode}.")
        if mode == "progressStall":
            raise WorkerError("progressStall", "Injected progress stall reached a bounded terminal failure.")
        if mode == "cancellationIgnored":
            context.cancel.set()
            raise WorkerError("cancelled", "Injected ignored cancellation was forced to a terminal state.")
        if mode == "hashMismatch":
            raise WorkerError("hashMismatch", "Injected hash mismatch.")
        if mode == "unexpectedOutputPath":
            raise WorkerError("unexpectedOutput", "Injected unexpected output path.")

    @staticmethod
    def _write_wav(path: Path, samples: list[float]) -> None:
        with wave.open(str(path), "wb") as writer:
            writer.setnchannels(1)
            writer.setsampwidth(2)
            writer.setframerate(48000)
            frames = b"".join(struct.pack("<h", max(-32768, min(32767, int(sample * 32767)))) for sample in samples)
            writer.writeframes(frames)


def _check(code: str, label: str, layer: str) -> dict[str, Any]:
    return {"code": code, "label": label, "layer": layer, "status": "passed", "message": "Deterministic mock check passed."}
