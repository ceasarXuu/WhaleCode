#!/usr/bin/env python3
"""Promote one passing live result to the cache surface baseline."""

from __future__ import annotations

import argparse
from pathlib import Path

from cache_surface import load_contract, surface_snapshot, write_json


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("benchmarks/cache-regression/cache-surface-contract.json"),
    )
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    contract_path = args.contract if args.contract.is_absolute() else repo / args.contract
    result_path = args.result if args.result.is_absolute() else repo / args.result
    contract = load_contract(contract_path)
    result = __import__("json").loads(result_path.read_text(encoding="utf-8"))
    if result.get("schema_version") != "whalecode-cache-hit-regression-v1":
        raise SystemExit("invalid cache regression result schema")
    if result.get("status") != "pass":
        raise SystemExit("cannot promote a non-passing cache regression")
    current_hash, _ = surface_snapshot(repo, contract, "worktree")
    if result.get("surface_sha256") != current_hash:
        raise SystemExit("result does not describe the current cache-sensitive surface")
    if result.get("actual_sample_runs", 0) > 2:
        raise SystemExit("result exceeds the approved two-sample regression shape")
    contract["baseline"] = {
        "surface_sha256": current_hash,
        "status": "live_verified",
        "source_commit": result["subject_commit"],
        "live_result_path": str(result_path.relative_to(repo)),
        "note": "由 Standard + map-request 最简缓存回归晋升。",
        "request_2_plus_hit_rate": {
            arm["arm"]: arm["request_2_plus_hit_rate"] for arm in result["arms"]
        },
    }
    write_json(contract_path, contract)
    print(f"promoted cache surface baseline: {current_hash}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
