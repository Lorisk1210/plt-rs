"""
The fuzzer's worker: runs programs and reports how each line ended.

The runner (`runner`) starts a worker as a process of its own, so that a
crash, a hang or runaway memory ends the worker, not the run. Tasks come
in on stdin, one JSON object per line: ``{"seed": n, "steps": k}`` for a
random program (`actions.Program`), ``{"lines": [...]}`` for a program
given as its lines. Events go out on stdout, one JSON object per line:
``line`` before each line runs, ``bug`` when one raised what no program
should see (a Rust panic turned into an exception, a ``MemoryError`` in
pltrs once memory ran away), and ``done`` when the task ended. A marker line on stderr starts each task,
so that the runner tells which task a panic's message on stderr belongs
to.

Every Python exception a line raises is the program's business, and is
not reported: matplotlib raises too.
"""

import faulthandler
import gc
import json
import os
import resource
import sys
import warnings

from . import actions

#: What a panic of pltrs (Rust) looks like once it reaches Python.
PANIC = "Rust panic"

#: The stderr line that starts a task.
MARKER = "@@pltrs-fuzz-task"

#: The memory a worker may use, in bytes: its address space where the
#: system caps it (Linux), its resident memory, which the runner watches,
#: where it cannot (macOS).
MEMORY_LIMIT = 6 << 30

#: Whether the system caps a process's address space (macOS refuses
#: `RLIMIT_AS`).
CAPS_ADDRESS_SPACE = sys.platform != "darwin"

#: The memory past which a worker's `MemoryError` counts as runaway.
RUNAWAY = 2 << 30


#: Where events go: the worker's stdout, which programs no longer reach
#: once `main` gives them stderr in its place.
_EVENTS = sys.stdout


def _send(event, **fields):
    _EVENTS.write(json.dumps({"event": event, **fields}) + "\n")
    _EVENTS.flush()


def _run_line(line, namespace):
    """Runs one line; reports it first, and a bug if it raised one."""
    _send("line", line=line)
    try:
        exec(compile(line, "<fuzz>", "exec"), namespace)  # noqa: S102 - the fuzzed program
    except MemoryError as error:
        if _raised_by_pltrs(error) and _ran_away():
            _send("bug", kind="memory", line=line, message="MemoryError")
    except Exception as error:  # noqa: BLE001 - the program's exceptions are its own
        message = str(error)
        if PANIC in message:
            _send("bug", kind="panic", line=line, message=message.splitlines()[0][:300])


def _ran_away():
    """Whether the worker's memory grew past `RUNAWAY` before a
    `MemoryError`: one raised at once (numpy refusing an array of 2³²
    axes, as it does in matplotlib) is the program's."""
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # Linux counts the peak in KiB, macOS in bytes.
    if sys.platform != "darwin":
        peak *= 1024
    return peak > RUNAWAY


def _raised_by_pltrs(error):
    """Whether pltrs raised *error*, rather than numpy making the
    program's absurd data."""
    traceback = error.__traceback__
    while traceback is not None:
        if f"{os.sep}pltrs{os.sep}" in traceback.tb_frame.f_code.co_filename:
            return True
        traceback = traceback.tb_next
    return False


def _random_program(seed, steps):
    program = actions.Program(seed)
    for line in actions.HEADER:
        _run_line(line, program.namespace)
    for _ in range(steps):
        try:
            line = program.next_line()
        except Exception as error:  # noqa: BLE001 - a broken object to choose from
            line = f"pass  # no action: {type(error).__name__}"
        _run_line(line, program.namespace)
        program.ran(line)


def _given_program(lines):
    namespace = {}
    for line in lines:
        _run_line(line, namespace)


def _reset(saved_rc):
    """Leaves the process as a new one would be for the next task, as far
    as the program let it (an rcParam it broke stays broken)."""
    import pltrs.pyplot as plt
    for reset in (lambda: plt.close("all"), lambda: plt.rcParams.update(saved_rc), gc.collect):
        try:
            reset()
        except Exception:  # noqa: BLE001 - the program's doing, reported by its lines
            pass


def main():
    """Runs the tasks read from stdin until it closes."""
    global _EVENTS
    # What programs print (a callback printing its event) goes to stderr,
    # out of the way of the events.
    _EVENTS = os.fdopen(os.dup(sys.stdout.fileno()), "w")
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    if CAPS_ADDRESS_SPACE:
        resource.setrlimit(resource.RLIMIT_AS, (MEMORY_LIMIT, MEMORY_LIMIT))
    # A crash prints where Python was, for the bug's report.
    faulthandler.enable()
    warnings.simplefilter("ignore")
    import pltrs
    pltrs.use("agg")
    saved_rc = {key: value for key, value in pltrs.rcParams.items() if key != "backend"}
    for request in sys.stdin:
        task = json.loads(request)
        sys.stderr.write(f"{MARKER} {task['id']}\n")
        sys.stderr.flush()
        _send("start", id=task["id"])
        if "lines" in task:
            _given_program(task["lines"])
        else:
            _random_program(task["seed"], task["steps"])
        _reset(saved_rc)
        _send("done", id=task["id"])
