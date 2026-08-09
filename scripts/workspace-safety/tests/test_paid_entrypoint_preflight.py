from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[3]
BENCHMARK_DIR = ROOT / "scripts/taskspace-benchmark"
sys.path.insert(0, str(BENCHMARK_DIR))


def load_script(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, BENCHMARK_DIR / filename)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PaidEntrypointPreflightTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.active = load_script("active_prefix_runner_test", "run-active-prefix-matrix.py")
        cls.provider = load_script(
            "provider_wire_cli_test", "r7_a2_b0_provider_wire_cli.py"
        )

    def test_active_prefix_workspace_failure_precedes_run_root_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            run_root = Path(raw_temp) / "run"
            error = self.active.WorkspacePreflightError("workspace_not_ready:test")
            argv = ["runner", "--run-root", str(run_root), "--plan-only"]
            with mock.patch.object(self.active, "require_ready", side_effect=error), mock.patch.object(
                sys, "argv", argv
            ), self.assertRaisesRegex(SystemExit, "workspace_not_ready:test"):
                self.active.main()
            self.assertFalse(run_root.exists())

    def test_active_prefix_ready_workspace_continues_to_input_validation(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            run_root = Path(raw_temp) / "run"
            argv = [
                "runner",
                "--run-root",
                str(run_root),
                "--candidate-app-server",
                "does-not-exist",
                "--previous-app-server",
                "does-not-exist",
                "--env-file",
                "does-not-exist",
            ]
            with mock.patch.object(self.active, "require_ready"), mock.patch.object(
                sys, "argv", argv
            ), self.assertRaisesRegex(SystemExit, "required input missing"):
                self.active.main()
            self.assertFalse(run_root.exists())

    def test_provider_workspace_failure_precedes_artifact_write(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            output = Path(raw_temp) / "result.json"
            raw_dir = Path(raw_temp) / "raw"
            error = self.provider.WorkspacePreflightError("workspace_not_ready:test")
            argv = [
                "probe",
                "--output",
                str(output),
                "--raw-dir",
                str(raw_dir),
            ]
            with mock.patch.object(self.provider, "require_ready", side_effect=error), mock.patch.object(
                sys, "argv", argv
            ), self.assertRaisesRegex(SystemExit, "workspace_not_ready:test"):
                self.provider.main()
            self.assertFalse(output.exists())
            self.assertFalse(raw_dir.exists())

    def test_provider_ready_workspace_delegates_to_probe(self) -> None:
        with tempfile.TemporaryDirectory() as raw_temp:
            output = Path(raw_temp) / "result.json"
            raw_dir = Path(raw_temp) / "raw"
            argv = [
                "probe",
                "--output",
                str(output),
                "--raw-dir",
                str(raw_dir),
            ]
            with mock.patch.object(self.provider, "require_ready"), mock.patch.object(
                self.provider.probe,
                "run_probe",
                return_value={"decision": {"overall": "proceed", "b1_allowed": True}},
            ) as run_probe, mock.patch.object(sys, "argv", argv):
                self.assertEqual(self.provider.main(), 0)
            run_probe.assert_called_once()
            self.assertFalse(output.exists())
            self.assertFalse(raw_dir.exists())


if __name__ == "__main__":
    unittest.main()
