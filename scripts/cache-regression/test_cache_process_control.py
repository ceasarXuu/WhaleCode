from __future__ import annotations

import ctypes
import json
import subprocess
import unittest
from unittest.mock import patch

from cache_process_control import (
    BenchmarkTimeoutError,
    _cleanup_run_secrets,
    _terminate_process_tree,
    cleanup_labeled_containers,
    cleanup_verified,
    run_benchmark_command,
)
from cache_run_execution_test_support import CacheRunExecutionFixture
from cache_windows_job import (
    EXTENDED_STARTUPINFO_PRESENT,
    _RETAINED_PROCESS_HANDLES,
    JobObjectError,
    _ProcessInformation,
    WindowsKillOnCloseJob,
    start_windows_job_process,
)
from cache_windows_test_support import FakeJobListAttribute


def _windows_process_mocks(on_create=None):
    kernel32 = unittest.mock.Mock()

    def create_process(*args):
        if on_create:
            on_create(args)
        process_info = ctypes.cast(
            args[-1], ctypes.POINTER(_ProcessInformation)
        ).contents
        process_info.hProcess = 100
        process_info.hThread = 101
        process_info.dwProcessId = 456
        return True

    kernel32.CreateProcessW.side_effect = create_process
    job = unittest.mock.Mock()
    job._kernel32 = kernel32
    return kernel32, job


