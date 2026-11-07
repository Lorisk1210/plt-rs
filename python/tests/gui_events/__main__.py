"""
Runs the harness's scenarios and prints what differs::

    python -m gui_events [--record | --golden] [--display :N ...] [SCENARIO...]

By default each scenario runs under matplotlib (QtAgg, or the backend the
scenario names) and under pltrs, at once, on two private X servers, and
their records and snapshots are compared. ``--record`` runs matplotlib
alone and keeps its runs as the goldens; ``--golden`` runs pltrs alone and
compares it with them, as the tests do. ``--display`` runs on the X
servers given instead (one or two). ``--wayland`` runs the programs on
Wayland, on a weston nested in each X server (Qt's Wayland platform for
matplotlib), comparing them with each other.

Run it with an interpreter that has Pillow (either venv); the programs
run in the venvs of their library.
"""

import argparse
import contextlib
import pathlib
import sys

from . import goldens
from .driver import REPO, differences_of, run, run_both
from .errors import ProgramFailed
from .scenarios import SCENARIOS
from .xdisplay import Unavailable, private_display


def _displays(given, stack):
    """The X servers to use: *given*, else two private ones."""
    if given:
        return given
    return [stack.enter_context(private_display()) for _ in range(2)]


def _one(name, args, displays):
    """Run scenario *name* as *args* say; the lines of what is wrong."""
    workdir = pathlib.Path(args.workdir)
    scenario = SCENARIOS[name]
    if args.record:
        if scenario.reference is None:
            return []
        goldens.record(name, run(name, "matplotlib", scenario.reference,
                                 displays[0], workdir))
        return []
    wayland = args.wayland
    if wayland and scenario.x11_only:
        return []
    if args.golden:
        got = run(name, "pltrs", "pltrs", displays[-1], workdir, wayland)
        return differences_of(name, goldens.load(name), got, workdir / name,
                              wayland=wayland)
    ref, got = run_both(name, displays, workdir, args.reference, wayland)
    return differences_of(name, ref, got, workdir / name, wayland=wayland)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="python -m gui_events")
    parser.add_argument("scenarios", nargs="*")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--record", action="store_true",
                      help="record matplotlib's runs as the goldens")
    mode.add_argument("--golden", action="store_true",
                      help="compare pltrs with the goldens")
    parser.add_argument("--display", action="append",
                        help="an X server to use (default: private ones)")
    parser.add_argument("--reference",
                        help="matplotlib's backend (default: the scenario's)")
    parser.add_argument("--wayland", action="store_true",
                        help="run on weston nested in the X servers")
    parser.add_argument("--workdir", default=str(REPO / "target/gui-events"))
    args = parser.parse_args(argv)
    failures = 0
    with contextlib.ExitStack() as stack:
        displays = _displays(args.display, stack)
        for name in args.scenarios or list(SCENARIOS):
            try:
                lines = _one(name, args, displays)
            except (ProgramFailed, Unavailable) as error:
                lines = [f"FAILED: {error}"]
            print(f"{name}: {'DIFFERENT' if lines else 'ok'}")
            for line in lines:
                print("   ", line)
            failures += bool(lines)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
