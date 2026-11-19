"""
The child process of ``compare.py``: runs one script, unchanged, under
matplotlib or under pltrs (Python), and reports when its figure was ready.

    python child.py run matplotlib|pltrs SCRIPT RESULT [--alias DIR] [--sources DIR]
    python child.py fonts RESULT [--alias DIR] [--sources DIR]

``run`` speaks this protocol with its parent:

1. It prepares the imports, with nothing but the standard library loaded:
   on matplotlib's side, ``pltrs`` and ``pltrs.*`` are matplotlib's
   modules (for scripts that import pltrs by name); on pltrs' side,
   pltrs' ``alias`` directory comes first on ``sys.path``, so that
   ``matplotlib``, ``mpl_toolkits`` and ``pylab`` are pltrs, and
   ``--sources`` (the checkout's ``python/``) next, when pltrs comes from
   the checkout. ``pyplot.show`` is replaced as pyplot loads (`_Ready`).
2. It prints ``READY`` and waits for the start stamp, ``time.time()`` of
   the parent, on one line of its standard input.
3. It runs the script as ``__main__``. Its first ``plt.show()`` draws
   every open figure and stamps the time: the figure is ready. Then,
   outside the span, it keeps a copy of each figure's picture and returns
   without blocking. A script that never calls ``show`` is stamped at its
   end, after its open figures are drawn.
4. It writes RESULT, a JSON object: the library, its version and where it
   was loaded from, the seconds from the start stamp to the figure ready
   (``ready``) and to the script's end (``end``), the peak resident memory
   in bytes, the PNG files of the pictures and the traceback if the script
   raised.

``fonts`` writes the files of the fonts pltrs ships for the clip's text,
as pltrs' font manager finds them.
"""

import argparse
import importlib
import importlib.abc
import importlib.util
import json
import pathlib
import runpy
import sys
import time
import traceback

#: The modules whose ``show`` is the moment a figure is ready.
PYPLOTS = ("matplotlib.pyplot", "pltrs.pyplot")
#: The files a script may save that the clip can show, as its picture
#: when it leaves no figure open.
IMAGE_SUFFIXES = (".png", ".jpg", ".jpeg", ".webp", ".gif", ".tif", ".tiff", ".bmp")


