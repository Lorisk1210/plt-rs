"""
What an event loop costs and how promptly it answers, measured on a
program (`loop_probe`) under either library, on the harness' private X
server or a weston nested in it: wake-ups and CPU time of an idle window,
the latency from a signal to `KeyboardInterrupt` in each blocking call,
a worker thread's progress while the window idles, an idle draw and a
close asked for on another thread, an animation's frames. The tests
(``test_idle_loop.py``) bound them; ``python -m gui_events.idling``
prints them.
"""

import json
import os
import pathlib
import queue
import signal
import subprocess
import sys
import threading
import time

from .driver import HERE, program_env, python_of
from .errors import ProgramFailed

#: How long a probe may take to show its window (on a loaded machine),
#: and to answer anything else: deadlines.
STARTUP, ANSWER = 60.0, 20.0
#: How long the window idles before it is measured or interrupted.
SETTLE = 0.5
#: The probe's actions that block until a signal interrupts them.
BLOCKING = ("show", "pause", "start_event_loop", "waitforbuttonpress",
            "ginput", "callback")


class Probe:
    """A running `loop_probe` and its records as they come."""

    def __init__(self, library, backend, action, display, workdir,
                 wayland=None):
        env = program_env(library, backend, display, wayland)
        workdir = pathlib.Path(workdir)
        workdir.mkdir(parents=True, exist_ok=True)
        self._err = open(workdir / f"{library}-{action}.err", "w")
        self.proc = subprocess.Popen(
            [str(python_of(library)), str(HERE / "loop_probe.py"), library,
             backend, action],
            env=env, cwd=workdir, stdout=subprocess.PIPE, stderr=self._err,
            text=True)
        self._records = queue.Queue()
        #: Every record read so far, in order.
        self.history = []
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        for line in self.proc.stdout:
            if line.startswith("{"):
                record = json.loads(line)
                self.history.append(record)
                self._records.put(record)
        self._records.put(None)

    def wait_for(self, name, timeout=ANSWER):
        """The next record *name* (those before it are dropped)."""
        deadline = time.monotonic() + timeout
        while True:
            left = deadline - time.monotonic()
            try:
                record = self._records.get(timeout=max(left, 0))
            except queue.Empty:
                raise ProgramFailed(f"no {name!r} within {timeout} s")
            if record is None:
                raise ProgramFailed(f"the probe ended ({self.proc.wait()}) "
                                    f"before {name!r}")
            if record["name"] == name:
                return record

    def interrupt(self):
        """Send SIGINT; the seconds until `KeyboardInterrupt` reached the
        loop."""
        sent = time.monotonic()
        os.kill(self.proc.pid, signal.SIGINT)
        return self.wait_for("interrupted")["t"] - sent

    def cost(self, seconds):
        """Wake-ups per second and CPU seconds per second of the program's
        threads over *seconds*."""
        before, start = usage(self.proc.pid), time.monotonic()
        time.sleep(seconds)
        after, took = usage(self.proc.pid), time.monotonic() - start
        return ((after[0] - before[0]) / took, (after[1] - before[1]) / took)

    def close(self):
        """End the probe, if it still runs."""
        if self.proc.poll() is None:
            self.proc.kill()
        self.proc.wait()
        self._err.close()


def usage(pid):
    """``(wake-ups, CPU seconds)`` of process *pid* so far: its threads'
    voluntary context switches on Linux, its idle wake-ups on macOS."""
    if sys.platform == "darwin":
        return _usage_macos(pid)
    switches = 0
    for status in pathlib.Path(f"/proc/{pid}/task").glob("*/status"):
        try:
            for line in status.read_text().splitlines():
                if line.startswith("voluntary_ctxt_switches"):
                    switches += int(line.split()[1])
        except OSError:
            continue
    fields = pathlib.Path(f"/proc/{pid}/stat").read_text().rpartition(
        ")")[2].split()
    ticks = os.sysconf("SC_CLK_TCK")
    return switches, (int(fields[11]) + int(fields[12])) / ticks


def _usage_macos(pid):
    """`usage` from ``top``: the process' idle wake-ups (``IDLEW``, its
    wake-ups from waiting) and its CPU time (``TIME``, ``[H:]MM:SS.ss``)."""
    out = subprocess.run(
        ["top", "-l", "1", "-pid", str(pid), "-stats", "pid,idlew,time"],
        capture_output=True, text=True, check=True).stdout
    fields = next(line.split() for line in reversed(out.splitlines())
                  if line.split() and line.split()[0] == str(pid))
    seconds = 0.0
    for part in fields[2].split(":"):
        seconds = 60 * seconds + float(part)
    return int(fields[1].rstrip("+")), seconds


def idle(library, backend, display, workdir, wayland=None, seconds=3.0):
    """An idle window in `plt.show`: ``{"wakeups": per second, "cpu":
    seconds per second, "interrupt": seconds}``."""
    probe = Probe(library, backend, "show", display, workdir, wayland)
    try:
        probe.wait_for("drawn", STARTUP)
        time.sleep(SETTLE)
        wakeups, cpu = probe.cost(seconds)
        return {"wakeups": wakeups, "cpu": cpu, "interrupt": probe.interrupt()}
    finally:
        probe.close()


