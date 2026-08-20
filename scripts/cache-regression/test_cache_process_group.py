from __future__ import annotations

import os
import signal
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_process_control import (
    BenchmarkTimeoutError,
    _terminate_process_tree,
    run_captured_command,
)


class CacheProcessGroupTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_captured_timeout_terminates_descendants(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 123
        process.poll.return_value = None
        process.communicate.side_effect = subprocess.TimeoutExpired(["pwsh"], 10)
        process.wait.return_value = 143
        process.returncode = 143
        with (
            patch("cache_process_control.subprocess.Popen", return_value=process),
            patch("cache_process_control.os.getpgid", return_value=123),
            patch("cache_process_control.os.killpg") as killpg,
            patch("cache_process_control._wait_for_process_group_exit", return_value=True),
            self.assertRaises(BenchmarkTimeoutError) as raised,
        ):
            run_captured_command(["pwsh", "preflight.ps1"], self.repo, 10)
        killpg.assert_called_once_with(123, signal.SIGTERM)
        self.assertEqual(
            raised.exception.process_tree_termination["status"], "terminated"
        )

    def test_posix_surviving_descendant_is_killed_and_rechecked(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 123
        process.poll.return_value = None
        process.wait.return_value = 143
        process.returncode = 143
        with (
            patch("cache_process_control.os.getpgid", return_value=123),
            patch("cache_process_control.os.killpg") as killpg,
            patch(
                "cache_process_control._wait_for_process_group_exit",
                side_effect=(False, True),
            ),
        ):
            result = _terminate_process_tree(process)
        self.assertEqual(
            killpg.call_args_list,
            [unittest.mock.call(123, signal.SIGTERM), unittest.mock.call(123, signal.SIGKILL)],
        )
        self.assertEqual(result["status"], "killed")
        self.assertTrue(result["descendants_guaranteed_terminated"])

    @unittest.skipUnless(os.name == "posix", "POSIX process-group contract")
    def test_real_term_ignoring_descendant_cannot_survive_timeout(self) -> None:
        with self.assertRaises(BenchmarkTimeoutError) as raised:
            run_captured_command(
                ["bash", "-c", "(trap '' TERM; sleep 30) & wait"],
                self.repo,
                1,
            )
        self.assertEqual(raised.exception.process_tree_termination["status"], "killed")
        self.assertTrue(
            raised.exception.process_tree_termination[
                "descendants_guaranteed_terminated"
            ]
        )


if __name__ == "__main__":
    unittest.main()
