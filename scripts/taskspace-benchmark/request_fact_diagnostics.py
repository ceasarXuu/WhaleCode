"""Build payload-free diagnostics for canonical request fact normalization."""

from __future__ import annotations

from collections import Counter
from typing import Any


def build_diagnostics(
    normalized: list[dict[str, Any]],
    claims: list[dict[str, Any]],
    findings: list[dict[str, Any]],
    counters: Counter[str],
    source_event_counts: dict[str, int],
) -> dict[str, Any]:
    finding_codes = Counter(finding["code"] for finding in findings)
    finding_sources = Counter(finding["source"] for finding in findings)
    attempts = [row for row in normalized if row["attempt_status"] == "observed"]
    return {
        "schema_version": "whalecode-request-facts-diagnostics-v1",
        "source_event_counts": source_event_counts,
        "normalized_counts": {
            "row_count": len(normalized),
            "attempt_count": len(attempts),
            "boundary_observed_count": sum(
                row["boundary_status"] == "observed" for row in normalized
            ),
            "completed_count": sum(
                row["terminal_status"] == "response_completed" for row in normalized
            ),
            "failed_or_cancelled_count": sum(
                row["terminal_status"]
                in {"response_failed", "cancelled", "retry_unauthorized"}
                for row in normalized
            ),
            "usage_count": sum(row["usage"] is not None for row in normalized),
        },
        "exclusions": {
            "state_snapshot_count": counters["state_snapshot_count"],
            "duplicate_event_count": counters["duplicate_event_count"],
        },
        "reconciliation": {
            "boundary_claim_count": len(claims),
            "boundary_matched_count": sum(
                row["boundary_status"] == "observed" for row in normalized
            ),
            "boundary_unattributed_count": finding_codes["boundary_unattributed"],
            "local_only_attempt_count": sum(
                row["boundary_status"] == "not_observed" for row in attempts
            ),
        },
        "findings": {
            "total_count": len(findings),
            "by_code": dict(sorted(finding_codes.items())),
            "by_source": dict(sorted(finding_sources.items())),
        },
        "evidence_source_refs": ["rollout", "wire", "boundary"],
    }
