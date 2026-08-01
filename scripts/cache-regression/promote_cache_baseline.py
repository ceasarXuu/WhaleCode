#!/usr/bin/env python3
"""Promote one user-accepted cache result into the protected baseline."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from accepted_cache_baseline import validate_run_evidence
from cache_evidence import canonical_json_sha256, file_sha256
from cache_json import strict_json_loads
from cache_source_evidence import protected_manifest
from cache_surface import load_contract, surface_snapshot, write_json


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


def render_insta_snapshot(path: Path, candidate: dict[str, Any]) -> str:
    content = path.read_text(encoding="utf-8")
    separator = "\n---\n"
    require(
        content.startswith("---\n") and separator in content,
        "invalid insta snapshot envelope",
    )
    header = content.split(separator, 1)[0]
    return (
        header
        + separator
        + json.dumps(candidate, ensure_ascii=False, indent=2, allow_nan=False)
        + "\n"
    )


def insta_snapshot_payload(path: Path) -> dict[str, Any]:
    content = path.read_text(encoding="utf-8")
    separator = "\n---\n"
    require(
        content.startswith("---\n") and separator in content,
        "invalid insta snapshot envelope",
    )
    payload = strict_json_loads(content.split(separator, 1)[1])
    require(isinstance(payload, dict), "insta snapshot payload must be an object")
    return payload


def validate_promotion(
    repo: Path,
    contract: dict[str, Any],
    result_path: Path,
    acceptance_path: Path,
) -> dict[str, Any]:
    return validate_run_evidence(
        repo,
        contract,
        "worktree",
        result_path.relative_to(repo).as_posix(),
        acceptance_path.relative_to(repo).as_posix(),
        require_current_head=True,
    )


def replacement_contents(
    repo: Path,
    contract: dict[str, Any],
    scenarios: list[dict[str, Any]],
) -> list[tuple[Path, str]]:
    protected = {
        path.resolve()
        for pattern in contract["free_validation"]["semantic_baseline_globs"]
        for path in repo.glob(pattern)
    }
    replacements = []
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
    return replacements


def promote(
    repo: Path,
    contract_path: Path,
    contract: dict[str, Any],
    result_path: Path,
    acceptance_path: Path,
) -> None:
    validated = validate_promotion(repo, contract, result_path, acceptance_path)
    result = validated["result"]
    acceptance = validated["acceptance"]
    proposal = validated["proposal"]
    replacements = replacement_contents(repo, contract, validated["scenarios"])

    for path, content in replacements:
        path.write_text(content, encoding="utf-8")

    surface_sha, _ = surface_snapshot(repo, contract, "worktree")
    manifest = protected_manifest(repo, contract, "worktree")
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
    try:
        promote(repo, contract_path, contract, result_path, acceptance_path)
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(f"promoted accepted cache baseline from {result_path.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
