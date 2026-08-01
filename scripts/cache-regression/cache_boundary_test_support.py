#!/usr/bin/env python3
"""Provider-boundary evidence builders shared by cache control-plane tests."""

from pathlib import Path

from cache_surface import write_json


def write_provider_boundary_evidence(
    path: Path, request_count: int, model: str = "deepseek-v4-flash"
) -> None:
    hashes = [f"{index:064x}" for index in range(1, request_count + 1)]
    write_json(
        path,
        {
            "schema_version": "whalecode-provider-boundary-evidence-v1",
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
