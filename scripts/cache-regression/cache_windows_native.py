#!/usr/bin/env python3
"""Small Win32 process identity helpers used by benchmark ownership."""

import ctypes
from ctypes import wintypes


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
