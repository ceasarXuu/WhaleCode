#!/usr/bin/env python3
"""Small Win32 process identity helpers used by benchmark ownership."""

import ctypes
from ctypes import wintypes


PROC_THREAD_ATTRIBUTE_JOB_LIST = 0x0002000D


class StartupInfo(ctypes.Structure):
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


class StartupInfoEx(ctypes.Structure):
    _fields_ = [
        ("StartupInfo", StartupInfo),
        ("lpAttributeList", wintypes.LPVOID),
    ]


class ProcessInformation(ctypes.Structure):
    _fields_ = [
        ("hProcess", wintypes.HANDLE),
        ("hThread", wintypes.HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


class JobListAttribute:
    def __init__(self, kernel32, job_handle: wintypes.HANDLE) -> None:
        size = ctypes.c_size_t()
        kernel32.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(size))
        if size.value == 0:
            raise OSError(
                ctypes.get_last_error(), "attribute list size discovery failed"
            )
        self._kernel32 = kernel32
        self._buffer = ctypes.create_string_buffer(size.value)
        self._pointer = ctypes.cast(self._buffer, wintypes.LPVOID)
        if not kernel32.InitializeProcThreadAttributeList(
            self._pointer, 1, 0, ctypes.byref(size)
        ):
            raise OSError(ctypes.get_last_error(), "attribute list init failed")
        self._initialized = True
        self._handles = (wintypes.HANDLE * 1)(job_handle)
        if not kernel32.UpdateProcThreadAttribute(
            self._pointer,
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST,
            ctypes.cast(self._handles, wintypes.LPVOID),
            ctypes.sizeof(self._handles),
            None,
            None,
        ):
            self.close()
            raise OSError(ctypes.get_last_error(), "job list attribute update failed")
        self.startup = StartupInfoEx()
        self.startup.StartupInfo.cb = ctypes.sizeof(StartupInfoEx)
        self.startup.lpAttributeList = self._pointer

    def __enter__(self):
        return self

    def __exit__(self, *_exc) -> None:
        self.close()

    def close(self) -> None:
        if getattr(self, "_initialized", False):
            self._kernel32.DeleteProcThreadAttributeList(self._pointer)
            self._initialized = False


def process_creation_time(kernel32, process_handle: wintypes.HANDLE) -> int:
    creation = wintypes.FILETIME()
    exit_time = wintypes.FILETIME()
    kernel = wintypes.FILETIME()
    user = wintypes.FILETIME()
    if not kernel32.GetProcessTimes(
        process_handle,
        ctypes.byref(creation),
        ctypes.byref(exit_time),
        ctypes.byref(kernel),
        ctypes.byref(user),
    ):
        raise OSError(ctypes.get_last_error(), "GetProcessTimes failed")
    return (int(creation.dwHighDateTime) << 32) | int(creation.dwLowDateTime)
