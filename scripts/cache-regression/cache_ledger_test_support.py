#!/usr/bin/env python3
"""Ledger builders shared by cache-regression tests."""

from pathlib import Path

from cache_cost import complete_cost_from_counts
from cache_evidence import file_sha256
from cache_surface import write_json


def write_settled_ledger(
    repo: Path,
    result: dict,
    proposal: dict,
    authorization: dict,
    result_path: Path,
    proposal_path: Path,
    authorization_path: Path,
    *,
    api_requests: int,
    input_tokens: int,
    cached_input_tokens: int,
    output_tokens: int,
) -> None:
    selection = proposal["selection"]
    route = result["provider_route_attestation"]
    route_identity = route["provider_routing"]
    write_json(
        repo / "benchmarks/whale-agent-run-ledger.json",
        {
            "entries": [
                {
                    "record_id": result["record_id"],
                    "status": "settled",
                    "started_at": result["started_at"],
                    "ended_at": result["ended_at"],
                    "elapsed_calendar_seconds": result["elapsed_seconds"],
                    "authorization": {
                        "status": "granted",
                        "id": authorization["authorization_id"],
                        "reference": authorization["approval_reference"],
                        "budget_summary": proposal["maximums"],
                    },
                    "execution": {
                        "provider": route_identity["logical_provider_id"],
                        "transport_provider": route_identity["transport_provider_id"],
                        "provider_descriptor_sha256": route[
                            "provider_descriptor_sha256"
                        ],
                        "model": selection["model"],
                        "sample_ids": selection["samples"],
                        "arm_ids": selection["arms"],
                        "repeats_per_arm_per_sample": selection["repeat"],
                        "planned_sample_runs": selection["planned_sample_runs"],
                        "actual_sample_runs": result["actual_sample_runs"],
                        "api_requests": api_requests,
                        "api_requests_evidence_status": "complete",
                    },
                    "tokens": {
                        "input": input_tokens,
                        "cached_input": cached_input_tokens,
                        "uncached_input": input_tokens - cached_input_tokens,
                        "output": output_tokens,
                    },
                    "monetary_cost": complete_cost_from_counts(
                        input_tokens,
                        cached_input_tokens,
                        output_tokens,
                        proposal["pricing_snapshot"],
                    ),
                    "evidence": {
                        "result_path": result_path.relative_to(repo).as_posix(),
                        "actual_run_root": result["run_root"],
                        "runner_exit_code": 0,
                        "outcome": "completed",
                        "usage_evidence_status": "complete",
                        "proposal_path": proposal_path.relative_to(repo).as_posix(),
                        "proposal_sha256": file_sha256(proposal_path),
                        "authorization_path": authorization_path.relative_to(
                            repo
                        ).as_posix(),
                        "authorization_sha256": file_sha256(authorization_path),
                        "provider_route_attestation_path": route["artifact_path"],
                        "provider_route_attestation_sha256": route["artifact_sha256"],
                    },
                }
            ]
        },
    )
