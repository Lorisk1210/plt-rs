"""Runs one program and records what it computed, for the differential tester.

    python trace.py --impl matplotlib|pltrs --mode program|steps PROGRAM OUT_DIR

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

The record goes to ``OUT_DIR/record.json``. Its own failures (not the
program's) exit with status 2.
"""

from __future__ import annotations

import argparse
import ast
import importlib.abc
import importlib.machinery
import importlib.util
import io
import json
import logging
import os
import pathlib
import sys
import time
import traceback
import warnings
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from describe import Describer, Names, public_mro  # noqa: E402

#: Recorded occurrences of one site; later ones are only counted.
MAX_OCCURRENCES = 30
#: Recorded warnings of one run; later ones are only counted.
MAX_WARNINGS = 500

#: The module renames of the import change, per implementation.
IMPORT_RENAMES = {
    "matplotlib": [],
    "pltrs": [("matplotlib", "pltrs"), ("mpl_toolkits", "pltrs.mpl_toolkits"), ("pylab", "pltrs.pylab")],
}
#: How each implementation's names read as matplotlib's.
NAME_PREFIXES = {
    "matplotlib": [],
    "pltrs": [("pltrs.mpl_toolkits", "mpl_toolkits"), ("pltrs.pylab", "pylab"), ("pltrs", "matplotlib")],
}

REC, EXC, SITE = "__difftest_record__", "__difftest_exception__", "__difftest_site__"


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


def instrument(source: str, filename: str, impl: str, steps: bool) -> ast.Module:
    """The program's module, its imports changed and its values recorded.
    Each top-level statement starts by naming itself (``__difftest_site__``);
    in steps mode it is also wrapped in a ``try`` recording its exception."""
    tree = ast.parse(source, filename)
    tree = Recording().visit(tree)
    body: list[ast.stmt] = []
    for index, statement in enumerate(tree.body):
        mark = ast.Expr(_call(SITE, ast.Constant(index), ast.Constant(statement.lineno)))
        body.append(ast.copy_location(mark, statement))
        if steps and not isinstance(statement, (ast.Import, ast.ImportFrom, ast.FunctionDef, ast.ClassDef)):
            handler = ast.ExceptHandler(
                type=ast.Name(id="BaseException", ctx=ast.Load()),
                name="__difftest_error__",
                body=[ast.Expr(_call(EXC, ast.Name(id="__difftest_error__", ctx=ast.Load())))],
            )
            statement = ast.copy_location(ast.Try(body=[statement], handlers=[handler], orelse=[], finalbody=[]), statement)
        body.append(statement)
    tree.body = body
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
        self.warnings_dropped = 0
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
        if len(self.warnings) >= MAX_WARNINGS:
            self.warnings_dropped += 1
            return
        self.warnings.append({
            "category": self.names.class_name(category),
            "message": self.names.text(message),
            "statement_line": self.statement_line,
            "line": lineno if filename == self.program else None,
        })


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


def _seed_random_generators() -> None:
    """The global generators start alike in both runs, and a generator made
    without a seed (``np.random.default_rng()``) gets a fixed one, so that
    programs drawing random data draw the same data under both."""
    import random

    import numpy as np

    random.seed(19680801)
    np.random.seed(19680801)
    default_rng = np.random.default_rng

    def seeded_default_rng(seed: Any = None) -> Any:
        """``np.random.default_rng``, seeded when no seed is given."""
        return default_rng(19680801 if seed is None else seed)

    np.random.default_rng = seeded_default_rng


def _save_figures(plt: Any, out: pathlib.Path, recorder: Recorder) -> list[dict[str, Any]]:
    figures = []
    for number in plt.get_fignums():
        entry: dict[str, Any] = {"number": number}
        try:
            figure = plt.figure(number)
            entry["label"] = figure.get_label()
            path = out / f"figure-{number}.png"
            figure.savefig(path)
            entry["png"] = path.name
        except Exception as err:  # noqa: BLE001 - recorded, as the program's
            entry["error"] = recorder._error(err)
        figures.append(entry)
    return figures


def main() -> int:
    """Run the program and write its record."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--impl", choices=sorted(IMPORT_RENAMES), required=True)
    parser.add_argument("--mode", choices=["program", "steps"], default="program")
    parser.add_argument("program", type=pathlib.Path)
    parser.add_argument("out", type=pathlib.Path)
    args = parser.parse_args()

    program = str(args.program.resolve())
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    workdir = out / "cwd"
    workdir.mkdir(exist_ok=True)
    os.chdir(workdir)

    package = __import__(args.impl)
    package_dirs = [str(pathlib.Path(package.__file__).parent)]
    names = Names(NAME_PREFIXES[args.impl], package_dirs, os.environ.get("MPLCONFIGDIR"))
    recorder = Recorder(names, program)
    code = compile(instrument(args.program.read_text(), program, args.impl, args.mode == "steps"),
                   program, "exec")
    plt = __import__(f"{args.impl}.pyplot", fromlist=["pyplot"])

    namespace = {
        "__name__": "__main__",
        "__file__": program,
        "__builtins__": __builtins__,
        REC: recorder.record,
        EXC: recorder.exception,
        SITE: recorder.site,
    }
    sys.argv = [program]
    sys.path[0] = str(args.program.resolve().parent)
    sys.meta_path.insert(0, _ChangedImports(sys.path[0], args.impl))
    _seed_random_generators()
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
        try:
            exec(code, namespace)
        except BaseException as err:  # noqa: BLE001 - the program's exception is a result
            uncaught = recorder._error(err)
        finally:
            sys.stdout = real_stdout
        elapsed = time.perf_counter() - started
        recorder.statement = recorder.statement_line = None
        figures = _save_figures(plt, out, recorder)
    logging.getLogger().removeHandler(log_handler)

    record = {
        "impl": args.impl,
        "mode": args.mode,
        "program": program,
        "seconds": round(elapsed, 3),
        "sites": recorder.sites,
        "warnings": recorder.warnings,
        "warnings_dropped": recorder.warnings_dropped,
        "errors": recorder.errors,
        "uncaught": uncaught,
        "stdout": names.text(stdout.getvalue())[:20000],
        "figures": figures,
    }
    (out / "record.json").write_text(json.dumps(record))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:  # noqa: BLE001 - the harness's own failure
        traceback.print_exc()
        sys.exit(2)
