from __future__ import annotations

import io
import json
import subprocess
import sys
import unittest
from pathlib import Path

from mam_voice_worker.errors import WorkerError
from mam_voice_worker.protocol import Emitter, MAX_MESSAGE_BYTES, decode_request, read_bounded_line


class ProtocolTests(unittest.TestCase):
    def test_decodes_versioned_request(self) -> None:
        request = decode_request(
            b'{"protocolVersion":1,"requestId":"r1","command":"hello","payload":{}}'
        )
        self.assertEqual(request.command, "hello")
        self.assertEqual(request.request_id, "r1")

    def test_rejects_protocol_unknown_command_and_malformed_json(self) -> None:
        cases = [
            b'{"protocolVersion":2,"requestId":"r1","command":"hello","payload":{}}',
            b'{"protocolVersion":1,"requestId":"r1","command":"shell","payload":{}}',
            b'not-json',
        ]
        for raw in cases:
            with self.subTest(raw=raw), self.assertRaises(WorkerError):
                decode_request(raw)

    def test_bounds_input_and_output(self) -> None:
        with self.assertRaises(WorkerError):
            read_bounded_line(io.BytesIO(b"x" * (MAX_MESSAGE_BYTES + 2)))
        stream = io.BytesIO()
        Emitter(stream).emit("r1", "log", {"message": "x" * (MAX_MESSAGE_BYTES + 10)})
        event = json.loads(stream.getvalue())
        self.assertEqual(event["event"], "failed")

    def test_worker_process_handshake_and_unknown_command_failure(self) -> None:
        worker_root = Path(__file__).resolve().parents[1]
        process = subprocess.Popen(
            [sys.executable, "-m", "mam_voice_worker"],
            cwd=worker_root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        assert process.stdin is not None and process.stdout is not None
        process.stdin.write('{"protocolVersion":1,"requestId":"hello-1","command":"hello","payload":{}}\n')
        process.stdin.flush()
        ready = json.loads(process.stdout.readline())
        self.assertEqual(ready["event"], "ready")
        process.stdin.write('{"protocolVersion":1,"requestId":"bad-1","command":"arbitrary","payload":{}}\n')
        process.stdin.flush()
        failed = json.loads(process.stdout.readline())
        self.assertEqual(failed["event"], "failed")
        self.assertEqual(failed["payload"]["code"], "unknownCommand")
        process.stdin.write('{"protocolVersion":1,"requestId":"stop","command":"shutdown","payload":{}}\n')
        process.stdin.flush()
        process.wait(timeout=5)
        self.assertEqual(process.returncode, 0)
        process.stdin.close()
        process.stdout.close()
        if process.stderr is not None:
            process.stderr.close()


if __name__ == "__main__":
    unittest.main()
