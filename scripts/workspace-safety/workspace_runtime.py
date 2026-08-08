"""Ready-only subprocess environment isolation for workspace commands."""

from __future__ import annotations

import os
import subprocess
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any, Callable

MANAGED_ENV_KEYS = {"WHALE_HOME", "CODEX_SQLITE_HOME", "CODEX_HOME", "PATH"}


class ExecError(RuntimeError):
    """A pre-launch failure with a stable mechanical reason code."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def build_child_environment(
    context: Mapping[str, Any], parent: Mapping[str, str]
) -> dict[str, str]:
    """Copy the parent and change only the managed workspace environment keys."""

    child = dict(parent)
    runtime_home = context["resources"]["runtime_home"]
    binary_dir = context["resources"]["binary_dir"]
    child["WHALE_HOME"] = runtime_home
    child["CODEX_SQLITE_HOME"] = runtime_home
    child.pop("CODEX_HOME", None)
    original_path = parent.get("PATH", os.defpath)
    remaining = [item for item in original_path.split(os.pathsep) if item != binary_dir]
    child["PATH"] = os.pathsep.join([binary_dir, *remaining])
    return child


def launch(
    command: Sequence[str],
    context: Mapping[str, Any],
    parent_environment: Mapping[str, str],
    executor: Callable[..., Any] = subprocess.run,
) -> int:
    """Launch a command in the canonical workspace with an isolated child env."""

    if not command:
        raise ExecError("command_missing", "exec requires a command after --")
    child_environment = build_child_environment(context, parent_environment)
    try:
        completed = executor(
            list(command),
            cwd=Path(context["canonical_root"]),
            env=child_environment,
            check=False,
        )
    except OSError as error:
        raise ExecError("command_launch_failed", type(error).__name__) from error
    return int(completed.returncode)
