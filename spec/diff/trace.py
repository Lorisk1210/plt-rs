"""Runs one program and records what it computed, for the differential tester.

    python trace.py --impl matplotlib|pltrs --mode program|steps|cells
                    [--in-tree TREE] PROGRAM OUT_DIR

The same program runs under matplotlib (the reference venv) and under
pltrs (Python) (the pltrs venv, ``python/`` on ``PYTHONPATH``); ``compare.py``
then compares the two records. Under pltrs the program's imports are
changed and nothing else: ``import matplotlib.pyplot as plt`` becomes
``import pltrs.pyplot as plt``, ``mpl_toolkits`` becomes
``pltrs.mpl_toolkits``.

The program is instrumented before it runs (``ast``): the value of every
expression statement, assignment and ``return`` is recorded, keyed by its
line and column and the how-many-th time it ran, as ``describe.py``
describes it. Warnings (``warnings`` and the ``logging`` warnings of the
packages) are recorded with the top-level statement that was running and
the line of the program they point at. Then every pyplot figure still open
is saved as PNG into OUT_DIR.

* ``program`` mode runs the program as a whole, as a gallery example runs:
  an exception stops it and is recorded with its line.
* ``steps`` mode runs each top-level statement on its own: an exception is
  recorded for its statement and the next statement runs, so that one
  generated program can try many calls.
* ``cells`` mode runs a script of cells (a notebook's or a Markdown page's,
  ``# %%`` lines starting each) as Jupyter runs them with the inline
  backend: an exception is recorded and stops its cell, and at the end of
  each cell the figures open are saved (with ``bbox_inches="tight"``, as
  the inline backend shows them) and closed. Jupyter's ``display`` prints
  what it is given (a figure apart, which the inline backend shows).

The program runs in ``OUT_DIR/cwd``; with ``--in-tree`` it is a file of
TREE (a corpus' tree laid out for the run, ``corpora.py``) and runs in its
own directory, paths in TREE reading as ``<cwd>`` in messages, and the
files it writes there are listed (``written``: their paths in TREE).

The record goes to ``OUT_DIR/record.json``. Its own failures (not the
program's) exit with status 2.
"""

from __future__ import annotations

import argparse
import ast
from collections import Counter
import importlib.abc
import importlib.machinery
import importlib.util
import io
import json
import logging
import os
import pathlib
import sys
import sysconfig
import time
import traceback
import types
import warnings
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from corpora import unchanged_copies  # noqa: E402
from describe import Describer, Names, public_mro  # noqa: E402
import ipython_shell  # noqa: E402

#: Recorded occurrences of one site; later ones are only counted.
MAX_OCCURRENCES = 30
#: Recorded warnings of one run; later ones are only counted, by message
#: (for the first ``MAX_WARNINGS`` messages), so that the counts compared
#: are those of the whole run whatever order the warnings came in.
MAX_WARNINGS = 500

#: The module renames of the import change, per implementation: none.
#: pltrs runs as its users have it, the ``matplotlib`` distribution: the
#: program and the libraries it plots through import ``matplotlib``, which
#: the alias loads as pltrs under matplotlib's names, before anything
#: imports ``pltrs`` by name, so that its classes' modules are matplotlib's
#: (arviz-plots picks its plotting backend by ``type(obj).__module__``).
#: A corpus' pltrs side has both distributions installed in its venv's
#: ``site-packages`` (``install.py``).
IMPORT_RENAMES = {
    "matplotlib": [],
    "pltrs": [],
}
#: How each implementation's names read as matplotlib's.
NAME_PREFIXES = {
    "matplotlib": [],
    "pltrs": [("pltrs.mpl_toolkits", "mpl_toolkits"), ("pltrs.pylab", "pylab"), ("pltrs", "matplotlib")],
}

REC, EXC, SITE = "__difftest_record__", "__difftest_exception__", "__difftest_site__"
CELL_END = "__difftest_cell_end__"
CALENDAR = "__difftest_calendar__"
#: The line starting a cell of a script in the percent format.
CELL_MARK = "# %%"
#: The moment (seconds since the epoch) the calendar reads of `time` and
#: of the program's date classes find (`_fix_the_calendar`,
#: `CalendarReads`).
CALENDAR_NOW = 1_700_000_000
#: The methods reading the calendar of the date classes (`datetime.date`,
#: `datetime.datetime`, pandas' ``Timestamp``), by what each reads it with.
CALENDAR_READS = {"now": "fromtimestamp", "today": "fromtimestamp", "utcnow": "utcfromtimestamp"}


