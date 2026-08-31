from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

from generate_candidate_replay import parse_apply_failures  # noqa: E402


class CandidateReplayTests(unittest.TestCase):
    def test_parse_apply_failures_keeps_conflicts_and_hard_failures(self) -> None:
        output = "\n".join(
            (
                "Applied patch to 'third_party/codex-cli/codex-rs/core/src/client.rs' with conflicts.",
                "error: patch failed: third_party/codex-cli/codex-rs/core/src/tools/router.rs:1",
                "error: third_party/codex-cli/other.rs: does not exist in index",
            )
        )
        self.assertEqual(
            {
                "codex-rs/core/src/client.rs": "three-way-conflict",
                "codex-rs/core/src/tools/router.rs": "patch-apply-failure",
            },
            parse_apply_failures(output),
        )


if __name__ == "__main__":
    unittest.main()
