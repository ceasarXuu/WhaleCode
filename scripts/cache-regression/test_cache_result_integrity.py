#!/usr/bin/env python3

from __future__ import annotations

import copy
import unittest

from cache_run_ledger import settle_entry


def cleanup_proof() -> dict:
    return {
        "status": "verified_absent",
        "container_ids": [],
        "stable_empty_polls": 3,
        "network_cleanup_status": "verified_absent",
        "network_ids": [],
        "secret_cleanup_status": "verified_absent",
        "secret_paths": [],
        "error": "",
    }


class CacheResultIntegrityTest(unittest.TestCase):
    def setUp(self) -> None:
        selection = {
            "model": "deepseek-v4-flash",
            "samples": ["authorized"],
            "arms": ["standard"],
            "repeat": 1,
            "planned_sample_runs": 1,
            "retry_sample_run_limit": 0,
            "maximum_sample_runs": 1,
            "stop_conditions": ["after_any_run_failure"],
            "selection_reason": "fixture",
        }
        self.entry = {
            "status": "running",
            "execution": {},
            "tokens": {},
            "monetary_cost": {
                "pricing_snapshot": {
                    "currency": "USD",
                    "cached_input_per_million": 0.0028,
                    "uncached_input_per_million": 0.14,
                    "output_per_million": 0.28,
                }
            },
            "evidence": {"approved_selection": selection},
        }
        scope = {"sample": "authorized", "arm": "standard", "repeat": 1}
        self.result = {
            "status": "completed",
            "started_at": "start",
            "ended_at": "end",
            "elapsed_seconds": 1.0,
            "actual_sample_runs": 1,
            "unverified_scope": [],
            "run_root": "target/run",
            "result_path": "benchmarks/result.json",
            "runner_exit_code": 0,
            "attempts": [
                {
                    **scope,
                    "run_id": "CACHE-001",
                    "status": "completed",
                    "exit_code": 0,
                    "timed_out": False,
                    "elapsed_seconds": 1.0,
                    "post_run_cleanup": cleanup_proof(),
                    "provider_boundary_request_count": 2,
                }
            ],
            "observations": [
                {
                    **scope,
                    "run_id": "CACHE-001",
                    "provider_requests": 2,
                    "input_tokens": 100,
                    "cached_input_tokens": 80,
                    "uncached_input_tokens": 20,
                    "output_tokens": 10,
                }
            ],
        }

    def test_direct_settlement_rejects_missing_or_unauthorized_scope(self) -> None:
        for mutation in ("missing", "unauthorized"):
            with self.subTest(mutation=mutation):
                result = copy.deepcopy(self.result)
                for item in (result["attempts"][0], result["observations"][0]):
                    if mutation == "missing":
                        for key in ("sample", "arm", "repeat"):
                            item.pop(key)
                    else:
                        item["sample"] = "unauthorized"
                with self.assertRaisesRegex(
                    ValueError, "scope is unauthorized|evidence is inconsistent"
                ):
                    settle_entry(copy.deepcopy(self.entry), result)


if __name__ == "__main__":
    unittest.main()
