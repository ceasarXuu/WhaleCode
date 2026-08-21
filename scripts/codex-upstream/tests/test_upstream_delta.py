from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

from generate_upstream_delta import crate_owner, generated_kind  # noqa: E402


class UpstreamDeltaMappingTests(unittest.TestCase):
    def test_workspace_files_have_workspace_owner(self) -> None:
        self.assertEqual("codex-rs-workspace", crate_owner("codex-rs/Cargo.lock"))

    def test_crate_path_uses_first_workspace_segment(self) -> None:
        self.assertEqual("core", crate_owner("codex-rs/core/src/lib.rs"))

    def test_non_rust_path_uses_top_level_owner(self) -> None:
        self.assertEqual("docs", crate_owner("docs/example.md"))

    def test_generated_families_are_explicit(self) -> None:
        self.assertEqual(
            "app-server-typescript",
            generated_kind(
                "codex-rs/app-server-protocol/schema/typescript/ClientRequest.ts"
            ),
        )
        self.assertEqual(
            "insta-snapshot",
            generated_kind("codex-rs/tui/src/snapshots/example.snap"),
        )

    def test_source_file_is_not_marked_generated(self) -> None:
        self.assertIsNone(generated_kind("codex-rs/core/src/lib.rs"))


if __name__ == "__main__":
    unittest.main()
