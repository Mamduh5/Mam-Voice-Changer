from __future__ import annotations

import io
import json
import tempfile
import unittest
import wave
from pathlib import Path

from mam_voice_worker.audio import validate_wav
from mam_voice_worker.backends.seed_vc import SeedVcBackend, _bounded_int
from mam_voice_worker.errors import WorkerError
from mam_voice_worker.protocol import Emitter


def write_wav(path: Path, frames: int = 160) -> None:
    with wave.open(str(path), "wb") as writer:
        writer.setnchannels(1)
        writer.setsampwidth(2)
        writer.setframerate(48000)
        writer.writeframes(b"\0\0" * frames)


class FakeContext:
    def __init__(self, output: Path) -> None:
        self.stream = io.BytesIO()
        self.emitter = Emitter(self.stream)
        self.output = output
        self.commands: list[tuple[list[str], Path]] = []

    def run_process(self, _request_id: str, command: list[str], working_directory: Path) -> None:
        self.commands.append((command, working_directory))
        if Path(command[1]).name == "train.py":
            run = working_directory / "runs" / working_directory.name
            run.mkdir(parents=True)
            (run / "ft_model.pth").write_bytes(b"fine-tuned")
        else:
            write_wav(self.output.parent / "backend-created.wav")


class BackendTests(unittest.TestCase):
    def configured_tree(self, root: Path, *, broken_import: bool = False) -> dict[str, object]:
        seed = root / "seed-vc"
        modules = seed / "modules"
        modules.mkdir(parents=True)
        (modules / "__init__.py").write_text("", encoding="utf-8")
        (modules / "commons.py").write_text(
            "this is invalid python" if broken_import else "VALUE = 1", encoding="utf-8"
        )
        (seed / "train.py").write_text("", encoding="utf-8")
        (seed / "inference.py").write_text("", encoding="utf-8")
        config = root / "config.yml"
        config.write_text("model: test\n", encoding="utf-8")
        checkpoint = root / "pretrained.pth"
        checkpoint.write_bytes(b"checkpoint")
        return {
            "seedVcDirectory": str(seed),
            "modelConfigurationPath": str(config),
            "pretrainedCheckpointPaths": [str(checkpoint)],
            "outputDirectory": str(root / "output"),
            "requestedDevice": "cpu",
            "requestedPrecision": "float32",
        }

    def test_validation_reports_capabilities_without_seed_vc_or_gpu_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            report = SeedVcBackend().validate(self.configured_tree(Path(temporary)))
            self.assertEqual(report["backendId"], "seed-vc-local")
            self.assertIn("cpu", report["devices"])
            self.assertEqual(report["protocolVersion"], 1)

    def test_validation_rejects_missing_checkpoint_and_backend_import_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = self.configured_tree(root)
            payload["pretrainedCheckpointPaths"] = [str(root / "missing.pth")]
            with self.assertRaises(WorkerError) as missing:
                SeedVcBackend().validate(payload)
            self.assertEqual(missing.exception.code, "missingPath")
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(WorkerError) as failed:
                SeedVcBackend().validate(self.configured_tree(Path(temporary), broken_import=True))
            self.assertEqual(failed.exception.code, "backendImportFailure")

    def test_training_controls_are_typed_and_bounded(self) -> None:
        self.assertEqual(_bounded_int({"maximumSteps": 100}, "maximumSteps", 10, 1000), 100)
        for value in ("100", True, 0, 1001):
            with self.subTest(value=value), self.assertRaises(WorkerError):
                _bounded_int({"maximumSteps": value}, "maximumSteps", 10, 1000)

    def test_audio_validation_rejects_empty_and_accepts_local_pcm(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            valid = root / "valid.wav"
            empty = root / "empty.wav"
            write_wav(valid)
            write_wav(empty, frames=0)
            self.assertEqual(validate_wav(valid)["sampleRate"], 48000)
            with self.assertRaises(WorkerError) as error:
                validate_wav(empty)
            self.assertEqual(error.exception.code, "emptyWav")

    def test_training_request_uses_fixed_argument_list_and_structured_events(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = self.configured_tree(root)
            snapshot = root / "snapshot"
            (snapshot / "audio").mkdir(parents=True)
            (snapshot / "snapshot.json").write_text("{}", encoding="utf-8")
            write_wav(snapshot / "audio" / "take.wav")
            job = root / "job-1"
            payload.update(
                {
                    "snapshotDirectory": str(snapshot),
                    "jobDirectory": str(job),
                    "trainingConfiguration": {
                        "maximumSteps": 100,
                        "saveInterval": 50,
                        "batchSize": 1,
                        "workerCount": 0,
                    },
                }
            )
            context = FakeContext(root / "unused.wav")
            result = SeedVcBackend().train("training-1", payload, context)
            self.assertEqual(result["trainingSummary"]["completedSteps"], 100)
            command, working_directory = context.commands[0]
            self.assertIsInstance(command, list)
            self.assertEqual(working_directory, job.resolve())
            self.assertIn("--dataset-dir", command)
            self.assertNotIn("shell", " ".join(command).lower())
            events = [json_line["event"] for json_line in _events(context.stream)]
            self.assertIn("phaseStarted", events)
            self.assertIn("checkpointSaved", events)
            self.assertIn("progress", events)

    def test_inference_request_validates_artifact_paths_and_expected_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = self.configured_tree(root)
            source = root / "source.wav"
            reference = root / "reference.wav"
            write_wav(source)
            write_wav(reference)
            artifact = root / "artifact"
            (artifact / "model").mkdir(parents=True)
            (artifact / "model" / "voice.pth").write_bytes(b"model")
            (artifact / "model" / "voice.yaml").write_text("model: test\n", encoding="utf-8")
            output = root / "result" / "synthetic.wav"
            payload.update(
                {
                    "artifactDirectory": str(artifact),
                    "sourcePath": str(source),
                    "referencePaths": [str(reference)],
                    "outputPath": str(output),
                    "modelFiles": [
                        {"relativePath": "model/voice.pth"},
                        {"relativePath": "model/voice.yaml"},
                    ],
                    "inferenceConfiguration": {
                        "diffusionSteps": 25,
                        "pitchAdjustmentSemitones": 0,
                        "lengthAdjustment": 1.0,
                        "f0Conditioning": False,
                        "precision": "float32",
                    },
                }
            )
            context = FakeContext(output)
            result = SeedVcBackend().infer("inference-1", payload, context)
            self.assertTrue(result["synthetic"])
            self.assertTrue(output.is_file())
            unsafe = dict(payload)
            unsafe["modelFiles"] = [{"relativePath": "../outside.pth"}]
            with self.assertRaises(WorkerError) as error:
                SeedVcBackend().infer("inference-2", unsafe, FakeContext(output))
            self.assertEqual(error.exception.code, "pathTraversal")


def _events(stream: io.BytesIO) -> list[dict[str, object]]:
    return [json.loads(line) for line in stream.getvalue().splitlines()]


if __name__ == "__main__":
    unittest.main()
