#!/usr/bin/env python3
"""Block cache-sensitive changes without a matching verified surface."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from cache_surface import (
    load_contract,
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
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    repo = args.repo_root.resolve()
    contract_path = args.contract
    if not contract_path.is_absolute():
        contract_path = repo / contract_path
    contract = load_contract(contract_path)
    actual_hash, entries = surface_snapshot(repo, contract, args.source)
    expected_hash = contract["baseline"]["surface_sha256"]
    changes = staged_sensitive_changes(repo, contract) if args.source == "index" else []
    baseline_status = contract["baseline"]["status"]
    accepted_statuses = {"structural_bootstrap", "live_verified"}
    passed = actual_hash == expected_hash and baseline_status in accepted_statuses
    result = {
        "schema_version": "whalecode-cache-regression-gate-v1",
        "status": "pass" if passed else "blocked",
        "source": args.source,
        "actual_surface_sha256": actual_hash,
        "expected_surface_sha256": expected_hash,
        "baseline_status": baseline_status,
        "surface_file_count": len(entries),
        "sensitive_changes": changes,
    }
    if args.json_output:
        output = args.json_output
        if not output.is_absolute():
            output = repo / output
        write_json(output, result)

    if passed:
        suffix = "（尚待首次真实缓存基线）" if baseline_status != "live_verified" else ""
        print(f"cache regression gate: PASS {actual_hash}{suffix}")
        return 0

    print("cache regression gate: BLOCKED")
    print(f"expected surface: {expected_hash}")
    print(f"actual surface:   {actual_hash}")
    if baseline_status not in accepted_statuses:
        print(f"- 当前基线状态为 {baseline_status}，最近一次真实缓存回归未通过。")
    if changes:
        print("可能影响缓存命中的变更：")
        for change in changes:
            reasons = "；".join(rule["reason"] for rule in change["rules"])
            print(f"- {change['path']}: {reasons}")
    else:
        print("- 当前缓存敏感面与已验证基线不一致；差异可能来自此前未验证提交。")
    print("下一步：说明变更为何会影响 provider 前缀，并向用户申请 2 个 sample run 预算。")
    print(
        "获批后运行: pwsh scripts/cache-regression/run_cache_hit_regression.ps1 "
        "-AuthorizationReference '<用户批准说明>'"
    )
    return 20


if __name__ == "__main__":
    raise SystemExit(main())
