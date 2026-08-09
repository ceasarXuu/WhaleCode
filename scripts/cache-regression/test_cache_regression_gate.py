#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from cache_baseline_test_support import stage_accepted_promotion
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
        (self.repo / "snapshots").mkdir()
        (self.repo / "snapshots/baseline.snap").write_text(
            '---\nsource: fixture.rs\n---\n{"wire": "stable"}\n',
            encoding="utf-8",
        )
        (self.repo / "free-validator.py").write_text(
            """import json
import os
from pathlib import Path
status = Path('free-validation-status.txt')
value = status.read_text(encoding='utf-8').strip() if status.exists() else 'pass'
report = os.environ.get('WHALE_CACHE_CHANGE_REPORT_DIR')
if report:
    snapshot = Path('snapshots/baseline.snap').read_text(encoding='utf-8')
    payload = json.loads(snapshot.split('\\n---\\n', 1)[1])
    if value == 'changed':
        payload = {'wire': 'changed'}
    Path(report, 'baseline.json').write_text(json.dumps(payload), encoding='utf-8')
print(f'fixture free validation: {value}')
raise SystemExit(7 if value == 'fail' else 0)
""",
            encoding="utf-8",
        )
        policy_path = self.repo / "scripts/cache-regression"
        policy_path.mkdir(parents=True)
        (policy_path / "check_cache_regression_gate.py").write_text(
            "# fixture policy\n", encoding="utf-8"
        )
        self.contract_path = self.repo / "contract.json"
        write_json(
            self.repo / "benchmarks/whale-agent-run-ledger.json",
            {"entries": []},
        )
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
            "free_validation": {
                "run_on_release": True,
                "semantic_baseline_globs": ["snapshots/*.snap"],
                "commands": [
                    {
                        "id": "fixture_final_wire",
                        "cwd": ".",
                        "argv": ["python3", "free-validator.py"],
                        "timeout_seconds": 10,
                        "change_report": {
                            "type": "final_wire_snapshot_set",
                            "baseline_globs": ["snapshots/*.snap"],
                        },
                    }
                ],
            },
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
        (self.repo / "free-validation-status.txt").write_text(
            "fail\n", encoding="utf-8"
        )
        (self.repo / "prompt/base.md").write_text("changed\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 20)
        self.assertIn("prompt/base.md", result.stdout)
        self.assertIn("改变固定提示词前缀", result.stdout)
        self.assertIn("免费 final-wire 验证失败", result.stdout)
        self.assertIn("fixture_final_wire: fail", result.stdout)

    def test_sensitive_semantic_equivalent_change_passes_free_validation(self) -> None:
        (self.repo / "prompt/base.md").write_text("comment-only\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("免费 final-wire 验证通过", result.stdout)

    def test_comparable_candidate_can_commit_but_release_stays_blocked(self) -> None:
        (self.repo / "free-validation-status.txt").write_text(
            "changed\n", encoding="utf-8"
        )
        (self.repo / "prompt/base.md").write_text("candidate\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)

        candidate = self.gate()

        self.assertEqual(candidate.returncode, 0, candidate.stdout + candidate.stderr)
        self.assertIn("候选变更；发布继续阻断", candidate.stdout)
        run("git", "commit", "-qm", "candidate", cwd=self.repo)
        release = self.gate_from_source("head", "--require-live-baseline")
        self.assertEqual(release.returncode, 20)
        self.assertIn("免费 final-wire 验证失败", release.stdout)

    def test_explicit_revalidation_requires_clean_failed_baseline(self) -> None:
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "live_regression_failed"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        run("git", "commit", "-qm", "failed baseline", cwd=self.repo)
        output = self.repo / "revalidation.json"

        result = self.gate_from_source(
            "head",
            "--require-live-baseline",
            "--require-clean-subject",
            "--request-revalidation",
            "--json-output",
            str(output),
        )

        self.assertEqual(result.returncode, 20)
        report = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(report["discovery_state"], "revalidation_requested")
        self.assertTrue(report["revalidation_requested"])
        self.assertTrue(report["free_validation"]["passed"])

    def test_explicit_revalidation_accepts_valid_stale_manifest(self) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        run("git", "commit", "-qm", "promote", cwd=self.repo)
        snapshot = self.repo / "snapshots/baseline.snap"
        snapshot.write_text(
            '---\nsource: fixture.rs\n---\n{"wire": "other"}\n',
            encoding="utf-8",
        )
        run("git", "add", "snapshots/baseline.snap", cwd=self.repo)
        run("git", "commit", "-qm", "change protected manifest", cwd=self.repo)
        output = self.repo / "revalidation.json"

        result = self.gate_from_source(
            "head",
            "--require-live-baseline",
            "--require-clean-subject",
            "--request-revalidation",
            "--json-output",
            str(output),
        )

        self.assertEqual(result.returncode, 20)
        report = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(report["discovery_state"], "revalidation_requested")
        self.assertTrue(report["accepted_baseline_validation"]["valid"])
        self.assertFalse(
            report["accepted_baseline_validation"]["manifest_matches_current"]
        )
        self.assertTrue(report["revalidation_requested"])

    def test_sensitive_staged_source_must_match_worktree(self) -> None:
        (self.repo / "prompt/base.md").write_text("staged\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        (self.repo / "prompt/base.md").write_text(
            "different worktree\n", encoding="utf-8"
        )

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("暂存源码与工作区不一致", result.stdout)

    def test_unstaged_baseline_cannot_control_index_validation(self) -> None:
        (self.repo / "prompt/base.md").write_text("staged\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        (self.repo / "snapshots/baseline.snap").write_text(
            '---\nsource: fixture.rs\n---\n{"wire": "unstaged"}\n',
            encoding="utf-8",
        )
        result = self.gate()
        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("不能代表 index", result.stdout)
        self.assertIn("snapshots/baseline.snap", result.stdout)

    def test_untracked_control_plane_input_blocks_index_validation(self) -> None:
        (self.repo / "prompt/base.md").write_text("staged\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        helper = self.repo / "scripts/cache-regression/untracked_helper.py"
        helper.write_text("# affects validation worktree\n", encoding="utf-8")
        result = self.gate()
        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("untracked_helper.py", result.stdout)

    def test_product_and_semantic_baseline_cannot_change_together(self) -> None:
        (self.repo / "prompt/base.md").write_text("changed\n", encoding="utf-8")
        (self.repo / "snapshots/baseline.snap").write_text(
            "changed snapshot\n", encoding="utf-8"
        )
        run("git", "add", "prompt/base.md", "snapshots/baseline.snap", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("基准与缓存敏感产品代码不能在同一提交", result.stdout)
        self.assertIn("snapshots/baseline.snap", result.stdout)

    def test_semantic_baseline_requires_independent_promotion(self) -> None:
        (self.repo / "snapshots/baseline.snap").write_text(
            "changed snapshot\n", encoding="utf-8"
        )
        run("git", "add", "snapshots/baseline.snap", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("独立的证据晋升流程", result.stdout)

    def test_product_change_cannot_self_authorize_with_baseline_hash(self) -> None:
        (self.repo / "prompt/base.md").write_text("verified\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        contract = load_contract(self.contract_path)
        promoted, _ = surface_snapshot(self.repo, contract, "index")
        contract["baseline"]["surface_sha256"] = promoted
        contract["baseline"]["status"] = "accepted"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("基准与缓存敏感产品代码不能在同一提交", result.stdout)

    def test_manual_failed_baseline_status_change_is_blocked(self) -> None:
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "live_regression_failed"
        write_json(self.contract_path, contract)
        run("git", "add", "contract.json", cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 20)

    def test_independent_accepted_promotion_passes(self) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        result = self.gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_accepted_promotion_rejects_tampered_decision(self) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        acceptance = (
            self.repo
            / "benchmarks/cache-regression/evidence/WAR-ACCEPTED/acceptance.json"
        )
        acceptance.write_text("{}\n", encoding="utf-8")
        run("git", "add", str(acceptance.relative_to(self.repo)), cwd=self.repo)
        result = self.gate()
        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("cache acceptance does not match result", result.stdout)

    def test_committed_accepted_baseline_passes_release_gate(self) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        run("git", "commit", "-qm", "promote", cwd=self.repo)
        result = self.gate_from_source("head", "--require-live-baseline")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_accepted_baseline_survives_semantically_unchanged_surface_commit(
        self,
    ) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        run("git", "commit", "-qm", "promote", cwd=self.repo)
        prompt = self.repo / "prompt/base.md"
        prompt.write_text(prompt.read_text(encoding="utf-8") + "\n", encoding="utf-8")
        run("git", "add", "prompt/base.md", cwd=self.repo)
        run("git", "commit", "-qm", "format-only", cwd=self.repo)
        result = self.gate_from_source("head", "--require-live-baseline")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_release_rejects_dirty_accepted_evidence(self) -> None:
        stage_accepted_promotion(self.repo, self.contract_path)
        run("git", "commit", "-qm", "promote", cwd=self.repo)
        acceptance = (
            self.repo
            / "benchmarks/cache-regression/evidence/WAR-ACCEPTED/acceptance.json"
        )
        acceptance.write_text("{}\n", encoding="utf-8")
        result = self.gate_from_source(
            "head", "--require-live-baseline", "--require-clean-subject"
        )
        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("acceptance.json (tracked)", result.stdout)

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
        self.assertIn("尚未形成有效的 accepted 基线", result.stdout)
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

    def test_head_source_reads_the_committed_contract(self) -> None:
        worktree_contract = load_contract(self.contract_path)
        worktree_contract["surface_rules"] = []
        write_json(self.contract_path, worktree_contract)

        result = self.gate_from_source("head")

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_release_head_rejects_dirty_sensitive_file(self) -> None:
        (self.repo / "prompt/base.md").write_text("dirty\n", encoding="utf-8")

        result = self.gate_from_source("head", "--require-clean-subject")

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("prompt/base.md (tracked)", result.stdout)

    def test_release_head_rejects_untracked_sensitive_file(self) -> None:
        (self.repo / "prompt/new.md").write_text("untracked\n", encoding="utf-8")

        result = self.gate_from_source("head", "--require-clean-subject")

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("prompt/new.md (untracked)", result.stdout)

    def test_release_head_allows_unrelated_dirty_file(self) -> None:
        (self.repo / "ordinary.txt").write_text("dirty\n", encoding="utf-8")

        result = self.gate_from_source("head", "--require-clean-subject")

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_release_head_rejects_dirty_control_plane(self) -> None:
        policy_path = (
            self.repo / "scripts/cache-regression/check_cache_regression_gate.py"
        )
        policy_path.write_text("# dirty policy\n", encoding="utf-8")

        result = self.gate_from_source("head", "--require-clean-subject")

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("check_cache_regression_gate.py (tracked)", result.stdout)

    def test_release_result_records_the_checked_head(self) -> None:
        output = self.repo / "gate-result.json"

        result = self.gate_from_source(
            "head", "--require-clean-subject", "--json-output", str(output)
        )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        payload = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(payload["source"], "head")
        self.assertEqual(
            payload["subject_commit"],
            run("git", "rev-parse", "HEAD", cwd=self.repo).stdout.strip(),
        )
        self.assertEqual(payload["release_relevant_changes"], [])

    def test_policy_and_baseline_change_cannot_self_authorize(self) -> None:
        policy_path = (
            self.repo / "scripts/cache-regression/check_cache_regression_gate.py"
        )
        policy_path.write_text("# changed policy\n", encoding="utf-8")
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "accepted"
        write_json(self.contract_path, contract)
        run("git", "add", "scripts", "contract.json", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("门禁政策与基线不能在同一提交中变更", result.stdout)

    def test_new_control_plane_helper_is_classified_as_policy(self) -> None:
        helper = self.repo / "scripts/cache-regression/new_helper.py"
        helper.write_text("# new policy helper\n", encoding="utf-8")
        contract = load_contract(self.contract_path)
        contract["baseline"]["status"] = "accepted"
        write_json(self.contract_path, contract)
        run("git", "add", "scripts", "contract.json", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("new_helper.py", result.stdout)
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
        policy_path = (
            self.repo / "scripts/cache-regression/check_cache_regression_gate.py"
        )
        policy_path.write_text("# changed policy\n", encoding="utf-8")
        (self.repo / "prompt/base.md").write_text("changed\n", encoding="utf-8")
        run("git", "add", "scripts", "prompt/base.md", cwd=self.repo)

        result = self.gate()

        self.assertEqual(result.returncode, 20, result.stdout + result.stderr)
        self.assertIn("门禁政策变更必须与缓存敏感产品变更分开提交", result.stdout)


if __name__ == "__main__":
    unittest.main()
