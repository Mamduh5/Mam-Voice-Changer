from __future__ import annotations

import json
import sys
import threading
from dataclasses import dataclass
from typing import Any, BinaryIO

from .errors import WorkerError

PROTOCOL_VERSION = 1
WORKER_VERSION = "0.2.0"
MAX_MESSAGE_BYTES = 256 * 1024
COMMANDS = {
    "hello",
    "validateBackend",
    "inspectCapabilities",
    "preprocessSnapshot",
    "startTraining",
    "resumeTraining",
    "cancelJob",
    "inspectArtifact",
    "runInference",
    "shutdown",
    "qualifyBackend",
    "inspectEnvironment",
    "inspectCheckpoint",
}


@dataclass(frozen=True)
class Request:
    protocol_version: int
    request_id: str
    command: str
    payload: dict[str, Any]


def decode_request(raw: bytes) -> Request:
    if len(raw) > MAX_MESSAGE_BYTES:
        raise WorkerError("messageTooLarge", "Worker request exceeds the protocol size limit.")
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise WorkerError("malformedRequest", "Worker request is not valid JSON.") from error
    if not isinstance(value, dict) or set(value) != {
        "protocolVersion",
        "requestId",
        "command",
        "payload",
    }:
        raise WorkerError("malformedRequest", "Worker request envelope is invalid.")
    if value["protocolVersion"] != PROTOCOL_VERSION:
        raise WorkerError("protocolMismatch", "Unsupported worker protocol version.")
    if not isinstance(value["requestId"], str) or not value["requestId"] or len(value["requestId"]) > 160:
        raise WorkerError("malformedRequest", "Worker request ID is invalid.")
    if value["command"] not in COMMANDS:
        raise WorkerError("unknownCommand", "Unknown worker command.")
    if not isinstance(value["payload"], dict):
        raise WorkerError("malformedRequest", "Worker payload must be an object.")
    return Request(
        protocol_version=value["protocolVersion"],
        request_id=value["requestId"],
        command=value["command"],
        payload=value["payload"],
    )


def read_bounded_line(stream: BinaryIO) -> bytes | None:
    raw = stream.readline(MAX_MESSAGE_BYTES + 2)
    if not raw:
        return None
    if len(raw) > MAX_MESSAGE_BYTES + 1 or (len(raw) > MAX_MESSAGE_BYTES and not raw.endswith(b"\n")):
        raise WorkerError("messageTooLarge", "Worker request exceeds the protocol size limit.")
    return raw.rstrip(b"\r\n")


class Emitter:
    def __init__(self, stream: BinaryIO | None = None) -> None:
        self._stream = stream or sys.stdout.buffer
        self._lock = threading.Lock()

    def emit(self, request_id: str, event: str, payload: dict[str, Any] | None = None) -> None:
        envelope = {
            "protocolVersion": PROTOCOL_VERSION,
            "requestId": request_id,
            "event": event,
            "payload": payload or {},
        }
        encoded = json.dumps(envelope, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        if len(encoded) > MAX_MESSAGE_BYTES:
            envelope["event"] = "failed"
            envelope["payload"] = {
                "code": "messageTooLarge",
                "message": "Worker response exceeded the protocol size limit.",
            }
            encoded = json.dumps(envelope, separators=(",", ":")).encode("utf-8")
        with self._lock:
            self._stream.write(encoded + b"\n")
            self._stream.flush()
