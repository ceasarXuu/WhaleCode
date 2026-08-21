from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_context.py"
SPEC = importlib.util.spec_from_file_location("workspace_context_state", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)


class WorkspaceStateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "same-name"
        self.root.mkdir()
        self.common = Path(self.temp.name) / "git-common"
        self.common.mkdir()
        self.resources = {
            "state_root": "/state/workspace",
            "runtime_home": "/state/workspace/home",
            "data_root": "/data/workspace",
            "binary_dir": "/data/workspace/bin",
        }
        self.current = {
            "workspace_id": workspace_context.derive_workspace_id(self.root),
            "canonical_root": str(self.root.resolve()),
            "git_common_dir": str(self.common.resolve()),
            "branch": "feature-a",
            "detached_head": False,
            "resources": self.resources,
        }
        self.marker = {
            "schema_version": 1,
            "workspace_id": self.current["workspace_id"],
            "canonical_root": self.current["canonical_root"],
            "git_common_dir": self.current["git_common_dir"],
            "branch": self.current["branch"],
            "resources": self.resources,
            "last_doctor": {"status": "passed", "diagnostic_codes": []},
        }

    def test_same_basename_roots_have_distinct_stable_ids(self) -> None:
        other_parent = Path(self.temp.name) / "other"
        other_root = other_parent / self.root.name
        other_root.mkdir(parents=True)

        first = workspace_context.derive_workspace_id(self.root)
        repeated = workspace_context.derive_workspace_id(self.root / ".")
        second = workspace_context.derive_workspace_id(other_root)

        self.assertEqual(first, repeated)
        self.assertNotEqual(first, second)
        self.assertRegex(first, r"^same-name-[a-f0-9]{10}$")

    def test_unbootstrapped_and_ready_states(self) -> None:
        self.assertEqual(
            {"code": "Unbootstrapped", "reason_code": "marker_missing"},
            workspace_context.evaluate_state(None, self.current),
        )
        self.assertEqual(
            {"code": "Ready", "reason_code": "workspace_ready"},
            workspace_context.evaluate_state(self.marker, self.current),
        )

    def test_branch_switch_is_stale_but_switch_back_is_current(self) -> None:
        switched = {**self.current, "branch": "feature-b"}
        advanced = {**self.current, "head": "different-commit-not-part-of-state"}

        self.assertEqual(
            "branch_changed",
            workspace_context.evaluate_state(self.marker, switched)["reason_code"],
        )
        self.assertEqual("Ready", workspace_context.evaluate_state(self.marker, advanced)["code"])

    def test_collisions_staleness_and_doctor_failure_are_distinct(self) -> None:
        cases = [
            ({**self.marker, "workspace_id": "other-0000000000"}, "Conflict"),
            ({**self.marker, "canonical_root": "/other/root"}, "Conflict"),
            ({**self.marker, "git_common_dir": "/other/git"}, "Stale"),
            ({**self.marker, "resources": {**self.resources, "binary_dir": "/other/bin"}}, "Stale"),
            (
                {**self.marker, "last_doctor": {"status": "failed", "diagnostic_codes": ["binary_missing"]}},
                "DoctorFailed",
            ),
        ]
        for marker, expected in cases:
            with self.subTest(expected=expected, marker=marker):
                self.assertEqual(expected, workspace_context.evaluate_state(marker, self.current)["code"])
        detached = {**self.current, "detached_head": True, "branch": None}
        self.assertEqual("Stale", workspace_context.evaluate_state(self.marker, detached)["code"])

    def test_marker_schema_required_fields_match_fixture(self) -> None:
        schema_path = MODULE_PATH.parent / "schemas" / "workspace-identity.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))

        self.assertEqual(set(schema["required"]), set(self.marker))
        self.assertEqual(
            set(schema["properties"]["resources"]["required"]),
            set(self.marker["resources"]),
        )
        self.assertEqual(
            set(schema["properties"]["last_doctor"]["required"]),
            set(self.marker["last_doctor"]),
        )


if __name__ == "__main__":
    unittest.main()
