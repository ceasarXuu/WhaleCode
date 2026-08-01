#!/usr/bin/env python3
"""Windows Job Object ownership for paid benchmark process trees."""

from __future__ import annotations

import ctypes
import subprocess
from ctypes import wintypes
from pathlib import Path

from cache_windows_owner_journal import (
    owner_records,
    remove_owner_journal,
    write_owner_journal,
)
from cache_windows_native import process_creation_time


JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS = 9
CREATE_SUSPENDED = 0x00000004
CREATE_NEW_PROCESS_GROUP = 0x00000200
WAIT_OBJECT_0 = 0x00000000
WAIT_TIMEOUT = 0x00000102
INFINITE = 0xFFFFFFFF
RESUME_THREAD_FAILED = 0xFFFFFFFF
TERMINATE_PROCESS_ATTEMPTS = 3
PROCESS_TERMINATE = 0x0001
SYNCHRONIZE = 0x00100000
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
_RETAINED_PROCESS_HANDLES: dict[
    int,
    tuple[
        object,
        wintypes.HANDLE | None,
        wintypes.HANDLE | None,
        Path | None,
        int | None,
    ],
] = {}


class JobObjectError(OSError):
    pass


class _IoCounters(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_uint64),
        ("WriteOperationCount", ctypes.c_uint64),
        ("OtherOperationCount", ctypes.c_uint64),
        ("ReadTransferCount", ctypes.c_uint64),
        ("WriteTransferCount", ctypes.c_uint64),
        ("OtherTransferCount", ctypes.c_uint64),
    ]


