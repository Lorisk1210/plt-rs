"""
The fuzzer's runner: hands programs to worker processes, tells the bugs
from the program's own exceptions, and reduces each bug to a minimal
program.

A bug is what no program should meet, whatever it does: a Rust panic
(turned into an exception, or only printed on stderr), a crash (the
worker killed by a signal or exiting), a hang (a program running past its
time) or runaway memory (``MemoryError``, an allocation failing under
the worker's memory limit, or, where the system cannot cap a process's
memory, the worker's resident memory passing that limit). Each is reduced (delta debugging over the
program's statements, each candidate in a new worker) to the smallest
program meeting the same bug, written to ``failures/`` in the fuzzer's
directory (``target/python-fuzz``).

Seeds choose the programs: a seed divisible by 3 mutates a program of the
differential tester's (`corpus`) when its programs can be had, any other
is a random program of the fuzzer's own (`actions`).
"""

import importlib.util
import json
import os
import pathlib
import queue
import re
import subprocess
import sys
import threading
from concurrent.futures import ThreadPoolExecutor

from . import actions, corpus
from .worker import CAPS_ADDRESS_SPACE, MARKER, MEMORY_LIMIT

#: The directory of the fuzzer's package, put on the workers' path.
TESTS = pathlib.Path(__file__).resolve().parents[1]


def _pltrs_home():
    """The directory this process finds pltrs in (the source tree's
    ``python``, or an installed one's site), else the source tree's."""
    spec = importlib.util.find_spec("pltrs")
    if spec is None or spec.origin is None:
        return TESTS.parents[1] / "python"
    return pathlib.Path(spec.origin).resolve().parents[1]


#: Where the workers import pltrs from.
PLTRS_HOME = _pltrs_home()

#: How long one program may run, in seconds.
PROGRAM_TIME = 120

#: How long a candidate of a reduction may run, in seconds: a program
#: running longer meets a hang.
REDUCTION_TIME = 30

#: The most candidates a reduction tries.
REDUCTION_RUNS = 300

#: How many programs a worker runs before a new one takes over.
WORKER_TASKS = 20

#: How often, in seconds, the runner looks at a worker's resident memory
#: where the system cannot cap it.
WATCH_INTERVAL = 0.5

#: The lines a worker runs at its start.
BOOT = (f"import sys; sys.path.insert(0, {str(TESTS)!r}); "
        "from fuzz import worker; worker.main()")

#: Where a panic of pltrs (Rust) reports itself on stderr.
PANICKED = re.compile(r"panicked at (\S+)")


class Bug:
    """A bug a program met: its kind, a signature telling bugs apart, and
    the program's lines up to it."""

    def __init__(self, kind, signature, lines, detail):
        self.kind = kind
        self.signature = signature
        self.lines = lines
        self.detail = detail
        self.seed = None
        self.reduced = None


