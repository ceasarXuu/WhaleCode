#!/usr/bin/env python3
"""Block cache-sensitive changes without a matching verified surface."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from cache_surface import (
    load_contract_from_source,
    source_matches_worktree,
    staged_sensitive_changes,
    surface_snapshot,
    write_json,
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("benchmarks/cache-regression/cache-surface-contract.json"),
    )
    parser.add_argument("--source", choices=["index", "head", "worktree"], default="index")
    parser.add_argument("--require-live-baseline", action="store_true")
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    repo = args.repo_root.resolve()
    contract_path = args.contract
    if not contract_path.is_absolute():
        contract_path = repo / contract_path
    contract = load_contract_from_source(repo, contract_path, args.source)
    contract_matches_worktree = args.source != "index" or source_matches_worktree(
        repo, contract_path, args.source
    )
    actual_hash, entries = surface_snapshot(repo, contract, args.source)
    expected_hash = contract["baseline"]["surface_sha256"]
    changes = staged_sensitive_changes(repo, contract) if args.source == "index" else []
    baseline_status = contract["baseline"]["status"]
    live_status_accepted = baseline_status == "live_verified"
    passed = (
        contract_matches_worktree
        and actual_hash == expected_hash
        and (live_status_accepted or not args.require_live_baseline)
    )
    result = {
        "schema_version": "whalecode-cache-regression-gate-v1",
        "status": "pass" if passed else "blocked",
        "source": args.source,
        "actual_surface_sha256": actual_hash,
        "expected_surface_sha256": expected_hash,
        "baseline_status": baseline_status,
        "require_live_baseline": args.require_live_baseline,
        "contract_matches_worktree": contract_matches_worktree,
        "surface_file_count": len(entries),
        "sensitive_changes": changes,
    }
    if args.json_output:
        output = args.json_output
        if not output.is_absolute():
            output = repo / output
        write_json(output, result)

    if passed:
        if baseline_status == "structural_bootstrap":
            suffix = "（尚待首次真实缓存基线）"
        elif baseline_status == "live_regression_failed":
            suffix = "（当前指纹未变；最近一次 live 回归失败）"
        else:
            suffix = ""
        print(f"cache regression gate: PASS {actual_hash}{suffix}")
        return 0

    print("cache regression gate: BLOCKED")
    print(f"expected surface: {expected_hash}")
    print(f"actual surface:   {actual_hash}")
    if not contract_matches_worktree:
        print("- 暂存合同与工作区合同不一致；请完整暂存或还原合同后重试。")
    if not live_status_accepted:
        print(f"- 当前基线状态为 {baseline_status}，尚未达到 live_verified。")
    if changes:
        print("可能影响缓存命中的变更：")
        for change in changes:
            reasons = "；".join(rule["reason"] for rule in change["rules"])
            print(f"- {change['path']}: {reasons}")
    elif actual_hash != expected_hash:
        print("- 当前缓存敏感面与已验证基线不一致；差异可能来自此前未验证提交。")
    if actual_hash != expected_hash:
        print("下一步：说明变更为何会影响 provider 前缀，并向用户申请 2 个 sample run 预算。")
    else:
        print("下一步：先修复已记录的缓存退化；敏感面变化后再申请 2 个 sample run 复验预算。")
    print(
        "获批后运行: pwsh scripts/cache-regression/run_cache_hit_regression.ps1 "
        "-AuthorizationReference '<用户批准说明>'"
    )
    return 20


if __name__ == "__main__":
    raise SystemExit(main())
