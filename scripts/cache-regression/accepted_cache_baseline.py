#!/usr/bin/env python3
"""Validate one durable, user-accepted cache baseline without interpretation."""

from __future__ import annotations

import fnmatch
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from cache_budget import BUDGET_PROPOSAL_SCHEMA_VERSION
from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256
from cache_run_contract import AUTHORIZATION_SCHEMA_VERSION
from cache_surface import read_content, tracked_paths
from promote_cache_baseline import ACCEPTANCE_SCHEMA_VERSION


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def relative_path(repo: Path, raw_path: str) -> str:
    require(isinstance(raw_path, str) and raw_path, "evidence path is missing")
    path = Path(raw_path)
    require(not path.is_absolute(), "evidence path must be repository-relative")
    try:
        return (repo / path).resolve().relative_to(repo.resolve()).as_posix()
    except ValueError as error:
        raise ValueError("evidence path escapes repository") from error


def source_bytes(repo: Path, raw_path: str, source: str) -> bytes:
    path = relative_path(repo, raw_path)
    try:
        return read_content(repo, path, source)
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError(f"accepted baseline evidence is missing: {path}") from error


def source_json(repo: Path, raw_path: str, source: str) -> dict[str, Any]:
    try:
        value = json.loads(source_bytes(repo, raw_path, source).decode("utf-8-sig"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"accepted baseline evidence is invalid: {raw_path}") from error
    require(isinstance(value, dict), "accepted baseline evidence must be an object")
    return value


def source_sha256(repo: Path, raw_path: str, source: str) -> str:
    return hashlib.sha256(source_bytes(repo, raw_path, source)).hexdigest()


def snapshot_payload(content: bytes) -> dict[str, Any]:
    text = content.decode("utf-8")
    separator = "\n---\n"
    require(
        text.startswith("---\n") and separator in text,
        "invalid accepted final-wire snapshot",
    )
    payload = json.loads(text.split(separator, 1)[1])
    require(isinstance(payload, dict), "final-wire snapshot must contain an object")
    return payload


def scenario_id(path: str) -> str:
    return Path(path).name.removesuffix(".snap").rsplit("__", 1)[-1]


def protected_manifest(
    repo: Path, contract: dict[str, Any], source: str
) -> list[dict[str, str]]:
    patterns = contract.get("free_validation", {}).get(
        "semantic_baseline_globs", []
    )
    paths = sorted(
        path
        for path in tracked_paths(repo, source)
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)
    )
    manifest = [
        {
            "scenario_id": scenario_id(path),
            "baseline_path": path,
            "payload_sha256": canonical_json_sha256(
                snapshot_payload(source_bytes(repo, path, source))
            ),
        }
        for path in paths
    ]
    ids = [item["scenario_id"] for item in manifest]
    require(manifest and len(ids) == len(set(ids)), "protected scenarios are invalid")
    return manifest


def validate_observation_files(
    repo: Path, result: dict[str, Any], source: str
) -> list[str]:
    evidence_paths = []
    prefix = f"benchmarks/cache-regression/evidence/{result['record_id']}/"
    observations = result.get("observations")
    require(isinstance(observations, list) and observations, "observations are missing")
    for observation in observations:
        artifacts = observation.get("artifacts", {})
        hashes = observation.get("artifact_sha256", {})
        for key in ("cache_summary", "request_summary", "metrics"):
            path = relative_path(repo, artifacts.get(key))
            require(path.startswith(prefix), "observation evidence is not durable")
            require(
                source_sha256(repo, path, source) == hashes.get(key),
                "observation evidence digest mismatch",
            )
            evidence_paths.append(path)
    return evidence_paths


def validate_ledger(
    repo: Path,
    result: dict[str, Any],
    acceptance: dict[str, Any],
    proposal_path: str,
    authorization_path: str,
    source: str,
) -> str:
    path = "benchmarks/whale-agent-run-ledger.json"
    ledger = source_json(repo, path, source)
    matches = [
        entry
        for entry in ledger.get("entries", [])
        if entry.get("record_id") == result["record_id"]
    ]
    require(len(matches) == 1, "accepted result has no unique ledger entry")
    entry = matches[0]
    require(entry.get("status") == "settled", "accepted ledger entry is not settled")
    require(
        entry.get("authorization", {}).get("reference")
        == result["authorization_reference"],
        "accepted ledger authorization mismatch",
    )
    require(
        entry.get("evidence", {}).get("result_path") == acceptance["result_path"],
        "accepted ledger result path mismatch",
    )
    evidence = entry.get("evidence", {})
    require(
        evidence.get("proposal_path") == proposal_path
        and evidence.get("proposal_sha256")
        == source_sha256(repo, proposal_path, source)
        and evidence.get("authorization_path") == authorization_path
        and evidence.get("authorization_sha256")
        == source_sha256(repo, authorization_path, source),
        "accepted ledger contract evidence mismatch",
    )
    return path


