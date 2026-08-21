"""Pure lightweight Ready gate for side-effecting entrypoints."""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any

GATE_SCHEMA_VERSION = 1


def evaluate_gate(
    marker: Mapping[str, Any] | None,
    current: Mapping[str, Any],
) -> dict[str, str]:
    if marker is None:
        return {"state": "Unbootstrapped", "reason_code": "marker_missing"}
    required = {
        "schema_version", "workspace_id", "canonical_root", "branch", "last_doctor"
    }
    if not required.issubset(marker):
        return {"state": "Conflict", "reason_code": "marker_invalid"}
    if marker.get("schema_version") != 1:
        return {"state": "Conflict", "reason_code": "marker_schema_unsupported"}
    if marker.get("workspace_id") != current.get("workspace_id"):
        return {"state": "Conflict", "reason_code": "workspace_id_collision"}
    if marker.get("canonical_root") != current.get("canonical_root"):
        return {"state": "Conflict", "reason_code": "workspace_root_collision"}
    if current.get("detached_head"):
        return {"state": "Stale", "reason_code": "detached_head"}
    if marker.get("branch") != current.get("branch"):
        return {"state": "Stale", "reason_code": "branch_changed"}
    last_doctor = marker.get("last_doctor")
    if not isinstance(last_doctor, Mapping) or last_doctor.get("status") != "passed":
        return {"state": "DoctorFailed", "reason_code": "last_doctor_not_passed"}
    return {"state": "Ready", "reason_code": "workspace_ready"}


def gate_result(marker: Mapping[str, Any] | None, current: Mapping[str, Any]) -> dict[str, Any]:
    state = evaluate_gate(marker, current)
    return {
        "schema_version": GATE_SCHEMA_VERSION,
        "workspace_id": current["workspace_id"],
        "ready": state["state"] == "Ready",
        **state,
        "recovery_command": (
            "python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json"
        ),
    }