def _renamed(name: str, renames: list[tuple[str, str]]) -> str:
    for old, new in renames:
        if name == old or name.startswith(old + "."):
            return new + name[len(old):]
    return name


class ImportChange(ast.NodeTransformer):
    """Changes the program's imports of matplotlib to pltrs's."""

    def __init__(self, renames: list[tuple[str, str]]) -> None:
        self.renames = renames

    def visit_Import(self, node: ast.Import) -> Any:
        """``import matplotlib...`` as the implementation's modules."""
        statements = []
        for alias in node.names:
            renamed = _renamed(alias.name, self.renames)
            if renamed == alias.name:
                statements.append(ast.Import(names=[alias]))
            elif alias.asname:
                statements.append(ast.Import(names=[ast.alias(renamed, alias.asname)]))
            else:
                # `import matplotlib.pyplot` binds `matplotlib`: import the
                # renamed module, then bind its top-level name.
                top = alias.name.split(".")[0]
                statements.append(ast.Import(names=[ast.alias(renamed, None)]))
                statements.append(ast.Import(names=[ast.alias(_renamed(top, self.renames), top)]))
        for statement in statements:
            ast.copy_location(statement, node)
        return statements

    def visit_ImportFrom(self, node: ast.ImportFrom) -> Any:
        """``from matplotlib... import ...`` from the implementation's modules."""
        if node.level == 0 and node.module:
            node.module = _renamed(node.module, self.renames)
        return node


def _call(name: str, *args: ast.expr) -> ast.Call:
    return ast.Call(func=ast.Name(id=name, ctx=ast.Load()), args=list(args), keywords=[])


def _site(node: ast.AST) -> ast.Constant:
    return ast.Constant(f"{node.lineno}:{node.col_offset}")


class Recording(ast.NodeTransformer):
    """Wraps the values of expression statements, assignments and returns
    in a call recording them."""

    def _wrap(self, node: ast.AST, value: ast.expr) -> ast.expr:
        return ast.copy_location(_call(REC, _site(node), value), value)

    def visit_Expr(self, node: ast.Expr) -> Any:
        """Record the value of an expression statement."""
        self.generic_visit(node)
        if isinstance(node.value, ast.Constant) or isinstance(node.value, (ast.Await, ast.Yield, ast.YieldFrom)):
            return node
        node.value = self._wrap(node, node.value)
        return node

    def visit_Assign(self, node: ast.Assign) -> Any:
        """Record the value assigned."""
        self.generic_visit(node)
        node.value = self._wrap(node, node.value)
        return node

    def visit_AnnAssign(self, node: ast.AnnAssign) -> Any:
        """Record the value assigned."""
        self.generic_visit(node)
        if node.value is not None:
            node.value = self._wrap(node, node.value)
        return node

    def visit_Return(self, node: ast.Return) -> Any:
        """Record the value returned."""
        self.generic_visit(node)
        if node.value is not None:
            node.value = self._wrap(node, node.value)
        return node


class CalendarReads(ast.NodeTransformer):
    """Has the program's calls of the calendar reads of a class
    (``datetime.now()``, ``date.today()``, ``pd.Timestamp.now(tz)``) go
    through `_read_calendar`, which C code's clock leaves no other way to
    fix."""

    def visit_Call(self, node: ast.Call) -> Any:
        self.generic_visit(node)
        if isinstance(node.func, ast.Attribute) and node.func.attr in CALENDAR_READS:
            call = _call(CALENDAR, node.func.value, ast.Constant(node.func.attr), *node.args)
            call.keywords = node.keywords
            return ast.copy_location(call, node)
        return node


def _read_calendar(owner: Any, name: str, *args: Any, **kwargs: Any) -> Any:
    """``owner.name(*args, **kwargs)``, at `CALENDAR_NOW` for a date class
    reading the calendar (`CALENDAR_READS`)."""
    import datetime

    if not (isinstance(owner, type) and issubclass(owner, datetime.date)):
        return getattr(owner, name)(*args, **kwargs)
    return getattr(owner, CALENDAR_READS[name])(CALENDAR_NOW, *args, **kwargs)