class Worker:
    """A worker process and the threads reading its output."""

    def __init__(self, directory):
        # The worker imports the pltrs this process does: the source
        # tree's, or an installed one.
        environment = dict(os.environ)
        environment["PYTHONPATH"] = os.pathsep.join(
            [str(PLTRS_HOME), environment.get("PYTHONPATH", "")])
        self.process = subprocess.Popen(
            [sys.executable, "-c", BOOT], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, cwd=directory, env=environment)
        self.events = queue.Queue()
        self.stderr = []
        threading.Thread(target=self._read_events, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()

    def _read_events(self):
        for line in self.process.stdout:
            self.events.put(json.loads(line))
        self.events.put(None)

    def _read_stderr(self):
        for line in self.process.stderr:
            self.stderr.append(line)

    def run(self, task, time_limit=PROGRAM_TIME):
        """Runs *task*, for up to *time_limit* seconds; returns the lines
        it ran and the bugs it met."""
        lines, bugs = [], []
        start = len(self.stderr)
        self.process.stdin.write(json.dumps(task) + "\n")
        self.process.stdin.flush()
        while True:
            event = self._next_event(time_limit)
            if event == "hang":
                self.stop()
                bugs.append(Bug("hang", "hang", lines, f"over {time_limit} s"))
                break
            if event == "memory":
                self.stop()
                bugs.append(Bug("memory", "memory:resident", lines,
                                f"over {MEMORY_LIMIT} bytes resident"))
                break
            if event is None:
                self.process.wait()
                stderr = "".join(self.stderr[start:])
                kind = "memory" if "memory allocation" in stderr else "crash"
                bugs.append(Bug(kind, f"{kind}:{self.process.returncode}", lines, stderr[-2000:]))
                break
            if event["event"] == "line":
                lines.append(event["line"])
            elif event["event"] == "bug":
                bugs.append(Bug(event["kind"], f"{event['kind']}:{event['message'][:120]}",
                                list(lines), event["message"]))
            elif event["event"] == "done":
                break
        bugs += self._stderr_panics(start, lines)
        return lines, bugs

    def _next_event(self, time_limit):
        """The worker's next event (None once it ended), or ``"hang"``
        when none comes within *time_limit* seconds, or ``"memory"`` when
        the worker's resident memory passes `MEMORY_LIMIT` meanwhile (only
        watched where the system cannot cap the worker's memory)."""
        if CAPS_ADDRESS_SPACE:
            try:
                return self.events.get(timeout=time_limit)
            except queue.Empty:
                return "hang"
        waited = 0.0
        while waited < time_limit:
            try:
                return self.events.get(timeout=WATCH_INTERVAL)
            except queue.Empty:
                waited += WATCH_INTERVAL
            if self._resident() > MEMORY_LIMIT:
                return "memory"
        return "hang"

    def _resident(self):
        """The worker's resident memory in bytes, as ``ps`` reports it
        (zero once the worker is gone)."""
        report = subprocess.run(["ps", "-o", "rss=", "-p", str(self.process.pid)],
                                capture_output=True, text=True, check=False).stdout.strip()
        return int(report) * 1024 if report.isdigit() else 0

    def _stderr_panics(self, start, lines):
        """Panics printed on stderr during the task (also those no
        exception carried, such as one caught while freeing an object)."""
        text = "".join(line for line in self.stderr[start:] if not line.startswith(MARKER))
        return [Bug("panic", f"panic at {where}", list(lines), text[-2000:])
                for where in dict.fromkeys(PANICKED.findall(text))]

    def alive(self):
        """Whether the process still runs."""
        return self.process.poll() is None

    def stop(self):
        """Ends the process."""
        if self.alive():
            self.process.kill()
        self.process.wait()


def _task(seed, steps, programs):
    if programs and seed % 3 == 0:
        source = programs[(seed // 3) % len(programs)]
        return {"id": seed, "lines": corpus.mutate(source, seed)}
    return {"id": seed, "seed": seed, "steps": steps}


def _run_seeds(seeds, steps, programs, directory):
    """Runs the programs of *seeds* in workers of their own, a new one
    every `WORKER_TASKS` programs and after each crash or hang."""
    found = []
    worker = None
    for count, seed in enumerate(seeds):
        if worker is None or not worker.alive() or count % WORKER_TASKS == 0:
            if worker is not None:
                worker.stop()
            worker = Worker(directory)
        _, bugs = worker.run(_task(seed, steps, programs))
        for bug in bugs:
            bug.seed = seed
        found += bugs
    if worker is not None:
        worker.stop()
    return found


def _meets(lines, signature, directory, time_limit=REDUCTION_TIME):
    """Whether the program *lines* meets the bug *signature*, run alone."""
    worker = Worker(directory)
    try:
        _, bugs = worker.run({"id": 0, "lines": lines}, time_limit)
    finally:
        worker.stop()
    return any(bug.signature == signature for bug in bugs)


def reduce(bug, directory):
    """The smallest program found meeting *bug* (delta debugging over the
    statements after the header, up to `REDUCTION_RUNS` candidates), or
    None if the bug needs the programs the worker ran before."""
    header = len(actions.HEADER)
    head, body = bug.lines[:header], bug.lines[header:]
    if not _meets(head + body, bug.signature, directory, PROGRAM_TIME):
        return None
    chunks, runs = 2, 0
    while len(body) >= 2 and runs < REDUCTION_RUNS:
        size = max(1, len(body) // chunks)
        pieces = [body[i:i + size] for i in range(0, len(body), size)]
        for index in range(len(pieces)):
            candidate = [line for i, piece in enumerate(pieces) if i != index for line in piece]
            runs += 1
            if _meets(head + candidate, bug.signature, directory):
                body, chunks = candidate, max(chunks - 1, 2)
                break
        else:
            if size == 1:
                break
            chunks = min(len(body), chunks * 2)
    return head + body


def run(scale=1, replay=None, jobs=4, reduce_bugs=True, directory=None, first=0):
    """Runs ``24 × scale`` programs from seed *first* on (or the one of
    seed *replay*) on *jobs* workers; returns the distinct bugs met, each
    reduced and written to ``failures/`` under *directory*."""
    directory = pathlib.Path(directory or TESTS.parents[1] / "target" / "python-fuzz")
    (directory / "failures").mkdir(parents=True, exist_ok=True)
    programs = corpus.load(directory)
    steps = 100
    seeds = [int(replay)] if replay is not None else list(range(first, first + 24 * scale))
    shares = [seeds[i::jobs] for i in range(jobs)]
    with ThreadPoolExecutor(jobs) as pool:
        found = [bug for bugs in pool.map(lambda share: _run_seeds(share, steps, programs,
                                                                   directory), shares)
                 for bug in bugs]
    distinct = {}
    for bug in found:
        distinct.setdefault(bug.signature, bug)
    if reduce_bugs:
        with ThreadPoolExecutor(jobs) as pool:
            reduced = list(pool.map(lambda bug: reduce(bug, directory), distinct.values()))
        for bug, program in zip(distinct.values(), reduced):
            bug.reduced = program
    for number, bug in enumerate(distinct.values()):
        program = bug.reduced or bug.lines
        note = ("reduced" if bug.reduced else
                "as run (did not reproduce alone)" if reduce_bugs else "as run")
        path = directory / "failures" / f"bug{number}-seed{bug.seed}.py"
        path.write_text(f"# {bug.signature}\n# seed {bug.seed}, {note}\n"
                        + "".join(f"# {line}\n" for line in bug.detail.splitlines()[-15:])
                        + "\n".join(program) + "\n")
        bug.path = path
    return list(distinct.values())


def summary(bugs):
    """One line per bug."""
    return "\n".join(f"{bug.signature} (seed {bug.seed}): {bug.path}" for bug in bugs)
