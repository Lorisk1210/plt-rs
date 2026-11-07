"""How much of matplotlib's public API the differential tester's programs
reach.

    python coverage.py [--suite gallery|generated|all] [PATTERN...] [--jobs N]
                       --root ROOT --out OUT --gallery GALLERY

``cargo xtask diff --coverage`` runs this in the reference venv. It

1. lists matplotlib 3.11's public callables (``api_index``: modules and
   classes as ``spec/api-audit/enumerate.py`` takes them; functions,
   classes, methods and properties, and the keyword parameters of the
   functions, methods and constructors);
2. runs every program of the tester (the gallery and the generated
   programs, as ``driver.py`` collects them) under matplotlib with its
   calls traced (``reach.py``), the runs cached by the program's text;
3. writes ``OUT/coverage.md``: the share of the callables and of their
   parameters the programs call themselves, overall, for the plotting
   methods (``Axes``, ``Axes3D`` and pyplot), per module and per class,
   the kinds of values the parameters are given (``coverage_kinds``:
   ``None``, each type, empty and single-element data, NaN, inf,
   negative numbers and zero, each string choice of matplotlib's checks,
   unknown keywords, aliases), then every callable and plotting parameter
   not reached and every string choice not given, and
   ``OUT/coverage.json`` with the same, per callable.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import api_index  # noqa: E402
import coverage_kinds  # noqa: E402
import driver  # noqa: E402
import kinds  # noqa: E402

#: Seconds a program may run.
TIMEOUT = 300
#: The groups (module, or module:Owner) whose methods are the plotting methods.
PLOTTING_GROUPS = ("matplotlib.axes:Axes", "mpl_toolkits.mplot3d:Axes3D", "matplotlib.pyplot")
#: The files the traced runs depend on, besides the program.
HARNESS = ("reach.py", "trace.py", "describe.py", "api_index.py", "kinds.py")


def _reach(setup: driver.Setup, api: pathlib.Path, program: driver.Program,
           digest: str) -> dict[str, Any] | None:
    """What ``program`` reaches, from the cache or a traced run (None if
    the run failed)."""
    directory = setup.out / "coverage" / "runs" / re.sub(r"[^\w.-]", "_", program.name)
    stamp, record = directory / "digest", directory / "reach.json"
    key = hashlib.sha256(program.path.read_bytes() + program.mode.encode() + digest.encode()).hexdigest()[:16]
    if stamp.exists() and stamp.read_text() == key:
        return json.loads(record.read_text()) if record.exists() else None
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)
    command = [sys.executable, str(HERE / "reach.py"), str(api), program.mode, str(program.path), str(directory)]
    try:
        done = subprocess.run(command, env=setup.environment("matplotlib"), capture_output=True, text=True,
                              timeout=TIMEOUT)
        (directory / "stderr.txt").write_text(done.stderr[-20000:])
    except subprocess.TimeoutExpired:
        (directory / "stderr.txt").write_text(f"timed out after {TIMEOUT} s")
    stamp.write_text(key)
    return json.loads(record.read_text()) if record.exists() else None


def _share(reached: int, total: int) -> str:
    return f"{reached}/{total} ({100 * reached / total:.1f}%)" if total else "-"


class Coverage:
    """The callables and parameters the runs reached."""

    def __init__(self, index: api_index.Index, runs: list[dict[str, Any]]) -> None:
        self.index = index
        self.params: dict[str, set[str]] = defaultdict(set)
        self.extra: dict[str, set[str]] = defaultdict(set)
        for run in runs:
            for key, params in run["calls"].items():
                self.params[key].update(params)
            for key, keys in run["extra"].items():
                self.extra[key].update(keys)
        # A class counts as reached when it is made or one of its own
        # members is reached.
        for callable_ in index.callables:
            if callable_.owner and callable_.key in self.params:
                self.params.setdefault(f"{callable_.module}:{callable_.owner}", set())
        self.kinds = coverage_kinds.KindsCoverage(index, runs, kinds.Checks(index.callables), set(self.params))

    def reached(self, callable_: api_index.Callable) -> bool:
        """Whether a program reached ``callable_``."""
        return callable_.key in self.params

    def missing_params(self, callable_: api_index.Callable) -> list[str]:
        """The parameters of ``callable_`` no program gave."""
        given = self.params.get(callable_.key, set())
        return [p for p in callable_.params if p not in given]

    def counts(self, callables: list[api_index.Callable]) -> tuple[int, int, int, int]:
        """Callables reached, callables, parameters given, parameters."""
        reached = sum(map(self.reached, callables))
        params = sum(len(c.params) for c in callables)
        given = params - sum(len(self.missing_params(c)) for c in callables)
        return reached, len(callables), given, params

    def report(self, programs: int, failed: list[str]) -> str:
        """The Markdown report."""
        everything = self.index.callables
        plotting = [c for c in everything if c.group in PLOTTING_GROUPS]
        reached, total, given, params = self.counts(everything)
        _, _, plot_given, plot_params = self.counts(plotting)
        lines = [
            "# Coverage of matplotlib's public API by the differential tester",
            "",
            f"Written by `cargo xtask diff --coverage` (`spec/diff/coverage.py`) from {programs} programs"
            + (f", {len(failed)} of which failed to run under the trace ({', '.join(failed[:10])})" if failed else "")
            + ". A callable counts when a program calls it (or reads it, for a property; refers to it, for an "
            "enumeration or exception class) itself, not when matplotlib calls it on the program's behalf; a "
            "parameter counts when such a call gives it, by keyword or position.",
            "",
            f"* Callables reached: {_share(reached, total)}",
            f"* Keyword parameters given: {_share(given, params)}",
            f"* Parameters of the plotting methods (`Axes`, `Axes3D`, pyplot) given: "
            f"{_share(plot_given, plot_params)}",
            *self.kinds.summary(),
            "",
            "## By module",
            "",
            "| module | callables | parameters |",
            "|---|---|---|",
        ]
        modules: dict[str, list[api_index.Callable]] = defaultdict(list)
        for callable_ in everything:
            modules[callable_.module].append(callable_)
        for module, callables in sorted(modules.items(), key=lambda item: -len(item[1])):
            r, t, g, p = self.counts(callables)
            lines.append(f"| {module} | {_share(r, t)} | {_share(g, p)} |")
        lines += ["", "## By class", "", "| class | callables | parameters |", "|---|---|---|"]
        groups = [(group, callables) for group, callables in self.index.groups() if ":" in group]
        for group, callables in sorted(groups, key=lambda item: -len(item[1])):
            r, t, g, p = self.counts(callables)
            lines.append(f"| {group} | {_share(r, t)} | {_share(g, p)} |")
        lines += ["", "## Not reached", ""]
        for group, callables in self.index.groups():
            missing = [c for c in callables if not self.reached(c)]
            if missing:
                names = ", ".join(f"`{c.name}`" + (" (deprecated)" if c.deprecated else "") for c in missing)
                lines.append(f"* **{group}**: {names}")
        lines += ["", "## Parameters of the plotting methods not given", ""]
        for callable_ in plotting:
            missing = self.missing_params(callable_)
            if missing:
                lines.append(f"* `{callable_.key}`: " + ", ".join(missing))
        lines += ["", *self.kinds.kinds_table(), "", *self.kinds.missing_choices()]
        return "\n".join(lines) + "\n"

    def as_json(self) -> dict[str, Any]:
        """Every callable with whether it was reached and its parameters."""
        return {"callables": [
            dict(c.as_json(), reached=self.reached(c), missing_params=self.missing_params(c),
                 extra=sorted(self.extra.get(c.key, ())), **self.kinds.as_json(c))
            for c in self.index.callables
        ]}


def main() -> int:
    """Trace the programs and write the report."""
    parser = argparse.ArgumentParser()
    parser.add_argument("patterns", nargs="*")
    parser.add_argument("--suite", choices=["gallery", "generated", "all"], default="all")
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--gallery", required=True)
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_intermixed_args()
    setup = driver.Setup(argparse.Namespace(root=args.root, out=args.out, gallery=args.gallery, pltrs_python=""))
    (setup.out / "coverage").mkdir(parents=True, exist_ok=True)

    api = setup.out / "coverage" / "api.jsonl"
    api.write_text("".join(json.dumps(r) + "\n" for r in api_index.enumerate_api(sys.executable)))
    digest = hashlib.sha256(b"".join((HERE / name).read_bytes() for name in HARNESS) + api.read_bytes()).hexdigest()

    programs: list[driver.Program] = []
    if args.suite in ("gallery", "all"):
        if not setup.gallery.is_dir():
            parser.error(f"no gallery at {setup.gallery} (MPL_SDIST names matplotlib's sdist)")
        programs += driver.gallery_programs(setup.gallery)
    if args.suite in ("generated", "all"):
        programs += driver.generated_programs(setup)
    if args.patterns:
        programs = [p for p in programs if any(pattern in p.name for pattern in args.patterns)]

    runs, failed = [], []
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        futures = {pool.submit(_reach, setup, api, program, digest): program for program in programs}
        for done, future in enumerate(concurrent.futures.as_completed(futures), 1):
            program, run = futures[future], future.result()
            if run is None:
                failed.append(program.name)
            else:
                runs.append(run)
            print(f"[{done}/{len(programs)}] {program.name}" + (": failed" if run is None else ""),
                  file=sys.stderr, flush=True)

    os.environ.setdefault("MPLBACKEND", "agg")
    coverage = Coverage(api_index.load(api), runs)
    report = coverage.report(len(programs), sorted(failed))
    (setup.out / "coverage.md").write_text(report)
    (setup.out / "coverage.json").write_text(json.dumps(coverage.as_json(), indent=1))
    print("\n".join(report.splitlines()[:17]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
