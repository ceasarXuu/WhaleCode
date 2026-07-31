#!/usr/bin/env python3
"""Process-tree and container cleanup controls for paid cache runs."""

from __future__ import annotations

import os
import signal
import subprocess
import time
from pathlib import Path
from typing import Any


CLEANUP_SUCCESS_STATUSES = frozenset({"verified_absent", "removed_verified"})
CLEANUP_STABLE_EMPTY_POLLS = 3
PROCESS_GROUP_TERMINATION_SECONDS = 5


class BenchmarkTimeoutError(TimeoutError):
    def __init__(self, command: list[str], timeout_seconds: int, termination: dict[str, Any]):
        super().__init__(f"benchmark exceeded {timeout_seconds}s: {command[0]}")
        self.process_tree_termination = termination


def cleanup_verified(result: dict[str, Any]) -> bool:
    return (
        result.get("status") in CLEANUP_SUCCESS_STATUSES
        and result.get("stable_empty_polls", 0) >= CLEANUP_STABLE_EMPTY_POLLS
        and result.get("network_cleanup_status")
        in {"verified_absent", "removed_verified"}
    )


def _terminate_process_tree(process: subprocess.Popen[Any]) -> dict[str, Any]:
    if process.poll() is not None:
        return {"status": "already_exited", "exit_code": process.returncode}
    try:
        if os.name == "posix":
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
        else:
            terminated = subprocess.run(
                ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                check=False,
                capture_output=True,
                text=True,
                timeout=PROCESS_GROUP_TERMINATION_SECONDS,
            )
            if terminated.returncode != 0 and process.poll() is None:
                return {
                    "status": "failed",
                    "error": terminated.stderr.strip() or "taskkill failed",
                }
        try:
            process.wait(timeout=PROCESS_GROUP_TERMINATION_SECONDS)
            return {"status": "terminated", "exit_code": process.returncode}
        except subprocess.TimeoutExpired:
            if os.name == "posix":
                os.killpg(os.getpgid(process.pid), signal.SIGKILL)
            else:
                return {
                    "status": "failed",
                    "error": "taskkill completed but the process tree remained alive",
                }
            process.wait(timeout=PROCESS_GROUP_TERMINATION_SECONDS)
            return {"status": "killed", "exit_code": process.returncode}
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"status": "failed", "error": f"{type(error).__name__}: {error}"}


def run_benchmark_command(
    command: list[str], cwd: Path, timeout_seconds: int
) -> subprocess.CompletedProcess[Any]:
    process = subprocess.Popen(
        command,
        cwd=cwd,
        start_new_session=os.name == "posix",
        creationflags=(
            getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
            if os.name == "nt"
            else 0
        ),
    )
    try:
        return_code = process.wait(timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        raise BenchmarkTimeoutError(
            command, timeout_seconds, _terminate_process_tree(process)
        ) from error
    except KeyboardInterrupt as error:
        error.process_tree_termination = _terminate_process_tree(process)
        raise
    return subprocess.CompletedProcess(command, return_code)


def cleanup_labeled_containers(run_id: str, grace_seconds: int) -> dict[str, Any]:
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
                    if network_cleanup["status"] == "failed":
                        return {
                            "status": "failed",
                            "container_ids": sorted(removed_ids),
                            "stable_empty_polls": stable_empty_polls,
                            "network_cleanup_status": "failed",
                            "network_ids": network_cleanup["network_ids"],
                            "error": network_cleanup["error"],
                        }
                    return {
                        "status": (
                            "removed_verified" if removed_ids else "verified_absent"
                        ),
                        "container_ids": sorted(removed_ids),
                        "stable_empty_polls": stable_empty_polls,
                        "network_cleanup_status": network_cleanup["status"],
                        "network_ids": network_cleanup["network_ids"],
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
        "error": last_error or "container cleanup grace expired",
    }


def _cleanup_labeled_networks(run_id: str, timeout_seconds: int) -> dict[str, Any]:
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
            timeout=min(10, timeout_seconds),
        )
        network_ids = [item for item in listed.stdout.splitlines() if item.strip()]
        if listed.returncode != 0:
            return {
                "status": "failed",
                "network_ids": network_ids,
                "error": listed.stderr.strip() or "docker network ls failed",
            }
        if not network_ids:
            return {"status": "verified_absent", "network_ids": [], "error": ""}
        removed = subprocess.run(
            ["docker", "network", "rm", *network_ids],
            check=False,
            capture_output=True,
            text=True,
            timeout=min(30, timeout_seconds),
        )
        return {
            "status": "removed_verified" if removed.returncode == 0 else "failed",
            "network_ids": network_ids,
            "error": removed.stderr.strip() if removed.returncode != 0 else "",
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        return {
            "status": "failed",
            "network_ids": [],
            "error": f"{type(error).__name__}: {error}",
        }
