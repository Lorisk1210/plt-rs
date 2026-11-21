"""
The measures of `gui_events.idling` on every system it runs on: a
process' wake-ups (Linux's voluntary context switches; on macOS the
kernel's wake-up counters, as ``top``'s ``IDLEW`` reads 0 even for a
process waking 20 times a second) and a thread's wait for a CPU (0 where
the system does not tell).
"""

import subprocess
import sys
import time

import pytest

from gui_events import idling, loop_probe


def _rate(source, seconds=1.5):
    """Wake-ups a second of a Python program running *source*, as
    `gui_events.idling.usage` counts them."""
    child = subprocess.Popen([sys.executable, "-c", source])
    try:
        time.sleep(0.5)
        before = idling.usage(child.pid)[0]
        time.sleep(seconds)
        return (idling.usage(child.pid)[0] - before) / seconds
    finally:
        child.kill()
        child.wait()


@pytest.mark.skipif(sys.platform == "win32",
                    reason="the idle measure counts wake-ups on Linux (/proc) and macOS; "
                           "the GUI harness does not run on Windows")
def test_wake_ups_are_counted_on_every_system():
    sleeping = _rate("import time\nwhile True: time.sleep(0.05)")
    idle = _rate("import time\ntime.sleep(60)")
    # 20 a second as asked; a loaded macOS runner coalesces timers down to
    # 10 (GitHub's macos-15, 2026-11-21), still far from an idle process.
    assert sleeping >= 5, sleeping
    assert idle < 2, idle


def test_a_threads_wait_for_a_cpu_reads_on_every_system():
    waited = loop_probe.cpu_wait()
    assert isinstance(waited, float) and waited >= 0
