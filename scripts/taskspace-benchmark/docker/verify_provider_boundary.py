#!/usr/bin/env python3
"""Render provider-boundary evidence from canonical request facts."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from request_facts import ANALYZER_VERSION, build_request_facts  # noqa: E402

SCHEMA_VERSION = "whalecode-provider-boundary-evidence-v2"


def reconcile(events_path: Path, wire_path: Path, expected_model: str) -> dict[str, object]:
    facts = build_request_facts(
        wire_path=wire_path,
        boundary_path=events_path,
        expected_model=expected_model,
    )
    boundary = [
        {key: claim[key] for key in ("count", "method", "path", "model", "body_sha256")}
        for claim in facts["boundary_claims"]
    ]
    attempts = [row for row in facts["rows"] if row["attempt_status"] == "observed"]
    crossed = [row for row in attempts if row["boundary_status"] == "observed"]
    errors = sorted({finding["code"] for finding in facts["findings"]})
    boundary_measured = facts["availability"]["boundary"] == "measured"
    correlation_measured = facts["availability"]["boundary_correlation"] == "measured"
    ambiguity_only = boundary_measured and set(errors) <= {"boundary_correlation_ambiguous"}
    status = (
        "reconciled"
        if boundary_measured and correlation_measured
        else "reconciled_correlation_incomparable"
        if ambiguity_only
        else "mismatch"
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "request_facts_analyzer_version": ANALYZER_VERSION,
        "status": status,
        "boundary_availability": facts["availability"]["boundary"],
        "boundary_correlation_availability": facts["availability"]["boundary_correlation"],
        "expected_model": expected_model,
        "allowed_method": "POST",
        "allowed_path": "/responses",
        "boundary_request_count": facts["summary"]["boundary_request_count"],
        "wire_request_count": len(crossed) if correlation_measured else None,
        "local_attempt_count": len(attempts),
        "local_only_attempt_count": facts["summary"]["local_only_attempt_count"],
        "boundary_requests": boundary,
        "wire_requests": [
            {
                "request_id": row["request_id"],
                "request_count_after": row["boundary_index"],
                "provider_payload_sha256": row["provider_payload_sha256"],
            }
            for row in crossed
        ],
        "local_attempts": [
            {
                "request_id": row["request_id"],
                "logical_request_id": row["logical_request_id"],
                "attempt_seq": row["attempt_seq"],
                "request_index": row["request_index"],
                "provider_payload_sha256": row["provider_payload_sha256"],
                "boundary_status": row["boundary_status"],
                "terminal_status": row["terminal_status"],
            }
            for row in attempts
        ],
        "errors": errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--events", type=Path, required=True)
    parser.add_argument("--wire", type=Path, required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = reconcile(args.events, args.wire, args.model)
    args.output.write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return 0 if str(result["status"]).startswith("reconciled") else 3


if __name__ == "__main__":
    raise SystemExit(main())
