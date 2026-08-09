#!/usr/bin/env python3
"""Provider-boundary evidence builders shared by cache control-plane tests."""

import hashlib
import json
from pathlib import Path

from cache_surface import write_json


def write_provider_boundary_evidence(
    path: Path,
    request_count: int,
    model: str = "deepseek-v4-flash",
    identity: str = "fixture",
) -> None:
    hashes = [
        hashlib.sha256(f"{identity}:{index}".encode()).hexdigest()
        for index in range(1, request_count + 1)
    ]
    write_json(
        path,
        {
            "schema_version": "whalecode-provider-boundary-evidence-v2",
            "status": "reconciled",
            "expected_model": model,
            "allowed_method": "POST",
            "allowed_path": "/responses",
            "boundary_request_count": request_count,
            "wire_request_count": request_count,
            "boundary_requests": [
                {
                    "count": index,
                    "method": "POST",
                    "path": "/responses",
                    "model": model,
                    "body_sha256": digest,
                }
                for index, digest in enumerate(hashes, 1)
            ],
            "wire_requests": [
                {
                    "request_id": f"request-{index}",
                    "request_count_after": index,
                    "provider_payload_sha256": digest,
                }
                for index, digest in enumerate(hashes, 1)
            ],
            "errors": [],
        },
    )


def write_provider_wire_trace(
    path: Path,
    usage_records: list[dict[str, int]],
    identity: str = "fixture",
) -> None:
    lines = []
    for index, usage in enumerate(usage_records, 1):
        request_id = f"request-{index}"
        digest = hashlib.sha256(f"{identity}:{index}".encode()).hexdigest()
        completed_usage = {
            "reasoning_output_tokens": 0,
            "total_tokens": usage["input_tokens"] + usage["output_tokens"],
            **usage,
        }
        lines.extend(
            (
                {
                    "schema_version": "provider-chat-wire-trace-v11",
                    "status": "payload_captured",
                    "request_id": request_id,
                    "logical_request_id": request_id,
                    "attempt_seq": 1,
                    "request_index": index,
                    "provider_payload_sha256": digest,
                },
                {
                    "schema_version": "provider-chat-wire-trace-v11",
                    "status": "response_completed",
                    "request_id": request_id,
                    "logical_request_id": request_id,
                    "attempt_seq": 1,
                    **completed_usage,
                },
            )
        )
    path.write_text(
        "".join(
            json.dumps(line, ensure_ascii=False, separators=(",", ":")) + "\n"
            for line in lines
        ),
        encoding="utf-8",
    )
