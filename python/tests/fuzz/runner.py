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

from . import actions, compare, corpus, memory
from .worker import CAPS_ADDRESS_SPACE, MARKER

#: The directory of the fuzzer's package, put on the workers' path.
TESTS = pathlib.Path(__file__).resolve().parents[1]

#: What makes ``import pltrs`` import matplotlib in a reference worker
#: (the leak harness's alias, `leaks.alias`).
ALIAS = TESTS / "leaks" / "alias"


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

#: The seeds whose programs met a bug once, run before the others in every
#: run (from any *first*), so that a bug coming back is met at any scale:
#: 22, a formatter saving its own figure (the native stack overflowed);
#: 1019, a locator whose ``nonsingular`` sets the other axis' limits while
#: the figure is drawn, and 100015, one measuring the figure while limits
#: are set (pltrs refused both); 101239, a norm drawing its figure while a
#: colorbar of it is ticked (a Rust panic).
FAILED_SEEDS = (22, 1019, 100015, 101239)

#: The soft stack limit, in KiB, the workers run under where the system
#: sets one (``ulimit -s``): below every platform's main-thread stack
#: (Windows' 2 MB), so that a recursion through native code meets the end
#: of the stack in any build. Seed 22 overflowed Linux's 8 MiB only in the
#: optimised build, whose frames are larger, not in the development build
#: the smoke tests run.
WORKER_STACK_KIB = 1024

#: The lines a worker runs at its start.
BOOT = (f"import sys; sys.path.insert(0, {str(TESTS)!r}); "
        "from fuzz import worker; worker.main()")

#: Where a panic of pltrs (Rust) reports itself on stderr.
PANICKED = re.compile(r"panicked at (\S+)")

#: The line pltrs (Rust) writes on a fatal fault in a worker (which sets
#: ``PLTRS_REPORT_FAULTS``), before Python's ``faulthandler`` reports where
#: Python and the native code were: the address the fault touched.
FAULT = re.compile(r"pltrs: fatal signal \d+ at address (0x[0-9a-f]+)")

#: How many lines of a bug's output its failure file keeps from the start
#: (a crash's: the fault, then where each thread was) and from the end.
DETAIL_HEAD = 40
DETAIL_TAIL = 15


class Bug:
    """A bug a program met: its kind, a signature telling bugs apart, and
    the program's lines up to it; *shared* once matplotlib met it too
    (runaway memory or a hang, see `run`), which is listed, not fixed."""

    def __init__(self, kind, signature, lines, detail):
        self.kind = kind
        self.signature = signature
        self.lines = lines
        self.detail = detail
        self.seed = None
        self.reduced = None
        self.shared = False


#: The kinds of bugs matplotlib may share: a program can run away or hang
#: under it too (its data limits near 1e300 on polar axes).
SHAREABLE = ("memory", "hang")


#: How a worker ends when an allocation fails in native code (an abort,
#: the system's kill) or a `MemoryError` escapes the worker's own code
#: (status 1), which a program running away under matplotlib meets as
#: well as a `MemoryError` of its own.
OUT_OF_MEMORY = ("crash:-6", "crash:-9", "crash:1")

#: How many times matplotlib runs a program to see whether it meets a
#: bug too (where memory runs out differs from run to run).
SHARED_RUNS = 3


