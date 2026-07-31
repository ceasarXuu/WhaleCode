#!/usr/bin/env python3
"""Print a user-selected cache smoke budget without executing or recording a run."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from cache_budget import (
    SUPPORTED_STOP_CONDITIONS,
    build_budget_proposal,
    repository_evidence_path,
)
from cache_surface import load_contract


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--gate-report", type=Path, required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--sample", action="append", required=True)
    parser.add_argument("--arm", action="append", required=True)
    parser.add_argument("--repeat", type=int, required=True)
    parser.add_argument("--retry-sample-run-limit", type=int, default=0)
    parser.add_argument("--max-provider-requests-per-run", type=int, required=True)
    parser.add_argument("--max-input-tokens-per-run", type=int, required=True)
    parser.add_argument("--max-output-tokens-per-run", type=int, required=True)
    parser.add_argument("--max-seconds-per-run", type=int, required=True)
    parser.add_argument(
        "--stop-condition",
        action="append",
        choices=SUPPORTED_STOP_CONDITIONS,
        required=True,
    )
    parser.add_argument("--selection-reason", required=True)
    args = parser.parse_args()

    repo = args.repo_root.resolve()
    contract = load_contract(
        repo / "benchmarks/cache-regression/cache-surface-contract.json"
    )
    report_path = repository_evidence_path(repo, args.gate_report)
    gate_report = json.loads(report_path.read_text(encoding="utf-8-sig"))
    subject_commit = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    try:
        proposal = build_budget_proposal(
            repo=repo,
            contract=contract,
            gate_report_path=report_path,
            gate_report=gate_report,
            subject_commit=subject_commit,
            model=args.model,
            samples=args.sample,
            arms=args.arm,
            repeat=args.repeat,
            retry_sample_run_limit=args.retry_sample_run_limit,
            max_provider_requests_per_run=args.max_provider_requests_per_run,
            max_input_tokens_per_run=args.max_input_tokens_per_run,
            max_output_tokens_per_run=args.max_output_tokens_per_run,
            max_seconds_per_run=args.max_seconds_per_run,
            stop_conditions=args.stop_condition,
            selection_reason=args.selection_reason,
        )
    except (KeyError, TypeError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(json.dumps(proposal, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