def _recording_exception(body: list[ast.stmt], at: ast.stmt) -> ast.Try:
    """*body* in a ``try`` recording its exception (``__difftest_exception__``)."""
    handler = ast.ExceptHandler(
        type=ast.Name(id="BaseException", ctx=ast.Load()),
        name="__difftest_error__",
        body=[ast.Expr(_call(EXC, ast.Name(id="__difftest_error__", ctx=ast.Load())))],
    )
    return ast.copy_location(ast.Try(body=body, handlers=[handler], orelse=[], finalbody=[]), at)


def _cell_starts(source: str) -> list[int]:
    """The lines starting the cells of a script in the percent format."""
    return [number for number, line in enumerate(source.splitlines(), 1) if line.startswith(CELL_MARK)]


def _leading(module: ast.Module) -> list[ast.stmt]:
    """What must stay at the top of *module*, as it was: its docstring, and
    its ``from __future__`` imports (in a notebook's cells, wherever they
    are: they apply to the cells after them, the whole script here)."""
    docstring = module.body[:1] if (
        module.body and isinstance(module.body[0], ast.Expr)
        and isinstance(module.body[0].value, ast.Constant) and isinstance(module.body[0].value.value, str)) else []
    futures = [statement for statement in module.body
               if isinstance(statement, ast.ImportFrom) and statement.module == "__future__"]
    return docstring + futures


def instrument(source: str, filename: str, impl: str, mode: str) -> ast.Module:
    """The program's module, its imports changed and its values recorded.
    Each top-level statement (but the docstring and the ``__future__``
    imports, which stay first) starts by naming itself
    (``__difftest_site__``); in steps mode it is also wrapped in a ``try``
    recording its exception, in cells mode each cell's statements are, and
    each cell ends by calling ``__difftest_cell_end__`` with its number."""
    tree = ast.parse(source, filename)
    leading = _leading(tree)
    tree.body = [statement for statement in tree.body if all(statement is not kept for kept in leading)]
    tree = Recording().visit(CalendarReads().visit(tree))
    body: list[ast.stmt] = []
    for index, statement in enumerate(tree.body):
        mark = ast.Expr(_call(SITE, ast.Constant(index), ast.Constant(statement.lineno)))
        body.append(ast.copy_location(mark, statement))
        if mode == "steps" and not isinstance(statement, (ast.Import, ast.ImportFrom, ast.FunctionDef, ast.ClassDef)):
            statement = _recording_exception([statement], statement)
        body.append(statement)
    if mode == "cells":
        starts = _cell_starts(source)
        cells: dict[int, list[ast.stmt]] = {}
        for statement in body:
            cell = sum(1 for start in starts if start <= statement.lineno)
            cells.setdefault(cell, []).append(statement)
        body = []
        for cell, statements in cells.items():
            body.append(_recording_exception(statements, statements[0]))
            body.append(ast.copy_location(ast.Expr(_call(CELL_END, ast.Constant(cell))), statements[-1]))
    tree.body = leading + body
    # Changed last, so that an import changed into two statements counts as
    # the one statement it was under both implementations.
    tree = ImportChange(IMPORT_RENAMES[impl]).visit(tree)
    return ast.fix_missing_locations(tree)


