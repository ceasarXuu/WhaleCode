#!/usr/bin/env python3
"""Construct one production-shaped accepted transition for gate tests."""

from __future__ import annotations

import subprocess
from pathlib import Path

from accepted_cache_baseline import ACCEPTANCE_SCHEMA_VERSION
from cache_budget import BUDGET_PROPOSAL_SCHEMA_VERSION
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256, file_sha256
from cache_run_analysis import analyze_artifacts
from cache_run_contract import AUTHORIZATION_SCHEMA_VERSION
from cache_source_evidence import protected_manifest
from cache_surface import load_contract, surface_snapshot, write_json


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


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


def proposal_id(proposal: dict) -> str:
    identity = {key: value for key, value in proposal.items() if key != "proposal_id"}
    return f"CBP-{canonical_json_sha256(identity)[:16].upper()}"


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
                        "model": selection["model"],
                        "sample_ids": selection["samples"],
                        "arm_ids": selection["arms"],
                        "repeats_per_arm_per_sample": selection["repeat"],
                        "planned_sample_runs": selection["planned_sample_runs"],
                        "actual_sample_runs": result["actual_sample_runs"],
                        "api_requests": api_requests,
                    },
                    "tokens": {
                        "input": input_tokens,
                        "cached_input": cached_input_tokens,
                        "uncached_input": input_tokens - cached_input_tokens,
                        "output": output_tokens,
                    },
                    "monetary_cost": {"status": "estimated"},
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
                    },
                }
            ]
        },
    )


