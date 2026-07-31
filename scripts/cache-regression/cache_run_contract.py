#!/usr/bin/env python3
"""Authorization and execution contracts for cache regression smoke runs."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any

from cache_budget import (
    build_budget_proposal,
    repository_evidence_path,
    validate_budget_proposal,
)
from cache_evidence import file_sha256


AUTHORIZATION_SCHEMA_VERSION = "whalecode-cache-budget-authorization-v1"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    require(isinstance(value, dict), "cache run contract root must be an object")
    return value


def validate_proposal_context(
    repo: Path,
    contract: dict[str, Any],
    proposal_path: Path,
    proposal: dict[str, Any],
) -> None:
    validate_budget_proposal(proposal)
    head = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    require(
        proposal.get("subject_commit") == head,
        "budget proposal subject is not current HEAD",
    )
    trigger = proposal["trigger"]
    gate_path = repository_evidence_path(repo, Path(trigger["gate_report_path"]))
    require(
        file_sha256(gate_path) == trigger["gate_report_sha256"],
        "gate report digest mismatch",
    )
    gate_report = read_json(gate_path)
    selection = proposal["selection"]
    limits = proposal["per_sample_run_limits"]
    expected = build_budget_proposal(
        repo=repo,
        contract=contract,
        gate_report_path=gate_path,
        gate_report=gate_report,
        subject_commit=head,
        model=selection["model"],
        samples=selection["samples"],
        arms=selection["arms"],
        repeat=selection["repeat"],
        retry_sample_run_limit=selection["retry_sample_run_limit"],
        max_provider_requests_per_run=limits["provider_requests"],
        max_input_tokens_per_run=limits["input_tokens"],
        max_output_tokens_per_run=limits["output_tokens"],
        max_seconds_per_run=limits["elapsed_seconds"],
        stop_conditions=selection["stop_conditions"],
        selection_reason=selection["selection_reason"],
    )
    require(proposal == expected, "budget proposal does not match current evidence")
    require(proposal_path.is_file(), "budget proposal file does not exist")


def validate_authorization(
    proposal: dict[str, Any], authorization: dict[str, Any]
) -> None:
    require(
        authorization.get("schema_version") == AUTHORIZATION_SCHEMA_VERSION,
        "invalid cache budget authorization schema",
    )
    require(authorization.get("status") == "granted", "cache budget is not granted")
    require(
        authorization.get("approved_by") == "user",
        "cache budget was not approved by the user",
    )
    require(
        isinstance(authorization.get("approval_reference"), str)
        and authorization["approval_reference"].strip() != "",
        "cache budget approval reference is missing",
    )
    require(
        isinstance(authorization.get("approved_at"), str)
        and authorization["approved_at"].strip() != "",
        "cache budget approval timestamp is missing",
    )
    require(
        authorization.get("proposal_id") == proposal["proposal_id"],
        "authorization proposal id mismatch",
    )
    require(
        authorization.get("proposal_sha256") == proposal["proposal_sha256"],
        "authorization proposal digest mismatch",
    )
    require(
        authorization.get("approved_selection") == proposal["selection"],
        "authorization selection does not match proposal",
    )
    require(
        authorization.get("approved_maximums") == proposal["maximums"],
        "authorization maximums do not match proposal",
    )


def load_authorized_proposal(
    repo: Path,
    contract: dict[str, Any],
    proposal_path: Path,
    authorization_path: Path,
) -> tuple[dict[str, Any], dict[str, Any], Path, Path]:
    proposal_path = repository_evidence_path(repo, proposal_path)
    authorization_path = repository_evidence_path(repo, authorization_path)
    proposal = read_json(proposal_path)
    authorization = read_json(authorization_path)
    validate_proposal_context(repo, contract, proposal_path, proposal)
    validate_authorization(proposal, authorization)
    return proposal, authorization, proposal_path, authorization_path


def execution_matrix(proposal: dict[str, Any]) -> list[dict[str, Any]]:
    selection = proposal["selection"]
    matrix = []
    for sample in selection["samples"]:
        for arm in selection["arms"]:
            for repeat_index in range(1, selection["repeat"] + 1):
                matrix.append({"sample": sample, "arm": arm, "repeat": repeat_index})
    require(
        len(matrix) == selection["planned_sample_runs"],
        "proposal sample-run count is inconsistent",
    )
    return matrix


def benchmark_command(
    repo: Path,
    whale_bin: Path,
    run_root: Path,
    run_id: str,
    proposal: dict[str, Any],
    execution: dict[str, Any],
) -> list[str]:
    arm = execution["arm"]
    policy = arm if arm != "standard" else "map-request"
    side = "left" if arm == "standard" else "right"
    return [
        "pwsh",
        "-NoProfile",
        "-File",
        str(repo / "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"),
        "-Scenario",
        execution["sample"],
        "-Repeats",
        "1",
        "-RunRoot",
        str(run_root),
        "-RunId",
        run_id,
        "-WhaleBin",
        str(whale_bin),
        "-Model",
        proposal["selection"]["model"],
        "-TaskSpaceProjectionPolicy",
        policy,
        "-RunSide",
        side,
        "-TimeoutSeconds",
        str(proposal["per_sample_run_limits"]["elapsed_seconds"]),
        "-EnableDockerImageCache",
    ]
