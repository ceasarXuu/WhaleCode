#!/usr/bin/env python3
"""Block cache-sensitive changes without a matching verified surface."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from cache_surface import (
    changed_paths_match_worktree,
    control_plane_change_summary,
    load_contract_from_source,
    release_relevant_changes,
    source_matches_worktree,
    semantic_baseline_changes,
    sensitive_changes,
    surface_snapshot,
    write_json,
)
from free_cache_contracts import run_free_validation, validate_free_validation


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
    parser.add_argument("--require-clean-subject", action="store_true")
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    repo = args.repo_root.resolve()
    if args.require_clean_subject and args.source != "head":
        parser.error("--require-clean-subject requires --source head")
    contract_path = args.contract
    if not contract_path.is_absolute():
        contract_path = repo / contract_path
    contract = load_contract_from_source(repo, contract_path, args.source)
    free_validation_config = contract.get("free_validation")
    if free_validation_config is not None:
        validate_free_validation(free_validation_config)
    contract_matches_worktree = args.source != "index" or source_matches_worktree(
        repo, contract_path, args.source
    )
    actual_hash, entries = surface_snapshot(repo, contract, args.source)
    expected_hash = contract["baseline"]["surface_sha256"]
    changes = sensitive_changes(repo, contract, args.source)
    semantic_baselines = semantic_baseline_changes(repo, contract, args.source)
    relevant_source_paths = sorted(
        {change["path"] for change in changes}.union(semantic_baselines)
    )
    relevant_source_matches_worktree = changed_paths_match_worktree(
        repo, relevant_source_paths, args.source
    )
    control_plane = control_plane_change_summary(
        repo, contract_path, args.source, contract
    )
    policy_changes = control_plane["policy_changes"]
    baseline_changed = control_plane["baseline_changed"]
    policy_baseline_conflict = bool(policy_changes) and baseline_changed
    policy_product_conflict = bool(policy_changes) and bool(changes)
    baseline_product_conflict = bool(semantic_baselines) and bool(changes)
    policy_only_surface_transition = (
        control_plane["contract_policy_changed"]
        and not baseline_changed
        and not changes
        and not args.require_live_baseline
    )
    release_changes = (
        release_relevant_changes(repo, contract_path, contract)
        if args.require_clean_subject
        else []
    )
    subject_commit = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    baseline_status = contract["baseline"]["status"]
    live_status_accepted = baseline_status == "live_verified"
    free_validation_required = bool(free_validation_config) and (
        bool(changes)
        or bool(
            args.require_live_baseline
            and free_validation_config.get("run_on_release", False)
        )
    )
    free_validation = None
    can_run_free_validation = (
        free_validation_required
        and contract_matches_worktree
        and relevant_source_matches_worktree
        and not policy_baseline_conflict
        and not policy_product_conflict
        and not baseline_product_conflict
        and not semantic_baselines
    )
    if can_run_free_validation:
        free_validation = run_free_validation(repo, free_validation_config)
    free_validation_passed = (
        not free_validation_required
        or (free_validation is not None and free_validation["passed"])
    )
    semantic_gate_passed = (
        free_validation_passed
        if free_validation_config is not None
        else actual_hash == expected_hash or policy_only_surface_transition
    )
    passed = (
        contract_matches_worktree
        and relevant_source_matches_worktree
        and not release_changes
        and not policy_baseline_conflict
        and not policy_product_conflict
        and not baseline_product_conflict
        and not semantic_baselines
        and semantic_gate_passed
        and (live_status_accepted or not args.require_live_baseline)
    )
    result = {
        "schema_version": "whalecode-cache-regression-gate-v1",
        "status": "pass" if passed else "blocked",
        "source": args.source,
        "subject_commit": subject_commit,
        "actual_surface_sha256": actual_hash,
        "expected_surface_sha256": expected_hash,
        "baseline_status": baseline_status,
        "require_live_baseline": args.require_live_baseline,
        "contract_matches_worktree": contract_matches_worktree,
        "policy_changes": policy_changes,
        "baseline_changed": baseline_changed,
        "policy_baseline_conflict": policy_baseline_conflict,
        "policy_product_conflict": policy_product_conflict,
        "semantic_baseline_changes": semantic_baselines,
        "baseline_product_conflict": baseline_product_conflict,
        "relevant_source_matches_worktree": relevant_source_matches_worktree,
        "free_validation_required": free_validation_required,
        "free_validation": free_validation,
        "require_clean_subject": args.require_clean_subject,
        "release_relevant_changes": release_changes,
        "surface_file_count": len(entries),
        "sensitive_changes": changes,
    }
    if args.json_output:
        output = args.json_output
        if not output.is_absolute():
            output = repo / output
        write_json(output, result)

    if passed:
        if policy_changes:
            suffix = "（待验证政策变更；发布保持阻断）"
        elif baseline_status == "structural_bootstrap":
            suffix = "（尚待首次真实缓存基线）"
        elif baseline_status == "live_regression_failed":
            suffix = "（当前指纹未变；最近一次 live 回归失败）"
        else:
            suffix = ""
        if free_validation_required:
            suffix = f"（免费 final-wire 验证通过）{suffix}"
        print(f"cache regression gate: PASS {actual_hash}{suffix}")
        return 0

    print("cache regression gate: BLOCKED")
    print(f"expected surface: {expected_hash}")
    print(f"actual surface:   {actual_hash}")
    if not contract_matches_worktree:
        print("- 暂存合同与工作区合同不一致；请完整暂存或还原合同后重试。")
    if not relevant_source_matches_worktree:
        print("- 缓存相关暂存源码与工作区不一致；免费验证无法证明即将提交的内容。")
    if policy_baseline_conflict:
        print("- 门禁政策与基线不能在同一提交中变更。")
    if policy_product_conflict:
        print("- 门禁政策变更必须与缓存敏感产品变更分开提交。")
    if baseline_product_conflict:
        print("- final-wire 基准与缓存敏感产品代码不能在同一提交中变更。")
    if semantic_baselines:
        print("- final-wire 基准只能通过独立的证据晋升流程更新：")
        for path in semantic_baselines:
            print(f"  - {path}")
    if policy_changes:
        print("门禁政策变更：")
        for path in policy_changes:
            print(f"- {path}")
    if release_changes:
        print("- release 受检 HEAD 与当前相关工作区不一致：")
        for change in release_changes:
            print(f"  - {change['path']} ({change['state']})")
    if not live_status_accepted:
        print(f"- 当前基线状态为 {baseline_status}，尚未达到 live_verified。")
    if changes:
        print("可能影响缓存命中的变更：")
        for change in changes:
            reasons = "；".join(rule["reason"] for rule in change["rules"])
            print(f"- {change['path']}: {reasons}")
    elif free_validation_config is None and actual_hash != expected_hash:
        print("- 当前缓存敏感面与已验证基线不一致；差异可能来自此前未验证提交。")
    if free_validation_required and free_validation is not None and not free_validation["passed"]:
        print("免费 final-wire 验证失败：")
        for command in free_validation["commands"]:
            print(
                f"- {command['id']}: {command['status']} "
                f"(exit={command['exit_code']}, timeout={command['timed_out']})"
            )
            for line in command["output_tail"]:
                print(f"  {line}")
        print("下一步：定位首个 final-wire 差异；确认是有意变更后再申请最小真实回归预算。")
    elif semantic_baselines or baseline_product_conflict:
        print("下一步：保留旧基准，先完成独立真实证据验证，再由晋升流程更新基准。")
    elif free_validation_config is None and actual_hash != expected_hash:
        print("下一步：说明变更为何会影响 provider 前缀，并向用户申请 2 个 sample run 预算。")
    else:
        print("下一步：修复上述门禁条件；不要通过修改基准掩盖失败。")
    return 20


if __name__ == "__main__":
    raise SystemExit(main())