class Recorder:
    """What one run computed."""

    def __init__(self, names: Names, program: str) -> None:
        self.names = names
        self.program = program
        self.describer = Describer(names)
        self.sites: dict[str, dict[str, Any]] = {}
        self.warnings: list[dict[str, Any]] = []
        #: (category, message, line) -> how many of the warnings beyond
        #: the recorded ones it was.
        self.warnings_dropped: Counter[tuple[str, str, int | None]] = Counter()
        #: The warnings beyond the recorded ones whose message was not
        #: counted.
        self.warnings_uncounted = 0
        self.errors: list[dict[str, Any]] = []
        self.statement: int | None = None
        self.statement_line: int | None = None

    def record(self, site: str, value: Any) -> Any:
        """Record *value* at *site*; return it."""
        entry = self.sites.setdefault(site, {"count": 0, "values": []})
        entry["count"] += 1
        if entry["count"] <= MAX_OCCURRENCES:
            entry["values"].append(self.describer.describe(value))
        return value

    def site(self, index: int, line: int) -> None:
        """Note the top-level statement running."""
        self.statement, self.statement_line = index, line

    def exception(self, error: BaseException) -> None:
        """Record the exception of a statement of a program run in steps."""
        self.errors.append(self._error(error))

    def _error(self, error: BaseException) -> dict[str, Any]:
        line = None
        for frame, lineno in traceback.walk_tb(error.__traceback__):
            if frame.f_code.co_filename == self.program:
                line = lineno
        return {
            "type": self.names.class_name(type(error)),
            "mro": public_mro(type(error)),
            "message": self.names.text(str(error)),
            "statement": self.statement,
            "statement_line": self.statement_line,
            "line": line,
            # Not compared: where it was raised, for whoever reads the report.
            "traceback": self.names.text("".join(traceback.format_tb(error.__traceback__)[-6:])),
        }

    def warning(self, category: type, message: str, filename: str, lineno: int | None) -> None:
        """Record a warning."""
        warning = {
            "category": self.names.class_name(category),
            "message": self.names.text(message),
            "statement_line": self.statement_line,
            "line": lineno if filename == self.program else None,
        }
        if len(self.warnings) < MAX_WARNINGS:
            self.warnings.append(warning)
            return
        key = warning["category"], warning["message"], warning["line"]
        if key in self.warnings_dropped or len(self.warnings_dropped) < MAX_WARNINGS:
            self.warnings_dropped[key] += 1
        else:
            self.warnings_uncounted += 1


class _LogWarnings(logging.Handler):
    """Records the packages' logged warnings as warnings of category
    ``logging.<LEVEL>``."""

    def __init__(self, recorder: Recorder) -> None:
        super().__init__(logging.WARNING)
        self.recorder = recorder

    def emit(self, record: logging.LogRecord) -> None:
        """Record a logged warning."""
        category = type(f"logging.{record.levelname}", (), {})
        category.__module__ = "logging"
        category.__qualname__ = record.levelname
        self.recorder.warning(category, record.getMessage(), record.pathname, None)


class _ChangedImports(importlib.abc.MetaPathFinder):
    """Imports the modules beside the program (the gallery's ``basic_units``)
    with their imports changed as the program's are."""

    def __init__(self, directory: str, impl: str) -> None:
        self.directory = directory
        self.impl = impl

    def find_spec(self, name: str, path: Any = None, target: Any = None) -> Any:
        """The spec of a module beside the program."""
        if path is not None:
            return None
        candidate = pathlib.Path(self.directory) / f"{name}.py"
        if not candidate.exists():
            return None
        loader = _ChangedImportsLoader(name, str(candidate))
        loader.impl = self.impl
        return importlib.util.spec_from_file_location(name, candidate, loader=loader)


class _ChangedImportsLoader(importlib.machinery.SourceFileLoader):
    """Compiles a module beside the program with its imports changed."""

    impl = "matplotlib"

    def source_to_code(self, data: Any, path: Any, *, _optimize: int = -1) -> Any:
        """The module's code, its imports changed."""
        tree = ImportChange(IMPORT_RENAMES[self.impl]).visit(ast.parse(data, path))
        return compile(ast.fix_missing_locations(tree), path, "exec", dont_inherit=True)

    def get_code(self, fullname: str) -> Any:
        """The module's code, never from a cached bytecode file."""
        return self.source_to_code(self.get_data(self.path), self.path)


def _check_loaded(package: Any, impl: str, installed: bool) -> None:
    """Stops the run unless ``import matplotlib`` gave *impl*'s package
    (pltrs through its alias, or matplotlib), and for a corpus' program
    (*installed*) gave it from the venv's ``site-packages``, as a user has
    it, not from a source tree."""
    found = pathlib.Path(package.__file__).resolve()
    if (impl == "pltrs") != (found.parent.name == "pltrs"):
        raise SystemExit(f"trace.py: `import matplotlib` gave {found}, not {impl}")
    site = pathlib.Path(sysconfig.get_path("purelib")).resolve()
    if installed and not found.is_relative_to(site):
        raise SystemExit(f"trace.py: `import matplotlib` gave {found}, outside {site}")


#: The attribute of `sys` holding the recorder of the run, which the
#: hooks of the instrumented program (`_record`, `_statement`,
#: `_statement_error`)
#: record with. The hooks find it there, not among their globals: a
#: program that hands its functions to other processes (joblib's and
#: multiprocess' pools, which pickle a function of ``__main__`` with the
#: globals it uses) would ship the whole recorder with each task, which
#: made iminuit's workers outgrow loky's memory check under one library
#: and not the other; in another process the hooks record nothing.
_RECORDER = "_difftest_recorder"


