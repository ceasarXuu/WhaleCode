"""Private, atomic persistence helpers for workspace bootstrap."""

from __future__ import annotations

import json
import os
import tempfile
from collections.abc import Mapping
from pathlib import Path
from typing import Any


class ApplyError(RuntimeError):
    """An apply failure with a stable mechanical reason code."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def _assert_safe_target(path: Path, boundary: Path) -> None:
    try:
        path.relative_to(boundary)
    except ValueError as error:
        raise ApplyError("resource_outside_xdg_root", str(path)) from error
    current = path
    while current != boundary:
        if current.is_symlink():
            raise ApplyError("resource_path_symlink", str(current))
        current = current.parent


def _private_directory(path: Path, boundary: Path) -> str:
    _assert_safe_target(path, boundary)
    if path.exists() and not path.is_dir():
        raise ApplyError("resource_not_directory", str(path))
    disposition = "reused" if path.is_dir() else "created"
    try:
        path.mkdir(mode=0o700, parents=True, exist_ok=True)
        os.chmod(path, 0o700)
    except OSError as error:
        raise ApplyError("resource_directory_failed", str(path)) from error
    return disposition


def ensure_resource_directories(resources: Mapping[str, str]) -> list[dict[str, str]]:
    """Create the four derived workspace resource directories with mode 0700."""

    required = ("state_root", "runtime_home", "data_root", "binary_dir")
    if set(resources) != set(required):
        raise ApplyError("resource_contract_invalid", "resource keys do not match schema")
    state_root = Path(resources["state_root"])
    data_root = Path(resources["data_root"])
    boundaries = {
        "state_root": state_root.parents[2],
        "runtime_home": state_root.parents[2],
        "data_root": data_root.parents[2],
        "binary_dir": data_root.parents[2],
    }
    results = []
    for name in required:
        path = Path(resources[name])
        results.append(
            {"resource": name, "path": str(path), "disposition": _private_directory(path, boundaries[name])}
        )
    return results


def atomic_write_json(path: Path, document: Mapping[str, Any]) -> str:
    """Atomically install canonical JSON with mode 0600, skipping identical bytes."""

    encoded = (
        json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    existed = path.exists()
    if path.is_symlink():
        raise ApplyError("marker_path_symlink", str(path))
    if path.exists():
        try:
            if path.read_bytes() == encoded:
                os.chmod(path, 0o600)
                return "reused"
        except OSError as error:
            raise ApplyError("marker_read_failed", str(path)) from error
    descriptor = -1
    temporary_name = ""
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            dir=path.parent, prefix=".workspace-identity.", suffix=".tmp"
        )
        os.fchmod(descriptor, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_name, path)
        temporary_name = ""
        os.chmod(path, 0o600)
        directory_fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    except OSError as error:
        raise ApplyError("marker_write_failed", str(path)) from error
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        if temporary_name:
            try:
                os.unlink(temporary_name)
            except FileNotFoundError:
                pass
    return "updated" if existed else "created"
