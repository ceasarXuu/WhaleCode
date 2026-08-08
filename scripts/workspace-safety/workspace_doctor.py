"""Deterministic workspace diagnostics and bounded audit events."""

from __future__ import annotations

import hashlib
import json
import os
import fcntl
from collections.abc import Mapping
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

DOCTOR_SCHEMA_VERSION = 1
AUDIT_SCHEMA_VERSION = 1
MAX_AUDIT_BYTES = 1024 * 1024
EVENT_FIELDS = {
    "schema_version",
    "recorded_at",
    "operation",
    "workspace_id",
    "status",
    "diagnostic_codes",
    "exit_code",
}


def _add(codes: list[str], condition: bool, code: str) -> None:
    if condition:
        codes.append(code)


def _binding_codes(context: Mapping[str, Any], marker: Mapping[str, Any] | None) -> list[str]:
    if marker is None:
        return ["marker_missing"]
    required = {
        "schema_version", "workspace_id", "canonical_root", "git_common_dir",
        "branch", "resources", "last_doctor",
    }
    if not required.issubset(marker):
        return ["marker_invalid"]
    if marker.get("schema_version") != 1:
        return ["marker_schema_unsupported"]
    codes: list[str] = []
    _add(codes, marker.get("workspace_id") != context["workspace_id"], "workspace_id_mismatch")
    _add(codes, marker.get("canonical_root") != context["canonical_root"], "workspace_root_mismatch")
    _add(codes, bool(context["detached_head"]), "detached_head")
    _add(codes, marker.get("git_common_dir") != context["git_common_dir"], "git_common_dir_mismatch")
    _add(codes, marker.get("branch") != context["branch"], "branch_mismatch")
    _add(codes, marker.get("resources") != context["resources"], "resource_paths_mismatch")
    return codes


def _resource_codes(resources: Mapping[str, str]) -> list[str]:
    codes = []
    for name in ("state_root", "runtime_home", "data_root", "binary_dir"):
        path = Path(resources[name])
        if path.is_symlink():
            codes.append(f"{name}_symlink")
        elif not path.is_dir():
            codes.append(f"{name}_missing")
        elif path.stat().st_mode & 0o077:
            codes.append(f"{name}_not_private")
    return codes


def _is_within(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _build_override_codes(context: Mapping[str, Any], environment: Mapping[str, str]) -> list[str]:
    common_root = Path(context["git_common_dir"]).parent.resolve(strict=False)
    workspace_root = Path(context["canonical_root"])
    if common_root == workspace_root:
        return []
    codes = []
    for name in ("CARGO_TARGET_DIR", "BAZEL_OUTPUT_BASE"):
        raw = environment.get(name)
        if not raw:
            continue
        override = Path(raw).expanduser()
        if not override.is_absolute():
            codes.append(f"{name.lower()}_relative")
            continue
        resolved = override.resolve(strict=False)
        if _is_within(resolved, common_root) and not _is_within(resolved, workspace_root):
            codes.append(f"{name.lower()}_shared_common_root")
    return codes


def _binary_codes(context: Mapping[str, Any]) -> list[str]:
    binary = Path(context["resources"]["binary_dir"]) / "whale"
    if not binary.is_file():
        return ["binary_missing"]
    if not os.access(binary, os.X_OK):
        return ["binary_not_executable"]
    attestation_path = Path(f"{binary}.build-attestation.json")
    if not attestation_path.is_file():
        return ["binary_attestation_missing"]
    try:
        attestation = json.loads(attestation_path.read_text(encoding="utf-8-sig"))
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return ["binary_attestation_invalid"]
    valid = (
        isinstance(attestation, dict)
        and attestation.get("schema_version") == 2
        and attestation.get("status") == "pass"
        and Path(str(attestation.get("whale_bin", ""))).resolve(strict=False) == binary.resolve()
        and attestation.get("whale_binary_sha256") == digest
        and attestation.get("repo_root") == context["canonical_root"]
        and attestation.get("worktree_clean") is True
    )
    return [] if valid else ["binary_attestation_invalid"]


def diagnose(
    context: Mapping[str, Any],
    marker: Mapping[str, Any] | None,
    environment: Mapping[str, str],
    require_binary: bool = False,
) -> dict[str, Any]:
    """Return fresh diagnostic evidence without trusting last_doctor."""

    codes = _binding_codes(context, marker)
    if not codes:
        codes.extend(_resource_codes(context["resources"]))
        codes.extend(_build_override_codes(context, environment))
        if require_binary:
            codes.extend(_binary_codes(context))
    codes = sorted(set(codes))
    return {
        "schema_version": DOCTOR_SCHEMA_VERSION,
        "workspace_id": context["workspace_id"],
        "status": "failed" if codes else "passed",
        "diagnostic_codes": codes,
        "require_binary": require_binary,
    }


def append_event(
    state_root: Path,
    event: Mapping[str, Any],
    max_bytes: int = MAX_AUDIT_BYTES,
) -> dict[str, Any]:
    """Append one allowlisted mechanical event without growing beyond max_bytes."""

    if not state_root.is_dir():
        return {"written": False, "reason_code": "audit_state_root_missing"}
    document = {key: event[key] for key in EVENT_FIELDS if key in event}
    document["schema_version"] = AUDIT_SCHEMA_VERSION
    document["recorded_at"] = datetime.now(timezone.utc).isoformat()
    encoded = (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode()
    if len(encoded) > 4096:
        return {"written": False, "reason_code": "audit_event_too_large"}
    log_path = state_root / "workspace-events.jsonl"
    try:
        descriptor = os.open(log_path, os.O_APPEND | os.O_CREAT | os.O_WRONLY, 0o600)
        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX)
            os.fchmod(descriptor, 0o600)
            if os.fstat(descriptor).st_size + len(encoded) > max_bytes:
                return {"written": False, "reason_code": "audit_log_limit_reached"}
            written = os.write(descriptor, encoded)
            if written != len(encoded):
                return {"written": False, "reason_code": "audit_short_write"}
            os.fsync(descriptor)
        finally:
            fcntl.flock(descriptor, fcntl.LOCK_UN)
            os.close(descriptor)
    except OSError:
        return {"written": False, "reason_code": "audit_write_failed"}
    return {"written": True, "reason_code": "audit_event_written"}


def audit_event(operation: str, result: Mapping[str, Any], exit_code: int | None = None) -> dict[str, Any]:
    event = {
        "operation": operation,
        "workspace_id": result["workspace_id"],
        "status": result["status"],
        "diagnostic_codes": result["diagnostic_codes"],
    }
    if exit_code is not None:
        event["exit_code"] = exit_code
    return event
