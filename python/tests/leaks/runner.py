"""
The leak harness's runner: runs the scenes (`probe`) under pltrs and
under matplotlib, and tells the leaks.

A scene leaks under pltrs when, once dropped, more of its objects are
alive under pltrs than under matplotlib (`Verdict.alive`); when pltrs
(Rust) keeps more figure states, native objects or removed artists at the
end of the iterations than halfway (`Verdict.native`: these counts come
back to the same value after every scene that frees what it made); or
when the heap or Python's tracked objects grow per iteration in the
second half under pltrs well past matplotlib's growth (`Verdict.growth`).
matplotlib has caches of its own, so it is the comparison that decides,
not zero.

Without the reference venv (the smoke tests' pltrs venv has no
matplotlib) matplotlib's outcomes come from `RECORDED`
(``reference.json``, which ``python -m leaks --record`` writes).
"""

import json
import os
import pathlib
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass

from . import scenes as catalogue

#: This package's directory.
HERE = pathlib.Path(__file__).resolve().parent

#: The repository's root.
ROOT = catalogue.ROOT

#: The reference venv's interpreter, which has matplotlib.
REFERENCE_PYTHON = ROOT / ".venv" / ("Scripts/python.exe" if sys.platform == "win32"
                                     else "bin/python")

#: matplotlib's outcomes, recorded by ``--record``.
RECORDED = HERE / "reference.json"

#: How much more per iteration the heap may grow under pltrs than under
#: matplotlib, in bytes: allocators keep and reuse memory in ways of their
#: own, a leak of a figure or of its artists is far larger.
HEAP_SLACK = 2048

#: How many more Python objects per iteration may stay under pltrs than
#: under matplotlib.
OBJECT_SLACK = 0.5

#: pltrs (Rust)'s counts that come back after every scene.
NATIVE_COUNTS = ("figure_states", "native_objects", "removed_artists")

#: How long one child may run, in seconds.
CHILD_TIME = 900


def _environment(work, reference):
    environment = dict(os.environ)
    tests = str(HERE.parent)
    first = str(HERE / "alias") if reference else str(ROOT / "python")
    environment["PYTHONPATH"] = os.pathsep.join([first, tests])
    environment["MPLBACKEND"] = "agg"
    environment["PLTRS_LOOK"] = "matplotlib"
    config = work / "config"
    environment.setdefault("MPLCONFIGDIR", str(config))
    environment.setdefault("PLTRSCONFIGDIR", str(config))
    return environment


def _interpreter(reference):
    return str(REFERENCE_PYTHON) if reference else sys.executable


def run_children(names, iterations, work, reference, jobs, batch, dropped_iterations=None):
    """The records of the scenes *names*, run in children of *batch*
    scenes each under matplotlib (*reference*) or pltrs (*iterations* of
    the kept scenes, *dropped_iterations* of the dropped ones, if given,
    see `probe`)."""
    work.mkdir(parents=True, exist_ok=True)
    environment = _environment(work, reference)
    batches = [names[i:i + batch] for i in range(0, len(names), batch)]

    def one(share):
        command = [_interpreter(reference), "-m", "leaks.probe", "--iterations",
                   str(iterations), *share]
        if dropped_iterations is not None:
            command[3:3] = ["--dropped-iterations", str(dropped_iterations)]
        try:
            done = subprocess.run(command, capture_output=True, text=True, cwd=work,
                                  env=environment, timeout=CHILD_TIME, check=False)
        except subprocess.TimeoutExpired:
            return [{"scene": name, "error": f"timed out after {CHILD_TIME} s"}
                    for name in share]
        records = [json.loads(line) for line in done.stdout.splitlines()
                   if line.startswith("{")]
        found = {record["scene"]: record for record in records if "scene" in record}
        failure = done.stderr[-1500:] or f"exit {done.returncode}"
        return [found.get(name, {"scene": name, "error": f"child ended: {failure}"})
                for name in share]

    with ThreadPoolExecutor(jobs) as pool:
        return {record["scene"]: record for records in pool.map(one, batches)
                for record in records}


