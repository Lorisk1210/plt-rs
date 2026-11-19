"""
Running one script under matplotlib and under pltrs (Python), one process
at a time, and what the runs measured.

Each run is a fresh process of `child.py` in a scratch directory of its
own, with ``MPLBACKEND=agg`` and configuration and cache directories that
are the tool's, never the user's. The parent sends the start stamp once
the child's interpreter is up, so the interpreter's own start is outside
the span and importing the library inside it.
"""

import json
import os
import pathlib
import statistics
import subprocess
import time
from dataclasses import dataclass, field

#: The child process that runs a script.
CHILD = pathlib.Path(__file__).resolve().with_name("child.py")
#: The libraries, in the order of the table and of the clip (left, right).
LIBRARIES = ("matplotlib", "pltrs")
#: ``PLTRS_LOOK`` for each look of pltrs' side.
LOOKS = {"classic": "matplotlib", "modern": "modern"}
#: Variables of the user's environment that would change what either
#: side draws or where it keeps its files; the runs set their own.
CLEARED = (
    "MPLBACKEND", "MPLCONFIGDIR", "PLTRSCONFIGDIR", "MATPLOTLIBRC",
    "PLTRS_LOOK", "PLTRS_FIGURE_LOOK", "PLTRS_CHROME_LOOK", "PYTHONIOENCODING",
)


@dataclass(frozen=True)
class Side:
    """One side of the comparison: a library and how its child runs."""

    #: ``matplotlib`` or ``pltrs``.
    library: str
    #: The interpreter running the child.
    python: str
    #: pltrs' side: the directory of the ``matplotlib`` alias, first on
    #: ``sys.path``.
    alias: str | None = None
    #: pltrs' side: the checkout's ``python/`` when pltrs comes from it,
    #: None when it comes from the interpreter's environment.
    sources: str | None = None

    def command(self, *arguments):
        """The child's command line for *arguments*."""
        command = [self.python, str(CHILD), *arguments]
        for option, value in (("--alias", self.alias), ("--sources", self.sources)):
            if value is not None:
                command += [option, value]
        return command

    def environment(self, look, config):
        """The child's environment in *look*, its configuration and cache
        directory *config*."""
        env = {name: value for name, value in os.environ.items() if name not in CLEARED}
        if "PYTHONPATH" in env:
            # The children run in scratch directories: the parent's
            # relative entries are made absolute.
            entries = env["PYTHONPATH"].split(os.pathsep)
            env["PYTHONPATH"] = os.pathsep.join(os.path.abspath(entry) for entry in entries if entry)
        env["MPLBACKEND"] = "agg"
        # The child's output goes into pipes, which Windows would encode
        # in its ANSI code page: a script printing what that lacks ("→")
        # would fail there on both sides, though it runs in a console.
        env["PYTHONIOENCODING"] = "utf-8"
        env["MPLCONFIGDIR"] = str(config)
        if self.library == "pltrs":
            env["PLTRSCONFIGDIR"] = str(config)
            env["PLTRS_LOOK"] = LOOKS[look]
        return env


@dataclass
class Run:
    """What one child process reported."""

    #: Seconds from the start stamp to the figure ready (``plt.show()``).
    ready: float | None = None
    #: Seconds from the start stamp to the script's end.
    end: float | None = None
    #: Peak resident memory in bytes, where the system gives it.
    peak_memory: int | None = None
    #: The PNG files of the figures' pictures (or the image it saved).
    pictures: list = field(default_factory=list)
    #: The library's version and the directory it was loaded from.
    version: str | None = None
    location: str | None = None
    #: The traceback (or the child's output) if the side failed.
    error: str | None = None


@dataclass
class Measured:
    """One side's runs of one script in one look."""

    side: Side
    runs: list = field(default_factory=list)
    #: The first failure; the side then stops running.
    error: str | None = None
    #: The unmeasured round's run, which tells the version and location.
    warm: Run | None = None

    @property
    def ready(self):
        """The ready times of the measured runs."""
        return [run.ready for run in self.runs]

    def median(self, name="ready"):
        """The median of the runs' *name* (``ready``, ``end``,
        ``peak_memory``), or None."""
        values = [getattr(run, name) for run in self.runs if getattr(run, name) is not None]
        return statistics.median(values) if values else None

    @property
    def pictures(self):
        """The pictures of the first measured run."""
        return self.runs[0].pictures if self.runs else []

    def add(self, run):
        """Records *run*: a measured run, or the side's failure."""
        if run.error is not None:
            self.error = run.error
        else:
            self.runs.append(run)


def run_once(side, script, look, directory, config):
    """Runs *script* once on *side* in *look*, in the new scratch
    *directory*, with the configuration and cache directory *config*;
    returns its `Run`."""
    directory.mkdir(parents=True)
    result = directory / "result.json"
    child = subprocess.Popen(
        side.command("run", side.library, str(script), str(result)),
        cwd=directory, env=side.environment(look, config),
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8", errors="replace")
    greeting = child.stdout.readline()
    if greeting.strip() != "READY":
        output, errors = child.communicate()
        return Run(error=f"the child did not start:\n{greeting}{output}{errors}")
    _output, errors = child.communicate(f"{time.time():.6f}\n")
    if not result.is_file():
        return Run(error=errors or f"the child exited with status {child.returncode}")
    return Run(**json.loads(result.read_text(encoding="utf-8")))


def measure(sides, script, look, runs, work, config):
    """Runs *script* on both *sides* in *look*: one unmeasured round,
    then *runs* rounds alternating which side goes first, each in a
    directory of its own under *work* and with the library's directory
    under *config*. Returns a `Measured` per library."""
    measured = {side.library: Measured(side) for side in sides}
    for round_ in range(runs + 1):
        order = sides if round_ % 2 == 0 else sides[::-1]
        for side in order:
            found = measured[side.library]
            if found.error is not None:
                continue
            directory = work / f"{side.library}-{round_}"
            run = run_once(side, script, look, directory, config / side.library)
            if round_ == 0:
                found.warm = run
                found.error = run.error
            else:
                found.add(run)
    return measured
