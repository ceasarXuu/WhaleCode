from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("check_zero_base.py")
SPEC = importlib.util.spec_from_file_location("check_zero_base", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class ZeroBaseGateTests(unittest.TestCase):
    def test_detects_retired_symbol_in_active_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "third_party/codex-cli/codex-rs/core/src/legacy.rs"
            source.parent.mkdir(parents=True)
            source.write_text("struct TaskSpaceEventStore;\n", encoding="utf-8")

            findings = MODULE.scan_zero_base(root)

            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0].symbol, "TaskSpaceEventStore")
            self.assertEqual(findings[0].line, 1)

    def test_ignores_historical_documentation(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            history = root / "docs/history.md"
            history.parent.mkdir(parents=True)
            history.write_text("Retired: taskspace_control\n", encoding="utf-8")

            self.assertEqual(MODULE.scan_zero_base(root), [])

    def test_detects_retired_map_schema_in_active_consumer(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "third_party/codex-cli/codex-rs/tui/src/viewer.rs"
            source.parent.mkdir(parents=True)
            source.write_text("const FIELD: &str = \"terminal_history\";\n", encoding="utf-8")

            findings = MODULE.scan_zero_base(root)

            self.assertEqual([finding.symbol for finding in findings], ["terminal_history"])

    def test_ignores_rust_inline_rejection_fixtures(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "third_party/codex-cli/codex-rs/protocol/src/taskspace.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "pub struct CurrentMap;\n#[cfg(test)]\nmod tests { const OLD: &str = \"TaskSpaceMapEdge\"; }\n",
                encoding="utf-8",
            )

            self.assertEqual(MODULE.scan_zero_base(root), [])

    def test_does_not_flag_standard_output_references(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "third_party/codex-cli/codex-rs/core/src/standard_tool.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "struct FunctionCallOutput { output_ref: String, raw_output: String }\n",
                encoding="utf-8",
            )

            self.assertEqual(MODULE.scan_zero_base(root), [])

    def test_accepts_clean_active_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source = root / "third_party/codex-cli/codex-rs/tools/src/registry.rs"
            source.parent.mkdir(parents=True)
            source.write_text("struct StandardToolRegistry;\n", encoding="utf-8")

            self.assertEqual(MODULE.scan_zero_base(root), [])


if __name__ == "__main__":
    unittest.main()
