"""``python -m number_types``: runs the sweep and prints where pltrs parts
from matplotlib.

    PYTHONPATH=python:python/tests .venv-pltrs/bin/python -m number_types [SCENE...]
    PYTHONPATH=python/tests/leaks/alias:python/tests .venv/bin/python -m number_types --record

Runs the scenes named (all without one; ``module.scene``, as
``runner.SCENES`` names them) on the library ``pltrs`` names, writes the
rows and ``report.md`` (the rows that differ from matplotlib's recorded
ones, by kind) to ``target/number_types`` and prints the counts.
``--record``, under matplotlib through the leak harness's alias, records
every row in ``reference.json``; ``--expect`` writes the rows that differ
now to ``expected.json``, each with its reason (``runner.REASONS``, which
must explain every one).
"""

import argparse
import json
import pathlib
import sys

import pltrs

from . import runner

#: Where the sweep writes its reports.
WORK = pathlib.Path(__file__).resolve().parents[3] / "target" / "number_types"


def report(differences, total):
    """The rows that differ, by kind, as Markdown."""
    kinds = runner.by_kind(differences)
    lines = [f"# Number types: {len(differences)} of {total} rows differ from matplotlib", "",
             "| kind | rows |", "|---|---|"]
    lines += [f"| {kind} | {len(rows)} |"
              for kind, rows in sorted(kinds.items(), key=lambda item: -len(item[1]))]
    lines += ["", "| row | matplotlib | pltrs | kind |", "|---|---|---|---|"]
    lines += [f"| {row} | {theirs} | {ours} | {runner.kind_of(theirs, ours)} |"
              for row, (theirs, ours) in sorted(differences.items())]
    return "\n".join(lines) + "\n"


def main():
    """Runs the sweep as the command line says."""
    parser = argparse.ArgumentParser(prog="python -m number_types", description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--expect", action="store_true")
    parser.add_argument("--work", type=pathlib.Path, default=WORK)
    parser.add_argument("scenes", nargs="*")
    arguments = parser.parse_args()
    pltrs.use("agg")
    ours = runner.run(arguments.scenes)
    if arguments.record:
        if pltrs.__name__ != "matplotlib" or arguments.scenes:
            sys.exit("--record runs every scene under matplotlib (see the module's docstring)")
        runner.RECORDED.write_text(json.dumps(ours, indent=0, sort_keys=True) + "\n", "utf-8")
        print(f"recorded {len(ours)} rows in {runner.RECORDED}")
        return 0
    theirs = runner.recorded()
    if arguments.scenes:
        theirs = {row: value for row, value in theirs.items() if row in ours}
    differences = runner.differing(ours, theirs)
    work = arguments.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    (work / "rows.json").write_text(json.dumps(ours, indent=0, sort_keys=True), "utf-8")
    (work / "report.md").write_text(report(differences, len(theirs)), "utf-8")
    if arguments.expect:
        if arguments.scenes:
            sys.exit("--expect takes every scene")
        unexplained = [row for row in differences if runner.reason_for(row) is None]
        if unexplained:
            sys.exit("rows differing for no reason of runner.REASONS: " + "; ".join(unexplained))
        expected = {row: {"matplotlib": theirs, "pltrs": ours, "reason": runner.reason_for(row)}
                    for row, (theirs, ours) in differences.items()}
        runner.EXPECTED.write_text(json.dumps(expected, indent=1, sort_keys=True) + "\n", "utf-8")
        print(f"expecting {len(expected)} rows to differ in {runner.EXPECTED}")
    for kind, rows in sorted(runner.by_kind(differences).items(), key=lambda item: -len(item[1])):
        print(f"{kind}: {len(rows)}")
    print(f"{len(differences)} of {len(theirs)} rows differ; see {work / 'report.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
