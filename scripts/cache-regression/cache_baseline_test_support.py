#!/usr/bin/env python3
"""Test-only construction of one valid accepted-baseline transition."""

from __future__ import annotations

import subprocess
from pathlib import Path

from accepted_cache_baseline import protected_manifest
from cache_budget import BUDGET_PROPOSAL_SCHEMA_VERSION
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256, file_sha256
from cache_run_contract import AUTHORIZATION_SCHEMA_VERSION
from cache_surface import load_contract, surface_snapshot, write_json


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


def stage_accepted_promotion(repo: Path, contract_path: Path) -> None:
    snapshot = repo / "snapshots/baseline.snap"
    snapshot.write_text(
        '---\nsource: fixture.rs\n---\n{"wire": "accepted"}\n', encoding="utf-8"
    )
    record_id = "WAR-ACCEPTED"
    root = repo / f"benchmarks/cache-regression/evidence/{record_id}/CACHE-001"
    root.mkdir(parents=True)
    artifacts = {}
    hashes = {}
    for key, filename in {
        "cache_summary": "provider-cache-trace-summary.json",
        "request_summary": "request-summary.json",
        "metrics": "metrics.json",
    }.items():
        path = root / filename
        write_json(path, {"fixture": key})
        artifacts[key] = path.relative_to(repo).as_posix()
        hashes[key] = file_sha256(path)
    proposal_path = repo / "benchmarks/cache-regression/proposal.json"
    authorization_path = repo / "benchmarks/cache-regression/authorization.json"
    result_path = repo / "benchmarks/cache-regression/results/result.json"
    acceptance_path = repo / "benchmarks/cache-regression/acceptance.json"
    scope = {
        "model": "deepseek-v4-flash",
        "samples": ["simple"],
        "arms": ["standard"],
        "repeat": 1,
        "planned_sample_runs": 1,
    }
    proposal = {
        "schema_version": BUDGET_PROPOSAL_SCHEMA_VERSION,
        "proposal_id": "CBP-ACCEPTED",
        "subject_commit": git(repo, "rev-parse", "HEAD"),
        "surface_sha256": None,
        "selection": scope,
        "maximums": {"provider_requests": 10},
    }
    authorization = {
        "schema_version": AUTHORIZATION_SCHEMA_VERSION,
        "status": "granted",
        "approved_by": "user",
        "approval_reference": "user approved fixture",
        "proposal_id": proposal["proposal_id"],
    }
    git(repo, "add", "snapshots")
    contract = load_contract(contract_path)
    surface, _ = surface_snapshot(repo, contract, "index")
    proposal["surface_sha256"] = surface
    proposal["proposal_sha256"] = canonical_json_sha256(proposal)
    authorization.update(
        {
            "proposal_sha256": proposal["proposal_sha256"],
            "approved_selection": scope,
            "approved_maximums": proposal["maximums"],
        }
    )
    write_json(proposal_path, proposal)
    write_json(authorization_path, authorization)
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_id": record_id,
        "status": "completed",
        "subject_commit": git(repo, "rev-parse", "HEAD"),
        "surface_sha256": surface,
        "proposal_id": proposal["proposal_id"],
        "proposal_sha256": proposal["proposal_sha256"],
        "authorization_reference": authorization["approval_reference"],
        "authorization_sha256": file_sha256(authorization_path),
        "observed_scope": scope,
        "unverified_scope": [],
        "actual_sample_runs": 1,
        "observations": [{"artifacts": artifacts, "artifact_sha256": hashes}],
    }
    write_json(result_path, result)
    acceptance = {
        "schema_version": "whalecode-cache-baseline-acceptance-v1",
        "status": "accepted",
        "accepted_by": "user",
        "acceptance_reference": "user accepted fixture",
        "result_path": result_path.relative_to(repo).as_posix(),
        "result_sha256": file_sha256(result_path),
        "proposal_path": proposal_path.relative_to(repo).as_posix(),
        "authorization_path": authorization_path.relative_to(repo).as_posix(),
        "accepted_scope": scope,
        "acknowledged_unverified_scope": [],
        "accepted_scenarios": [],
    }
    git(repo, "add", "benchmarks")
    manifest = protected_manifest(repo, contract, "index")
    acceptance["accepted_scenarios"] = [
        {
            "scenario_id": manifest[0]["scenario_id"],
            "after_payload_sha256": manifest[0]["payload_sha256"],
        }
    ]
    write_json(acceptance_path, acceptance)
    write_json(
        repo / "benchmarks/whale-agent-run-ledger.json",
        {
            "entries": [
                {
                    "record_id": record_id,
                    "status": "settled",
                    "authorization": {"reference": authorization["approval_reference"]},
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
            **scope,
            "actual_sample_runs": 1,
            "unverified_scope": [],
        },
    }
    write_json(contract_path, contract)
    git(repo, "add", contract_path.name, "benchmarks")
