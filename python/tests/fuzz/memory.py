"""
The memory of the fuzzer's processes, as each system tells it: the
worker's own peak resident memory, and the runner's view of a worker's
resident memory (``ps`` on Unix). Windows has neither ``resource`` nor
``ps``, and lets a process commit far more than its working set (the page
file grows): there both are the process's private commit.
"""

import sys

if sys.platform == "win32":
    import ctypes
    from ctypes import wintypes

    class _Counters(ctypes.Structure):
        """``PROCESS_MEMORY_COUNTERS``."""
        _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
            (name, ctypes.c_size_t) for name in (
                "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
                "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage")]

    #: ``PROCESS_QUERY_LIMITED_INFORMATION``: enough to read the counters.
    _QUERY = 0x1000

    _kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    _kernel32.OpenProcess.restype = wintypes.HANDLE
    _kernel32.GetCurrentProcess.restype = wintypes.HANDLE
    _kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    _kernel32.K32GetProcessMemoryInfo.argtypes = [
        wintypes.HANDLE, ctypes.POINTER(_Counters), wintypes.DWORD]

    def _counters(handle):
        """The memory counters of the process *handle*, None when they
        cannot be read."""
        counters = _Counters()
        counters.cb = ctypes.sizeof(counters)
        if not _kernel32.K32GetProcessMemoryInfo(handle, ctypes.byref(counters),
                                                 counters.cb):
            return None
        return counters

    def peak_resident():
        """This process's largest private commit so far, in bytes."""
        counters = _counters(_kernel32.GetCurrentProcess())
        return counters.PeakPagefileUsage if counters else 0

    def resident(pid):
        """The private commit of the process *pid* in bytes, zero once it
        is gone."""
        handle = _kernel32.OpenProcess(_QUERY, False, pid)
        if not handle:
            return 0
        try:
            counters = _counters(handle)
            return counters.PagefileUsage if counters else 0
        finally:
            _kernel32.CloseHandle(handle)

else:
    import resource
    import subprocess

    def peak_resident():
        """This process's largest resident memory so far, in bytes (Linux
        counts it in KiB, macOS in bytes)."""
        peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        return peak if sys.platform == "darwin" else peak * 1024

    def resident(pid):
        """The resident memory of the process *pid* in bytes, as ``ps``
        reports it (zero once the process is gone)."""
        report = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)],
                                capture_output=True, text=True, check=False).stdout.strip()
        return int(report) * 1024 if report.isdigit() else 0
