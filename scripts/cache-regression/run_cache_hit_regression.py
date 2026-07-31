#!/usr/bin/env python3
"""Run the two-arm Whale cache smoke and settle the global run ledger."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from cache_evidence import (
    RESULT_SCHEMA_VERSION,
    canonical_json_sha256,
    evidence_manifest,
    expected_run_plan,
    file_sha256,
)
from cache_surface import load_contract, surface_snapshot, write_json


def now() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def ensure_deepseek_api_key(repo: Path) -> str:
    if os.environ.get("DEEPSEEK_API_KEY", "").strip():
        return "process_environment"
    env_path = repo / ".env.local"
    if not env_path.is_file():
        raise RuntimeError("DEEPSEEK_API_KEY is missing and .env.local does not exist")
    for raw_line in env_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line.removeprefix("export ").lstrip()
        key, separator, value = line.partition("=")
        if separator and key.strip() == "DEEPSEEK_API_KEY":
            value = value.strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
                value = value[1:-1]
            if not value:
                break
            os.environ["DEEPSEEK_API_KEY"] = value
            return ".env.local"
    raise RuntimeError("DEEPSEEK_API_KEY is missing from .env.local")


def find_run_dir(run_root: Path, sample: str) -> Path:
    candidates = [path for path in (run_root / sample).iterdir() if path.is_dir()]
    if not candidates:
        raise RuntimeError("benchmark produced no sample run directory")
    return max(candidates, key=lambda path: path.stat().st_mtime_ns)


def analyze_artifacts(
    cache_path: Path, request_path: Path, metrics_path: Path, arm: str
) -> dict[str, Any]:
    cache = read_json(cache_path)
    request = read_json(request_path)["rollout_trace"]
    metrics = read_json(metrics_path)
    input_tokens = int(request["input_tokens"])
    cached_tokens = int(request["cached_input_tokens"])
    return {
        "arm": arm,
        "logical_mode": metrics["logical_mode"],
        "provider_requests": int(cache["provider_request_count"]),
        "request_2_plus_count": int(cache["request_2_plus_count"]),
        "request_2_plus_hit_rate": float(cache["request_2_plus_hit_rate"]),
        "trace_coverage": float(cache["trace_coverage"]),
        "cache_usage_missing_count": int(cache["cache_usage_missing_count"]),
        "input_tokens": input_tokens,
        "cached_input_tokens": cached_tokens,
        "uncached_input_tokens": input_tokens - cached_tokens,
        "output_tokens": int(request["output_tokens"]),
        "business_success": bool(metrics["business_success"]),
        "artifacts": {
            "cache_summary": str(cache_path),
            "request_summary": str(request_path),
            "metrics": str(metrics_path),
        },
        "artifact_sha256": {
            "cache_summary": file_sha256(cache_path),
            "request_summary": file_sha256(request_path),
            "metrics": file_sha256(metrics_path),
        },
    }


def analyze_arm(run_dir: Path, side: str, arm: str) -> dict[str, Any]:
    artifacts = run_dir / "pair-001" / side / "artifacts"
    return analyze_artifacts(
        artifacts / "provider-cache-trace-summary.json",
        artifacts / "request-summary.json",
        artifacts / "metrics.json",
        arm,
    )


def arm_passes(
    arm: dict[str, Any],
    policy: dict[str, Any],
    baseline: dict[str, Any] | None = None,
) -> bool:
    floor = float(policy["absolute_floor"][arm["arm"]])
    absolute_pass = (
        arm["business_success"]
        and arm["provider_requests"] >= 2
        and arm["request_2_plus_count"] >= int(policy["min_request_2_plus_count"])
        and arm["trace_coverage"] >= float(policy["min_trace_coverage"])
        and arm["cache_usage_missing_count"] == 0
        and arm["request_2_plus_hit_rate"] >= floor
    )
    if not absolute_pass or not baseline or baseline.get("status") != "live_verified":
        return absolute_pass
    prior_rates = baseline.get("request_2_plus_hit_rate", {})
    prior_rate = prior_rates.get(arm["arm"])
    if prior_rate is None:
        return absolute_pass
    max_drop = float(policy["max_drop_from_live_baseline"])
    return arm["request_2_plus_hit_rate"] >= float(prior_rate) - max_drop


def record_failed_baseline(
    contract_path: Path,
    contract: dict[str, Any],
    result: dict[str, Any],
) -> None:
    contract["baseline"].update(
        {
            "status": "live_regression_failed",
            "live_result_path": result["result_path"],
            "note": "最近一次已授权真实缓存回归未通过；修复并重新获批验证前保持阻断。",
        }
    )
    write_json(contract_path, contract)


def should_record_failed_baseline(result: dict[str, Any]) -> bool:
    return result["status"] != "pass" and result["actual_sample_runs"] > 0


def planned_entry(
    record_id: str,
    contract: dict[str, Any],
    authorization: str,
    surface_sha: str,
    head: str,
    run_root: Path,
) -> dict[str, Any]:
    live = contract["live_regression"]
    plan = expected_run_plan(contract)
    return {
        "record_id": record_id,
        "record_type": "run_batch",
        "status": "planned",
        "started_at": None,
        "ended_at": None,
        "elapsed_calendar_seconds": None,
        "aggregate_agent_wall_time_ms": None,
        "reason": "缓存敏感面回归：验证 Standard 与 map-request 的 provider request-2+ 前缀缓存。",
        "authorization": {
            "required": True,
            "status": "granted",
            "reference": authorization,
            "budget_summary": {
                "sample_run_limit": plan["planned_sample_runs"],
                "automatic_retry_limit": plan["automatic_retries"],
                "sample": plan["sample"],
                "arms": plan["arms"],
            },
            "note": "仅允许 single-file-fast-fix 的 Standard 与 map-request 各一次。",
        },
        "execution": {
            "provider": "deepseek",
            "model": plan["model"],
            "batch_count": 1,
            "sample_ids": [plan["sample"]],
            "arm_ids": plan["arms"],
            "repeats_per_arm_per_sample": plan["repeat"],
            "planned_sample_runs": plan["planned_sample_runs"],
            "actual_sample_runs": 0,
            "api_requests": 0,
        },
        "tokens": {"input": 0, "cached_input": 0, "uncached_input": 0, "output": 0},
        "monetary_cost": {
            "status": "planned",
            "currency": live["pricing_snapshot"]["currency"],
            "amount": None,
            "actual_billed_amount": None,
            "components": None,
            "pricing_snapshot": live["pricing_snapshot"],
            "formula": None,
            "note": "运行后按 provider token 遥测估算。",
        },
        "evidence": {
            "planned_run_root": str(run_root),
            "subject_commit": head,
            "surface_sha256": surface_sha,
            "stop_conditions": "两臂各首次完成、失败或超时后停止；禁止自动重试。",
        },
    }


def settle_entry(
    entry: dict[str, Any],
    result: dict[str, Any],
    started: float,
    run_exit: int,
) -> None:
    arms = result.get("arms", [])
    totals = {
        key: sum(int(arm[key]) for arm in arms)
        for key in (
            "provider_requests",
            "input_tokens",
            "cached_input_tokens",
            "uncached_input_tokens",
            "output_tokens",
        )
    }
    pricing = entry["monetary_cost"]["pricing_snapshot"]
    components = {
        "cached_input": totals["cached_input_tokens"] / 1_000_000 * pricing["cached_input_per_million"],
        "uncached_input": totals["uncached_input_tokens"] / 1_000_000 * pricing["uncached_input_per_million"],
        "output": totals["output_tokens"] / 1_000_000 * pricing["output_per_million"],
    }
    amount = sum(components.values())
    entry["status"] = "settled" if arms else "failed"
    entry["started_at"] = result["started_at"]
    entry["ended_at"] = now()
    entry["elapsed_calendar_seconds"] = round(time.time() - started)
    entry["aggregate_agent_wall_time_ms"] = None
    entry["execution"]["actual_sample_runs"] = len(arms)
    entry["execution"]["api_requests"] = totals["provider_requests"]
    entry["tokens"] = {
        "input": totals["input_tokens"],
        "cached_input": totals["cached_input_tokens"],
        "uncached_input": totals["uncached_input_tokens"],
        "output": totals["output_tokens"],
    }
    entry["monetary_cost"].update(
        {
            "status": "estimated" if arms else "unavailable",
            "amount": round(amount, 10) if arms else None,
            "components": components if arms else None,
            "formula": (
                "cached_input/1e6*cached_rate + uncached_input/1e6*miss_rate "
                "+ output/1e6*output_rate"
                if arms
                else None
            ),
            "note": "按 provider token 遥测和运行时冻结价格估算。" if arms else "无完整 token 证据。",
        }
    )
    entry["evidence"].update(
        {
            "actual_run_root": result.get("run_dir"),
            "result_path": result.get("result_path"),
            "runner_exit_code": run_exit,
            "outcome": result.get("status", "failed"),
        }
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--authorization-reference", required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--whale-bin", type=Path, default=Path.home() / ".whale/bin/whale")
    parser.add_argument("--run-root", type=Path)
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    contract_path = repo / "benchmarks/cache-regression/cache-surface-contract.json"
    contract = load_contract(contract_path)
    live = contract["live_regression"]
    plan = expected_run_plan(contract)
    if plan["planned_sample_runs"] != 2 or plan["automatic_retries"] != 0:
        raise SystemExit("cache regression contract exceeds the authorized run shape")
    credential_source = ensure_deepseek_api_key(repo)
    surface_sha, _ = surface_snapshot(repo, contract, "worktree")
    head = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    record_id = f"WAR-{stamp}-CACHE-REGRESSION-{uuid.uuid4().hex[:8].upper()}"
    run_root = args.run_root or repo / "target/cache-hit-regression" / record_id
    ledger_path = repo / "benchmarks/whale-agent-run-ledger.json"
    ledger = read_json(ledger_path)
    entry = planned_entry(
        record_id, contract, args.authorization_reference, surface_sha, head, run_root
    )
    ledger["entries"].insert(0, entry)
    ledger["updated_at"] = now()
    write_json(ledger_path, ledger)

    started = time.time()
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "record_id": record_id,
        "status": "failed",
        "started_at": now(),
        "subject_commit": head,
        "surface_sha256": surface_sha,
        "authorization_reference": args.authorization_reference,
        "run_plan": plan,
        "policy_sha256": canonical_json_sha256(live),
        "actual_sample_runs": 0,
        "credential_source": credential_source,
        "arms": [],
    }
    command = [
        "pwsh",
        "-NoProfile",
        "-File",
        str(repo / "scripts/taskspace-benchmark/run-taskspace-benchmark.ps1"),
        "-Scenario",
        plan["sample"],
        "-Repeats",
        str(plan["repeat"]),
        "-RunRoot",
        str(run_root),
        "-WhaleBin",
        str(args.whale_bin),
        "-Model",
        plan["model"],
        "-TaskSpaceProjectionPolicy",
        "map-request",
        "-RunSide",
        "both",
        "-EnableDockerImageCache",
    ]
    run_exit = 1
    try:
        completed = subprocess.run(command, cwd=repo, check=False)
        run_exit = completed.returncode
        run_dir = find_run_dir(run_root, live["sample"])
        result["run_dir"] = str(run_dir.relative_to(repo))
        for side, arm_name in (("left", "standard"), ("right", "map-request")):
            try:
                result["arms"].append(analyze_arm(run_dir, side, arm_name))
            except (FileNotFoundError, KeyError, ValueError, TypeError) as error:
                result.setdefault("arm_errors", {})[arm_name] = (
                    f"{type(error).__name__}: {error}"
                )
        result["actual_sample_runs"] = len(result["arms"])
        for arm in result["arms"]:
            arm["passed"] = arm_passes(arm, live, contract["baseline"])
        result["status"] = (
            "pass"
            if run_exit == 0
            and len(result["arms"]) == 2
            and all(arm["passed"] for arm in result["arms"])
            else "fail"
        )
        result["evidence_sha256"] = canonical_json_sha256(
            evidence_manifest(result["arms"])
        )
    except BaseException as error:
        result["error"] = f"{type(error).__name__}: {error}"

    result_dir = repo / "benchmarks/cache-regression/results"
    result_path = result_dir / f"{record_id}.json"
    result["result_path"] = str(result_path.relative_to(repo))
    write_json(result_path, result)
    if should_record_failed_baseline(result):
        record_failed_baseline(contract_path, contract, result)
    settle_entry(entry, result, started, run_exit)
    ledger["updated_at"] = now()
    write_json(ledger_path, ledger)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result["status"] == "pass" else 3


if __name__ == "__main__":
    raise SystemExit(main())
