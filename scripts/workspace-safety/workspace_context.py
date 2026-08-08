#!/usr/bin/env python3
"""Pure workspace identity and bootstrap state primitives."""

from __future__ import annotations

import hashlib
import os
import re
from collections.abc import Mapping
from pathlib import Path
from typing import Any

MARKER_SCHEMA_VERSION = 1

UNBOOTSTRAPPED = "Unbootstrapped"
READY = "Ready"
STALE = "Stale"
CONFLICT = "Conflict"
DOCTOR_FAILED = "DoctorFailed"


def canonical_path(path: Path | str) -> str:
    """Return the canonical absolute representation used by identity contracts."""

    return str(Path(path).expanduser().resolve(strict=True))


def sanitize_display_name(name: str) -> str:
    """Return a readable ASCII slug without using it as the unique identity."""

    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-._").lower()
    return slug or "workspace"


def derive_workspace_id(canonical_root: Path | str) -> str:
    """Derive a readable ID whose suffix prevents same-basename collisions."""

    root = canonical_path(canonical_root)
    display_name = sanitize_display_name(Path(root).name)
    digest = hashlib.sha256(os.fsencode(root)).hexdigest()[:10]
    return f"{display_name}-{digest}"


def _result(code: str, reason_code: str) -> dict[str, str]:
    return {"code": code, "reason_code": reason_code}


def evaluate_state(
    marker: Mapping[str, Any] | None,
    current: Mapping[str, Any],
) -> dict[str, str]:
    """Evaluate the five-state contract without reading or writing the filesystem."""

    if marker is None:
        return _result(UNBOOTSTRAPPED, "marker_missing")
    required_marker = {
        "schema_version",
        "workspace_id",
        "canonical_root",
        "git_common_dir",
        "branch",
        "resources",
        "last_doctor",
    }
    if not required_marker.issubset(marker):
        return _result(CONFLICT, "marker_invalid")
    if marker.get("schema_version") != MARKER_SCHEMA_VERSION:
        return _result(CONFLICT, "marker_schema_unsupported")
    if marker.get("workspace_id") != current.get("workspace_id"):
        return _result(CONFLICT, "workspace_id_collision")
    if marker.get("canonical_root") != current.get("canonical_root"):
        return _result(CONFLICT, "workspace_root_collision")
    if current.get("detached_head"):
        return _result(STALE, "detached_head")
    if marker.get("git_common_dir") != current.get("git_common_dir"):
        return _result(STALE, "git_common_dir_changed")
    if marker.get("branch") != current.get("branch"):
        return _result(STALE, "branch_changed")
    current_resources = current.get("resources")
    if current_resources is not None and marker.get("resources") != current_resources:
        return _result(STALE, "resource_paths_changed")
    doctor = marker.get("last_doctor")
    if not isinstance(doctor, Mapping) or doctor.get("status") != "passed":
        return _result(DOCTOR_FAILED, "last_doctor_not_passed")
    return _result(READY, "workspace_ready")
