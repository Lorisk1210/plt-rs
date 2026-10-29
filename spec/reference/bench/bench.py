"""The Python side of ``cargo xtask bench``: times the scripts of
``benchmarks/python`` in matplotlib or pltrs (whichever the interpreter
and its scripts directory import), renders them for the pixel
comparison, and times fresh processes.

    bench.py warm SCRIPT [--format FMT | --layout] [--option KEY=VALUE]...
                         [--min-runs N] [--min-seconds S] [--max-runs N]
    bench.py png SCRIPT OUT [--option KEY=VALUE]...
    bench.py cold SPEC
    bench.py versions

``warm`` imports the script (its imports and data, untimed), draws and
saves its figure once to warm up, then times drawing, saving to memory and
closing the figure until it has run ``--min-runs`` times and for
``--min-seconds``, but at most ``--max-runs`` times; the garbage collector
runs before each run, outside the clock. It prints ``times`` followed by
the seconds of each run, or ``unsupported`` followed by the exception the
script raised (pltrs lacking a feature), tab-separated.

``png`` saves the script's figure as a PNG file for the pixel comparison,
with the same ``savefig`` options; it prints ``ok`` or ``unsupported``.

``cold`` runs commands in fresh processes. SPEC is JSON: ``{"min_runs":
N, "min_seconds": S, "max_runs": N, "commands": [{"label": L, "argv":
[...], "env": {...}, "stderr": PATH}]}``. The commands take turns, round
after round, until each has run as the counts ask; each run is timed from
``posix_spawn`` to the child's exit, and ``wait4`` reports its peak
resident memory. One line per run: label, seconds, peak memory in KiB and
exit code, tab-separated.

``versions`` prints the interpreter's Python, numpy and (where installed)
matplotlib versions, and today's date.
"""

import argparse
import datetime
import gc
import importlib.util
import io
import json
import os
import pathlib
import platform
import sys
import time


def load(script):
    """Import ``script`` as a module, with its directory (where
    ``_common`` lives) first on the path."""
    sys.path.insert(0, str(script.parent))
    spec = importlib.util.spec_from_file_location(f"bench_{script.stem}", script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def savefig_options(pairs):
    """``KEY=VALUE`` pairs as ``savefig`` keywords."""
    return dict(pair.split("=", 1) for pair in pairs)


def attempt(action):
    """Run ``action``; print ``unsupported`` and the exception, and return
    False, if it raises."""
    try:
        action()
    except Exception as err:  # noqa: BLE001  (any failure means unsupported)
        message = " ".join(f"{type(err).__name__}: {err}".split())
        print(f"unsupported\t{message}", flush=True)
        return False
    return True


def warm(args):
    """The ``warm`` subcommand."""
    script = pathlib.Path(args.script)
    state = {}

    def prepare():
        state["module"] = load(script)
        from _common import finish

        state["finish"] = finish

    if not attempt(prepare):
        return
    module, finish = state["module"], state["finish"]
    options = savefig_options(args.option)
    if args.format is not None:
        options["format"] = args.format

    def once():
        fig = module.draw()
        finish(fig, None if args.layout else io.BytesIO(), options)
        module.plt.close(fig)

    if not attempt(once):
        return
    times = []
    while len(times) < args.max_runs and (
            len(times) < args.min_runs or sum(times) < args.min_seconds):
        gc.collect()
        start = time.perf_counter()
        once()
        times.append(time.perf_counter() - start)
    print("times\t" + "\t".join(f"{t:.9f}" for t in times), flush=True)


def png(args):
    """The ``png`` subcommand."""
    script = pathlib.Path(args.script)

    def render():
        module = load(script)
        from _common import finish

        finish(module.draw(), args.out, {**savefig_options(args.option), "format": "png"})

    if attempt(render):
        print("ok", flush=True)


def spawn(command):
    """Run ``command`` once in a fresh process: its seconds from spawn to
    exit, its peak resident memory in KiB and its exit code."""
    argv = command["argv"]
    env = {**os.environ, **command.get("env", {})}
    actions = [
        (os.POSIX_SPAWN_OPEN, 0, os.devnull, os.O_RDONLY, 0),
        (os.POSIX_SPAWN_OPEN, 1, os.devnull, os.O_WRONLY, 0),
        (os.POSIX_SPAWN_OPEN, 2, command.get("stderr", os.devnull),
         os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644),
    ]
    start = time.perf_counter()
    pid = os.posix_spawn(argv[0], argv, env, file_actions=actions)
    _, status, usage = os.wait4(pid, 0)
    seconds = time.perf_counter() - start
    return seconds, usage.ru_maxrss, os.waitstatus_to_exitcode(status)


def cold(args):
    """The ``cold`` subcommand."""
    spec = json.loads(args.spec)
    commands = spec["commands"]
    runs = [0] * len(commands)
    spent = [0.0] * len(commands)

    def wanted(i):
        return runs[i] < spec["max_runs"] and (
            runs[i] < spec["min_runs"] or spent[i] < spec["min_seconds"])

    while pending := [i for i in range(len(commands)) if wanted(i)]:
        for i in pending:
            seconds, rss, code = spawn(commands[i])
            runs[i] += 1
            spent[i] += seconds
            print(f"{commands[i]['label']}\t{seconds:.9f}\t{rss}\t{code}", flush=True)


def versions(_args):
    """The ``versions`` subcommand."""
    import numpy

    print(f"python\t{platform.python_version()}")
    print(f"numpy\t{numpy.__version__}")
    try:
        import matplotlib
    except ImportError:
        pass
    else:
        print(f"matplotlib\t{matplotlib.__version__}")
    print(f"date\t{datetime.date.today().isoformat()}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(required=True)

    timed = commands.add_parser("warm", help="time a script in this process")
    timed.add_argument("script")
    output = timed.add_mutually_exclusive_group(required=True)
    output.add_argument("--format", help="the format saved to memory")
    output.add_argument("--layout", action="store_true", help="lay out without drawing")
    timed.add_argument("--option", action="append", default=[], help="a savefig KEY=VALUE")
    timed.add_argument("--min-runs", type=int, default=5)
    timed.add_argument("--min-seconds", type=float, default=1.0)
    timed.add_argument("--max-runs", type=int, default=50)
    timed.set_defaults(run=warm)

    rendered = commands.add_parser("png", help="save a script's figure as PNG")
    rendered.add_argument("script")
    rendered.add_argument("out")
    rendered.add_argument("--option", action="append", default=[], help="a savefig KEY=VALUE")
    rendered.set_defaults(run=png)

    spawned = commands.add_parser("cold", help="time commands in fresh processes")
    spawned.add_argument("spec", help="what to run, as JSON")
    spawned.set_defaults(run=cold)

    commands.add_parser("versions", help="print the versions").set_defaults(run=versions)

    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
