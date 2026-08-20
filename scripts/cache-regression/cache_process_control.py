#!/usr/bin/env python3
"""Process-tree and container cleanup controls for paid cache runs."""

from __future__ import annotations

import os
import signal
import subprocess
import time
from pathlib import Path
from typing import Any

from cache_cleanup_contract import CLEANUP_STABLE_EMPTY_POLLS
from cache_windows_job import WindowsKillOnCloseJob, start_windows_job_process


PROCESS_GROUP_TERMINATION_SECONDS = 5
PROCESS_GROUP_GRACE_SECONDS = 0.5


class BenchmarkTimeoutError(TimeoutError):
    def __init__(
        self, command: list[str], timeout_seconds: int, termination: dict[str, Any]
    ):
        super().__init__(f"benchmark exceeded {timeout_seconds}s: {command[0]}")
        self.process_tree_termination = termination


def _terminate_process_tree(
    process: Any, windows_job: WindowsKillOnCloseJob | None = None
) -> dict[str, Any]:
    if windows_job is not None:
        if not windows_job.owns_process_tree:
            return {
                "status": "failed",
                "method": "windows_job_object",
                "descendants_guaranteed_terminated": False,
                "error": "job object does not own the benchmark process tree",
            }
        close_error = None
        terminate_succeeded = False
        try:
            windows_job.close()
        except OSError as error:
            close_error = error
            try:
                windows_job.terminate()
                terminate_succeeded = True
            except OSError as fallback_error:
                return {
                    "status": "failed",
                    "method": "windows_job_object_explicit_terminate",
                    "descendants_guaranteed_terminated": False,
                    "error": (
                        f"job close failed: {close_error}; "
                        f"explicit termination failed: {fallback_error}"
                    ),
                }
            release_error = None
            for _ in range(3):
                try:
                    windows_job.close()
                    release_error = None
                    break
                except OSError as error:
                    release_error = error
            if release_error is not None:
                return {
                    "status": "failed",
                    "method": "windows_job_object_explicit_terminate",
                    "descendants_guaranteed_terminated": terminate_succeeded,
                    "error": f"job handle release failed: {release_error}",
                }
        wait_error = None
        process_release_error = None
        try:
            process.wait(timeout=PROCESS_GROUP_TERMINATION_SECONDS)
        except (OSError, subprocess.TimeoutExpired) as error:
            wait_error = error
            for _ in range(3):
                try:
                    process.close()
                    process_release_error = None
                    break
                except AttributeError:
                    break
                except OSError as close_failure:
                    process_release_error = close_failure
            if process_release_error is not None:
                return {
                    "status": "failed",
                    "method": "windows_job_object_handle_release",
                    "descendants_guaranteed_terminated": True,
                    "error": f"process handle release failed: {process_release_error}",
                }
        if close_error is None or terminate_succeeded:
            return {
                "status": "terminated",
                "exit_code": process.returncode,
                "method": (
                    "windows_job_object_explicit_terminate"
                    if close_error
                    else "windows_job_object"
                ),
                "descendants_guaranteed_terminated": True,
                **(
                    {"process_wait_error": f"{type(wait_error).__name__}: {wait_error}"}
                    if wait_error
                    else {}
                ),
            }
    process_group_id = None
    if os.name == "posix":
        process_group_id = getattr(process, "_whale_process_group_id", None)
        if type(process_group_id) is not int:
            try:
                process_group_id = os.getpgid(process.pid)
            except ProcessLookupError:
                if process.poll() is not None:
                    return {
                        "status": "already_exited",
                        "exit_code": process.returncode,
                        "descendants_guaranteed_terminated": False,
                    }
                raise
    if process.poll() is not None and process_group_id is None:
        return {"status": "already_exited", "exit_code": process.returncode}
    try:
        if os.name == "posix":
            os.killpg(process_group_id, signal.SIGTERM)
        else:
            terminated = subprocess.run(
                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                check=False,
                capture_output=True,
                text=True,
                timeout=PROCESS_GROUP_TERMINATION_SECONDS,
            )
            if terminated.returncode != 0:
                return {
                    "status": "failed",
                    "method": "taskkill_fallback",
                    "descendants_guaranteed_terminated": False,
                    "error": terminated.stderr.strip() or "taskkill failed",
                }
        try:
            process.wait(timeout=PROCESS_GROUP_TERMINATION_SECONDS)
        except subprocess.TimeoutExpired:
            pass
        if os.name != "posix":
            if process.poll() is None:
                return {
                    "status": "failed",
                    "error": "taskkill completed but the process tree remained alive",
                }
            return {
                "status": "terminated",
                "exit_code": process.returncode,
                "method": "taskkill_fallback",
                "descendants_guaranteed_terminated": True,
            }
        if _wait_for_process_group_exit(process_group_id, PROCESS_GROUP_GRACE_SECONDS):
            return {
                "status": "terminated",
                "exit_code": process.returncode,
                "method": "posix_process_group",
                "descendants_guaranteed_terminated": True,
            }
        os.killpg(process_group_id, signal.SIGKILL)
        if process.poll() is None:
            process.wait(timeout=PROCESS_GROUP_TERMINATION_SECONDS)
        if not _wait_for_process_group_exit(
            process_group_id, PROCESS_GROUP_TERMINATION_SECONDS
        ):
            return {
                "status": "failed",
                "method": "posix_process_group",
                "descendants_guaranteed_terminated": False,
                "error": "process group remained after SIGKILL",
            }
        return {
            "status": "killed",
            "exit_code": process.returncode,
            "method": "posix_process_group",
            "descendants_guaranteed_terminated": True,
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        return {
            "status": "failed",
            "descendants_guaranteed_terminated": False,
            "error": f"{type(error).__name__}: {error}",
        }


def _process_group_exists(process_group_id: int) -> bool:
    try:
        os.killpg(process_group_id, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _wait_for_process_group_exit(process_group_id: int, timeout_seconds: float) -> bool:
    deadline = time.monotonic() + timeout_seconds
    while _process_group_exists(process_group_id):
        if time.monotonic() >= deadline:
            return False
        time.sleep(0.05)
    return True


def run_benchmark_command(
    command: list[str], cwd: Path, timeout_seconds: int
) -> subprocess.CompletedProcess[Any]:
    windows_job = None
    if os.name == "nt":
        process, windows_job = start_windows_job_process(command, cwd)
    else:
        process = subprocess.Popen(command, cwd=cwd, start_new_session=True)
        process._whale_process_group_id = os.getpgid(process.pid)
    try:
        return_code = process.wait(timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        raise BenchmarkTimeoutError(
            command, timeout_seconds, _terminate_process_tree(process, windows_job)
        ) from error
    except BaseException as error:
        error.process_tree_termination = _terminate_process_tree(process, windows_job)
        raise
    if windows_job is not None:
        termination = _terminate_process_tree(process, windows_job)
        if termination["status"] != "terminated":
            raise OSError(
                "completed benchmark process tree could not be released: "
                + termination.get("error", "unknown Windows Job Object failure")
            )
    return subprocess.CompletedProcess(command, return_code)


def run_captured_command(
    command: list[str], cwd: Path, timeout_seconds: int
) -> subprocess.CompletedProcess[str]:
    process = subprocess.Popen(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=os.name == "posix",
        creationflags=(
            subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0
        ),
    )
    if os.name == "posix":
        process._whale_process_group_id = os.getpgid(process.pid)
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        termination = _terminate_process_tree(process)
        for stream in (process.stdout, process.stderr):
            if stream is not None:
                stream.close()
        raise BenchmarkTimeoutError(command, timeout_seconds, termination) from error
    except BaseException as error:
        error.process_tree_termination = _terminate_process_tree(process)
        raise
    return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)


def cleanup_labeled_containers(
    run_id: str, grace_seconds: int, run_root: Path
) -> dict[str, Any]:
    deadline = time.monotonic() + grace_seconds
    removed_ids: set[str] = set()
    stable_empty_polls = 0
    last_error = ""
    while time.monotonic() < deadline:
        remaining = max(1, int(deadline - time.monotonic()))
        try:
            listed = subprocess.run(
                [
                    "docker",
                    "ps",
                    "-aq",
                    "--filter",
                    f"label=whalecode.run_id={run_id}",
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=min(10, remaining),
            )
            container_ids = [
                item for item in listed.stdout.splitlines() if item.strip()
            ]
            if listed.returncode != 0:
                stable_empty_polls = 0
                last_error = listed.stderr.strip() or "docker ps failed"
            elif not container_ids:
                stable_empty_polls += 1
                if stable_empty_polls >= CLEANUP_STABLE_EMPTY_POLLS:
                    network_cleanup = _cleanup_labeled_networks(run_id, remaining)
                    secret_cleanup = _cleanup_run_secrets(run_root, run_id)
                    if (
                        network_cleanup["status"] == "failed"
                        or secret_cleanup["status"] == "failed"
                    ):
                        return {
                            "status": "failed",
                            "container_ids": sorted(removed_ids),
                            "stable_empty_polls": stable_empty_polls,
                            "network_cleanup_status": "failed",
                            "network_ids": network_cleanup["network_ids"],
                            "secret_cleanup_status": secret_cleanup["status"],
                            "secret_paths": secret_cleanup["secret_paths"],
                            "error": network_cleanup["error"]
                            or secret_cleanup["error"],
                        }
                    return {
                        "status": (
                            "removed_verified" if removed_ids else "verified_absent"
                        ),
                        "container_ids": sorted(removed_ids),
                        "stable_empty_polls": stable_empty_polls,
                        "network_cleanup_status": network_cleanup["status"],
                        "network_ids": network_cleanup["network_ids"],
                        "secret_cleanup_status": secret_cleanup["status"],
                        "secret_paths": secret_cleanup["secret_paths"],
                        "error": "",
                    }
            else:
                stable_empty_polls = 0
                removed = subprocess.run(
                    ["docker", "rm", "--force", *container_ids],
                    check=False,
                    capture_output=True,
                    text=True,
                    timeout=min(30, remaining),
                )
                removed_ids.update(container_ids)
                last_error = removed.stderr.strip() if removed.returncode != 0 else ""
        except (OSError, subprocess.TimeoutExpired) as error:
            stable_empty_polls = 0
            last_error = f"{type(error).__name__}: {error}"
        time.sleep(min(1.0, max(0.0, deadline - time.monotonic())))
    return {
        "status": "failed",
        "container_ids": sorted(removed_ids),
        "stable_empty_polls": stable_empty_polls,
        "network_cleanup_status": "not_attempted",
        "network_ids": [],
        "secret_cleanup_status": "not_attempted",
        "secret_paths": [],
        "error": last_error or "container cleanup grace expired",
    }


def _cleanup_labeled_networks(run_id: str, timeout_seconds: int) -> dict[str, Any]:
    deadline = time.monotonic() + max(1, timeout_seconds)
    removed_ids: set[str] = set()
    stable_empty_polls = 0
    last_error = ""
    while time.monotonic() < deadline:
        remaining = max(1, int(deadline - time.monotonic()))
        try:
            listed = subprocess.run(
                [
                    "docker",
                    "network",
                    "ls",
                    "-q",
                    "--filter",
                    f"label=whalecode.run_id={run_id}",
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=min(10, remaining),
            )
            network_ids = [item for item in listed.stdout.splitlines() if item.strip()]
            if listed.returncode != 0:
                stable_empty_polls = 0
                last_error = listed.stderr.strip() or "docker network ls failed"
            elif not network_ids:
                stable_empty_polls += 1
                if stable_empty_polls >= CLEANUP_STABLE_EMPTY_POLLS:
                    return {
                        "status": (
                            "removed_verified" if removed_ids else "verified_absent"
                        ),
                        "network_ids": sorted(removed_ids),
                        "error": "",
                    }
            else:
                stable_empty_polls = 0
                removed = subprocess.run(
                    ["docker", "network", "rm", *network_ids],
                    check=False,
                    capture_output=True,
                    text=True,
                    timeout=min(30, remaining),
                )
                removed_ids.update(network_ids)
                if removed.returncode != 0:
                    return {
                        "status": "failed",
                        "network_ids": sorted(removed_ids),
                        "error": removed.stderr.strip() or "docker network rm failed",
                    }
        except (OSError, subprocess.TimeoutExpired) as error:
            stable_empty_polls = 0
            last_error = f"{type(error).__name__}: {error}"
        time.sleep(min(1.0, max(0.0, deadline - time.monotonic())))
    return {
        "status": "failed",
        "network_ids": sorted(removed_ids),
        "error": last_error or "network cleanup grace expired",
    }


def _cleanup_run_secrets(run_root: Path, run_id: str) -> dict[str, Any]:
    root = run_root.resolve()
    candidates = sorted(root.glob(f"*/{run_id}/**/.container-secrets"))
    removed: list[str] = []
    try:
        for directory in candidates:
            resolved = directory.resolve()
            if directory.is_symlink() or not resolved.is_relative_to(root):
                raise ValueError(f"secret directory escapes run root: {directory}")
            for path in directory.iterdir():
                if (
                    path.is_symlink()
                    or not path.is_file()
                    or not path.name.startswith("deepseek-")
                    or path.suffix != ".secret"
                ):
                    raise ValueError(f"unexpected secret path: {path}")
                size = path.stat().st_size
                with path.open("r+b", buffering=0) as stream:
                    stream.write(b"\0" * size)
                    stream.flush()
                    os.fsync(stream.fileno())
                path.unlink()
                removed.append(str(path))
            directory.rmdir()
        remaining = list(root.glob(f"*/{run_id}/**/.container-secrets/*"))
        if remaining:
            raise ValueError("provider secret files remain after cleanup")
        return {
            "status": "removed_verified" if removed else "verified_absent",
            "secret_paths": removed,
            "error": "",
        }
    except (OSError, ValueError) as error:
        return {
            "status": "failed",
            "secret_paths": removed,
            "error": f"{type(error).__name__}: {error}",
        }