def memory_limit(jobs):
    """The memory each of *jobs* workers may use: 6 GiB, or less, so that
    together with the runner they stay within the machine's physical
    memory."""
    limit = 6 << 30
    try:
        physical = os.sysconf("SC_PHYS_PAGES") * os.sysconf("SC_PAGE_SIZE")
    except (AttributeError, ValueError, OSError):
        return limit
    return min(limit, physical // (jobs + 1))


def _memory_limit():
    """The memory a worker may use (`memory_limit`, as `run` set it)."""
    return int(os.environ.get("PLTRS_FUZZ_MEMORY", 6 << 30))


def real(bugs):
    """The bugs of *bugs* matplotlib did not meet too."""
    return [bug for bug in bugs if not bug.shared]


def _interpreter():
    """The program running this interpreter in a new process, with the
    variables it needs: on Windows a venv's ``python.exe`` is a launcher
    starting the base interpreter as a child, whose memory the runner would
    not see, so the base interpreter runs directly, named the venv's as the
    launcher names it."""
    if sys.platform == "win32" and sys.prefix != sys.base_prefix:
        return sys._base_executable, {"__PYVENV_LAUNCHER__": sys.executable}
    return sys.executable, {}


class Worker:
    """A worker process and the threads reading its output."""

    def __init__(self, directory, reference=False):
        # The worker imports the pltrs this process does: the source
        # tree's, or an installed one; a *reference* worker imports
        # matplotlib for it, with the reference venv's interpreter.
        program, variables = _interpreter()
        environment = {**os.environ, **variables}
        environment["PYTHONPATH"] = os.pathsep.join(
            [str(PLTRS_HOME), environment.get("PYTHONPATH", "")])
        environment["PLTRS_REPORT_FAULTS"] = "1"
        if reference:
            program = str(corpus.REFERENCE_PYTHON)
            environment["PYTHONPATH"] = str(ALIAS)
        command = [program, "-c", BOOT]
        if os.name == "posix":
            # The shell sets the limit and becomes the interpreter.
            command = ["/bin/sh", "-c",
                       f'ulimit -s {WORKER_STACK_KIB} 2>/dev/null; exec "$0" "$@"', *command]
        self.process = subprocess.Popen(
            command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, encoding="utf-8", errors="replace", cwd=directory,
            env=environment)
        # The interpreter running the worker, once it tells: on Windows, a
        # venv's python.exe is a launcher starting it as a process of its
        # own (which ends with the launcher).
        self.pid = self.process.pid
        # The main thread's stack, as the worker reports it.
        self.stack = None
        self.events = queue.Queue()
        self.stderr = []
        threading.Thread(target=self._read_events, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()

    def _read_events(self):
        for line in self.process.stdout:
            event = json.loads(line)
            if event["event"] == "ready":
                self.pid = event["pid"]
                self.stack = event.get("stack")
            else:
                self.events.put(event)
        self.events.put(None)

    def _read_stderr(self):
        for line in self.process.stderr:
            self.stderr.append(line)

    def run(self, task, time_limit=PROGRAM_TIME):
        """Runs *task*, for up to *time_limit* seconds; returns the lines
        it ran and the bugs it met (and keeps the exception each line
        raised, or None, in `outcomes`)."""
        lines, bugs = [], []
        self.outcomes = []
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
                                f"over {_memory_limit()} bytes resident"))
                break
            if event is None:
                self.process.wait()
                stderr = "".join(self.stderr[start:])
                kind = "memory" if "memory allocation" in stderr else "crash"
                detail = _fault_note(stderr, self.stack) + stderr
                bugs.append(Bug(kind, f"{kind}:{self.process.returncode}", lines, detail))
                break
            if event["event"] == "line":
                lines.append(event["line"])
            elif event["event"] == "ran":
                self.outcomes.append(event["raised"])
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
        the worker's resident memory passes `_memory_limit` meanwhile (only
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
            if memory.resident(self.pid) > _memory_limit():
                return "memory"
        return "hang"

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
        return {"id": seed, "lines": corpus.mutate(corpus.chosen(programs, seed // 3), seed)}
    return {"id": seed, "seed": seed, "steps": steps}


def _run_seeds(seeds, steps, programs, directory, ran=None):
    """Runs the programs of *seeds* in workers of their own, a new one
    every `WORKER_TASKS` programs and after each crash or hang; appends
    ``(seed, lines, outcomes)`` of each program meeting no bug to *ran*,
    if given."""
    found = []
    worker = None
    for count, seed in enumerate(seeds):
        if worker is None or not worker.alive() or count % WORKER_TASKS == 0:
            if worker is not None:
                worker.stop()
            worker = Worker(directory)
        lines, bugs = worker.run(_task(seed, steps, programs))
        for bug in bugs:
            bug.seed = seed
        found += bugs
        if ran is not None and not bugs:
            ran.append((seed, lines, worker.outcomes))
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


def run(scale=1, replay=None, jobs=4, reduce_bugs=True, directory=None, first=0,
        compare_with_matplotlib=False):
    """Runs the programs of `FAILED_SEEDS`, then ``24 × scale`` programs
    from seed *first* on (or the one of seed *replay* only) on *jobs*
    workers, each capped at `memory_limit`; returns the distinct bugs
    met, each reduced and written to ``failures/`` under *directory*. A
    runaway or hang the program meets under matplotlib too (where the
    reference venv is) is shared, written as ``shared<n>-seed<s>.py``. With
    *compare_with_matplotlib*, the programs meeting no bug run under
    matplotlib too, and where a line raised another exception than
    matplotlib's (or none where it raised one) is written to
    ``differences.md`` under *directory* (`.compare`)."""
    directory = pathlib.Path(directory or TESTS.parents[1] / "target" / "python-fuzz").resolve()
    (directory / "failures").mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("PLTRS_FUZZ_MEMORY", str(memory_limit(jobs)))
    programs = corpus.load(directory)
    steps = 100
    if replay is not None:
        seeds = [int(replay)]
    else:
        seeds = [*FAILED_SEEDS, *(seed for seed in range(first, first + 24 * scale)
                                  if seed not in FAILED_SEEDS)]
    shares = [seeds[i::jobs] for i in range(jobs)]
    ran = [] if compare_with_matplotlib else None
    with ThreadPoolExecutor(jobs) as pool:
        found = [bug for bugs in pool.map(lambda share: _run_seeds(share, steps, programs,
                                                                   directory, ran), shares)
                 for bug in bugs]
    if ran:
        compare.write(compare.differences(ran, directory, jobs), directory)
    distinct = {}
    for bug in found:
        distinct.setdefault(bug.signature, bug)
    if reduce_bugs:
        with ThreadPoolExecutor(jobs) as pool:
            reduced = list(pool.map(lambda bug: reduce(bug, directory), distinct.values()))
        for bug, program in zip(distinct.values(), reduced):
            bug.reduced = program
    for bug in distinct.values():
        bug.shared = _shared(bug, directory)
    for number, bug in enumerate(distinct.values()):
        program = bug.reduced or bug.lines
        note = ("reduced" if bug.reduced else
                "as run (did not reproduce alone)" if reduce_bugs else "as run")
        if bug.shared:
            note += "; matplotlib meets it too"
        name = "shared" if bug.shared else "bug"
        path = directory / "failures" / f"{name}{number}-seed{bug.seed}.py"
        path.write_text(f"# {bug.signature}\n# seed {bug.seed}, {note}\n"
                        + "".join(f"# {line}\n" for line in _kept(bug.detail.splitlines()))
                        + "\n".join(program) + "\n", encoding="utf-8")
        bug.path = path
    return list(distinct.values())


def _fault_note(stderr, stack):
    """A line placing the address a crash touched (``FAULT``) against the
    worker's main stack, ``[lowest, highest]`` (`worker._main_stack`):
    below the lowest by less than the guard gap glibc and the kernel leave
    is a stack overflow. Empty without the address."""
    found = FAULT.search(stderr)
    if found is None:
        return ""
    address = int(found.group(1), 16)
    if stack is None:
        return f"fault at {address:#x}\n"
    lowest, highest = stack
    if lowest - (1 << 20) <= address < lowest + 4096:
        where = f"{(lowest - address) / 1024:.1f} KiB below the main thread's stack limit: a stack overflow"
    elif lowest <= address < highest:
        where = f"in the main thread's stack, {(highest - address) / 1024:.1f} KiB below its top"
    else:
        where = "outside the main thread's stack"
    return f"fault at {address:#x}, {where}\n"


def _kept(lines):
    """The lines of a bug's output its failure file keeps: the first
    `DETAIL_HEAD` and the last `DETAIL_TAIL`."""
    if len(lines) <= DETAIL_HEAD + DETAIL_TAIL:
        return lines
    left_out = len(lines) - DETAIL_HEAD - DETAIL_TAIL
    return [*lines[:DETAIL_HEAD], f"[{left_out} lines left out]", *lines[-DETAIL_TAIL:]]


def _shared(bug, directory):
    """Whether matplotlib meets *bug*, a runaway or a hang, running its
    program (where the reference venv is) in one of `SHARED_RUNS` runs."""
    if bug.kind not in SHAREABLE or not corpus.REFERENCE_PYTHON.exists():
        return False
    for _ in range(SHARED_RUNS):
        worker = Worker(directory, reference=True)
        try:
            _, theirs = worker.run({"id": bug.seed, "lines": bug.reduced or bug.lines})
        finally:
            worker.stop()
        if any(their.kind == bug.kind
               or (bug.kind == "memory" and their.signature in OUT_OF_MEMORY)
               for their in theirs):
            return True
    return False


def summary(bugs):
    """One line per bug, those matplotlib shares marked."""
    return "\n".join(f"{'shared with matplotlib: ' if bug.shared else ''}{bug.signature} "
                     f"(seed {bug.seed}): {bug.path}" for bug in bugs)
