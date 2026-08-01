#!/usr/bin/env python3

from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import patch

from cache_windows_job import (
    JobObjectError,
    WindowsJobProcess,
    start_windows_job_process,
)


class CacheWindowsOwnershipTest(unittest.TestCase):
    def test_job_is_closed_when_process_setup_is_interrupted(self) -> None:
        job = unittest.mock.Mock()
        job._kernel32 = unittest.mock.Mock()
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch(
                "cache_windows_job._configure_process_signatures",
                side_effect=KeyboardInterrupt,
            ),
            self.assertRaises(KeyboardInterrupt),
        ):
            start_windows_job_process(["pwsh", "runner.ps1"], Path.cwd())
        job.close.assert_called_once()

    def test_process_handle_close_failure_is_retryable(self) -> None:
        kernel32 = unittest.mock.Mock()
        kernel32.WaitForSingleObject.return_value = 0
        kernel32.GetExitCodeProcess.return_value = True
        kernel32.CloseHandle.side_effect = [False, True]
        process = WindowsJobProcess(["pwsh"], 456, 100, kernel32)
        with (
            patch(
                "cache_windows_job.ctypes.get_last_error", return_value=5, create=True
            ),
            self.assertRaisesRegex(JobObjectError, "CloseHandle"),
        ):
            process.wait(timeout=1)

        self.assertEqual(process.wait(timeout=1), 0)
        self.assertIsNone(process._handle)
        self.assertEqual(kernel32.CloseHandle.call_count, 2)


if __name__ == "__main__":
    unittest.main()