def validate_accepted_baseline(
    repo: Path,
    contract: dict[str, Any],
    source: str,
    actual_surface_sha256: str,
) -> dict[str, Any]:
    baseline = contract.get("baseline", {})
    require(baseline.get("status") == "accepted", "baseline status is not accepted")
    require(
        baseline.get("surface_sha256") == actual_surface_sha256,
        "accepted baseline surface does not match current source",
    )
    manifest = protected_manifest(repo, contract, source)
    require(
        baseline.get("final_wire_manifest") == manifest,
        "accepted final-wire manifest does not match current snapshots",
    )
    require(
        baseline.get("final_wire_manifest_sha256")
        == canonical_json_sha256(manifest),
        "accepted final-wire manifest digest mismatch",
    )

    smoke = baseline.get("smoke_evidence", {})
    acceptance_evidence = baseline.get("acceptance_evidence", {})
    result_path = relative_path(repo, smoke.get("result_path"))
    acceptance_path = relative_path(repo, acceptance_evidence.get("path"))
    require(
        source_sha256(repo, result_path, source) == smoke.get("result_sha256"),
        "accepted result digest mismatch",
    )
    require(
        source_sha256(repo, acceptance_path, source)
        == acceptance_evidence.get("sha256"),
        "accepted decision digest mismatch",
    )
    result = source_json(repo, result_path, source)
    acceptance = source_json(repo, acceptance_path, source)
    require(
        result.get("schema_version") == RESULT_SCHEMA_VERSION
        and result.get("status") == "completed",
        "accepted result is not complete v3 evidence",
    )
    require(
        result.get("subject_commit") == baseline.get("source_commit")
        and result.get("surface_sha256") == baseline.get("surface_sha256"),
        "accepted result source identity mismatch",
    )
    require(result.get("unverified_scope") == [], "accepted result has unverified scope")
    observed_scope = result.get("observed_scope", {})
    require(
        result.get("actual_sample_runs") == observed_scope.get("planned_sample_runs")
        == len(result.get("observations", [])),
        "accepted result sample count mismatch",
    )
    expected_smoke = {
        key: observed_scope[key] for key in ("model", "samples", "arms", "repeat")
    }
    expected_smoke.update(
        {
            "actual_sample_runs": result.get("actual_sample_runs"),
            "unverified_scope": result.get("unverified_scope"),
        }
    )
    require(
        all(smoke.get(key) == value for key, value in expected_smoke.items()),
        "accepted smoke boundary does not match result",
    )
    require(
        acceptance.get("schema_version") == ACCEPTANCE_SCHEMA_VERSION
        and acceptance.get("status") == "accepted"
        and acceptance.get("accepted_by") == "user",
        "accepted decision is invalid",
    )
    require(
        acceptance.get("result_path") == result_path
        and acceptance.get("result_sha256") == smoke.get("result_sha256")
        and acceptance.get("accepted_scope") == observed_scope
        and acceptance.get("acknowledged_unverified_scope") == [],
        "accepted decision does not match result",
    )
    require(
        acceptance.get("acceptance_reference")
        == baseline.get("acceptance_reference"),
        "accepted decision reference mismatch",
    )

    proposal_path = relative_path(repo, acceptance.get("proposal_path"))
    authorization_path = relative_path(repo, acceptance.get("authorization_path"))
    proposal = source_json(repo, proposal_path, source)
    authorization = source_json(repo, authorization_path, source)
    proposal_without_digest = dict(proposal)
    proposal_without_digest.pop("proposal_sha256", None)
    require(
        proposal.get("schema_version") == BUDGET_PROPOSAL_SCHEMA_VERSION
        and proposal.get("proposal_id") == result.get("proposal_id")
        and proposal.get("proposal_sha256") == result.get("proposal_sha256")
        and proposal.get("proposal_sha256")
        == canonical_json_sha256(proposal_without_digest)
        and proposal.get("subject_commit") == result.get("subject_commit")
        and proposal.get("surface_sha256") == result.get("surface_sha256")
        and proposal.get("selection") == observed_scope,
        "accepted proposal identity mismatch",
    )
    require(
        authorization.get("schema_version") == AUTHORIZATION_SCHEMA_VERSION
        and authorization.get("status") == "granted"
        and authorization.get("approved_by") == "user"
        and source_sha256(repo, authorization_path, source)
        == result.get("authorization_sha256")
        and authorization.get("approval_reference")
        == result.get("authorization_reference"),
        "accepted authorization identity mismatch",
    )
    require(
        authorization.get("proposal_id") == proposal.get("proposal_id")
        and authorization.get("proposal_sha256") == proposal.get("proposal_sha256")
        and authorization.get("approved_selection") == proposal.get("selection")
        and authorization.get("approved_maximums") == proposal.get("maximums"),
        "accepted authorization scope mismatch",
    )
    accepted_scenarios = acceptance.get("accepted_scenarios")
    require(
        isinstance(accepted_scenarios, list) and accepted_scenarios,
        "accepted scenario set is empty",
    )
    manifest_by_id = {item["scenario_id"]: item for item in manifest}
    accepted_paths = []
    for item in accepted_scenarios:
        scenario = manifest_by_id.get(item.get("scenario_id"))
        require(
            scenario is not None
            and scenario["payload_sha256"] == item.get("after_payload_sha256"),
            "accepted scenario does not match final-wire manifest",
        )
        accepted_paths.append(scenario["baseline_path"])
    require(
        len(accepted_paths) == len(set(accepted_paths)),
        "accepted scenario set contains duplicates",
    )
    evidence_paths = [
        result_path,
        acceptance_path,
        proposal_path,
        authorization_path,
        validate_ledger(
            repo,
            result,
            acceptance,
            proposal_path,
            authorization_path,
            source,
        ),
        *validate_observation_files(repo, result, source),
    ]
    return {
        "valid": True,
        "accepted_scenario_paths": sorted(accepted_paths),
        "evidence_paths": sorted(set(evidence_paths)),
    }