class _BasicLimitInformation(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_int64),
        ("PerJobUserTimeLimit", ctypes.c_int64),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class _ExtendedLimitInformation(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _BasicLimitInformation),
        ("IoInfo", _IoCounters),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class _StartupInfo(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("lpReserved", wintypes.LPWSTR),
        ("lpDesktop", wintypes.LPWSTR),
        ("lpTitle", wintypes.LPWSTR),
        ("dwX", wintypes.DWORD),
        ("dwY", wintypes.DWORD),
        ("dwXSize", wintypes.DWORD),
        ("dwYSize", wintypes.DWORD),
        ("dwXCountChars", wintypes.DWORD),
        ("dwYCountChars", wintypes.DWORD),
        ("dwFillAttribute", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("wShowWindow", wintypes.WORD),
        ("cbReserved2", wintypes.WORD),
        ("lpReserved2", ctypes.POINTER(ctypes.c_ubyte)),
        ("hStdInput", wintypes.HANDLE),
        ("hStdOutput", wintypes.HANDLE),
        ("hStdError", wintypes.HANDLE),
    ]


class _ProcessInformation(ctypes.Structure):
    _fields_ = [
        ("hProcess", wintypes.HANDLE),
        ("hThread", wintypes.HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


class WindowsKillOnCloseJob:
    def __init__(self) -> None:
        self._kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        self._configure_signatures()
        self._handle = self._kernel32.CreateJobObjectW(None, None)
        self._assigned = False
        if not self._handle:
            raise JobObjectError(ctypes.get_last_error(), "CreateJobObjectW failed")
        limits = _ExtendedLimitInformation()
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        configured = self._kernel32.SetInformationJobObject(
            self._handle,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
            ctypes.byref(limits),
            ctypes.sizeof(limits),
        )
        if not configured:
            error = ctypes.get_last_error()
            self.close()
            raise JobObjectError(error, "SetInformationJobObject failed")

    def _configure_signatures(self) -> None:
        self._kernel32.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
        self._kernel32.CreateJobObjectW.restype = wintypes.HANDLE
        self._kernel32.SetInformationJobObject.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            wintypes.LPVOID,
            wintypes.DWORD,
        ]
        self._kernel32.SetInformationJobObject.restype = wintypes.BOOL
        self._kernel32.AssignProcessToJobObject.argtypes = [
            wintypes.HANDLE,
            wintypes.HANDLE,
        ]
        self._kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
        self._kernel32.TerminateJobObject.argtypes = [
            wintypes.HANDLE,
            wintypes.UINT,
        ]
        self._kernel32.TerminateJobObject.restype = wintypes.BOOL
        self._kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        self._kernel32.CloseHandle.restype = wintypes.BOOL

    @property
    def owns_process_tree(self) -> bool:
        return self._assigned

    def assign_handle(self, process_handle: wintypes.HANDLE) -> None:
        if not self._kernel32.AssignProcessToJobObject(self._handle, process_handle):
            raise JobObjectError(
                ctypes.get_last_error(), "AssignProcessToJobObject failed"
            )
        self._assigned = True

    def close(self) -> None:
        if self._handle:
            if not self._kernel32.CloseHandle(self._handle):
                raise JobObjectError(ctypes.get_last_error(), "CloseHandle failed")
            self._handle = None

    def terminate(self, exit_code: int = 1) -> None:
        if not self._handle or not self._assigned:
            raise JobObjectError("job object does not own a process tree")
        if not self._kernel32.TerminateJobObject(self._handle, exit_code):
            raise JobObjectError(ctypes.get_last_error(), "TerminateJobObject failed")


class WindowsJobProcess:
    def __init__(
        self,
        args: list[str],
        process_id: int,
        process_handle: wintypes.HANDLE,
        kernel32,
    ) -> None:
        self.args = args
        self.pid = process_id
        self.returncode: int | None = None
        self._handle = process_handle
        self._kernel32 = kernel32

    def poll(self) -> int | None:
        if self.returncode is not None:
            return self.returncode
        wait_result = self._kernel32.WaitForSingleObject(self._handle, 0)
        if wait_result == WAIT_TIMEOUT:
            return None
        if wait_result != WAIT_OBJECT_0:
            raise JobObjectError(ctypes.get_last_error(), "WaitForSingleObject failed")
        return self._read_returncode()

    def wait(self, timeout: float | None = None) -> int:
        if self.returncode is not None:
            return self.returncode
        timeout_ms = INFINITE if timeout is None else max(0, int(timeout * 1000))
        wait_result = self._kernel32.WaitForSingleObject(self._handle, timeout_ms)
        if wait_result == WAIT_TIMEOUT:
            raise subprocess.TimeoutExpired(self.args, timeout)
        if wait_result != WAIT_OBJECT_0:
            raise JobObjectError(ctypes.get_last_error(), "WaitForSingleObject failed")
        return self._read_returncode()

    def _read_returncode(self) -> int:
        exit_code = wintypes.DWORD()
        if not self._kernel32.GetExitCodeProcess(self._handle, ctypes.byref(exit_code)):
            raise JobObjectError(ctypes.get_last_error(), "GetExitCodeProcess failed")
        self._close_handle()
        self.returncode = int(exit_code.value)
        return self.returncode

    def close(self) -> None:
        self._close_handle()

    def _close_handle(self) -> None:
        if self._handle:
            if not self._kernel32.CloseHandle(self._handle):
                raise JobObjectError(ctypes.get_last_error(), "CloseHandle failed")
            self._handle = None


def start_windows_job_process(
    command: list[str], cwd: Path
) -> tuple[WindowsJobProcess, WindowsKillOnCloseJob]:
    recover_durable_process_cleanup(cwd)
    if not retry_retained_process_cleanup():
        raise JobObjectError("a previously created benchmark process is still owned")
    job = WindowsKillOnCloseJob()
    process_info = None
    thread_handle = None
    owner_path = None
    creation_time = None
    try:
        kernel32 = job._kernel32
        _configure_process_signatures(kernel32)
        startup = _StartupInfo()
        startup.cb = ctypes.sizeof(startup)
        process_info = _ProcessInformation()
        command_line = ctypes.create_unicode_buffer(subprocess.list2cmdline(command))
        created = kernel32.CreateProcessW(
            None,
            command_line,
            None,
            None,
            False,
            CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP,
            None,
            str(cwd),
            ctypes.byref(startup),
            ctypes.byref(process_info),
        )
        if not created:
            raise JobObjectError(ctypes.get_last_error(), "CreateProcessW failed")
        thread_handle = process_info.hThread
        creation_time = process_creation_time(kernel32, process_info.hProcess)
        owner_path = write_owner_journal(
            cwd, int(process_info.dwProcessId), creation_time
        )
        job.assign_handle(process_info.hProcess)
        remove_owner_journal(owner_path)
        owner_path = None
        if kernel32.ResumeThread(thread_handle) == RESUME_THREAD_FAILED:
            raise JobObjectError(ctypes.get_last_error(), "ResumeThread failed")
        _close_handle(kernel32, thread_handle)
        thread_handle = None
        process = WindowsJobProcess(
            command,
            int(process_info.dwProcessId),
            process_info.hProcess,
            kernel32,
        )
    except BaseException as error:
        cleanup_errors = []
        if process_info is not None and process_info.hProcess:
            if owner_path is None:
                try:
                    creation_time = creation_time or process_creation_time(
                        kernel32, process_info.hProcess
                    )
                    owner_path = write_owner_journal(
                        cwd, int(process_info.dwProcessId), creation_time
                    )
                except BaseException as journal_error:
                    cleanup_errors.append(
                        "process owner journal failed: "
                        f"{type(journal_error).__name__}: {journal_error}"
                    )
            try:
                cleanup_errors.extend(
                    _terminate_and_close_created_process(
                        kernel32,
                        process_info.hProcess,
                        thread_handle or process_info.hThread,
                        int(process_info.dwProcessId),
                        owner_path,
                        creation_time,
                    )
                )
            except BaseException as cleanup_error:
                cleanup_errors.append(
                    f"process cleanup interrupted: {type(cleanup_error).__name__}: "
                    f"{cleanup_error}"
                )
        try:
            job.close()
        except BaseException as cleanup_error:
            cleanup_errors.append(f"job close failed: {cleanup_error}")
        if cleanup_errors:
            raise JobObjectError("; ".join(cleanup_errors)) from error
        raise
    return process, job


def _configure_process_signatures(kernel32) -> None:
    kernel32.CreateProcessW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.LPWSTR,
        wintypes.LPVOID,
        wintypes.LPVOID,
        wintypes.BOOL,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.LPCWSTR,
        ctypes.POINTER(_StartupInfo),
        ctypes.POINTER(_ProcessInformation),
    ]
    kernel32.CreateProcessW.restype = wintypes.BOOL
    kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
    kernel32.ResumeThread.restype = wintypes.DWORD
    kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel32.WaitForSingleObject.restype = wintypes.DWORD
    kernel32.GetExitCodeProcess.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(wintypes.DWORD),
    ]
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
    kernel32.TerminateProcess.restype = wintypes.BOOL
    kernel32.GetProcessTimes.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(wintypes.FILETIME),
        ctypes.POINTER(wintypes.FILETIME),
        ctypes.POINTER(wintypes.FILETIME),
        ctypes.POINTER(wintypes.FILETIME),
    ]
    kernel32.GetProcessTimes.restype = wintypes.BOOL