def _recorder() -> Any:
    """The recorder of the run, None in another process (see `_RECORDER`).
    (Annotated `Any`: a pickled hook would carry the class it names.)"""
    return getattr(sys, _RECORDER, None)


def _record(site: str, value: Any) -> Any:
    """Record *value* at *site* (the call the instrumented program makes);
    return it. In code numba compiles (py-pde compiles the program's
    functions) it is *value* alone (`_NumbaTaught`)."""
    recorder = _recorder()
    return value if recorder is None else recorder.record(site, value)


def _statement(index: int, line: int) -> None:
    """Note the top-level statement running (`Recorder.site`)."""
    if (recorder := _recorder()) is not None:
        recorder.site(index, line)


def _statement_error(error: BaseException) -> None:
    """Record the exception of a statement (`Recorder.exception`)."""
    if (recorder := _recorder()) is not None:
        recorder.exception(error)


class _NumbaTaught(importlib.abc.MetaPathFinder):
    """Teaches numba, once the program imports it, to compile `_record` as
    returning its value, so that the program's functions numba compiles
    compile as they do uninstrumented (else numba fails on an untyped
    global, under both implementations alike)."""

    finding = False

    def find_spec(self, name: str, path: Any = None, target: Any = None) -> Any:
        """numba's own spec, its module taught once executed. (Libraries
        ask for the spec to see whether numba is installed, py-pde's
        ``importlib.util.find_spec``, before they import it.)"""
        if name != "numba" or self.finding:
            return None
        self.finding = True
        try:
            spec = importlib.util.find_spec(name)
        finally:
            self.finding = False
        if spec is None or spec.loader is None:
            return spec
        execute = spec.loader.exec_module

        def execute_and_teach(module: Any) -> None:
            execute(module)
            if self in sys.meta_path:
                sys.meta_path.remove(self)
            from numba.extending import overload

            # Unannotated: numba compares the typing signature with the
            # implementation's, annotations included.
            @overload(_record)
            def _record_in_numba(site, value):  # type: ignore[no-untyped-def]  # noqa: ANN001, ANN202
                return lambda site, value: value

        spec.loader.exec_module = execute_and_teach  # type: ignore[method-assign]
        return spec


def _seed_random_generators() -> None:
    """The global generators start alike in both runs, and a generator made
    without a seed (``np.random.default_rng()``, ``random.Random()``, as the
    libraries a program calls make them) gets a fixed one, so that programs
    drawing random data draw the same data under both."""
    import random

    import numpy as np

    random.seed(19680801)
    np.random.seed(19680801)
    default_rng = np.random.default_rng
    seed = random.Random.seed

    def seeded_default_rng(seed: Any = None) -> Any:
        """``np.random.default_rng``, seeded when no seed is given."""
        return default_rng(19680801 if seed is None else seed)

    def seeded_seed(self: random.Random, a: Any = None, version: int = 2) -> None:
        """``random.Random.seed``, with a fixed seed when none is given."""
        seed(self, 19680801 if a is None else a, version)

    np.random.default_rng = seeded_default_rng
    random.Random.seed = seeded_seed  # type: ignore[method-assign]


def _fix_the_calendar() -> None:
    """The calendar reads of `time` given no time (``time.localtime()``,
    ``time.strftime(format)``, ...) find `CALENDAR_NOW` in both runs, as
    the random generators start alike: statsmodels' summaries print the
    time of day they were made (the program's own reads of its date
    classes are `CalendarReads`'). The clocks timing what a program does
    (``time.time``, ``perf_counter``) run on."""
    import functools
    import time

    def at_now(read: Any, position: int) -> Any:
        """*read*, given `CALENDAR_NOW`'s time where its argument at
        *position* is left out or None."""
        def now(*args: Any) -> Any:
            if len(args) > position and args[position] is not None:
                return read(*args)
            moment = CALENDAR_NOW if read in (localtime, gmtime, ctime) else localtime(CALENDAR_NOW)
            return read(*args[:position], moment)
        return functools.wraps(read)(now)

    localtime, gmtime, ctime = time.localtime, time.gmtime, time.ctime
    time.localtime, time.gmtime, time.ctime = (at_now(read, 0) for read in (localtime, gmtime, ctime))
    time.asctime = at_now(time.asctime, 0)
    time.strftime = at_now(time.strftime, 1)


