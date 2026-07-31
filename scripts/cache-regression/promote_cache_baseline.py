#!/usr/bin/env python3
"""Promote accepted final-wire candidates with one precisely scoped smoke result."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from cache_evidence import RESULT_SCHEMA_VERSION, canonical_json_sha256, file_sha256
from cache_run_analysis import analyze_artifacts
from cache_run_contract import execution_matrix, load_authorized_proposal, read_json
from cache_surface import load_contract, surface_snapshot, write_json


ACCEPTANCE_SCHEMA_VERSION = "whalecode-cache-baseline-acceptance-v1"
OBSERVATION_EVIDENCE_KEYS = (
    "provider_usage_contract_version",
    "logical_mode",
    "provider_requests",
    "request_2_plus_count",
    "request_2_plus_hit_rate",
    "request_2_plus_cached_input_tokens",
    "request_2_plus_uncached_input_tokens",
    "trace_coverage",
    "cache_usage_missing_count",
    "input_tokens",
    "cached_input_tokens",
    "uncached_input_tokens",
    "output_tokens",
    "business_success",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def evidence_path(repo: Path, raw_path: str) -> Path:
    path = Path(raw_path)
    resolved = (path if path.is_absolute() else repo / path).resolve()
    try:
        resolved.relative_to(repo)
    except ValueError as error:
        raise ValueError("cache evidence must be inside the repository") from error
    require(resolved.is_file(), "cache evidence file does not exist")
    return resolved


def validate_observation(repo: Path, observation: dict[str, Any]) -> None:
    artifacts = observation.get("artifacts", {})
    hashes = observation.get("artifact_sha256", {})
    paths = {
        key: evidence_path(repo, artifacts[key])
        for key in ("cache_summary", "request_summary", "metrics")
    }
    require(
        all(file_sha256(path) == hashes.get(key) for key, path in paths.items()),
        f"artifact digest mismatch for {observation['sample']}/{observation['arm']}",
    )
    recomputed = analyze_artifacts(
        paths["cache_summary"],
        paths["request_summary"],
        paths["metrics"],
        observation["arm"],
    )
    require(
        all(
            observation.get(key) == recomputed[key] for key in OBSERVATION_EVIDENCE_KEYS
        ),
        f"artifact metrics mismatch for {observation['sample']}/{observation['arm']}",
    )
    require(
        observation["business_success"] is True, "smoke contains a business failure"
    )
    require(
        observation["provider_requests"] >= 2,
        "smoke has fewer than two provider requests",
    )
    require(
        observation["request_2_plus_count"] >= 1, "smoke has no request-2+ evidence"
    )
    require(observation["trace_coverage"] == 1.0, "smoke trace coverage is incomplete")
    require(
        observation["cache_usage_missing_count"] == 0, "smoke cache usage is incomplete"
    )


def validate_ledger(
    repo: Path,
    result_path: Path,
    result: dict[str, Any],
    proposal: dict[str, Any],
    proposal_path: Path,
    authorization_path: Path,
    authorization: dict[str, Any],
) -> None:
    ledger = read_json(repo / "benchmarks/whale-agent-run-ledger.json")
    matches = [
        entry
        for entry in ledger["entries"]
        if entry.get("record_id") == result.get("record_id")
    ]
    require(len(matches) == 1, "result must have exactly one matching ledger entry")
    entry = matches[0]
    execution = entry.get("execution", {})
    evidence = entry.get("evidence", {})
    selection = proposal["selection"]
    require(entry.get("status") == "settled", "ledger entry is not settled")
    require(
        entry.get("authorization", {}).get("status") == "granted",
        "ledger authorization is not granted",
    )
    require(
        entry.get("authorization", {}).get("reference")
        == authorization["approval_reference"],
        "ledger authorization reference mismatch",
    )
    require(
        execution.get("model") == selection["model"],
        "ledger model does not match proposal",
    )
    require(
        execution.get("sample_ids") == selection["samples"],
        "ledger samples do not match proposal",
    )
    require(
        execution.get("arm_ids") == selection["arms"],
        "ledger arms do not match proposal",
    )
    require(
        execution.get("repeats_per_arm_per_sample") == selection["repeat"],
        "ledger repeat does not match proposal",
    )
    require(
        execution.get("planned_sample_runs") == selection["planned_sample_runs"],
        "ledger plan count does not match proposal",
    )
    require(
        execution.get("actual_sample_runs") == result["actual_sample_runs"],
        "ledger actual count does not match result",
    )
    require(
        evidence.get("result_path") == result_path.relative_to(repo).as_posix(),
        "ledger result path mismatch",
    )
    require(
        evidence.get("proposal_path") == proposal_path.relative_to(repo).as_posix(),
        "ledger proposal path mismatch",
    )
    require(
        evidence.get("proposal_sha256") == file_sha256(proposal_path),
        "ledger proposal digest mismatch",
    )
    require(
        evidence.get("authorization_path")
        == authorization_path.relative_to(repo).as_posix(),
        "ledger authorization path mismatch",
    )
    require(
        evidence.get("authorization_sha256") == file_sha256(authorization_path),
        "ledger authorization digest mismatch",
    )


def changed_scenarios(gate_report: dict[str, Any]) -> list[dict[str, Any]]:
    scenarios = []
    for command in gate_report["free_validation"]["commands"]:
        report = command.get("change_report")
        if report is None:
            continue
        scenarios.extend(
            scenario
            for scenario in report["scenarios"]
            if scenario["status"] == "changed"
        )
    scenarios = sorted(scenarios, key=lambda item: item["scenario_id"])
    scenario_ids = [scenario["scenario_id"] for scenario in scenarios]
    require(
        len(scenario_ids) == len(set(scenario_ids)),
        "duplicate changed scenario in gate report",
    )
    return scenarios


def validate_acceptance(
    repo: Path,
    result_path: Path,
    result: dict[str, Any],
    scenarios: list[dict[str, Any]],
    acceptance: dict[str, Any],
) -> None:
    require(
        acceptance.get("schema_version") == ACCEPTANCE_SCHEMA_VERSION,
        "invalid baseline acceptance schema",
    )
    require(acceptance.get("status") == "accepted", "baseline result is not accepted")
    require(
        acceptance.get("accepted_by") == "user",
        "baseline result was not accepted by the user",
    )
    require(
        isinstance(acceptance.get("accepted_at"), str)
        and acceptance["accepted_at"].strip(),
        "baseline acceptance timestamp is missing",
    )
    require(
        isinstance(acceptance.get("acceptance_reference"), str)
        and acceptance["acceptance_reference"].strip(),
        "baseline acceptance reference is missing",
    )
    require(
        acceptance.get("result_path") == result_path.relative_to(repo).as_posix(),
        "accepted result path mismatch",
    )
    require(
        acceptance.get("result_sha256") == file_sha256(result_path),
        "accepted result digest mismatch",
    )
    require(
        acceptance.get("accepted_scope") == result["observed_scope"],
        "accepted smoke scope mismatch",
    )
    require(
        acceptance.get("acknowledged_unverified_scope") == result["unverified_scope"],
        "accepted unverified scope mismatch",
    )
    expected_scenarios = [
        {
            "scenario_id": scenario["scenario_id"],
            "after_payload_sha256": scenario["after_payload_sha256"],
        }
        for scenario in scenarios
    ]
    require(
        acceptance.get("accepted_scenarios") == expected_scenarios,
        "accepted final-wire scenarios mismatch",
    )


def validate_promotion(
    repo: Path,
    contract: dict[str, Any],
    result_path: Path,
    result: dict[str, Any],
    acceptance: dict[str, Any],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    require(
        result.get("schema_version") == RESULT_SCHEMA_VERSION, "invalid result schema"
    )
    require(result.get("status") == "completed", "cannot promote an incomplete result")
    require(
        result.get("unverified_scope") == [], "result has unverified selected scope"
    )
    head = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    require(result.get("subject_commit") == head, "result subject is not current HEAD")
    proposal_path = evidence_path(repo, acceptance["proposal_path"])
    authorization_path = evidence_path(repo, acceptance["authorization_path"])
    proposal, authorization, proposal_path, authorization_path = load_authorized_proposal(
        repo, contract, proposal_path, authorization_path
    )
    require(
        result.get("proposal_id") == proposal["proposal_id"],
        "result proposal id mismatch",
    )
    require(
        result.get("proposal_sha256") == proposal["proposal_sha256"],
        "result proposal digest mismatch",
    )
    require(
        result.get("authorization_reference") == authorization["approval_reference"],
        "result authorization reference mismatch",
    )
    require(
        result.get("authorization_sha256") == file_sha256(authorization_path),
        "result authorization digest mismatch",
    )
    require(
        result.get("observed_scope") == proposal["selection"],
        "result observed scope does not match proposal",
    )
    expected_matrix = execution_matrix(proposal)
    observations = result.get("observations")
    require(isinstance(observations, list), "result observations are missing")
    actual_matrix = [
        {key: item[key] for key in ("sample", "arm", "repeat")} for item in observations
    ]
    require(
        actual_matrix == expected_matrix,
        "result observations do not match proposal matrix",
    )
    require(
        result.get("actual_sample_runs") == len(expected_matrix),
        "result sample count does not match proposal",
    )
    for observation in observations:
        validate_observation(repo, observation)
    expected_evidence = [
        {**scope, "artifact_sha256": observation["artifact_sha256"]}
        for scope, observation in zip(expected_matrix, observations)
    ]
    require(
        result.get("evidence_sha256") == canonical_json_sha256(expected_evidence),
        "result evidence digest mismatch",
    )
    validate_ledger(
        repo,
        result_path,
        result,
        proposal,
        proposal_path,
        authorization_path,
        authorization,
    )
    gate_path = evidence_path(repo, proposal["trigger"]["gate_report_path"])
    gate_report = read_json(gate_path)
    scenarios = changed_scenarios(gate_report)
    validate_acceptance(repo, result_path, result, scenarios, acceptance)
    return scenarios, proposal


def render_insta_snapshot(path: Path, candidate: dict[str, Any]) -> str:
    content = path.read_text(encoding="utf-8")
    separator = "\n---\n"
    require(
        content.startswith("---\n") and separator in content,
        "invalid insta snapshot envelope",
    )
    header = content.split(separator, 1)[0]
    return (
        header + separator + json.dumps(candidate, ensure_ascii=False, indent=2) + "\n"
    )


def insta_snapshot_payload(path: Path) -> dict[str, Any]:
    content = path.read_text(encoding="utf-8")
    separator = "\n---\n"
    require(
        content.startswith("---\n") and separator in content,
        "invalid insta snapshot envelope",
    )
    payload = json.loads(content.split(separator, 1)[1])
    require(isinstance(payload, dict), "insta snapshot payload must be an object")
    return payload


def snapshot_scenario_id(path: Path) -> str:
    return path.name.removesuffix(".snap").rsplit("__", 1)[-1]


def promote(
    repo: Path,
    contract_path: Path,
    contract: dict[str, Any],
    result_path: Path,
    result: dict[str, Any],
    acceptance_path: Path,
    acceptance: dict[str, Any],
) -> None:
    scenarios, proposal = validate_promotion(
        repo, contract, result_path, result, acceptance
    )
    protected = {
        path.resolve()
        for pattern in contract["free_validation"]["semantic_baseline_globs"]
        for path in repo.glob(pattern)
    }
    replacements: list[tuple[Path, str]] = []
    for scenario in scenarios:
        path = (repo / scenario["baseline_path"]).resolve()
        require(path in protected, "candidate baseline is outside the protected set")
        candidate = scenario.get("candidate_payload")
        require(
            isinstance(candidate, dict), "changed scenario has no candidate payload"
        )
        require(
            canonical_json_sha256(insta_snapshot_payload(path))
            == scenario["before_payload_sha256"],
            "protected baseline changed after discovery",
        )
        require(
            canonical_json_sha256(candidate) == scenario["after_payload_sha256"],
            "candidate payload digest mismatch",
        )
        replacements.append((path, render_insta_snapshot(path, candidate)))
    for path, content in replacements:
        path.write_text(content, encoding="utf-8")

    manifest = [
        {
            "scenario_id": snapshot_scenario_id(path),
            "baseline_path": path.relative_to(repo).as_posix(),
            "payload_sha256": canonical_json_sha256(insta_snapshot_payload(path)),
        }
        for path in sorted(protected)
    ]
    scenario_ids = [item["scenario_id"] for item in manifest]
    require(
        len(scenario_ids) == len(set(scenario_ids)),
        "protected final-wire scenario ids are not unique",
    )
    surface_sha, _ = surface_snapshot(repo, contract, "worktree")
    contract["baseline"] = {
        "status": "accepted",
        "surface_sha256": surface_sha,
        "source_commit": result["subject_commit"],
        "accepted_at": acceptance["accepted_at"],
        "acceptance_reference": acceptance["acceptance_reference"],
        "acceptance_evidence": {
            "path": acceptance_path.relative_to(repo).as_posix(),
            "sha256": file_sha256(acceptance_path),
        },
        "final_wire_manifest": manifest,
        "final_wire_manifest_sha256": canonical_json_sha256(manifest),
        "smoke_evidence": {
            "result_path": result_path.relative_to(repo).as_posix(),
            "result_sha256": file_sha256(result_path),
            "proposal_id": proposal["proposal_id"],
            "model": result["observed_scope"]["model"],
            "samples": result["observed_scope"]["samples"],
            "arms": result["observed_scope"]["arms"],
            "repeat": result["observed_scope"]["repeat"],
            "actual_sample_runs": result["actual_sample_runs"],
            "unverified_scope": result["unverified_scope"],
        },
        "note": "仅接受所列 final-wire 场景，并仅引用实际执行的 smoke 范围。",
    }
    write_json(contract_path, contract)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument("--acceptance", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("benchmarks/cache-regression/cache-surface-contract.json"),
    )
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    contract_path = (
        args.contract if args.contract.is_absolute() else repo / args.contract
    )
    result_path = evidence_path(repo, args.result.as_posix())
    acceptance_path = evidence_path(repo, args.acceptance.as_posix())
    contract = load_contract(contract_path)
    result = read_json(result_path)
    acceptance = read_json(acceptance_path)
    try:
        promote(
            repo,
            contract_path,
            contract,
            result_path,
            result,
            acceptance_path,
            acceptance,
        )
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(f"promoted accepted cache baseline from {result['record_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
