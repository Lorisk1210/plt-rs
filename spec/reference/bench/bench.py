"""The Python side of ``cargo xtask bench``: serves the scripts of
``benchmarks/python`` to the harness in matplotlib or pltrs (whichever the
interpreter and the scripts' directory import), and times fresh processes.

    bench.py serve SCRIPT [--format FMT | --layout] [--option KEY=VALUE]...
    bench.py cold SPEC
    bench.py versions

``serve`` imports the script (its imports and its data), then answers
requests read from standard input, one per line, with one line each on
standard output:

* ``run`` makes a timed run: the figure is drawn and saved to memory in
  FMT with the ``savefig`` options given (or only laid out, ``--layout``)
  while the clock runs. Before the clock starts, the caches of laid-out
  text that outlive a figure are dropped (``drop_text_caches``) and the
  garbage collector runs; after it stops, the figure is closed. The reply
  is ``time``, the seconds and the size of the document in bytes.
* ``save FMT PATH`` draws the figure and saves it to PATH in FMT, with the
  same options (``html`` saves an animation's player page); the reply is
  ``ok``.
* ``quit`` ends the process.

A request the script cannot fulfil (pltrs lacking a feature) is answered
with ``unsupported`` and the exception; loading the script is answered
with ``ready`` or that. Fields are tab-separated.

``cold`` runs commands in fresh processes. SPEC is JSON: ``{"min_runs":
N, "min_seconds": S, "max_runs": N, "commands": [{"label": L, "argv":
[...], "env": {...}, "stderr": PATH}]}``. The commands run in rounds, each
once per round in an order rotated from round to round, until every
command has run ``min_runs`` times and the slowest for ``min_seconds`` (or
the rounds reach ``max_runs``), so that every command runs equally often and the
runs of a round can be paired. A run is timed from ``posix_spawn`` to the
child's exit, and ``wait4`` reports its peak resident memory. One line per
run: round, label, seconds, peak memory in KiB and exit code.

``versions`` prints what the column runs on: the Python, numpy and Pillow
versions (and matplotlib's where installed), the interpreter's real path,
a digest of numpy's compiled core (so that two venvs can be shown to run
the same numpy), the zlib versions that compress PNG and PDF streams, and
today's date.
"""

import argparse
import datetime
import gc
import hashlib
import importlib
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


def failure(err):
    """The reply to a request that raised ``err``."""
    message = " ".join(f"{type(err).__name__}: {err}".split())
    return f"unsupported\t{message}"


def text_cache_droppers(plt):
    """What drops the caches of laid-out text that outlive a figure in the
    library ``plt`` belongs to.

    matplotlib keeps one: the parsed formulas of mathtext
    (``MathTextParser._parse_cached``, the last 50). Its text metrics live
    with each renderer, so with each figure, and its fonts stay loaded.
    pltrs (Python) has the same mathtext cache and, below it, the text
    engine of pltrs (Rust), which keeps every line and formula it laid out
    and every glyph outline it made; ``_native.text_engine_clear_caches``
    drops those (fonts stay loaded there too).
    """
    package = importlib.import_module(plt.__name__.split(".")[0])
    mathtext = importlib.import_module(f"{package.__name__}.mathtext")
    droppers = [mathtext.MathTextParser._parse_cached.cache_clear]
    native = getattr(package, "_native", None)
    if native is not None:
        droppers.append(native.text_overrides.text_engine_clear_caches)
    return droppers


class Server:
    """A script loaded for timed runs and saves."""

    def __init__(self, args):
        self.module = load(pathlib.Path(args.script))
        from _common import finish

        self.finish = finish
        self.layout = args.layout
        self.options = savefig_options(args.option)
        if args.format is not None:
            self.options["format"] = args.format
        self.drop_caches = text_cache_droppers(self.module.plt)

    def run(self):
        """A timed run: the seconds and the size of the document."""
        for drop in self.drop_caches:
            drop()
        gc.collect()
        target = None if self.layout else io.BytesIO()
        start = time.perf_counter()
        self.finish(self.module.draw(), target, self.options)
        seconds = time.perf_counter() - start
        self.module.plt.close("all")
        size = 0 if target is None else target.getbuffer().nbytes
        return f"time\t{seconds:.9f}\t{size}"

    def save(self, fmt, path):
        """Saves the figure to ``path`` in ``fmt``."""
        self.finish(self.module.draw(), path, {**self.options, "format": fmt})
        self.module.plt.close("all")
        return "ok"


def serve(args):
    """The ``serve`` subcommand."""
    try:
        server = Server(args)
    except Exception as err:  # noqa: BLE001  (any failure means unsupported)
        print(failure(err), flush=True)
        return
    print("ready", flush=True)
    for line in sys.stdin:
        request = line.rstrip("\n").split("\t")
        if request == ["quit"]:
            return
        try:
            if request == ["run"]:
                reply = server.run()
            elif len(request) == 3 and request[0] == "save":
                reply = server.save(request[1], request[2])
            else:
                reply = f"error\tunknown request {line.strip()!r}"
        except Exception as err:  # noqa: BLE001
            server.module.plt.close("all")
            reply = failure(err)
        print(reply, flush=True)


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
    spent = [0.0] * len(commands)
    rounds = 0

    def wanted():
        return rounds < spec["max_runs"] and (
            rounds < spec["min_runs"] or max(spent) < spec["min_seconds"])

    while wanted():
        shift = rounds % len(commands)
        for i in [*range(shift, len(commands)), *range(shift)]:
            seconds, rss, code = spawn(commands[i])
            spent[i] += seconds
            print(f"{rounds}\t{commands[i]['label']}\t{seconds:.9f}\t{rss}\t{code}",
                  flush=True)
        rounds += 1


def digest(path):
    """The first 16 hex digits of the SHA-256 of the file at ``path``."""
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()[:16]


def versions(_args):
    """The ``versions`` subcommand."""
    import zlib

    import numpy
    import PIL
    from PIL import features

    core = importlib.import_module("numpy._core._multiarray_umath")
    print(f"python\t{platform.python_version()}")
    print(f"interpreter\t{os.path.realpath(sys._base_executable)}")
    print(f"numpy\t{numpy.__version__}")
    print(f"numpy core\t{digest(core.__file__)}")
    print(f"pillow\t{PIL.__version__}")
    print(f"pillow zlib\t{features.version('zlib')}")
    if "zlib_ng" in features.features:
        print(f"pillow zlib-ng\t{features.version('zlib_ng')}")
    print(f"python zlib\t{zlib.ZLIB_RUNTIME_VERSION}")
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

    served = commands.add_parser("serve", help="serve a script's timed runs")
    served.add_argument("script")
    output = served.add_mutually_exclusive_group(required=True)
    output.add_argument("--format", help="the format saved to memory")
    output.add_argument("--layout", action="store_true", help="lay out without rendering")
    served.add_argument("--option", action="append", default=[], help="a savefig KEY=VALUE")
    served.set_defaults(run=serve)

    spawned = commands.add_parser("cold", help="time commands in fresh processes")
    spawned.add_argument("spec", help="what to run, as JSON")
    spawned.set_defaults(run=cold)

    commands.add_parser("versions", help="print the versions").set_defaults(run=versions)

    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
