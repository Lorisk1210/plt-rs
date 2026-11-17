"""``python -m mutability``: runs the sweep and prints where pltrs parts
from matplotlib.

    PYTHONPATH=python:python/tests .venv-pltrs/bin/python -m mutability [--jobs J] [PATTERN...]
    PYTHONPATH=python/tests/leaks/alias:python/tests .venv/bin/python -m mutability --record

Runs every row whose name contains a PATTERN (all without one) on the
library ``pltrs`` names, writes the outcomes to ``target/mutability``
(``outcomes.json``, and ``report.md``: the rows that differ from
matplotlib's recorded outcomes, by kind) and prints the counts.
``--record``, under matplotlib through the leak harness's alias, finds
the surface's rows and records every outcome in ``reference.json``.
``--expect`` writes the rows that differ now, with the keys they differ
on, to ``expected.json``: what ``python/tests/test_mut1.py`` holds pltrs
to until a row is done.
"""

import argparse
import json
import pathlib
import sys

import pltrs

from . import runner, surface, threed

#: Where the sweep writes its reports.
WORK = pathlib.Path(__file__).resolve().parents[3] / "target" / "mutability"


def _record(jobs):
    """Record matplotlib's surface and outcomes."""
    if pltrs.__name__ != "matplotlib":
        sys.exit("--record runs under matplotlib (see the module's docstring)")
    pltrs.use("agg")
    reference = {"matplotlib": pltrs.__version__, "surface": surface.discover(),
                 "art3d": threed.discover()}
    reference["outcomes"] = runner.outcomes(runner.found_in(reference), jobs=jobs)
    runner.RECORDED.write_text(json.dumps(reference, indent=1, sort_keys=True) + "\n", "utf-8")
    print(f"recorded {len(reference['outcomes'])} rows in {runner.RECORDED}")


def report(differences, total):
    """The rows that differ, by kind, as Markdown."""
    count = sum(map(len, differences.values()))
    lines = [f"# Changes in place: {count} of {total} rows differ from matplotlib", ""]
    for kind, _ in runner.KINDS:
        rows = differences.get(kind, {})
        if rows:
            lines += [f"## {kind}: {len(rows)}", ""]
            lines += [f"* `{name}`: {', '.join(keys)}" for name, keys in sorted(rows.items())]
            lines.append("")
    return "\n".join(lines)


def main():
    """Runs the sweep as the command line says."""
    parser = argparse.ArgumentParser(prog="python -m mutability", description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--expect", action="store_true")
    parser.add_argument("--work", type=pathlib.Path, default=WORK)
    parser.add_argument("patterns", nargs="*")
    arguments = parser.parse_args()
    if arguments.record:
        _record(arguments.jobs)
        return 0
    reference = runner.recorded()
    ours = runner.outcomes(runner.found_in(reference), arguments.patterns, arguments.jobs)
    differences = runner.differences(ours, reference["outcomes"])
    work = arguments.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    (work / "outcomes.json").write_text(json.dumps(ours, indent=1, sort_keys=True), "utf-8")
    (work / "report.md").write_text(report(differences, len(ours)), "utf-8")
    if arguments.expect:
        if arguments.patterns:
            sys.exit("--expect takes every row: no PATTERN")
        known = runner.differing(ours, reference["outcomes"])
        runner.EXPECTED.write_text(json.dumps(known, indent=1, sort_keys=True) + "\n", "utf-8")
        print(f"expecting {len(known)} rows to differ in {runner.EXPECTED}")
    for kind, _ in runner.KINDS:
        if kind in differences:
            print(f"{kind}: {len(differences[kind])}")
    print(f"{sum(map(len, differences.values()))} of {len(ours)} rows differ; see {work / 'report.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
