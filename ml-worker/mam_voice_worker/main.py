from __future__ import annotations

import os
import queue
import subprocess
import sys
import threading
from pathlib import Path
from typing import Any

from .backends import MockQualificationBackend, SeedVcBackend
from .errors import WorkerError
from .protocol import Emitter, Request, WORKER_VERSION, decode_request, read_bounded_line


class WorkerContext:
    def __init__(self, emitter: Emitter) -> None:
        self.emitter = emitter
        self.cancel = threading.Event()
        self.shutdown = threading.Event()
        self._process: subprocess.Popen[str] | None = None
        self._process_lock = threading.Lock()
        self._job_lock = threading.Lock()

    def start_job(self, request: Request, target: Any) -> None:
        if not self._job_lock.acquire(blocking=False):
            self.emitter.emit(request.request_id, "failed", {"code": "jobActive", "message": "A worker job is already active."})
            return
        self.cancel.clear()

        def run() -> None:
            try:
                result = target(request.request_id, request.payload, self)
                if self.cancel.is_set():
                    self.emitter.emit(request.request_id, "cancelled", {"message": "Worker job cancelled."})
                else:
                    self.emitter.emit(request.request_id, "completed", result)
            except WorkerError as error:
                event = "cancelled" if error.code == "cancelled" else "failed"
                self.emitter.emit(request.request_id, event, {"code": error.code, "message": error.message})
            except Exception:
                self.emitter.emit(request.request_id, "failed", {"code": "internalError", "message": "The isolated worker failed unexpectedly."})
            finally:
                with self._process_lock:
                    self._process = None
                self._job_lock.release()

        threading.Thread(target=run, name="mam-voice-worker-job", daemon=True).start()

    def run_process(self, request_id: str, command: list[str], working_directory: Path) -> None:
        if any(not isinstance(item, str) or "\x00" in item for item in command):
            raise WorkerError("invalidArguments", "Backend arguments are invalid.")
        environment = {
            key: value
            for key, value in os.environ.items()
            if key.upper() in {"SYSTEMROOT", "WINDIR", "PATH", "TEMP", "TMP"}
        }
        environment.update(
            {
                "PYTHONNOUSERSITE": "1",
                "HF_HUB_OFFLINE": "1",
                "TRANSFORMERS_OFFLINE": "1",
                "MAM_VOICE_NO_DOWNLOADS": "1",
            }
        )
        creation_flags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0
        process = subprocess.Popen(
            command,
            cwd=working_directory,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            errors="replace",
            creationflags=creation_flags,
        )
        with self._process_lock:
            self._process = process
        messages: queue.Queue[tuple[str, str]] = queue.Queue(maxsize=256)

        def drain(label: str, stream: Any) -> None:
            try:
                for line in stream:
                    bounded = line.rstrip()[:2000]
                    try:
                        messages.put((label, bounded), timeout=0.1)
                    except queue.Full:
                        pass
            finally:
                stream.close()

        assert process.stdout is not None and process.stderr is not None
        readers = [
            threading.Thread(target=drain, args=("stdout", process.stdout), daemon=True),
            threading.Thread(target=drain, args=("stderr", process.stderr), daemon=True),
        ]
        for reader in readers:
            reader.start()
        while process.poll() is None:
            if self.cancel.wait(0.05):
                self._terminate_process(process)
                raise WorkerError("cancelled", "Worker job cancelled.")
            self._emit_log_queue(request_id, messages)
        for reader in readers:
            reader.join(timeout=1.0)
        self._emit_log_queue(request_id, messages)
        if process.returncode != 0:
            raise WorkerError("backendProcessFailed", f"Backend process exited with code {process.returncode}.")

    def request_cancel(self) -> None:
        self.cancel.set()
        with self._process_lock:
            process = self._process
        if process is not None:
            self._terminate_process(process)

    def _emit_log_queue(self, request_id: str, messages: queue.Queue[tuple[str, str]]) -> None:
        while True:
            try:
                stream, message = messages.get_nowait()
            except queue.Empty:
                return
            self.emitter.emit(request_id, "log", {"stream": stream, "message": message})

    @staticmethod
    def _terminate_process(process: subprocess.Popen[str]) -> None:
        if process.poll() is not None:
            return
        if os.name == "nt":
            subprocess.run(
                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
        else:
            process.terminate()
        try:
            process.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            process.kill()


def dispatch(request: Request, context: WorkerContext, backend: SeedVcBackend) -> bool:
    selected: Any = (
        MockQualificationBackend()
        if request.payload.get("backendId") == "mock-qualification"
        else backend
    )
    if request.command == "hello":
        context.emitter.emit(request.request_id, "ready", {"workerVersion": WORKER_VERSION, "protocolVersion": 1})
    elif request.command in ("validateBackend", "inspectCapabilities"):
        try:
            report = selected.inspect_seed_vc(request.payload)
            context.emitter.emit(request.request_id, "capabilityReport", report)
            context.emitter.emit(request.request_id, "completed", {"capabilityReport": report})
        except WorkerError as error:
            context.emitter.emit(request.request_id, "failed", {"code": error.code, "message": error.message})
    elif request.command == "preprocessSnapshot":
        context.start_job(request, selected.preprocess_snapshot)
    elif request.command in ("startTraining", "resumeTraining"):
        context.start_job(request, selected.fine_tune_seed_vc)
    elif request.command == "runInference":
        context.start_job(request, selected.convert_with_seed_vc)
    elif request.command == "inspectArtifact":
        context.emitter.emit(request.request_id, "completed", {"valid": True})
    elif request.command in ("qualifyBackend", "inspectEnvironment"):
        context.start_job(request, selected.qualify)
    elif request.command == "inspectCheckpoint":
        context.start_job(request, selected.inspect_checkpoint)
    elif request.command == "cancelJob":
        context.request_cancel()
    elif request.command == "shutdown":
        context.request_cancel()
        context.shutdown.set()
        return False
    return True


def main() -> int:
    emitter = Emitter()
    context = WorkerContext(emitter)
    backend = SeedVcBackend()
    while not context.shutdown.is_set():
        try:
            raw = read_bounded_line(sys.stdin.buffer)
            if raw is None:
                context.request_cancel()
                return 0
            request = decode_request(raw)
            if not dispatch(request, context, backend):
                return 0
        except WorkerError as error:
            emitter.emit("unknown", "failed", {"code": error.code, "message": error.message})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
