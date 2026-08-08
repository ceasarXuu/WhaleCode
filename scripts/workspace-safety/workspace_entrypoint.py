"""Shared fail-closed preflight for side-effecting Linux entrypoints."""

from __future__ import annotations

import importlib.util
from pathlib import Path
from types import ModuleType


class WorkspacePreflightError(RuntimeError):
    """A stable workspace preflight failure."""


def _load_api() -> ModuleType:
    path = Path(__file__).with_name("workspace_context.py")
    spec = importlib.util.spec_from_file_location("whalecode_workspace_context", path)
    if spec is None or spec.loader is None:
        raise WorkspacePreflightError("workspace_context_unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def require_ready(repo: Path) -> dict[str, object]:
    api = _load_api()
    result = api.require_ready(repo)
    if not result["ready"]:
        raise WorkspacePreflightError(
            f"workspace_not_ready:{result['state']}:{result['reason_code']}"
        )
    return result


def resolve_workspace_binary(repo: Path, requested: Path | None) -> Path:
    api = _load_api()
    gate = api.require_ready(repo)
    if not gate["ready"]:
        raise WorkspacePreflightError(
            f"workspace_not_ready:{gate['state']}:{gate['reason_code']}"
        )
    context = api.resolve_context(repo)
    expected = Path(context["resources"]["binary_dir"]) / "whale"
    if requested is not None and requested.expanduser().resolve(strict=False) != expected:
        raise WorkspacePreflightError("whale_binary_outside_workspace_slot")
    diagnosis = api.run_doctor(repo, require_binary=True)
    if diagnosis["status"] != "passed":
        codes = ",".join(diagnosis["diagnostic_codes"])
        raise WorkspacePreflightError(f"workspace_binary_invalid:{codes}")
    return expected
