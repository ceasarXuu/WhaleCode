"""Portable Win32 test doubles for cache benchmark process ownership."""

from cache_windows_native import StartupInfoEx


class FakeJobListAttribute:
    def __init__(self, _kernel32, _job_handle) -> None:
        self.startup = StartupInfoEx()

    def __enter__(self):
        return self

    def __exit__(self, *_exc) -> None:
        pass
