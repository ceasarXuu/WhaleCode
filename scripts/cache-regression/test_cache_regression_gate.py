#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from cache_surface import load_contract, surface_snapshot, write_json


SCRIPT_DIR = Path(__file__).resolve().parent
GATE = SCRIPT_DIR / "check_cache_regression_gate.py"


def run(*args: str, cwd: Path, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, check=check, text=True, capture_output=True)


class CacheRegressionGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        run("git", "init", "-q", cwd=self.repo)
        run("git", "config", "user.email", "test@example.com", cwd=self.repo)
        run("git", "config", "user.name", "Test", cwd=self.repo)
        (self.repo / "prompt").mkdir()
        (self.repo / "prompt/base.md").write_text("stable\n", encoding="utf-8")
        (self.repo / "ordinary.txt").write_text("ordinary\n", encoding="utf-8")
        policy_path = self.repo / "scripts/cache-regression"
        policy_path.mkdir(parents=True)
        (policy_path / "check_cache_regression_gate.py").write_text(
            "# fixture policy\n", encoding="utf-8"
        )
        self.contract_path = self.repo / "contract.json"
        contract = {
            "schema_version": "whalecode-cache-surface-v1",
            "baseline": {
                "surface_sha256": "",
                "status": "structural_bootstrap",
                "source_commit": "fixture",
                "live_result_path": None,
            },
            "surface_rules": [
                {
                    "id": "prompt",
                    "globs": ["prompt/**"],
                    "reason": "改变固定提示词前缀",
                }
            ],
            "live_regression": {},
        }
        write_json(self.contract_path, contract)
        run("git", "add", ".", cwd=self.repo)
        contract = load_contract(self.contract_path)
        baseline, _ = surface_snapshot(self.repo, contract, "index")
        contract["baseline"]["surface_sha256"] = baseline
        write_json(self.contract_path, contract)
        run("git", "add", ".", cwd=self.repo)
        run("git", "commit", "-qm", "baseline", cwd=self.repo)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def gate_from_source(
        self, source: str, *extra: str
    ) -> subprocess.CompletedProcess[str]:
        return run(
            "python3",
            str(GATE),
            "--repo-root",
            str(self.repo),
            "--contract",
            str(self.contract_path),
            "--source",
            source,
            *extra,
            cwd=self.repo,
            check=False,
        )

    def gate(self, *extra: str) -> subprocess.CompletedProcess[str]:
        return self.gate_from_source("index", *extra)

    def test_ordinary_change_passes(self) -> None:
        (self.repo / "ordinary.txt").write_text("changed\n", encoding="utf-8")
        run("git", "add", "ordinary.txt", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_sensitive_change_blocks_with_reason(self) -> None:
        (self.repo / "prompt/base.md").write_text("changed\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 20)
        self.assertIn("prompt/base.md", result.stdout)
        self.assertIn("改变固定提示词前缀", result.stdout)
        self.assertIn("2 个 sample run", result.stdout)

    def test_matching_promoted_hash_unblocks(self) -> None:
        (self.repo / "prompt/base.md").write_text("verified\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        contract = load_contract(self.contract_path)
        promoted, _ = surface_snapshot(self.repo, contract, "index")
        contract["baseline"]["surface_sha256"] = promoted
        contract["baseline"]["status"] = "live_verified"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn("尚待首次", result.stdout)

    def test_failed_live_baseline_allows_unrelated_commit(self) -> None:
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "live_regression_failed"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 0)
        self.assertIn("live 回归失败", result.stdout)

    def test_release_gate_blocks_failed_live_baseline(self) -> None:
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "live_regression_failed"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        result = self.gate("--require-live-baseline")
        self.assertEqual(result.returncode, 20)
        self.assertIn("live_regression_failed", result.stdout)
        self.assertNotIn("敏感面与已验证基线不一致", result.stdout)

    def test_release_gate_blocks_structural_bootstrap(self) -> None:
        result = self.gate("--require-live-baseline")
        self.assertEqual(result.returncode, 20)
        self.assertIn("structural_bootstrap", result.stdout)
        self.assertIn("尚未达到 live_verified", result.stdout)
        self.assertNotIn("敏感面与已验证基线不一致", result.stdout)

    def test_index_source_reads_the_staged_contract(self) -> None:
        worktree_contract = self.contract_path.read_text(encoding="utf-8")
        staged_contract = load_contract(self.contract_path)
        staged_contract["surface_rules"] = []
        write_json(self.contract_path, staged_contract)
        run("git", "add", "contract.json", cwd=self.repo)
        self.contract_path.write_text(worktree_contract, encoding="utf-8")

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("暂存合同与工作区合同不一致", result.stdout)
        self.assertIn("敏感面与已验证基线不一致", result.stdout)

    def test_head_source_reads_the_committed_contract(self) -> None:
        worktree_contract = load_contract(self.contract_path)
        worktree_contract["surface_rules"] = []
        write_json(self.contract_path, worktree_contract)

        result = self.gate_from_source("head")

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_policy_and_baseline_change_cannot_self_authorize(self) -> None:
        policy_path = self.repo / "scripts/cache-regression/check_cache_regression_gate.py"
        policy_path.write_text("# changed policy\n", encoding="utf-8")
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "live_verified"
        write_json(self.contract_path, contract)
        run("git", "add", "scripts", "contract.json", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("门禁政策与基线不能在同一提交中变更", result.stdout)

    def test_contract_policy_change_can_land_without_promotion(self) -> None:
        contract = load_contract(self.contract_path)
        contract["surface_rules"] = []
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("待验证政策变更", result.stdout)

    def test_policy_change_cannot_include_sensitive_product_change(self) -> None:
        policy_path = self.repo / "scripts/cache-regression/check_cache_regression_gate.py"
        policy_path.write_text("# changed policy\n", encoding="utf-8")
        (self.repo / "prompt/base.md").write_text("changed\n", encoding="utf-8")
        run("git", "add", "scripts", "prompt/base.md", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("门禁政策变更必须与缓存敏感产品变更分开提交", result.stdout)


if __name__ == "__main__":
    unittest.main()
