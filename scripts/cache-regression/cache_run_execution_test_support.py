#!/usr/bin/env python3
"""Shared repository fixture for paid cache-run execution tests."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_provider_route_test_support import route_summary
from cache_surface import write_json


class CacheRunExecutionFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        contract_path = (
            self.repo / "benchmarks/cache-regression/cache-surface-contract.json"
        )
        contract_path.parent.mkdir(parents=True)
        self.pricing = {
            "currency": "USD",
            "cached_input_per_million": 0.0028,
            "uncached_input_per_million": 0.14,
            "output_per_million": 0.28,
        }
        write_json(
            contract_path,
            {
                "schema_version": "whalecode-cache-surface-v1",
                "baseline": {
                    "surface_sha256": "fixture",
                    "status": "live_regression_failed",
                },
                "surface_rules": [],
                "pricing_snapshot": self.pricing,
            },
        )
        write_json(
            self.repo / "benchmarks/whale-agent-run-ledger.json",
            {
                "schema_version": "whale-agent-run-ledger-v1",
                "updated_at": "2026-08-01T00:00:00+08:00",
                "entries": [],
            },
        )
        self.proposal_path = self.repo / "benchmarks/cache-regression/proposal.json"
        self.authorization_path = (
            self.repo / "benchmarks/cache-regression/authorization.json"
        )
        self.proposal = {
            "proposal_id": "CBP-FIXTURE",
            "proposal_sha256": "a" * 64,
            "subject_commit": "b" * 40,
            "surface_sha256": "c" * 64,
            "selection": {
                "model": "deepseek-v4-flash",
                "samples": ["simple"],
                "arms": ["standard", "map-request"],
                "repeat": 1,
                "planned_sample_runs": 2,
                "retry_sample_run_limit": 0,
                "maximum_sample_runs": 2,
                "stop_conditions": ["after_any_run_failure"],
                "selection_reason": "approved fixture",
            },
            "per_sample_run_limits": {
                "provider_requests": 10,
                "elapsed_seconds": 60,
                "cleanup_grace_seconds": 120,
            },
            "per_sample_run_budget_limits": {
                "input_tokens": 1000,
                "output_tokens": 100,
                "estimated_cost": 0.5,
                "currency": "USD",
            },
            "approved_maximums": {
                "provider_requests": 20,
                "input_tokens": 2000,
                "output_tokens": 200,
                "elapsed_seconds": 360,
                "estimated_cost": 1.0,
                "currency": "USD",
            },
            "provider_hard_limits": {
                "max_input_tokens_per_request": 1_000_000,
                "max_output_tokens_per_request": 384_000,
            },
            "pricing_snapshot": self.pricing,
            "evidence_boundary": "fixture scope only",
        }
        self.authorization = {
            "authorization_id": "CBA-FIXTURE-001",
            "approval_reference": "fixture approval",
        }
        write_json(self.proposal_path, self.proposal)
        write_json(self.authorization_path, self.authorization)
        self.workspace_binary = self.repo / "target/debug/whale"
        self.workspace_binary_patcher = patch(
            "run_cache_hit_regression.resolve_workspace_binary",
            return_value=self.workspace_binary,
        )
        self.workspace_binary_mock = self.workspace_binary_patcher.start()
        self.addCleanup(self.workspace_binary_patcher.stop)
        self.provider_route = route_summary()
        self.binary_health_patcher = patch(
            "run_cache_hit_regression.run_whale_binary_health_preflight",
            return_value={"status": "passed"},
        )
        self.binary_health_mock = self.binary_health_patcher.start()
        self.addCleanup(self.binary_health_patcher.stop)
        self.route_preflight_patcher = patch(
            "run_cache_hit_regression.run_provider_route_preflight",
            return_value=self.provider_route,
        )
        self.route_preflight_mock = self.route_preflight_patcher.start()
        self.addCleanup(self.route_preflight_patcher.stop)

    def tearDown(self) -> None:
        self.temp.cleanup()