def _close_handle(kernel32, handle: wintypes.HANDLE) -> None:
    if handle and not kernel32.CloseHandle(handle):
        raise JobObjectError(ctypes.get_last_error(), "CloseHandle failed")


def _terminate_and_close_created_process(
    kernel32,
    process_handle: wintypes.HANDLE,
    thread_handle: wintypes.HANDLE,
    process_id: int,
    owner_path: Path | None = None,
    creation_time: int | None = None,
) -> list[str]:
    errors = []
    terminated = not process_handle
    if process_handle:
        for _ in range(TERMINATE_PROCESS_ATTEMPTS):
            try:
                terminate_requested = kernel32.TerminateProcess(process_handle, 1)
                wait_ms = 5000 if terminate_requested else 0
                terminated = (
                    kernel32.WaitForSingleObject(process_handle, wait_ms)
                    == WAIT_OBJECT_0
                )
            except BaseException as error:
                errors.append(
                    "TerminateProcess attempt interrupted: "
                    f"{type(error).__name__}: {error}"
                )
            if terminated:
                break
    if process_handle and not terminated:
        try:
            fallback = subprocess.run(
                ["taskkill", "/PID", str(process_id), "/T", "/F"],
                check=False,
                capture_output=True,
                text=True,
                timeout=5,
            )
            terminated = fallback.returncode == 0 and (
                kernel32.WaitForSingleObject(process_handle, 5000) == WAIT_OBJECT_0
            )
            fallback_error = fallback.stderr.strip()
        except BaseException as error:
            fallback_error = f"{type(error).__name__}: {error}"
        if not terminated:
            errors.append(
                "TerminateProcess failed; "
                + (fallback_error or "taskkill failed")
                + f"; PID {process_id} handle retained"
            )
    retained_process = process_handle if process_handle and not terminated else None
    retained_thread = None
    handles = [("thread", thread_handle)]
    if process_handle and terminated:
        handles.append(("process", process_handle))
    for label, handle in handles:
        try:
            _close_handle(kernel32, handle)
        except BaseException as error:
            errors.append(f"{label} handle close failed: {error}")
            if label == "process":
                retained_process = handle
            else:
                retained_thread = handle
    if retained_process or retained_thread:
        _RETAINED_PROCESS_HANDLES[process_id] = (
            kernel32,
            retained_process,
            retained_thread,
            owner_path,
            creation_time,
        )
    else:
        _RETAINED_PROCESS_HANDLES.pop(process_id, None)
        if terminated:
            remove_owner_journal(owner_path)
    return errors


def retry_retained_process_cleanup() -> bool:
    """Retry and retain ownership of any suspended process left by setup failure."""
    for process_id, (
        kernel32,
        process_handle,
        thread_handle,
        owner_path,
        creation_time,
    ) in list(_RETAINED_PROCESS_HANDLES.items()):
        _terminate_and_close_created_process(
            kernel32,
            process_handle,
            thread_handle,
            process_id,
            owner_path,
            creation_time,
        )
    return not _RETAINED_PROCESS_HANDLES


def recover_durable_process_cleanup(cwd: Path) -> None:
    records = owner_records(cwd)
    if not records:
        return
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _configure_process_signatures(kernel32)
    kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel32.OpenProcess.restype = wintypes.HANDLE
    for path, process_id, creation_time in records:
        process_handle = kernel32.OpenProcess(
            PROCESS_TERMINATE | SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            False,
            process_id,
        )
        if not process_handle:
            if ctypes.get_last_error() == 87:
                remove_owner_journal(path)
                continue
            raise JobObjectError(
                ctypes.get_last_error(),
                f"OpenProcess failed for owned PID {process_id}",
            )
        if process_creation_time(kernel32, process_handle) != creation_time:
            _close_handle(kernel32, process_handle)
            remove_owner_journal(path)
            continue
        errors = _terminate_and_close_created_process(
            kernel32,
            process_handle,
            None,
            process_id,
            path,
            creation_time,
        )
        if errors or process_id in _RETAINED_PROCESS_HANDLES:
            raise JobObjectError("; ".join(errors) or "owned process cleanup failed")
