#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path

from cache_surface import (
    is_cache_control_plane_path,
    is_cache_evidence_path,
    load_contract,
    matching_rules,
)
from free_cache_contracts import validate_free_validation


REPO = Path(__file__).resolve().parents[2]
CONTRACT_PATH = REPO / "benchmarks/cache-regression/cache-surface-contract.json"

KNOWN_PRODUCTION_ENTRIES = [
    "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_standard.md",
    "third_party/codex-cli/codex-rs/core/src/context/mod.rs",
    "third_party/codex-cli/codex-rs/core/src/session/mod.rs",
    "third_party/codex-cli/codex-rs/core/src/session/turn.rs",
    "third_party/codex-cli/codex-rs/core/src/session/mcp.rs",
    "third_party/codex-cli/codex-rs/core/src/client.rs",
    "third_party/codex-cli/codex-rs/core/src/tools/spec.rs",
    "third_party/codex-cli/codex-rs/core/src/mcp_tool_exposure.rs",
    "third_party/codex-cli/codex-rs/core/src/plugins/injection.rs",
    "third_party/codex-cli/codex-rs/core/src/skills.rs",
    "third_party/codex-cli/codex-rs/core/src/compact.rs",
    "third_party/codex-cli/codex-rs/codex-api/src/common.rs",
    "third_party/codex-cli/codex-rs/codex-api/src/endpoint/responses.rs",
    "third_party/codex-cli/codex-rs/codex-api/src/requests/responses.rs",
    "third_party/codex-cli/codex-rs/protocol/src/models.rs",
    "third_party/codex-cli/codex-rs/Cargo.lock",
    "third_party/codex-cli/codex-rs/tools/src/tool_spec.rs",
    "third_party/codex-cli/codex-rs/model-provider-info/src/lib.rs",
    "third_party/codex-cli/codex-rs/models-manager/models.json",
]


class CacheSurfaceContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load_contract(CONTRACT_PATH)

    def test_known_production_entries_are_risk_classified(self) -> None:
        missing = [
            path
            for path in KNOWN_PRODUCTION_ENTRIES
            if not matching_rules(path, self.contract)
        ]
        self.assertEqual(missing, [])

    def test_test_sources_are_not_product_risk_inputs(self) -> None:
        paths = [
            "scripts/cache-regression/test_cache_regression_gate.py",
            "third_party/codex-cli/codex-rs/core/tests/suite/cache_payload_contract.rs",
            "third_party/codex-cli/codex-rs/core/tests/suite/snapshots/example.snap",
        ]
        self.assertEqual(
            [path for path in paths if matching_rules(path, self.contract)], []
        )

    def test_free_validation_contract_is_well_formed(self) -> None:
        validate_free_validation(self.contract["free_validation"])

    def test_final_wire_fixtures_are_control_plane_not_product_inputs(self) -> None:
        paths = [
            "third_party/codex-cli/codex-rs/core/tests/common/cache_payload.rs",
            "third_party/codex-cli/codex-rs/core/tests/suite/cache_final_wire.rs",
            "third_party/codex-cli/codex-rs/core/tests/suite/cache_payload_mcp_contract.rs",
        ]
        self.assertTrue(all(is_cache_control_plane_path(path) for path in paths))
        self.assertTrue(all(not matching_rules(path, self.contract) for path in paths))

    def test_formal_consumers_and_test_registries_are_control_plane(self) -> None:
        paths = [
            "scripts/taskspace-benchmark/lib/e3-start-gate.ps1",
            "scripts/taskspace-benchmark/lib/cache-regression-fixture.ps1",
            "scripts/taskspace-benchmark/write-release-decision.ps1",
            "third_party/codex-cli/codex-rs/core/tests/all.rs",
            "third_party/codex-cli/codex-rs/core/tests/common/lib.rs",
            "third_party/codex-cli/codex-rs/core/tests/suite/mod.rs",
        ]
        self.assertTrue(all(is_cache_control_plane_path(path) for path in paths))
        self.assertTrue(all(not matching_rules(path, self.contract) for path in paths))

    def test_cache_run_evidence_is_release_relevant_not_policy(self) -> None:
        path = "benchmarks/cache-regression/evidence/WAR-1/CACHE-001/metrics.json"
        self.assertTrue(is_cache_evidence_path(path))
        self.assertFalse(is_cache_control_plane_path(path))

    def test_final_wire_matrix_emits_change_report_and_includes_tool_wire(self) -> None:
        commands = {
            command["id"]: command
            for command in self.contract["free_validation"]["commands"]
        }
        matrix = commands["final_wire_matrix"]

        self.assertEqual(matrix["argv"][-1], "cache_payload_")
        self.assertEqual(matrix["change_report"]["type"], "final_wire_snapshot_set")
        tool_wire = commands["tool_wire_contract"]
        self.assertEqual(
            tool_wire["argv"][-1], "taskspace_tools_use_production_wire_schema"
        )
        baseline_patterns = self.contract["free_validation"]["semantic_baseline_globs"]
        protected = {
            path for pattern in baseline_patterns for path in REPO.glob(pattern)
        }
        reported = {
            path
            for command in commands.values()
            for pattern in command.get("change_report", {}).get("baseline_globs", [])
            for path in REPO.glob(pattern)
        }
        self.assertEqual(reported, protected)


if __name__ == "__main__":
    unittest.main()