def _save_figures(plt: Any, out: pathlib.Path, recorder: Recorder, cell: int | None = None) -> list[dict[str, Any]]:
    """Saves every figure open (those of *cell*: as the inline backend
    shows them) and describes them."""
    figures = []
    for number in plt.get_fignums():
        entry: dict[str, Any] = {"number": number if cell is None else f"{cell}:{number}"}
        try:
            figure = plt.figure(number)
            entry["label"] = figure.get_label()
            path = out / (f"figure-{number}.png" if cell is None else f"cell-{cell}-figure-{number}.png")
            figure.savefig(path, **({} if cell is None else {"bbox_inches": "tight"}))
            entry["png"] = path.name
        except Exception as err:  # noqa: BLE001 - recorded, as the program's
            entry["error"] = recorder._error(err)
        figures.append(entry)
    return figures


def _save_windows(shown: list[Any], out: pathlib.Path, recorder: Recorder,
                  saved: int) -> list[dict[str, Any]]:
    """Saves the figures *shown* in a toolkit's windows that pyplot does not
    keep (`_save_figures` saves those), numbered after the *saved* ones in
    the order their canvases were made, and describes them."""
    from matplotlib._pylab_helpers import Gcf
    kept = {id(manager.canvas.figure) for manager in Gcf.get_all_fig_managers()}
    windows = []
    for number, figure in enumerate((figure for figure in shown if id(figure) not in kept), saved + 1):
        entry: dict[str, Any] = {"number": f"window:{number}"}
        try:
            entry["label"] = figure.get_label()
            path = out / f"window-{number}.png"
            figure.savefig(path)
            entry["png"] = path.name
        except Exception as err:  # noqa: BLE001 - recorded, as the program's
            entry["error"] = recorder._error(err)
        windows.append(entry)
    return windows


def _display(*objs: Any, **_options: Any) -> None:
    """Jupyter's ``display`` as a run in cells mode sees it: the plain-text
    representation of each object printed, but of a figure, which the
    inline backend shows as an image (saved at the cell's end here)."""
    for obj in objs:
        if type(obj).__name__ not in ("Figure", "SubFigure"):
            print(repr(obj))


def _written(tree: pathlib.Path, program: pathlib.Path) -> list[str]:
    """The files the program wrote in *tree*: those that are no links into
    the corpus' tree nor copies standing in for such links left unchanged
    (where the system makes no links), but the program itself and Python's
    caches."""
    copies = unchanged_copies(tree)
    written = (path.relative_to(tree).as_posix() for path in tree.rglob("*")
               if path.is_file() and not path.is_symlink() and path != program
               and "__pycache__" not in path.parts)
    return sorted(name for name in written if name not in copies)


