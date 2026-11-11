"""
Whether the processes an input passes through still have work to do: the
X server that takes the faked input, the weston that hands it on (on
Wayland) and the program that handles it.

The program's log alone cannot tell: a program that waits for a CPU
writes nothing however long it waits (one starved for 237 ms in the
middle of an event's callbacks let the driver send the next input
first, under a load of 84 on 32 cores). The kernel can: a thread with
work to do is runnable (state ``R``, or ``D`` while the kernel works for
it) until it is done and sleeps, whether it runs or waits for a CPU, so
a thread runnable at two samples in a row is busy. One runnable at a
single sample may only be waking briefly: pltrs' loop wakes every 20 ms
(``_pltrs_loop.SLICE``), and is seen runnable at about 1 in 100 samples
on an idle machine, 1 in 20 under load. See `Driver.settle`.
"""

import os

#: The states of a thread with work to do (``/proc/PID/task/TID/stat``):
#: running or waiting for a CPU, or in the kernel uninterruptibly.
BUSY_STATES = frozenset([b"R", b"D"])
#: More than a ``stat`` file holds.
_STAT_SIZE = 1024


def _state(stat):
    """The state field of the bytes of a ``stat`` file (after the
    command, which may hold spaces and parentheses)."""
    return stat.rpartition(b")")[2].split(maxsplit=1)[0]


class Processes:
    """
    The threads of the processes *pids* (None and those that ended are
    left out), sampled cheaply: each thread's ``stat`` file stays open
    and is read again from its start, one system call a thread (opening
    it anew takes ten times as long on a loaded machine, as long as a
    sample's interval for a program of 40 threads). Close it after use
    (``with``).
    """

    def __init__(self, pids):
        self._pids = [pid for pid in pids if pid is not None]
        #: The open ``stat`` files, by ``(pid, tid)``.
        self._stats = {}

    def __enter__(self):
        return self

    def __exit__(self, *exc_info):
        self.close()

    def close(self):
        """Close the files kept open."""
        for fd in self._stats.values():
            os.close(fd)
        self._stats.clear()

    def busy(self):
        """The threads with work to do now: a set of ``(pid, tid)``."""
        busy = set()
        listed = set()
        for pid in self._pids:
            try:
                tids = os.listdir(f"/proc/{pid}/task")
            except OSError:
                continue
            for tid in tids:
                thread = (pid, tid)
                listed.add(thread)
                if self._state(thread) in BUSY_STATES:
                    busy.add(thread)
        for gone in self._stats.keys() - listed:
            os.close(self._stats.pop(gone))
        return busy

    def _state(self, thread):
        """The state of *thread*, or None when it ended meanwhile."""
        fd = self._stats.get(thread)
        try:
            if fd is None:
                pid, tid = thread
                fd = self._stats[thread] = os.open(
                    f"/proc/{pid}/task/{tid}/stat", os.O_RDONLY)
            stat = os.pread(fd, _STAT_SIZE, 0)
        except OSError:
            return None
        return _state(stat) if stat else None