def stage_accepted_promotion(repo: Path, contract_path: Path) -> None:
    snapshot = repo / "snapshots/baseline.snap"
    before = {"wire": "stable"}
    after = {"wire": "accepted"}
    snapshot.write_text(
        '---\nsource: fixture.rs\n---\n{"wire": "accepted"}\n', encoding="utf-8"
    )
    record_id = "WAR-ACCEPTED"
    evidence_root = repo / f"benchmarks/cache-regression/evidence/{record_id}"
    artifacts_root = evidence_root / "CACHE-001/artifacts"
    artifacts_root.mkdir(parents=True)
    cache_path = artifacts_root / "provider-cache-trace-summary.json"
    request_path = artifacts_root / "request-summary.json"
    metrics_path = artifacts_root / "metrics.json"
    boundary_path = artifacts_root / "provider-boundary-evidence.json"
    write_json(
        cache_path,
        {
            "provider_request_count": 3,
            "request_2_plus_count": 2,
            "request_2_plus_cached_input_tokens": 90,
            "request_2_plus_uncached_input_tokens": 10,
            "request_2_plus_hit_rate": 0.9,
            "trace_coverage": 1.0,
            "cache_usage_missing_count": 0,
        },
    )
    write_json(
        request_path,
        {
            "rollout_trace": {
                "input_tokens": 100,
                "cached_input_tokens": 90,
                "output_tokens": 10,
            }
        },
    )
    write_json(
        metrics_path,
        {"logical_mode": "standard", "business_success": True},
    )
    write_provider_boundary_evidence(boundary_path, 3)
    observation = analyze_artifacts(
        cache_path,
        request_path,
        metrics_path,
        boundary_path,
        "standard",
        "deepseek-v4-flash",
    )
    observation["artifacts"] = {
        key: Path(path).relative_to(repo).as_posix()
        for key, path in observation["artifacts"].items()
    }
    observation.update(
        {
            "sample": "simple",
            "repeat": 1,
            "elapsed_seconds": 1.0,
            "budget_observation_exceeded": [],
            "run_id": "CACHE-001",
        }
    )

    contract = load_contract(contract_path)
    git(repo, "add", "snapshots")
    surface, _ = surface_snapshot(repo, contract, "index")
    gate_path = evidence_root / "gate.json"
    scenario = {
        "scenario_id": "baseline",
        "comparison_object": "normalized_final_wire_snapshot",
        "baseline_path": "snapshots/baseline.snap",
        "status": "changed",
        "first_difference": "/wire",
        "before_payload_sha256": canonical_json_sha256(before),
        "after_payload_sha256": canonical_json_sha256(after),
        "candidate_payload": after,
    }
    write_json(
        gate_path,
        {
            "schema_version": "whalecode-cache-regression-gate-v1",
            "status": "blocked",
            "discovery_state": "changed",
            "subject_commit": git(repo, "rev-parse", "HEAD"),
            "actual_surface_sha256": surface,
            "free_validation": {
                "passed": False,
                "commands": [
                    {
                        "id": "fixture_final_wire",
                        "status": "fail",
                        "change_report": {
                            "status": "changed",
                            "scenarios": [scenario],
                        },
                    }
                ],
            },
        },
    )

    selection = {
        "model": "deepseek-v4-flash",
        "samples": ["simple"],
        "arms": ["standard"],
        "repeat": 1,
        "planned_sample_runs": 1,
        "retry_sample_run_limit": 0,
        "maximum_sample_runs": 1,
        "stop_conditions": ["after_any_run_failure"],
        "selection_reason": "production-shaped fixture",
    }
    provider_hard_limits = {
        "max_input_tokens_per_request": 1_000_000,
        "max_output_tokens_per_request": 384_000,
    }
    maximums = {
        "provider_requests": 10,
        "input_tokens": 10_000_000,
        "output_tokens": 3_840_000,
        "elapsed_seconds": 240,
        "estimated_cost": 2.4752,
        "currency": "USD",
    }
    proposal = {
        "schema_version": BUDGET_PROPOSAL_SCHEMA_VERSION,
        "created_at": "2026-07-31T12:00:00+08:00",
        "subject_commit": git(repo, "rev-parse", "HEAD"),
        "surface_sha256": surface,
        "trigger": {
            "gate_report_path": gate_path.relative_to(repo).as_posix(),
            "gate_report_sha256": file_sha256(gate_path),
            "failed_free_commands": ["fixture_final_wire"],
        },
        "selection": selection,
        "per_sample_run_limits": {
            "provider_requests": 10,
            "elapsed_seconds": 120,
            "cleanup_grace_seconds": 120,
        },
        "per_sample_run_observation_thresholds": {
            "input_tokens": 100_000,
            "output_tokens": 5_000,
        },
        "observation_threshold_totals": {
            "input_tokens": 100_000,
            "output_tokens": 5_000,
            "estimated_cost": 0.0154,
            "currency": "USD",
        },
        "maximums": maximums,
        "pricing_snapshot": {
            "currency": "USD",
            "cached_input_per_million": 0.0028,
            "uncached_input_per_million": 0.14,
            "output_per_million": 0.28,
        },
        "provider_hard_limits": provider_hard_limits,
        "cost_assumption": (
            "hard request count multiplied by provider maximum input/output tokens; "
            "all input priced as cache miss"
        ),
        "enforcement_boundary": {
            "hard_before_start": [
                "subject_commit",
                "surface_sha256",
                "proposal_and_authorization_identity",
                "sample_arm_repeat_matrix",
                "no_automatic_retries",
            ],
            "hard_during_run": ["provider_requests", "elapsed_seconds"],
            "bounded_cleanup_after_run": ["cleanup_grace_seconds"],
            "hard_by_provider_per_request": ["input_tokens", "output_tokens"],
            "observed_after_each_run": [
                "input_tokens",
                "output_tokens",
                "estimated_cost",
            ],
        },
        "evidence_boundary": "fixture scope only",
    }
    proposal["proposal_id"] = proposal_id(proposal)
    proposal["proposal_sha256"] = canonical_json_sha256(proposal)
    proposal_path = evidence_root / "proposal.json"
    write_json(proposal_path, proposal)

    authorization = {
        "schema_version": AUTHORIZATION_SCHEMA_VERSION,
        "status": "granted",
        "approved_by": "user",
        "authorization_id": "CBA-FIXTURE-001",
        "approval_reference": "user approved fixture",
        "approved_at": "2026-07-31T12:00:00+08:00",
        "proposal_id": proposal["proposal_id"],
        "proposal_sha256": proposal["proposal_sha256"],
        "approved_selection": selection,
        "approved_maximums": maximums,
    }
    authorization_path = evidence_root / "authorization.json"
    write_json(authorization_path, authorization)
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_id": record_id,
        "status": "completed",
        "started_at": "2026-07-31T12:00:00+08:00",
        "ended_at": "2026-07-31T12:00:01+08:00",
        "elapsed_seconds": 1.0,
        "subject_commit": proposal["subject_commit"],
        "surface_sha256": surface,
        "proposal_id": proposal["proposal_id"],
        "proposal_sha256": proposal["proposal_sha256"],
        "authorization_reference": authorization["approval_reference"],
        "authorization_sha256": file_sha256(authorization_path),
        "observed_scope": selection,
        "unverified_scope": [],
        "actual_sample_runs": 1,
        "credential_source": "fixture",
        "run_root": "target/cache-fixture",
        "observations": [observation],
        "attempts": [
            {
                "sample": "simple",
                "arm": "standard",
                "repeat": 1,
                "run_id": "CACHE-001",
                "status": "completed",
                "exit_code": 0,
                "timed_out": False,
                "elapsed_seconds": 1.0,
                "post_run_cleanup": {
                    "status": "verified_absent",
                    "container_ids": [],
                    "stable_empty_polls": 3,
                    "error": "",
                },
            }
        ],
        "evidence_sha256": canonical_json_sha256(
            [
                {
                    "sample": "simple",
                    "arm": "standard",
                    "repeat": 1,
                    "artifact_sha256": observation["artifact_sha256"],
                }
            ]
        ),
    }
    result_path = evidence_root / "result.json"
    result["result_path"] = result_path.relative_to(repo).as_posix()
    result["runner_exit_code"] = 0
    write_json(result_path, result)
    acceptance = {
        "schema_version": ACCEPTANCE_SCHEMA_VERSION,
        "status": "accepted",
        "accepted_by": "user",
        "accepted_at": "2026-07-31T13:00:00+08:00",
        "acceptance_reference": "user accepted fixture",
        "result_path": result_path.relative_to(repo).as_posix(),
        "result_sha256": file_sha256(result_path),
        "proposal_path": proposal_path.relative_to(repo).as_posix(),
        "authorization_path": authorization_path.relative_to(repo).as_posix(),
        "accepted_scope": selection,
        "acknowledged_unverified_scope": [],
        "accepted_scenarios": [
            {
                "scenario_id": "baseline",
                "after_payload_sha256": scenario["after_payload_sha256"],
            }
        ],
    }
    acceptance_path = evidence_root / "acceptance.json"
    write_json(acceptance_path, acceptance)
    write_json(
        repo / "benchmarks/whale-agent-run-ledger.json",
        {
            "entries": [
                {
                    "record_id": record_id,
                    "status": "settled",
                    "started_at": result["started_at"],
                    "ended_at": result["ended_at"],
                    "elapsed_calendar_seconds": result["elapsed_seconds"],
                    "authorization": {
                        "status": "granted",
                        "id": authorization["authorization_id"],
                        "reference": authorization["approval_reference"],
                        "budget_summary": maximums,
                    },
                    "execution": {
                        "model": selection["model"],
                        "sample_ids": selection["samples"],
                        "arm_ids": selection["arms"],
                        "repeats_per_arm_per_sample": selection["repeat"],
                        "planned_sample_runs": 1,
                        "actual_sample_runs": 1,
                        "api_requests": 3,
                    },
                    "tokens": {
                        "input": 100,
                        "cached_input": 90,
                        "uncached_input": 10,
                        "output": 10,
                    },
                    "monetary_cost": {"status": "estimated"},
                    "evidence": {
                        "result_path": acceptance["result_path"],
                        "actual_run_root": result["run_root"],
                        "runner_exit_code": 0,
                        "outcome": "completed",
                        "usage_evidence_status": "complete",
                        "proposal_path": acceptance["proposal_path"],
                        "proposal_sha256": file_sha256(proposal_path),
                        "authorization_path": acceptance["authorization_path"],
                        "authorization_sha256": file_sha256(authorization_path),
                    },
                }
            ]
        },
    )
    git(repo, "add", "benchmarks")
    manifest = protected_manifest(repo, contract, "index")
    contract["baseline"] = {
        "status": "accepted",
        "surface_sha256": surface,
        "source_commit": result["subject_commit"],
        "acceptance_reference": acceptance["acceptance_reference"],
        "acceptance_evidence": {
            "path": acceptance_path.relative_to(repo).as_posix(),
            "sha256": file_sha256(acceptance_path),
        },
        "final_wire_manifest": manifest,
        "final_wire_manifest_sha256": canonical_json_sha256(manifest),
        "smoke_evidence": {
            "result_path": acceptance["result_path"],
            "result_sha256": acceptance["result_sha256"],
            "proposal_id": proposal["proposal_id"],
            **selection,
            "actual_sample_runs": 1,
            "unverified_scope": [],
        },
    }
    write_json(contract_path, contract)
    git(repo, "add", contract_path.name, "benchmarks", "snapshots")
