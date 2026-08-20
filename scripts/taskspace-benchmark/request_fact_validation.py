"""Mechanical ordering checks for normalized provider attempts."""

from __future__ import annotations

from collections import defaultdict
from typing import Any


def validate_attempt_sequences(
    rows: dict[str, dict[str, Any]],
    findings: list[dict[str, Any]],
    completed_status: str,
) -> None:
    attempts = [row for row in rows.values() if "attempt" in row]
    indexes = sorted(row["attempt"]["request_index"] for row in attempts)
    if indexes != list(range(1, len(indexes) + 1)):
        findings.append({"code": "attempt_index_sequence_invalid", "source": "wire"})
    logical: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in attempts:
        identity = row.get("identity")
        if identity:
            logical[identity[0]].append(row)
    for logical_id, group in logical.items():
        ordered = sorted(group, key=lambda row: row["attempt"]["request_index"])
        sequences = [row["identity"][1] for row in ordered]
        if sequences != list(range(1, len(sequences) + 1)):
            findings.append(
                {
                    "code": "logical_attempt_sequence_invalid",
                    "source": "wire",
                    "logical_request_id": logical_id,
                }
            )
        completed = [row for row in ordered if row.get("terminal", {}).get("status") == completed_status]
        if len(completed) > 1 or (completed and completed[0] is not ordered[-1]):
            findings.append(
                {
                    "code": "logical_completion_sequence_invalid",
                    "source": "wire",
                    "logical_request_id": logical_id,
                }
            )
