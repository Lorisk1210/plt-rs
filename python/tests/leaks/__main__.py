"""``python -m leaks``: runs the leak harness and prints the leaks.

    PYTHONPATH=python:python/tests .venv-pltrs/bin/python -m leaks [--iterations N]
        [--jobs J] [--short] [--record] [--serve N] [PATTERN...]

Runs every scene whose name contains a PATTERN (all without one) under
pltrs and under matplotlib (the reference venv, else the outcomes
``reference.json`` recorded), writes ``report.md`` and ``report.json``
into ``target/leaks``, and prints the scenes that leak. ``--short`` runs
the smoke tests' mode (the harness's own scenes, no growth loop for
dropped scenes, `runner.SHORT`), ``--record`` records matplotlib's
outcomes of those into ``reference.json``, ``--serve N`` runs the
long-running process instead (`serve`) under both.
"""

import argparse
import json
import pathlib
import sys

from . import runner, serve

#: Where the harness works and writes its reports.
WORK = runner.ROOT / "target" / "leaks"


def main():
    """Runs the harness as the command line says."""
    parser = argparse.ArgumentParser(prog="python -m leaks", description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--iterations", type=int, default=200)
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--short", action="store_true")
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--serve", type=int, default=0, metavar="N")
    parser.add_argument("--work", type=pathlib.Path, default=WORK)
    parser.add_argument("patterns", nargs="*")
    arguments = parser.parse_args()
    work = arguments.work.resolve()
    if arguments.serve:
        print(serve.compare(arguments.serve, work / "serve"))
        return 0
    names = runner.selected(arguments.patterns, own=arguments.short or arguments.record)
    if arguments.record:
        failed = runner.record(names, work, arguments.jobs)
        for name in failed:
            print(f"{name}: failed under matplotlib")
        print(f"recorded {len(names)} scenes in {runner.RECORDED}")
        return 1 if failed else 0
    settings = (runner.SHORT if arguments.short
                else {"iterations": arguments.iterations, "batch": 1})
    verdicts, pltrs, matplotlib = runner.run(names, settings, work, arguments.jobs)
    work.mkdir(parents=True, exist_ok=True)
    (work / "report.md").write_text(runner.report(verdicts, pltrs, matplotlib), "utf-8")
    (work / "report.json").write_text(json.dumps({"pltrs": pltrs, "matplotlib": matplotlib},
                                                 indent=1), "utf-8")
    bad = [verdict for verdict in verdicts if verdict.leaks or verdict.error]
    unexpected = [verdict for verdict in verdicts if verdict.unexpected]
    for verdict in bad:
        print(runner.summary(verdict))
    print(f"{len(verdicts)} scenes, {sum(v.leaks for v in verdicts)} leaking, "
          f"{sum(bool(v.error) for v in verdicts)} failing, {len(unexpected)} of them for no "
          f"known cause; report in {work / 'report.md'}")
    return 1 if unexpected else 0


sys.exit(main())