def interrupt_latency(library, backend, action, display, workdir,
                      wayland=None):
    """The seconds from SIGINT to `KeyboardInterrupt` in blocking call
    *action* (see `BLOCKING`), the window idle, or for ``callback`` a
    moment into the callback's work."""
    probe = Probe(library, backend, action, display, workdir, wayland)
    try:
        probe.wait_for("drawn", STARTUP)
        if action == "callback":
            probe.wait_for("working")
            time.sleep(0.2)
        else:
            time.sleep(SETTLE)
        return probe.interrupt()
    finally:
        probe.close()


def worker_progress(library, backend, display, workdir, wayland=None):
    """What a worker thread gets done while the window idles, as a share
    of what it gets done while the main thread sleeps."""
    probe = Probe(library, backend, "worker", display, workdir, wayland)
    try:
        record = probe.wait_for("worker", STARTUP)
        return record["looping"] / record["sleeping"]
    finally:
        probe.close()


def queued(library, backend, display, workdir, wayland=None):
    """An idle draw and a close asked for on a worker thread:
    ``{"redraw": seconds until drawn, "close": seconds until plt.show
    returned}``."""
    probe = Probe(library, backend, "queued", display, workdir, wayland)
    try:
        probe.wait_for("drawn", STARTUP)
        asked = probe.wait_for("asked")["t"]
        redrawn = probe.wait_for("redrawn")["t"]
        closing = probe.wait_for("closing")["t"]
        closed = probe.wait_for("closed")["t"]
        return {"redraw": redrawn - asked, "close": closed - closing}
    finally:
        probe.close()


def animation(library, backend, display, workdir, wayland=None,
              seconds=3.0):
    """A 50 ms animation: its cost (as `idle`'s) and its frames' spacing,
    raw and less the time the program waited for a CPU, in seconds."""
    probe = Probe(library, backend, "animation", display, workdir, wayland)
    try:
        probe.wait_for("drawn", STARTUP)
        time.sleep(SETTLE)
        wakeups, cpu = probe.cost(seconds)
        probe.interrupt()
        frames = [r for r in probe.history if r["name"] == "frame"]
    finally:
        probe.close()
    times = [r["t"] for r in frames]
    own = [r["t"] - r["waited"] for r in frames]
    gaps = [b - a for a, b in zip(times, times[1:])]
    own_gaps = [b - a for a, b in zip(own, own[1:])]
    return {"wakeups": wakeups, "cpu": cpu, "frames": len(frames),
            "mean": sum(gaps) / len(gaps), "max": max(gaps),
            "own_mean": sum(own_gaps) / len(own_gaps),
            "own_max": max(own_gaps)}


def main():
    """Print every measurement for pltrs and matplotlib's QtAgg, on X11
    and on Wayland (private servers), or on the session's own display
    (``--platform native``: macOS, Windows, a desktop)."""
    import argparse

    from .driver import REPO
    from .xdisplay import private_display, private_wayland
    parser = argparse.ArgumentParser(prog="python -m gui_events.idling")
    parser.add_argument("--library", action="append",
                        choices=["pltrs", "matplotlib"])
    parser.add_argument("--platform", action="append",
                        choices=["x11", "wayland", "native"])
    parser.add_argument("--reference", default="qtagg",
                        help="matplotlib's backend (default: qtagg)")
    parser.add_argument("--workdir", default=str(REPO / "target/idling"))
    args = parser.parse_args()
    workdir = pathlib.Path(args.workdir).resolve()
    platforms = args.platform or ["x11", "wayland"]
    if platforms == ["native"]:
        for library in args.library or ["pltrs", "matplotlib"]:
            backend = "pltrs" if library == "pltrs" else args.reference
            report(library, "native", backend, None,
                   workdir / "native" / library, None)
        return
    with private_display() as display:
        for platform in platforms:
            for library in args.library or ["pltrs", "matplotlib"]:
                backend = "pltrs" if library == "pltrs" else args.reference
                place = workdir / platform / library
                if platform == "x11":
                    report(library, platform, backend, display, place, None)
                    continue
                with private_wayland(display, place / "weston") as weston:
                    report(library, platform, backend, display, place,
                           weston.socket)


def report(library, platform, backend, display, workdir, wayland):
    """Print the measurements of *library* on *platform*."""
    def show(name, measure, *args):
        try:
            value = measure(library, backend, *args, display, workdir,
                            wayland)
        except ProgramFailed as error:
            value = f"failed: {error}"
        print(json.dumps({"library": library, "platform": platform,
                          "measure": name, "value": value}), flush=True)
    show("idle", idle)
    for action in BLOCKING:
        show(f"interrupt {action}", interrupt_latency, action)
    show("worker", worker_progress)
    if library == "pltrs":
        # Qt's widgets may not be used from other threads.
        show("queued", queued)
    show("animation", animation)


if __name__ == "__main__":
    main()
