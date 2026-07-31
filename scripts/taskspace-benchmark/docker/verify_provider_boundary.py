#!/usr/bin/env python3
"""Reconcile provider-boundary dispatches with Whale provider wire evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "whalecode-provider-boundary-evidence-v1"


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        return []
    values = []
    for line_number, raw in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
        if not raw.strip():
            continue
        value = json.loads(raw)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_number}: JSON object required")
        values.append(value)
    return values


def reconcile(events_path: Path, wire_path: Path, expected_model: str) -> dict[str, Any]:
    claimed = [
        {
            "count": event.get("count"),
            "method": event.get("method"),
            "path": event.get("path"),
            "model": event.get("model"),
            "body_sha256": event.get("body_sha256"),
        }
        for event in read_jsonl(events_path)
        if event.get("event") == "provider_request_claimed"
    ]
    wire = [
        {
            "request_id": event.get("request_id"),
            "request_count_after": event.get("request_count_after"),
            "provider_payload_sha256": event.get("provider_payload_sha256"),
        }
        for event in read_jsonl(wire_path)
        if event.get("status") == "payload_captured"
        and event.get("provider_payload_sha256")
    ]
    errors = []
    for index, request in enumerate(claimed, 1):
        if request["count"] != index:
            errors.append(f"boundary_count_sequence_invalid:{index}")
        if request["method"] != "POST" or request["path"] != "/responses":
            errors.append(f"boundary_route_invalid:{index}")
        if request["model"] != expected_model:
            errors.append(f"boundary_model_invalid:{index}")
    boundary_hashes = [request["body_sha256"] for request in claimed]
    wire_hashes = [request["provider_payload_sha256"] for request in wire]
    if boundary_hashes != wire_hashes:
        errors.append("provider_dispatch_trace_mismatch")
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "reconciled" if not errors else "mismatch",
        "expected_model": expected_model,
        "allowed_method": "POST",
        "allowed_path": "/responses",
        "boundary_request_count": len(claimed),
        "wire_request_count": len(wire),
        "boundary_requests": claimed,
        "wire_requests": wire,
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
    return 0 if result["status"] == "reconciled" else 3


if __name__ == "__main__":
    raise SystemExit(main())
