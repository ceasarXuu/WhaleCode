#!/usr/bin/env python3
"""Global fail-closed dispatcher for worktree-isolated Whale development builds."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

WHALE_DEV_DISPATCHER_SCHEMA = 1
MARKER_SCHEMA_VERSION = 1
ATTESTATION_SCHEMA_VERSION = 2


class DispatchError(RuntimeError):
    """A mechanical dispatch failure with a stable reason code."""

    def __init__(self, code: str, detail: str) -> None:
        super().__init__(detail)
        self.code = code


def _git(start: Path, *args: str) -> str:
    environment = {
        **os.environ,
        "GIT_OPTIONAL_LOCKS": "0",
        "GIT_TERMINAL_PROMPT": "0",
    }
    result = subprocess.run(
        ["git", "-C", str(start), *args],
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )
    if result.returncode != 0:
        raise DispatchError(
            "workspace_not_found", "current directory is not in a Git worktree"
        )
    return result.stdout.strip()


def _environment_root(environment: Mapping[str, str], key: str, fallback: Path) -> Path:
    raw = environment.get(key)
    path = Path(raw).expanduser() if raw else fallback
    if not path.is_absolute():
        raise DispatchError(
            "environment_path_relative", f"{key} must be an absolute path"
        )
    return path.resolve(strict=False)


def _resolve_git_path(root: Path, raw: str) -> str:
    path = Path(raw)
    return str((path if path.is_absolute() else root / path).resolve(strict=False))


def _workspace_id(root: Path) -> str:
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", root.name).strip("-._").lower()
    slug = slug or "workspace"
    digest = hashlib.sha256(os.fsencode(str(root))).hexdigest()[:10]
    return f"{slug}-{digest}"


def resolve_workspace(
    start: Path | str,
    environment: Mapping[str, str],
) -> dict[str, Any]:
    start_path = Path(start).expanduser().resolve(strict=True)
    if start_path.is_file():
        start_path = start_path.parent
    root = Path(_git(start_path, "rev-parse", "--show-toplevel")).resolve(strict=True)
    git_common_dir = _resolve_git_path(
        root, _git(root, "rev-parse", "--git-common-dir")
    )
    branch = _git(root, "symbolic-ref", "--quiet", "--short", "HEAD")
    workspace_id = _workspace_id(root)
    home = _environment_root(environment, "HOME", Path.home())
    xdg_state = _environment_root(environment, "XDG_STATE_HOME", home / ".local/state")
    xdg_data = _environment_root(environment, "XDG_DATA_HOME", home / ".local/share")
    state_root = xdg_state / "whalecode/workspaces" / workspace_id
    resources = {
        "state_root": str(state_root),
        "runtime_home": str(state_root / "home"),
        "data_root": str(xdg_data / "whalecode/workspaces" / workspace_id),
        "binary_dir": str(xdg_data / "whalecode/workspaces" / workspace_id / "bin"),
    }
    return {
        "workspace_id": workspace_id,
        "canonical_root": str(root),
        "git_common_dir": git_common_dir,
        "branch": branch,
        "resources": resources,
        "marker_path": str(state_root / "workspace-identity.json"),
    }


def _load_json(path: Path, missing_code: str, invalid_code: str) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink():
        raise DispatchError(missing_code, str(path))
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise DispatchError(invalid_code, type(error).__name__) from error
    if not isinstance(value, dict):
        raise DispatchError(invalid_code, "expected a JSON object")
    return value


def _validate_private_directories(resources: Mapping[str, str]) -> None:
    for name, raw in resources.items():
        path = Path(raw)
        if path.is_symlink() or not path.is_dir():
            raise DispatchError("workspace_resource_invalid", name)
        if path.stat().st_mode & 0o077:
            raise DispatchError("workspace_resource_not_private", name)


def validate_workspace(context: Mapping[str, Any]) -> Path:
    marker = _load_json(
        Path(context["marker_path"]),
        "workspace_not_bootstrapped",
        "workspace_marker_invalid",
    )
    expected = {
        "schema_version": MARKER_SCHEMA_VERSION,
        "workspace_id": context["workspace_id"],
        "canonical_root": context["canonical_root"],
        "git_common_dir": context["git_common_dir"],
        "branch": context["branch"],
        "resources": context["resources"],
    }
    if any(marker.get(key) != value for key, value in expected.items()):
        raise DispatchError(
            "workspace_marker_stale", "marker does not match this worktree"
        )
    last_doctor = marker.get("last_doctor")
    if not isinstance(last_doctor, dict) or last_doctor.get("status") != "passed":
        raise DispatchError(
            "workspace_doctor_failed", "last workspace doctor did not pass"
        )
    _validate_private_directories(context["resources"])

    binary = Path(context["resources"]["binary_dir"]) / "whale"
    if not binary.is_file() or binary.is_symlink() or not os.access(binary, os.X_OK):
        raise DispatchError("workspace_binary_missing", str(binary))
    attestation = _load_json(
        Path(f"{binary}.build-attestation.json"),
        "workspace_attestation_missing",
        "workspace_attestation_invalid",
    )
    try:
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    except OSError as error:
        raise DispatchError(
            "workspace_binary_unreadable", type(error).__name__
        ) from error
    valid_attestation = (
        attestation.get("schema_version") == ATTESTATION_SCHEMA_VERSION
        and attestation.get("status") == "pass"
        and attestation.get("repo_root") == context["canonical_root"]
        and attestation.get("worktree_clean") is True
        and Path(str(attestation.get("whale_bin", ""))).resolve(strict=False)
        == binary.resolve()
        and attestation.get("whale_binary_sha256") == digest
    )
    if not valid_attestation:
        raise DispatchError(
            "workspace_attestation_invalid", "binary identity check failed"
        )
    return binary


def child_environment(
    context: Mapping[str, Any], parent: Mapping[str, str]
) -> dict[str, str]:
    child = dict(parent)
    runtime_home = context["resources"]["runtime_home"]
    binary_dir = context["resources"]["binary_dir"]
    child["WHALE_HOME"] = runtime_home
    child["CODEX_SQLITE_HOME"] = runtime_home
    child.pop("CODEX_HOME", None)
    remaining = [
        item
        for item in parent.get("PATH", os.defpath).split(os.pathsep)
        if item != binary_dir
    ]
    child["PATH"] = os.pathsep.join([binary_dir, *remaining])
    return child


def dispatch(
    args: Sequence[str],
    start: Path | str,
    environment: Mapping[str, str],
) -> int:
    context = resolve_workspace(start, environment)
    binary = validate_workspace(context)
    child = child_environment(context, environment)
    if list(args) in (["--version"], ["-V"]):
        result = subprocess.run(
            [str(binary), "--version"],
            check=False,
            capture_output=True,
            text=True,
            env=child,
        )
        version = (result.stdout or result.stderr).strip()
        if result.returncode != 0 or not version:
            raise DispatchError(
                "workspace_version_probe_failed", str(result.returncode)
            )
        print(f"whale-dev {version} [{context['workspace_id']}]")
        return 0
    os.execve(binary, ["whale-dev", *args], child)
    return 0


def main() -> int:
    try:
        return dispatch(sys.argv[1:], Path.cwd(), os.environ)
    except DispatchError as error:
        print(f"whale-dev: {error.code}: {error}", file=sys.stderr)
        print(
            "Run workspace bootstrap and `bash scripts/install-whale-local.sh --scope workspace` in the target worktree.",
            file=sys.stderr,
        )
        return 2
    except OSError as error:
        print(f"whale-dev: launch_failed: {type(error).__name__}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
