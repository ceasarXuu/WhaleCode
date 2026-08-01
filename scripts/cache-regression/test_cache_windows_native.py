#!/usr/bin/env python3

from __future__ import annotations

import ctypes
import unittest
from ctypes import wintypes

from cache_windows_native import (
    PROC_THREAD_ATTRIBUTE_JOB_LIST,
    JobListAttribute,
    StartupInfoEx,
)


class CacheWindowsNativeTest(unittest.TestCase):
    def test_job_list_attribute_is_initialized_and_released(self) -> None:
        kernel32 = unittest.mock.Mock()

        def initialize(pointer, _count, _flags, size_pointer):
            size = ctypes.cast(size_pointer, ctypes.POINTER(ctypes.c_size_t)).contents
            if pointer is None:
                size.value = 128
                return False
            return True

        kernel32.InitializeProcThreadAttributeList.side_effect = initialize
        kernel32.UpdateProcThreadAttribute.return_value = True

        with JobListAttribute(kernel32, 77) as attribute:
            self.assertIsInstance(attribute.startup, StartupInfoEx)
            self.assertEqual(
                attribute.startup.StartupInfo.cb, ctypes.sizeof(StartupInfoEx)
            )
            update = kernel32.UpdateProcThreadAttribute.call_args.args
            self.assertEqual(update[2], PROC_THREAD_ATTRIBUTE_JOB_LIST)
            handles = ctypes.cast(update[3], ctypes.POINTER(wintypes.HANDLE))
            self.assertEqual(handles.contents.value, 77)

        self.assertEqual(kernel32.InitializeProcThreadAttributeList.call_count, 2)
        kernel32.DeleteProcThreadAttributeList.assert_called_once()

    def test_job_list_attribute_releases_list_when_update_fails(self) -> None:
        kernel32 = unittest.mock.Mock()

        def initialize(pointer, _count, _flags, size_pointer):
            size = ctypes.cast(size_pointer, ctypes.POINTER(ctypes.c_size_t)).contents
            if pointer is None:
                size.value = 64
                return False
            return True

        kernel32.InitializeProcThreadAttributeList.side_effect = initialize
        kernel32.UpdateProcThreadAttribute.return_value = False

        with (
            unittest.mock.patch(
                "cache_windows_native.ctypes.get_last_error",
                return_value=87,
                create=True,
            ),
            self.assertRaisesRegex(OSError, "job list attribute update failed"),
        ):
            JobListAttribute(kernel32, 77)

        kernel32.DeleteProcThreadAttributeList.assert_called_once()


if __name__ == "__main__":
    unittest.main()
