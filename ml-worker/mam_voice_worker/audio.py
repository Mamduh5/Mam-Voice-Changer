from __future__ import annotations

import math
import wave
from pathlib import Path

from .errors import WorkerError


def validate_wav(path: Path, maximum_seconds: float = 30.0) -> dict[str, float | int]:
    if not path.is_file() or path.suffix.lower() != ".wav":
        raise WorkerError("invalidWav", "Expected a local WAV file.")
    try:
        with wave.open(str(path), "rb") as reader:
            channels = reader.getnchannels()
            sample_rate = reader.getframerate()
            frames = reader.getnframes()
            sample_width = reader.getsampwidth()
    except (wave.Error, OSError) as error:
        raise WorkerError("invalidWav", "Cannot read the configured WAV file.") from error
    if channels not in (1, 2) or sample_rate not in (22050, 44100, 48000) or sample_width not in (2, 3, 4):
        raise WorkerError("invalidWav", "WAV format is not supported by the worker boundary.")
    if frames <= 0:
        raise WorkerError("emptyWav", "WAV file is empty.")
    duration = frames / sample_rate
    if not math.isfinite(duration) or duration > maximum_seconds:
        raise WorkerError("invalidWav", "WAV duration exceeds the configured offline limit.")
    return {"channels": channels, "sampleRate": sample_rate, "frames": frames, "durationSeconds": duration}

