from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts/workspace-safety/check_workspace_references.py"


def load_gate():
    spec = importlib.util.spec_from_file_location("workspace_reference_gate_test", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class ReferenceGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gate = load_gate()

    def test_new_legacy_default_is_rejected(self) -> None:
        violations = self.gate.inspect_sources(
            {"scripts/new-runner.py": 'binary = "~/.whale/bin/whale"'},
            allowances=(),
            contracts={},
        )
        self.assertTrue(any("legacy-whale-bin" in item for item in violations))

    def test_new_shared_build_override_is_rejected(self) -> None:
        violations = self.gate.inspect_sources(
            {"scripts/new-runner.ps1": "$env:CARGO_TARGET_DIR = $Shared"},
            allowances=(),
            contracts={},
        )
        self.assertTrue(any("shared-cargo-target" in item for item in violations))

    def test_missing_preflight_token_is_rejected(self) -> None:
        violations = self.gate.inspect_sources(
            {"scripts/paid.py": "send_request()"},
            allowances=(),
            contracts={"scripts/paid.py": ("require_ready(",)},
        )
        self.assertEqual(
            violations,
            ["scripts/paid.py: missing workspace preflight token 'require_ready('"],
        )

    def test_narrow_allowance_is_accepted_and_capped(self) -> None:
        allowance = self.gate.Allowance(
            "scripts/user-install.sh", "legacy-whale-bin", 1, "Explicit user scope."
        )
        accepted = self.gate.inspect_sources(
            {"scripts/user-install.sh": "~/.whale/bin"},
            allowances=(allowance,),
            contracts={},
        )
        rejected = self.gate.inspect_sources(
            {"scripts/user-install.sh": "~/.whale/bin\n~/.whale/bin"},
            allowances=(allowance,),
            contracts={},
        )
        self.assertEqual(accepted, [])
        self.assertTrue(any("allowed 1" in item for item in rejected))

    def test_current_repository_passes(self) -> None:
        self.assertEqual(
            self.gate.inspect_sources(self.gate.discover_sources(ROOT)), []
        )


if __name__ == "__main__":
    unittest.main()
