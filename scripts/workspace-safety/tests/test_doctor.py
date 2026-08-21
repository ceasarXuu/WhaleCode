from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "workspace_context.py"
SPEC = importlib.util.spec_from_file_location("workspace_context_doctor", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
workspace_context = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(workspace_context)
doctor = workspace_context.workspace_doctor


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
    ).stdout.strip()


class WorkspaceDoctorTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-q", "-b", "main")
        git(self.root, "config", "user.name", "Doctor Test")
        git(self.root, "config", "user.email", "doctor@example.invalid")
        (self.root / "README.md").write_text("fixture\n", encoding="utf-8")
        git(self.root, "add", "README.md")
        git(self.root, "commit", "-q", "-m", "initial")
        self.home = Path(self.temp.name) / "home"
        self.environment = {
            "HOME": str(self.home),
            "XDG_STATE_HOME": str(self.home / "state"),
            "XDG_DATA_HOME": str(self.home / "data"),
        }

    def apply(self) -> dict[str, object]:
        plan = workspace_context.build_plan(self.root, self.environment)
        return workspace_context.apply_plan(self.root, plan["fingerprint"], self.environment)

    def test_unbootstrapped_doctor_does_not_create_a_log(self) -> None:
        result = workspace_context.run_doctor(self.root, self.environment)

        self.assertEqual("failed", result["status"])
        self.assertEqual(["marker_missing"], result["diagnostic_codes"])
        self.assertFalse(result["audit"]["written"])
        self.assertFalse(self.home.exists())

    def test_apply_runs_doctor_and_records_redacted_events(self) -> None:
        self.apply()
        result = workspace_context.run_doctor(self.root, self.environment)
        context = workspace_context.resolve_context(self.root, self.environment)
        log_path = Path(context["resources"]["state_root"]) / "workspace-events.jsonl"
        events = [json.loads(line) for line in log_path.read_text().splitlines()]

        self.assertEqual("passed", result["status"])
        self.assertGreaterEqual(len(events), 2)
        self.assertEqual(0o600, log_path.stat().st_mode & 0o777)
        for event in events:
            self.assertLessEqual(set(event), doctor.EVENT_FIELDS)
        rendered = log_path.read_text(encoding="utf-8")
        self.assertNotIn(str(self.root), rendered)
        self.assertNotIn("HOME", rendered)

    def test_branch_and_permissions_have_stable_codes(self) -> None:
        self.apply()
        context = workspace_context.resolve_context(self.root, self.environment)
        os.chmod(context["resources"]["runtime_home"], 0o755)
        permission_result = workspace_context.run_doctor(self.root, self.environment)
        git(self.root, "checkout", "-q", "-b", "other")
        branch_result = workspace_context.run_doctor(self.root, self.environment)

        self.assertIn("runtime_home_not_private", permission_result["diagnostic_codes"])
        self.assertEqual(["branch_mismatch"], branch_result["diagnostic_codes"])

    def test_marker_binding_failures_have_stable_codes(self) -> None:
        self.apply()
        context = workspace_context.resolve_context(self.root, self.environment)
        marker_path = Path(context["marker_path"])
        marker = json.loads(marker_path.read_text(encoding="utf-8"))
        cases = [
            ({key: value for key, value in marker.items() if key != "branch"}, "marker_invalid"),
            ({**marker, "schema_version": 99}, "marker_schema_unsupported"),
            ({**marker, "workspace_id": "other-0000000000"}, "workspace_id_mismatch"),
            ({**marker, "canonical_root": "/other"}, "workspace_root_mismatch"),
            ({**marker, "git_common_dir": "/other/.git"}, "git_common_dir_mismatch"),
            ({**marker, "branch": "other"}, "branch_mismatch"),
            ({**marker, "resources": {}}, "resource_paths_mismatch"),
        ]
        for changed, code in cases:
            with self.subTest(code=code):
                result = doctor.diagnose(context, changed, self.environment)
                self.assertEqual([code], result["diagnostic_codes"])

    def test_each_resource_reports_missing_private_and_symlink(self) -> None:
        self.apply()
        context = workspace_context.resolve_context(self.root, self.environment)
        marker = json.loads(Path(context["marker_path"]).read_text())
        for name, raw_path in context["resources"].items():
            path = Path(raw_path)
            with self.subTest(name=name, failure="permissions"):
                path.chmod(0o755)
                result = doctor.diagnose(context, marker, self.environment)
                self.assertIn(f"{name}_not_private", result["diagnostic_codes"])
                path.chmod(0o700)
            with self.subTest(name=name, failure="symlink"):
                backup = path.with_name(f"{path.name}.fixture-backup")
                path.rename(backup)
                path.symlink_to(backup, target_is_directory=True)
                result = doctor.diagnose(context, marker, self.environment)
                self.assertIn(f"{name}_symlink", result["diagnostic_codes"])
                path.unlink()
                backup.rename(path)
            with self.subTest(name=name, failure="missing"):
                backup = path.with_name(f"{path.name}.fixture-backup")
                path.rename(backup)
                result = doctor.diagnose(context, marker, self.environment)
                self.assertIn(f"{name}_missing", result["diagnostic_codes"])
                backup.rename(path)

    def test_require_binary_validates_attestation(self) -> None:
        self.apply()
        missing = workspace_context.run_doctor(
            self.root, self.environment, require_binary=True
        )
        context = workspace_context.resolve_context(self.root, self.environment)
        binary = Path(context["resources"]["binary_dir"]) / "whale"
        binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        not_executable = workspace_context.run_doctor(
            self.root, self.environment, require_binary=True
        )
        binary.chmod(0o700)
        no_attestation = workspace_context.run_doctor(
            self.root, self.environment, require_binary=True
        )
        attestation = {
            "schema_version": 2,
            "status": "pass",
            "repo_root": str(self.root.resolve()),
            "worktree_clean": True,
            "whale_bin": str(binary.resolve()),
            "whale_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }
        Path(f"{binary}.build-attestation.json").write_text(
            json.dumps(attestation), encoding="utf-8"
        )
        invalid_attestation = {**attestation, "whale_binary_sha256": "0" * 64}
        Path(f"{binary}.build-attestation.json").write_text(
            json.dumps(invalid_attestation), encoding="utf-8"
        )
        invalid = workspace_context.run_doctor(
            self.root, self.environment, require_binary=True
        )
        Path(f"{binary}.build-attestation.json").write_text(
            json.dumps(attestation), encoding="utf-8"
        )
        ready = workspace_context.run_doctor(
            self.root, self.environment, require_binary=True
        )

        self.assertIn("binary_missing", missing["diagnostic_codes"])
        self.assertIn("binary_not_executable", not_executable["diagnostic_codes"])
        self.assertIn("binary_attestation_missing", no_attestation["diagnostic_codes"])
        self.assertIn("binary_attestation_invalid", invalid["diagnostic_codes"])
        self.assertEqual("passed", ready["status"])

    def test_linked_worktree_rejects_shared_common_root_build_override(self) -> None:
        linked = Path(self.temp.name) / "linked"
        git(self.root, "branch", "feature")
        git(self.root, "worktree", "add", "-q", str(linked), "feature")
        plan = workspace_context.build_plan(linked, self.environment)
        workspace_context.apply_plan(linked, plan["fingerprint"], self.environment)
        linked_environment = {**self.environment, "CARGO_TARGET_DIR": str(self.root / "target")}
        result = workspace_context.run_doctor(linked, linked_environment)

        self.assertIn(
            "cargo_target_dir_shared_common_root", result["diagnostic_codes"]
        )
        direct_context = workspace_context.resolve_context(linked, self.environment)
        marker = json.loads(Path(direct_context["marker_path"]).read_text())
        relative = doctor.diagnose(
            direct_context,
            marker,
            {**self.environment, "CARGO_TARGET_DIR": "relative", "BAZEL_OUTPUT_BASE": "also-relative"},
        )
        shared_bazel = doctor.diagnose(
            direct_context,
            marker,
            {**self.environment, "BAZEL_OUTPUT_BASE": str(self.root / "bazel-output")},
        )
        self.assertIn("cargo_target_dir_relative", relative["diagnostic_codes"])
        self.assertIn("bazel_output_base_relative", relative["diagnostic_codes"])
        self.assertIn("bazel_output_base_shared_common_root", shared_bazel["diagnostic_codes"])

    def test_audit_limit_and_corrupt_line_do_not_hide_diagnosis(self) -> None:
        self.apply()
        context = workspace_context.resolve_context(self.root, self.environment)
        state_root = Path(context["resources"]["state_root"])
        log_path = state_root / "workspace-events.jsonl"
        with log_path.open("ab") as stream:
            stream.write(b"not-json\n")
        outcome = doctor.append_event(
            state_root,
            {"operation": "doctor", "workspace_id": "fixture", "status": "passed", "diagnostic_codes": []},
            max_bytes=log_path.stat().st_size,
        )
        result = workspace_context.run_doctor(self.root, self.environment)

        self.assertEqual("audit_log_limit_reached", outcome["reason_code"])
        self.assertEqual("passed", result["status"])

    def test_doctor_schema_matches_result(self) -> None:
        result = workspace_context.run_doctor(self.root, self.environment)
        schema = json.loads(
            (MODULE_PATH.parent / "schemas/workspace-doctor.schema.json").read_text()
        )

        self.assertEqual(set(schema["required"]), set(result))

    def test_doctor_cli_returns_stable_failure_and_json(self) -> None:
        completed = subprocess.run(
            ["python3", str(MODULE_PATH), "doctor", "--repo-root", str(self.root), "--json"],
            check=False,
            capture_output=True,
            text=True,
            env={**os.environ, **self.environment},
        )

        self.assertEqual(5, completed.returncode)
        self.assertEqual(["marker_missing"], json.loads(completed.stdout)["diagnostic_codes"])


if __name__ == "__main__":
    unittest.main()
