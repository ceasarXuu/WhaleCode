from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


class WorkspaceDocumentationTest(unittest.TestCase):
    def test_required_documents_link_the_authoritative_runbook(self) -> None:
        runbook = ROOT / "runbooks/local-workspace-safety.md"
        self.assertTrue(runbook.is_file())
        for relative in ("AGENTS.md", "README.md"):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("runbooks/local-workspace-safety.md", text)
        workflow = (ROOT / "docs/runbooks/development-workflow.md").read_text(
            encoding="utf-8"
        )
        self.assertIn("../../runbooks/local-workspace-safety.md", workflow)

    def test_agent_rules_require_plan_apply_and_ready_checks(self) -> None:
        text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        for token in (
            "bootstrap plan --json",
            "bootstrap apply --expect <fingerprint>",
            "install-whale-local.sh --scope workspace",
            "doctor --require-binary",
            "require-ready",
            "全局`whale-dev`",
            "全局`whale`只用于release",
        ):
            self.assertIn(token, text)

    def test_runbook_documents_all_states_and_no_fallback_boundary(self) -> None:
        text = (ROOT / "runbooks/local-workspace-safety.md").read_text(encoding="utf-8")
        for state in (
            "Unbootstrapped",
            "Ready",
            "Stale",
            "Conflict",
            "DoctorFailed",
        ):
            self.assertIn(state, text)
        self.assertIn("不把PATH上的全局`whale`", text)
        self.assertIn("`whale-dev`必须按cwd解析worktree", text)
        self.assertIn("开发流程规范，不是产品运行时协议", text)


if __name__ == "__main__":
    unittest.main()
