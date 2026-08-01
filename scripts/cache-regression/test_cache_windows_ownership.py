#!/usr/bin/env python3

from __future__ import annotations

import ctypes
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_windows_job import (
    _RETAINED_PROCESS_HANDLES,
    _ProcessInformation,
    JobObjectError,
    WindowsJobProcess,
    retry_retained_process_cleanup,
    start_windows_job_process,
)


class CacheWindowsOwnershipTest(unittest.TestCase):
    def setUp(self) -> None:
        _RETAINED_PROCESS_HANDLES.clear()

    def tearDown(self) -> None:
        _RETAINED_PROCESS_HANDLES.clear()

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

    def test_nested_cleanup_interrupt_still_releases_job_and_handles(self) -> None:
        kernel32 = unittest.mock.Mock()

        def create_process(*args):
            process_info = ctypes.cast(
                args[-1], ctypes.POINTER(_ProcessInformation)
            ).contents
            process_info.hProcess = 100
            process_info.hThread = 101
            process_info.dwProcessId = 456
            return True

        kernel32.CreateProcessW.side_effect = create_process
        kernel32.TerminateProcess.side_effect = [KeyboardInterrupt(), True]
        kernel32.WaitForSingleObject.return_value = 0
        kernel32.CloseHandle.return_value = True
        job = unittest.mock.Mock(_kernel32=kernel32)
        job.assign_handle.side_effect = JobObjectError("assignment failed")
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch("cache_windows_job._configure_process_signatures"),
            self.assertRaisesRegex(JobObjectError, "TerminateProcess attempt"),
        ):
            start_windows_job_process(["pwsh", "runner.ps1"], Path.cwd())

        job.close.assert_called_once()
        self.assertCountEqual(
            [call.args[0] for call in kernel32.CloseHandle.call_args_list],
            [100, 101],
        )

    def test_unconfirmed_process_remains_owned_until_retry_succeeds(self) -> None:
        kernel32 = unittest.mock.Mock()
        kernel32.TerminateProcess.return_value = False
        kernel32.WaitForSingleObject.return_value = 0x00000102
        kernel32.CloseHandle.return_value = True
        _RETAINED_PROCESS_HANDLES[456] = (kernel32, 100)

        with patch(
            "cache_windows_job.subprocess.run",
            return_value=subprocess.CompletedProcess([], 1, "", "taskkill failed"),
        ):
            self.assertFalse(retry_retained_process_cleanup())
        self.assertIn(456, _RETAINED_PROCESS_HANDLES)
        kernel32.CloseHandle.assert_not_called()

        kernel32.TerminateProcess.return_value = True
        kernel32.WaitForSingleObject.return_value = 0
        self.assertTrue(retry_retained_process_cleanup())
        self.assertNotIn(456, _RETAINED_PROCESS_HANDLES)
        kernel32.CloseHandle.assert_called_once_with(100)


if __name__ == "__main__":
    unittest.main()
