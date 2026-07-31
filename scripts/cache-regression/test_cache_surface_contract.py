#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path

from cache_surface import load_contract, matching_rules
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
    "third_party/codex-cli/codex-rs/tools/src/tool_spec.rs",
    "third_party/codex-cli/codex-rs/model-provider-info/src/lib.rs",
    "third_party/codex-cli/codex-rs/models-manager/models.json",
]


class CacheSurfaceContractTest(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load_contract(CONTRACT_PATH)

    def test_known_production_entries_are_risk_classified(self) -> None:
        missing = [
            path for path in KNOWN_PRODUCTION_ENTRIES if not matching_rules(path, self.contract)
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


if __name__ == "__main__":
    unittest.main()