def _growth(samples, key):
    """The growth of *key* per iteration from the second quarter of the
    iterations to the last, each quarter taken at its lowest (a library
    sweeping what it keeps now and then grows and shrinks in steps; a
    leak grows from quarter to quarter); None without enough samples."""
    if not samples:
        return None
    points = sorted((int(count), sample[key]) for count, sample in samples.items()
                    if key in sample)
    if len(points) < 4:
        return None
    quarter = len(points) // 4
    second, last = points[quarter:2 * quarter], points[-quarter:]
    distance = last[0][0] - second[0][0]
    if distance <= 0:
        return None
    return (min(value for _, value in last) - min(value for _, value in second)) / distance


@dataclass
class Verdict:
    """What one scene shows under pltrs against matplotlib."""

    scene: str
    #: Objects alive once dropped, by class, under pltrs beyond matplotlib's.
    alive: dict
    #: Native counts growing per iteration under pltrs.
    native: dict
    #: (pltrs, matplotlib) growth per iteration of the heap and objects,
    #: where pltrs's passes matplotlib's.
    growth: dict
    #: An error of either run, not a leak.
    error: str = ""
    #: Why the scene leaks or fails, where the cause is known
    #: (`scenes.KNOWN`).
    known: str = ""

    @property
    def leaks(self):
        """Whether the scene leaks under pltrs."""
        return bool(self.alive or self.native or self.growth)

    @property
    def unexpected(self):
        """Whether the scene leaks or fails for no known cause."""
        return (self.leaks or bool(self.error)) and not self.known


def judge(name, pltrs, matplotlib):
    """The verdict on scene *name* from its two records."""
    errors = [f"{label}: {record['error']}" for label, record in
              (("pltrs", pltrs), ("matplotlib", matplotlib)) if record.get("error")]
    if errors:
        return Verdict(name, {}, {}, {}, "; ".join(errors))
    reference_alive = matplotlib.get("alive", {})
    alive = {kind: count - reference_alive.get(kind, 0)
             for kind, count in pltrs.get("alive", {}).items()
             if count > reference_alive.get(kind, 0)}
    native = {}
    # (Where matplotlib keeps what the scene makes, so may pltrs.)
    theirs = _growth(matplotlib.get("samples"), "objects")
    if theirs is None or theirs <= OBJECT_SLACK:
        for key in NATIVE_COUNTS:
            rate = _growth(pltrs.get("samples"), key)
            if rate and rate > 0:
                native[key] = rate
    growth = {}
    for key, slack in (("heap", HEAP_SLACK), ("objects", OBJECT_SLACK)):
        ours, theirs = (_growth(record.get("samples"), key) for record in (pltrs, matplotlib))
        if ours is not None and theirs is not None and ours > max(theirs, 0) + slack:
            growth[key] = (ours, theirs)
    return Verdict(name, alive, native, growth)


def selected(patterns, own=False):
    """The names of the scenes matching any of *patterns* (substrings),
    all without patterns (the harness's *own* only, if asked); those
    needing TeX only with TeX on ``PATH``."""
    tex = shutil.which("latex") is not None and shutil.which("xelatex") is not None
    scenes = catalogue.own_scenes() if own else catalogue.all_scenes()
    return [name for name, scene in scenes.items()
            if (tex or not scene.usetex)
            and (not patterns or any(pattern in name for pattern in patterns))]


#: The short mode (the smoke tests'): iterations of the kept scenes, of
#: the dropped ones (none: their check alone), and scenes per child.
SHORT = {"iterations": 80, "dropped_iterations": 0, "batch": 16}


def reference_records(names, settings, work, jobs, recorded=False):
    """matplotlib's records: run where the reference venv is (unless
    *recorded*), else the recorded ones (`RECORDED`, made with the
    `SHORT` settings)."""
    if REFERENCE_PYTHON.exists() and not recorded:
        return run_children(names, settings["iterations"], work / "matplotlib", True, jobs,
                            settings["batch"], settings.get("dropped_iterations"))
    recorded = json.loads(RECORDED.read_text("utf-8"))
    return {name: recorded.get(name, {"scene": name, "error": "not recorded"})
            for name in names}


