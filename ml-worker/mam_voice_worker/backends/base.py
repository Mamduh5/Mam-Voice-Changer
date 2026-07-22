from __future__ import annotations

from abc import ABC, abstractmethod
from pathlib import Path
from typing import Any


class Backend(ABC):
    @abstractmethod
    def validate(self, payload: dict[str, Any]) -> dict[str, Any]: ...

    @abstractmethod
    def train(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]: ...

    @abstractmethod
    def infer(self, request_id: str, payload: dict[str, Any], context: Any) -> dict[str, Any]: ...


def require_path(value: Any, label: str, *, directory: bool = False) -> Path:
    if not isinstance(value, str) or not value or len(value) > 2000:
        from ..errors import WorkerError

        raise WorkerError("invalidConfiguration", f"{label} is required.")
    path = Path(value)
    valid = path.is_dir() if directory else path.is_file()
    if not valid:
        from ..errors import WorkerError

        raise WorkerError("missingPath", f"{label} is missing.")
    return path.resolve()