class CacheProcessControlTest(CacheRunExecutionFixture):
    def setUp(self) -> None:
        super().setUp()
        _RETAINED_PROCESS_HANDLES.clear()
        self.job_attribute = patch(
            "cache_windows_job.JobListAttribute", FakeJobListAttribute
        )
        self.job_attribute.start()

    def tearDown(self) -> None:
        self.job_attribute.stop()
        _RETAINED_PROCESS_HANDLES.clear()
        super().tearDown()

    def test_timeout_cleanup_removes_all_run_labeled_containers(self) -> None:
        completed = [
            type(
                "Completed", (), {"returncode": 0, "stdout": "one\ntwo\n", "stderr": ""}
            )(),
            type(
                "Completed", (), {"returncode": 0, "stdout": "one\ntwo\n", "stderr": ""}
            )(),
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(6)
            ],
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-001", 10, self.repo)
        self.assertEqual(result["status"], "removed_verified")
        self.assertEqual(result["container_ids"], ["one", "two"])
        self.assertEqual(
            run.call_args_list[0].args[0][-1],
            "label=whalecode.run_id=CACHE-001",
        )
        self.assertEqual(
            run.call_args_list[1].args[0], ["docker", "rm", "--force", "one", "two"]
        )
        self.assertEqual(result["stable_empty_polls"], 3)

    def test_timeout_terminates_the_benchmark_process_group(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 123
        process.poll.return_value = None
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["pwsh"], 10),
            143,
        ]
        process.returncode = 143
        with (
            patch(
                "cache_process_control.subprocess.Popen", return_value=process
            ) as popen,
            patch("cache_process_control.os.getpgid", return_value=123),
            patch("cache_process_control.os.killpg") as killpg,
        ):
            with self.assertRaises(BenchmarkTimeoutError) as raised:
                run_benchmark_command(["pwsh", "runner.ps1"], self.repo, 10)
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        killpg.assert_called_once_with(123, unittest.mock.ANY)
        self.assertEqual(
            raised.exception.process_tree_termination["status"], "terminated"
        )

    def test_windows_timeout_terminates_entire_process_tree(self) -> None:
        process = unittest.mock.Mock()
        process.pid = 456
        process.poll.return_value = None
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["pwsh"], 10),
            1,
        ]
        process.returncode = 1
        job = unittest.mock.Mock()
        job.owns_process_tree = True
        with (
            patch("cache_process_control.os.name", "nt"),
            patch(
                "cache_process_control.start_windows_job_process",
                return_value=(process, job),
            ) as start,
        ):
            with self.assertRaises(BenchmarkTimeoutError) as raised:
                run_benchmark_command(["pwsh", "runner.ps1"], self.repo, 10)
        self.assertEqual(
            raised.exception.process_tree_termination["status"], "terminated"
        )
        self.assertEqual(
            raised.exception.process_tree_termination["method"],
            "windows_job_object",
        )
        self.assertTrue(
            raised.exception.process_tree_termination[
                "descendants_guaranteed_terminated"
            ]
        )
        start.assert_called_once_with(["pwsh", "runner.ps1"], self.repo)
        job.close.assert_called()
        process.terminate.assert_not_called()
        process.kill.assert_not_called()

    def test_windows_process_is_created_in_job_before_thread_resume(self) -> None:
        events = []

        def record_creation(args):
            events.append("create_in_job")
            self.assertTrue(args[5] & EXTENDED_STARTUPINFO_PRESENT)

        kernel32, job = _windows_process_mocks(record_creation)
        kernel32.ResumeThread.side_effect = lambda _handle: events.append("resume") or 0
        kernel32.CloseHandle.return_value = True
        job.mark_creation_assigned.side_effect = lambda: events.append("owned")
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch("cache_windows_job._configure_process_signatures"),
        ):
            process, returned_job = start_windows_job_process(
                ["pwsh", "runner.ps1"], self.repo
            )
        self.assertEqual(events, ["create_in_job", "owned", "resume"])
        self.assertEqual(process.pid, 456)
        self.assertIs(returned_job, job)
        kernel32.TerminateProcess.assert_not_called()

    def test_windows_post_create_setup_failure_terminates_suspended_process(
        self,
    ) -> None:
        kernel32, job = _windows_process_mocks()
        kernel32.TerminateProcess.return_value = True
        kernel32.WaitForSingleObject.return_value = 0
        kernel32.CloseHandle.return_value = True
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch("cache_windows_job._configure_process_signatures"),
            patch(
                "cache_windows_job.write_owner_journal",
                side_effect=JobObjectError("journal failed"),
            ),
            self.assertRaisesRegex(JobObjectError, "journal failed"),
        ):
            start_windows_job_process(["pwsh", "runner.ps1"], self.repo)
        kernel32.ResumeThread.assert_not_called()
        kernel32.TerminateProcess.assert_called_once_with(100, 1)
        job.close.assert_called_once()

    def test_windows_post_create_failure_reports_unconfirmed_termination(self) -> None:
        kernel32, job = _windows_process_mocks()
        kernel32.TerminateProcess.return_value = False
        kernel32.CloseHandle.return_value = True
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch("cache_windows_job._configure_process_signatures"),
            patch(
                "cache_windows_job.remove_owner_journal",
                side_effect=JobObjectError("journal removal failed"),
            ),
            patch(
                "cache_windows_job.subprocess.run",
                return_value=subprocess.CompletedProcess([], 1, "", "taskkill failed"),
            ),
            self.assertRaisesRegex(JobObjectError, "TerminateProcess"),
        ):
            start_windows_job_process(["pwsh", "runner.ps1"], self.repo)
        closed_handles = [call.args[0] for call in kernel32.CloseHandle.call_args_list]
        self.assertNotIn(100, closed_handles)
        self.assertIn(101, closed_handles)
        self.assertGreater(kernel32.TerminateProcess.call_count, 1)
        journals = list(
            (self.repo / "benchmarks/cache-regression/windows-process-owners").glob(
                "*.json"
            )
        )
        self.assertEqual(len(journals), 1)
        self.assertEqual(json.loads(journals[0].read_text())["pid"], 456)

    def test_windows_job_close_retains_handle_when_close_fails(self) -> None:
        kernel32 = unittest.mock.Mock()
        kernel32.CloseHandle.return_value = False
        job = WindowsKillOnCloseJob.__new__(WindowsKillOnCloseJob)
        job._kernel32 = kernel32
        job._handle = 77
        job._assigned = True
        with (
            patch(
                "cache_windows_job.ctypes.get_last_error", return_value=5, create=True
            ),
            self.assertRaisesRegex(JobObjectError, "CloseHandle"),
        ):
            job.close()
        self.assertEqual(job._handle, 77)

    def test_windows_job_close_failure_terminates_tree_explicitly(self) -> None:
        process = unittest.mock.Mock()
        process.wait.return_value = 1
        process.returncode = 1
        job = unittest.mock.Mock()
        job.owns_process_tree = True
        job.close.side_effect = [JobObjectError("close failed"), None]

        result = _terminate_process_tree(process, job)

        self.assertEqual(result["status"], "terminated")
        self.assertTrue(result["descendants_guaranteed_terminated"])
        job.terminate.assert_called_once()
        self.assertEqual(job.close.call_count, 2)

    def test_windows_job_fallback_always_retries_handle_release(self) -> None:
        process = unittest.mock.Mock()
        process.wait.side_effect = OSError("wait failed")
        job = unittest.mock.Mock()
        job.owns_process_tree = True
        job.close.side_effect = [JobObjectError("close failed"), None]

        result = _terminate_process_tree(process, job)

        self.assertEqual(result["status"], "terminated")
        job.terminate.assert_called_once()
        self.assertEqual(job.close.call_count, 2)

    def test_windows_create_interrupt_cleans_suspended_process(self) -> None:
        kernel32, job = _windows_process_mocks()

        def interrupted_create(*args):
            process_info = ctypes.cast(
                args[-1], ctypes.POINTER(_ProcessInformation)
            ).contents
            process_info.hProcess = 100
            process_info.hThread = 101
            process_info.dwProcessId = 456
            raise KeyboardInterrupt

        kernel32.CreateProcessW.side_effect = interrupted_create
        kernel32.TerminateProcess.return_value = True
        kernel32.WaitForSingleObject.return_value = 0
        kernel32.CloseHandle.return_value = True
        with (
            patch("cache_windows_job.WindowsKillOnCloseJob", return_value=job),
            patch("cache_windows_job._configure_process_signatures"),
            self.assertRaises(KeyboardInterrupt),
        ):
            start_windows_job_process(["pwsh", "runner.ps1"], self.repo)

        kernel32.TerminateProcess.assert_called_once_with(100, 1)
        self.assertCountEqual(
            [call.args[0] for call in kernel32.CloseHandle.call_args_list],
            [100, 101],
        )
        job.close.assert_called_once()

    def test_windows_wait_failure_still_closes_owned_job(self) -> None:
        process = unittest.mock.Mock()
        process.wait.side_effect = [OSError("wait failed"), 1]
        process.returncode = 1
        job = unittest.mock.Mock()
        job.owns_process_tree = True
        with (
            patch("cache_process_control.os.name", "nt"),
            patch(
                "cache_process_control.start_windows_job_process",
                return_value=(process, job),
            ),
            self.assertRaisesRegex(OSError, "wait failed"),
        ):
            run_benchmark_command(["pwsh", "runner.ps1"], self.repo, 10)
        job.close.assert_called_once()

    def test_windows_taskkill_fallback_never_accepts_parent_exit_as_tree_proof(
        self,
    ) -> None:
        process = unittest.mock.Mock()
        process.pid = 789
        process.poll.side_effect = [None, 1]
        failed = subprocess.CompletedProcess(
            ["taskkill", "/PID", "789", "/T", "/F"], 128, "", "not found"
        )
        with (
            patch("cache_process_control.os.name", "nt"),
            patch("cache_process_control.subprocess.run", return_value=failed),
        ):
            result = _terminate_process_tree(process)
        self.assertEqual(result["status"], "failed")
        self.assertFalse(result["descendants_guaranteed_terminated"])

    def test_cleanup_catches_container_that_appears_after_first_empty_poll(
        self,
    ) -> None:
        completed = [
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type(
                "Completed", (), {"returncode": 0, "stdout": "late\n", "stderr": ""}
            )(),
            type(
                "Completed", (), {"returncode": 0, "stdout": "late\n", "stderr": ""}
            )(),
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(6)
            ],
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-LATE", 10, self.repo)
        self.assertEqual(result["status"], "removed_verified")
        self.assertEqual(result["container_ids"], ["late"])
        self.assertEqual(
            run.call_args_list[2].args[0], ["docker", "rm", "--force", "late"]
        )

    def test_cleanup_removes_provider_boundary_networks(self) -> None:
        completed = [
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(3)
            ],
            type(
                "Completed", (), {"returncode": 0, "stdout": "net-one\n", "stderr": ""}
            )(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            type(
                "Completed", (), {"returncode": 0, "stdout": "net-late\n", "stderr": ""}
            )(),
            type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})(),
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(3)
            ],
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed) as run,
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-NETWORK", 10, self.repo)
        self.assertEqual(result["network_cleanup_status"], "removed_verified")
        self.assertEqual(result["network_ids"], ["net-late", "net-one"])
        self.assertEqual(
            run.call_args_list[4].args[0],
            ["docker", "network", "rm", "net-one"],
        )
        self.assertEqual(
            run.call_args_list[6].args[0],
            ["docker", "network", "rm", "net-late"],
        )

    def test_cleanup_erases_host_provider_secret_before_verification(self) -> None:
        secret_dir = (
            self.repo
            / "simple/CACHE-SECRET/pair-001/left/artifacts.provider-supervisor"
            / ".container-secrets"
        )
        secret_dir.mkdir(parents=True)
        secret = secret_dir / "deepseek-fixture.secret"
        secret.write_text("paid-provider-secret", encoding="utf-8")
        completed = [
            *[
                type("Completed", (), {"returncode": 0, "stdout": "", "stderr": ""})()
                for _ in range(6)
            ]
        ]
        with (
            patch("cache_process_control.subprocess.run", side_effect=completed),
            patch("cache_process_control.time.sleep"),
        ):
            result = cleanup_labeled_containers("CACHE-SECRET", 10, self.repo)
        self.assertEqual(result["secret_cleanup_status"], "removed_verified")
        self.assertFalse(secret.exists())
        self.assertFalse(secret_dir.exists())

    def test_empty_secret_directory_is_verified_absent(self) -> None:
        secret_dir = self.repo / "simple/CACHE-EMPTY/.container-secrets"
        secret_dir.mkdir(parents=True)

        result = _cleanup_run_secrets(self.repo, "CACHE-EMPTY")

        self.assertEqual(result["status"], "verified_absent")
        self.assertEqual(result["secret_paths"], [])
        self.assertFalse(secret_dir.exists())

    def test_only_verified_cleanup_statuses_allow_completion(self) -> None:
        self.assertTrue(
            cleanup_verified(
                {
                    "status": "verified_absent",
                    "container_ids": [],
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                    "network_ids": [],
                    "secret_cleanup_status": "verified_absent",
                    "secret_paths": [],
                    "error": "",
                }
            )
        )
        self.assertTrue(
            cleanup_verified(
                {
                    "status": "removed_verified",
                    "container_ids": ["removed-container"],
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "removed_verified",
                    "network_ids": ["removed-network"],
                    "secret_cleanup_status": "removed_verified",
                    "secret_paths": ["removed-secret"],
                    "error": "",
                }
            )
        )
        self.assertFalse(
            cleanup_verified({"status": "verified_absent", "stable_empty_polls": 1})
        )
        self.assertFalse(cleanup_verified({"status": "failed"}))
        self.assertFalse(cleanup_verified({"status": "removed"}))
        self.assertFalse(
            cleanup_verified(
                {
                    "status": "verified_absent",
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                    "secret_cleanup_status": "failed",
                }
            )
        )
        for field, value in (
            ("container_ids", ["remaining-container"]),
            ("network_ids", ["remaining-network"]),
            ("secret_paths", ["remaining-secret"]),
            ("error", "cleanup reported an error"),
        ):
            with self.subTest(field=field):
                proof = {
                    "status": "verified_absent",
                    "container_ids": [],
                    "stable_empty_polls": 3,
                    "network_cleanup_status": "verified_absent",
                    "network_ids": [],
                    "secret_cleanup_status": "verified_absent",
                    "secret_paths": [],
                    "error": "",
                }
                proof[field] = value
                self.assertFalse(cleanup_verified(proof))


if __name__ == "__main__":
    unittest.main()