def record(names, work, jobs):
    """Writes matplotlib's records of *names*, run with the `SHORT`
    settings, to `RECORDED`: what is alive and how its objects grow;
    returns the names of the scenes failing under matplotlib."""
    records = run_children(names, SHORT["iterations"], work / "matplotlib", True, jobs,
                           SHORT["batch"], SHORT["dropped_iterations"])
    kept = {}
    for name, found in sorted(records.items()):
        entry = {key: value for key, value in found.items()
                 if key in ("scene", "kept", "alive", "error")}
        if "samples" in found:
            # The objects only: the counts the judge compares with.
            entry["samples"] = {count: {"objects": sample["objects"]}
                                for count, sample in found["samples"].items()}
        kept[name] = entry
    RECORDED.write_text(json.dumps(kept, indent=1, sort_keys=True) + "\n", "utf-8")
    return [name for name, entry in kept.items() if "error" in entry]


def _cell(value):
    if isinstance(value, float):
        return f"{value:,.0f}" if abs(value) >= 10 else f"{value:.2f}"
    return str(value)


def report(verdicts, pltrs, matplotlib):
    """A Markdown table of every scene: what stays alive, how the heap and
    the objects grow under each, and pltrs (Rust)'s counts."""
    lines = ["| scene | alive (matplotlib) | alive (pltrs) | heap/iter mpl | heap/iter pltrs "
             "| objects/iter mpl | objects/iter pltrs | native/iter | verdict |",
             "|---|---|---|---|---|---|---|---|---|"]
    for verdict in verdicts:
        ours, theirs = pltrs[verdict.scene], matplotlib[verdict.scene]
        cells = [verdict.scene, sum(theirs.get("alive", {}).values()),
                 sum(ours.get("alive", {}).values())]
        for key in ("heap", "objects"):
            for record in (theirs, ours):
                rate = _growth(record.get("samples"), key)
                cells.append("" if rate is None else rate)
        cells.append(", ".join(f"{key} {rate:+.2f}" for key, rate in verdict.native.items()))
        state = verdict.error and "error" or ("LEAK" if verdict.leaks else "ok")
        cells.append(f"{state} (known)" if verdict.known and state != "ok" else state)
        lines.append("| " + " | ".join(_cell(cell) for cell in cells) + " |")
    return "\n".join(lines) + "\n"


def summary(verdict):
    """One line on a leaking or failing scene."""
    known = f" [known: {verdict.known[:120]}]" if verdict.known else ""
    if verdict.error:
        return f"{verdict.scene}: error ({verdict.error[:300]}){known}"
    parts = []
    if verdict.alive:
        parts.append("alive " + ", ".join(f"{kind} {count}" for kind, count in
                                         sorted(verdict.alive.items())))
    if verdict.native:
        parts.append("native " + ", ".join(f"{key} {rate:+.2f}/iteration"
                                          for key, rate in verdict.native.items()))
    for key, (ours, theirs) in verdict.growth.items():
        parts.append(f"{key} {ours:+,.0f}/iteration (matplotlib {theirs:+,.0f})")
    return f"{verdict.scene}: " + "; ".join(parts) + known


def run(names, settings, work, jobs, recorded=False):
    """Runs *names* under both with *settings* (``iterations``,
    ``dropped_iterations``, ``batch``), matplotlib's outcomes the
    *recorded* ones if asked; the verdicts and the records."""
    pltrs = run_children(names, settings["iterations"], work / "pltrs", False, jobs,
                         settings["batch"], settings.get("dropped_iterations"))
    matplotlib = reference_records(names, settings, work, jobs, recorded)
    verdicts = [judge(name, pltrs[name], matplotlib[name]) for name in names]
    for verdict in verdicts:
        verdict.known = catalogue.KNOWN.get(verdict.scene, "")
    return verdicts, pltrs, matplotlib
