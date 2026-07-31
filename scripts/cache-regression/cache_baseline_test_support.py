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


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


def proposal_id(proposal: dict) -> str:
    identity = {key: value for key, value in proposal.items() if key != "proposal_id"}
    return f"CBP-{canonical_json_sha256(identity)[:16].upper()}"


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
    observation = analyze_artifacts(cache_path, request_path, metrics_path, "standard")
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
    maximums = {
        "provider_requests": 10,
        "input_tokens": 100_000,
        "output_tokens": 5_000,
        "elapsed_seconds": 120,
        "estimated_cost": 1.0,
        "currency": "USD",
    }
    proposal = {
        "schema_version": BUDGET_PROPOSAL_SCHEMA_VERSION,
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
            "input_tokens": 100_000,
            "output_tokens": 5_000,
            "elapsed_seconds": 120,
        },
        "maximums": maximums,
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
        "subject_commit": proposal["subject_commit"],
        "surface_sha256": surface,
        "proposal_id": proposal["proposal_id"],
        "proposal_sha256": proposal["proposal_sha256"],
        "authorization_reference": authorization["approval_reference"],
        "authorization_sha256": file_sha256(authorization_path),
        "observed_scope": selection,
        "unverified_scope": [],
        "actual_sample_runs": 1,
        "observations": [observation],
        "attempts": [
            {
                "sample": "simple",
                "arm": "standard",
                "repeat": 1,
                "status": "completed",
                "exit_code": 0,
                "timed_out": False,
                "elapsed_seconds": 1.0,
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
    write_json(result_path, result)
    acceptance = {
        "schema_version": ACCEPTANCE_SCHEMA_VERSION,
        "status": "accepted",
        "accepted_by": "user",
        "accepted_at": "2026-08-01T13:00:00+08:00",
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
                    "authorization": {
                        "id": authorization["authorization_id"],
                        "reference": authorization["approval_reference"],
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
                    "evidence": {
                        "result_path": acceptance["result_path"],
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