def main() -> int:
    """Run the program and write its record."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--impl", choices=sorted(IMPORT_RENAMES), required=True)
    parser.add_argument("--mode", choices=["program", "steps", "cells"], default="program")
    parser.add_argument("--in-tree", type=pathlib.Path)
    parser.add_argument("--shell", choices=["ipython"])
    parser.add_argument("--gui", action="store_true",
                        help="end the toolkits' main loops once the windows drew (gui_shim.py)")
    parser.add_argument("program", type=pathlib.Path)
    parser.add_argument("out", type=pathlib.Path)
    args = parser.parse_args()

    program = str(args.program.resolve())
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if args.in_tree is None:
        workdir = out / "cwd"
        workdir.mkdir(exist_ok=True)
        os.chdir(workdir)
    else:
        workdir = args.in_tree.resolve()
        os.chdir(args.program.resolve().parent)

    package = __import__("matplotlib")
    _check_loaded(package, args.impl, installed=args.in_tree is not None)
    package_dirs = [str(pathlib.Path(package.__file__).parent)]
    names = Names(NAME_PREFIXES[args.impl], package_dirs, os.environ.get("MPLCONFIGDIR"), str(workdir), sys.prefix,
                  str(out.resolve()))
    recorder = Recorder(names, program)
    setattr(sys, _RECORDER, recorder)
    text = importlib.util.decode_source(args.program.read_bytes())
    if args.shell:
        text = ipython_shell.python_of(text)
    code = compile(instrument(text, program, args.impl, args.mode), program, "exec")
    plt = __import__("matplotlib.pyplot", fromlist=["pyplot"])
    figures: list[dict[str, Any]] = []
    windows: list[dict[str, Any]] = []
    if args.gui:
        import gui_shim
        gui_shim.install(lambda shown: windows.extend(_save_windows(shown, out, recorder, len(windows))))

    def cell_end(cell: int) -> None:
        """Show the cell's figures as the inline backend does: saved, then
        closed."""
        figures.extend(_save_figures(plt, out, recorder, cell))
        plt.close("all")

    # The program runs as a module of its own installed as `__main__`, as
    # `python program.py` (runpy) runs it: pickle finds the classes and
    # functions it defines (`__main__.X`), and a process spawned by it
    # (multiprocessing's spawn, loky) imports `__main__` from the
    # program's file. The tracer's module is put back once it ran.
    main_module = types.ModuleType("__main__")
    namespace = main_module.__dict__
    namespace.update({
        "__file__": program,
        "__builtins__": __builtins__,
        REC: _record,
        EXC: _statement_error,
        SITE: _statement,
        CELL_END: cell_end,
        CALENDAR: _read_calendar,
    })
    if args.mode == "cells":
        namespace["display"] = _display
        # A notebook's `__main__` is the kernel's user module, which a
        # spawned or forkserver child neither imports nor runs (its spec
        # names `__main__`, so multiprocessing and loky leave the child's
        # main as it is): the program's file is not run again there.
        main_module.__spec__ = importlib.machinery.ModuleSpec("__main__", None)
    if args.shell:
        ipython_shell.start(namespace)
    sys.argv = [program]
    # The program's directory in place of the harness', whose modules
    # (coverage.py, kinds.py...) would hide the libraries of their names
    # (numba imports coverage when it is importable).
    # The harness' own modules leave sys.modules too, so that the program
    # importing a library named as one (describe) imports the library.
    harness = str(pathlib.Path(__file__).resolve().parent)
    sys.path[:] = [str(args.program.resolve().parent),
                   *(entry for entry in sys.path[1:] if entry != harness)]
    for name, module in list(sys.modules.items()):
        if module is not sys.modules["__main__"] and str(getattr(module, "__file__", "")).startswith(harness + os.sep):
            del sys.modules[name]
    sys.meta_path.insert(0, _ChangedImports(sys.path[0], args.impl))
    if "numba" not in sys.modules:
        sys.meta_path.insert(0, _NumbaTaught())
    _seed_random_generators()
    _fix_the_calendar()
    stdout = io.StringIO()
    real_stdout = sys.stdout
    log_handler = _LogWarnings(recorder)
    logging.getLogger().addHandler(log_handler)
    started = time.perf_counter()
    uncaught = None
    with warnings.catch_warnings():
        warnings.simplefilter("always")
        warnings.showwarning = lambda message, category, filename, lineno, file=None, line=None: (
            recorder.warning(category, str(message), filename, lineno))
        sys.stdout = stdout
        tracer_module = sys.modules["__main__"]
        sys.modules["__main__"] = main_module
        try:
            exec(code, namespace)
        except BaseException as err:  # noqa: BLE001 - the program's exception is a result
            uncaught = recorder._error(err)
        finally:
            sys.stdout = real_stdout
            sys.modules["__main__"] = tracer_module
        elapsed = time.perf_counter() - started
        recorder.statement = recorder.statement_line = None
        if args.gui:
            gui_shim.finish()
        figures += _save_figures(plt, out, recorder) + windows
    logging.getLogger().removeHandler(log_handler)

    record = {
        "impl": args.impl,
        "mode": args.mode,
        "program": program,
        "seconds": round(elapsed, 3),
        "sites": recorder.sites,
        "warnings": recorder.warnings,
        "warnings_dropped": [[*key, count] for key, count in recorder.warnings_dropped.items()],
        "warnings_uncounted": recorder.warnings_uncounted,
        "errors": recorder.errors,
        "uncaught": uncaught,
        "stdout": names.text(stdout.getvalue())[:20000],
        "figures": figures,
        "written": [] if args.in_tree is None else _written(workdir, args.program.resolve()),
    }
    (out / "record.json").write_text(json.dumps(record))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:  # noqa: BLE001 - the harness's own failure
        traceback.print_exc()
        sys.exit(2)