class PltrsIsMatplotlib(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    """matplotlib's side: finds ``pltrs`` and ``pltrs.*`` and loads
    matplotlib's modules for them (``pltrs.mpl_toolkits.*`` as
    ``mpl_toolkits.*``), so that a script importing pltrs by name runs on
    matplotlib."""

    def find_spec(self, name, path=None, target=None):
        if name == "pltrs" or name.startswith("pltrs."):
            return importlib.util.spec_from_loader(name, self)
        return None

    def create_module(self, spec):
        name = spec.name.removeprefix("pltrs")
        if name.startswith(".mpl_toolkits"):
            return importlib.import_module(name.removeprefix("."))
        return importlib.import_module("matplotlib" + name)

    def exec_module(self, module):
        """The module ran when matplotlib imported it."""


class _ShowHook(importlib.abc.MetaPathFinder):
    """Replaces ``show`` in pyplot, under either name, right after the
    module runs and before the script sees it."""

    def __init__(self, ready):
        self.ready = ready

    def find_spec(self, name, path=None, target=None):
        if name not in PYPLOTS:
            return None
        spec = self._spec_from_others(name, path, target)
        if spec is not None and spec.loader is not None:
            spec.loader = _ShowReplacingLoader(spec.loader, self.ready)
        return spec

    def _spec_from_others(self, name, path, target):
        """The spec the finders after this one give for *name*."""
        finders = sys.meta_path
        for finder in finders[finders.index(self) + 1:]:
            find = getattr(finder, "find_spec", None)
            spec = find(name, path, target) if find else None
            if spec is not None:
                return spec
        return None


class _ShowReplacingLoader(importlib.abc.Loader):
    """A pyplot's loader, which then replaces the module's ``show``; it
    answers for the loader it wraps in all else."""

    def __init__(self, loader, ready):
        self.loader = loader
        self.ready = ready

    def __getattr__(self, name):
        return getattr(self.loader, name)

    def create_module(self, spec):
        return self.loader.create_module(spec)

    def exec_module(self, module):
        self.loader.exec_module(module)
        module.show = self.ready.show_function()


class _Ready:
    """When the script's figure was ready, and its pictures."""

    def __init__(self, start):
        #: Seconds from the parent's start stamp (its ``time.time()``) to
        #: this moment, and this moment on the steady clock, on which
        #: every span is measured from here. The system's clock steps by
        #: 16 ms on Windows before Python 3.13: a reading of it and a span
        #: of the steady clock do not add up, and the span to the end came
        #: out before the figure was ready there.
        self._before = max(0.0, time.time() - start)
        self._since = time.perf_counter()
        #: Seconds from the start to the figure ready, once stamped.
        self.ready = None
        #: Seconds spent keeping pictures, left out of the span to the end.
        self.keeping = 0.0
        #: The pictures, ``(width, height, RGBA bytes)``, of the open
        #: figures when the figure was ready.
        self.pictures = []

    def elapsed(self):
        """Seconds since the parent's start stamp."""
        return self._before + time.perf_counter() - self._since

    def show_function(self):
        """pyplot's ``show`` in the child: the first call draws the open
        figures, stamps the time and keeps their pictures; it never
        blocks. A plain function, as pyplot gives its ``show`` the
        backend's signature."""
        def show(*args, **kwargs):
            if self.ready is None:
                self.stamp()

        return show

    def stamp(self):
        """Draws every open figure, stamps the time, then keeps the
        figures' pictures (outside the span)."""
        figures = _open_figures()
        for figure in figures:
            figure.canvas.draw()
        self.ready = self.elapsed()
        began = time.perf_counter()
        self.pictures = [_picture(figure) for figure in figures]
        self.keeping += time.perf_counter() - began


def _open_figures():
    """The figures pyplot holds open, in their order, or none if the
    script never imported pyplot."""
    for name in PYPLOTS:
        pyplot = sys.modules.get(name)
        if pyplot is not None:
            managers = pyplot._pylab_helpers.Gcf.get_all_fig_managers()
            return [manager.canvas.figure for manager in managers]
    return []


def _picture(figure):
    """*figure*'s drawn picture: its size and its RGBA pixels."""
    width, height = figure.canvas.get_width_height(physical=True)
    return width, height, bytes(figure.canvas.buffer_rgba())


def _run_script(script, ready):
    """Runs *script* as ``__main__`` and stamps the figure ready at its
    end if its ``show`` did not; returns the traceback if it raised."""
    sys.argv = [str(script)]
    try:
        try:
            runpy.run_path(str(script), run_name="__main__")
        except SystemExit as stop:
            if stop.code not in (None, 0):
                return f"SystemExit: {stop.code}\n"
        if ready.ready is None:
            ready.stamp()
    except Exception:  # noqa: BLE001  (any error of the script is its side's failure)
        return traceback.format_exc()
    return None


def _peak_memory():
    """The process's peak resident memory in bytes, where there is one."""
    try:
        import resource
    except ImportError:
        return None
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return peak if sys.platform == "darwin" else peak * 1024


def _save_pictures(pictures, directory):
    """Writes the kept pictures as PNG files; returns their paths."""
    from PIL import Image

    paths = []
    for index, (width, height, pixels) in enumerate(pictures):
        path = directory / f"figure-{index + 1}.png"
        Image.frombuffer("RGBA", (width, height), pixels, "raw", "RGBA", 0, 1).save(path)
        paths.append(str(path))
    return paths


def _last_saved_image(directory):
    """The image file the script saved last in *directory*, or None."""
    saved = [path for path in directory.iterdir() if path.suffix.lower() in IMAGE_SUFFIXES]
    return str(max(saved, key=lambda path: path.stat().st_mtime_ns)) if saved else None


def _library(name):
    """The version of the library the script got as matplotlib (*name*)
    and the directory it was loaded from, after the run."""
    module = sys.modules.get("matplotlib") or importlib.import_module(name)
    location = pathlib.Path(module.__file__).parent
    version = module.__version__ if name == "matplotlib" else _pltrs_version(location)
    return version, str(location)


def _wrong_library(name):
    """Why the library the script got is not *name*'s, or None: a
    matplotlib that pltrs' alias served on matplotlib's side (an alias on
    the user's ``PYTHONPATH``), or matplotlib's own on pltrs' side."""
    aliased = "_pltrs_matplotlib_alias" in sys.modules
    if name == "matplotlib" and aliased:
        return "matplotlib's side loaded pltrs under matplotlib's name; is pltrs' alias on PYTHONPATH?\n"
    if name == "pltrs" and "matplotlib" in sys.modules and not aliased:
        return "pltrs' side loaded matplotlib itself instead of pltrs under its name\n"
    return None


def _pltrs_version(location):
    """pltrs' own version: its checkout's (the workspace's ``Cargo.toml``)
    when the package at *location* is a checkout's ``python/pltrs``, else
    its installed distribution's."""
    cargo = location.parents[1] / "Cargo.toml"
    if cargo.is_file():
        import tomllib

        with open(cargo, "rb") as file:
            return tomllib.load(file)["workspace"]["package"]["version"]
    from importlib import metadata

    try:
        return metadata.version("pltrs")
    except metadata.PackageNotFoundError:
        return "unknown"


def _prepare_imports(library, script, alias, sources):
    """The import system of the side: the script's directory where Python
    puts it, and which modules ``matplotlib`` and ``pltrs`` name."""
    sys.path[0] = str(script.parent)
    if library == "matplotlib":
        sys.meta_path.insert(0, PltrsIsMatplotlib())
    else:
        front = [alias] + ([sources] if sources is not None else [])
        sys.path[:0] = front


def run(args):
    """``run``: the protocol of the module's docstring."""
    script = pathlib.Path(args.script).resolve()
    result = pathlib.Path(args.result)
    _prepare_imports(args.library, script, args.alias, args.sources)
    print("READY", flush=True)
    ready = _Ready(float(sys.stdin.readline()))
    sys.meta_path.insert(0, _ShowHook(ready))
    error = _run_script(script, ready) or _wrong_library(args.library)
    end = ready.elapsed() - ready.keeping
    report = {"ready": ready.ready, "end": end, "peak_memory": _peak_memory(), "error": error}
    pictures = _save_pictures(ready.pictures, result.parent)
    report["pictures"] = pictures or [path for path in [_last_saved_image(pathlib.Path.cwd())] if path]
    try:
        report["version"], report["location"] = _library(args.library)
    except Exception:  # noqa: BLE001  (a side that cannot load its library says so)
        report["version"], report["location"] = None, None
        report["error"] = report["error"] or traceback.format_exc()
    result.write_text(json.dumps(report), encoding="utf-8")


def fonts(args):
    """``fonts``: the regular and bold files of the font pltrs' modern
    look draws with, else of DejaVu Sans, as pltrs' font manager finds
    them."""
    sys.path[:0] = [args.alias] + ([args.sources] if args.sources is not None else [])
    from pltrs import font_manager

    found = {}
    for weight in ("regular", "bold"):
        for family in ("Inter", "DejaVu Sans"):
            properties = font_manager.FontProperties(family=family, weight=weight)
            try:
                found[weight] = font_manager.findfont(properties, fallback_to_default=False)
                break
            except ValueError:
                continue
    pathlib.Path(args.result).write_text(json.dumps(found), encoding="utf-8")


def _arguments():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[1])
    commands = parser.add_subparsers(dest="command", required=True)
    running = commands.add_parser("run")
    running.add_argument("library", choices=("matplotlib", "pltrs"))
    running.add_argument("script")
    running.add_argument("result")
    finding = commands.add_parser("fonts")
    finding.add_argument("result")
    for command in (running, finding):
        command.add_argument("--alias")
        command.add_argument("--sources")
    return parser.parse_args()


if __name__ == "__main__":
    arguments = _arguments()
    (run if arguments.command == "run" else fonts)(arguments)
