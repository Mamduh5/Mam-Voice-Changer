from __future__ import annotations

import io
import json
import tempfile
import threading
import unittest
import wave
from pathlib import Path

from mam_voice_worker.backends.mock import MockQualificationBackend
from mam_voice_worker.backends.seed_vc import SeedVcBackend, _version_matches
from mam_voice_worker.errors import WorkerError
from mam_voice_worker.protocol import Emitter


class Context:
    def __init__(self) -> None:
        self.stream = io.BytesIO()
        self.emitter = Emitter(self.stream)
        self.cancel = threading.Event()


class MockQualificationTests(unittest.TestCase):
    def test_environment_fingerprint_events_and_dependency_free_report(self) -> None:
        context = Context()
        result = MockQualificationBackend().qualify("qualification-1", {}, context)
        self.assertEqual(result["python"]["implementation"], "mock")
        self.assertEqual(result["worker"]["adapterVersion"], "mam-mock-adapter-v1")
        self.assertFalse(result["accelerator"]["cudaAvailable"])
        self.assertEqual(result["resources"]["riskLevel"], "low")
        events = [json.loads(line)["event"] for line in context.stream.getvalue().splitlines()]
        self.assertEqual(
            events,
            ["packageReport", "acceleratorReport", "backendImportReport", "audioSmokeReport"],
        )

    def test_optional_inference_smoke_creates_labeled_structural_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "synthetic-smoke.wav"
            context = Context()
            result = MockQualificationBackend().qualify(
                "qualification-inference",
                {"runInferenceSmokeTest": True, "inferenceSmokeOutputPath": str(output)},
                context,
            )
            self.assertTrue(output.is_file())
            with wave.open(str(output), "rb") as reader:
                self.assertGreater(reader.getnframes(), 0)
            self.assertIn("inference", [check["layer"] for check in result["checks"]])
            events = [json.loads(line)["event"] for line in context.stream.getvalue().splitlines()]
            self.assertEqual(events[-1], "inferenceSmokeReport")

    def test_mock_end_to_end_training_resume_inference_and_checkpoint_inspection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            context = Context()
            backend = MockQualificationBackend()
            trained = backend.fine_tune_seed_vc(
                "training-1", {"jobDirectory": str(root / "job")}, context
            )
            self.assertEqual(trained["trainingSummary"]["completedSteps"], 100)
            checkpoint = root / "job" / trained["artifactFiles"][0]
            inspected = backend.inspect_checkpoint(
                "checkpoint-1", {"checkpointPath": str(checkpoint)}, context
            )
            self.assertTrue(inspected["structurallyUsable"])
            resumed = backend.fine_tune_seed_vc(
                "resume-1", {"jobDirectory": str(root / "resume"), "resume": True}, context
            )
            self.assertIn("resumed-model.pth", resumed["artifactFiles"][0])
            output = root / "result" / "synthetic.wav"
            converted = backend.convert_with_seed_vc(
                "inference-1", {"outputPath": str(output)}, context
            )
            self.assertTrue(converted["synthetic"])
            with wave.open(str(output), "rb") as reader:
                self.assertGreater(reader.getnframes(), 0)

    def test_mock_failure_injection_reaches_stable_terminal_errors(self) -> None:
        backend = MockQualificationBackend()
        for mode in (
            "workerCrash",
            "progressStall",
            "cancellationIgnored",
            "hashMismatch",
            "unexpectedOutputPath",
            "partialExport",
            "partialImport",
            "interruptedSnapshot",
            "interruptedQualification",
            "interruptedTraining",
            "interruptedInference",
        ):
            with self.subTest(mode=mode), self.assertRaises(WorkerError):
                backend.qualify("failure-1", {"failureMode": mode}, Context())

    def test_invalid_empty_and_nonfinite_generated_output_modes_are_rejected_or_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            backend = MockQualificationBackend()
            for mode in ("invalidGeneratedWav", "nonFiniteGeneratedWav"):
                output = root / mode / "synthetic.wav"
                result = backend.convert_with_seed_vc(
                    "inference-failure", {"outputPath": str(output), "failureMode": mode}, Context()
                )
                self.assertTrue(result["synthetic"])
                self.assertTrue(output.is_file())
            with self.assertRaises(WorkerError) as empty:
                backend.convert_with_seed_vc(
                    "inference-empty",
                    {
                        "outputPath": str(root / "empty" / "synthetic.wav"),
                        "failureMode": "emptyGeneratedWav",
                    },
                    Context(),
                )
            self.assertEqual(empty.exception.code, "emptyWav")


class SeedQualificationBoundaryTests(unittest.TestCase):
    def test_configuration_parse_checkpoint_hash_and_version_requirements(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint = root / "base.pth"
            checkpoint.write_bytes(b"checkpoint")
            backend = SeedVcBackend()
            inspected = backend.inspect_checkpoint(
                "checkpoint-1", {"checkpointPath": str(checkpoint)}, Context()
            )
            self.assertEqual(len(inspected["sha256"]), 64)
            with self.assertRaises(WorkerError) as mismatch:
                backend.inspect_checkpoint(
                    "checkpoint-2",
                    {"checkpointPath": str(checkpoint), "expectedSha256": "0" * 64},
                    Context(),
                )
            self.assertEqual(mismatch.exception.code, "checkpointMismatch")
        self.assertTrue(_version_matches("2.1.0", ">=2.0,<3"))
        self.assertFalse(_version_matches("3.0.0", ">=2.0,<3"))

    def test_fixed_adapter_never_requests_shell_execution(self) -> None:
        import inspect

        source = inspect.getsource(SeedVcBackend)
        self.assertNotIn("shell=True", source)
        self.assertNotIn("pip install", source)
        self.assertNotIn("git checkout", source)
        self.assertNotIn("huggingface", source.lower())


if __name__ == "__main__":
    unittest.main()
