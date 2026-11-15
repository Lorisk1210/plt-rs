"""The census of matplotlib's methods and functions that libraries replace
by assignment rather than by subclassing.

    python patches_census.py [--out FILE] SOURCE...

Runs under matplotlib (the reference venv). The SOURCEs are read as
``private_census.py`` reads them (``site-packages`` directories and trees
of programs; copies of matplotlib's code and the libraries' tests are not
read), with its receiver-following reader (`private_reader.Reader`).

A replacement is an assignment whose target is an attribute of a class,
a module or an object of matplotlib's naming one of its methods or
functions (``Axes.plot = f``, ``pyplot.show = f``, ``label.set_y = f``),
``setattr`` with the name spelled out, or ``unittest.mock.patch.object``
in a library's own code. Its target is ``class``, ``module`` or
``instance`` (an object of a class of matplotlib's, or one reading cannot
follow whose name only matplotlib's classes define: ``candidates`` then
names them). What the replacement does is read from the value assigned:
``wraps`` (it calls the method it replaces, kept beforehand: ``orig =
Axes.plot``, or called through its class: ``Axes.plot(self, ...)``),
``records`` (it also appends to a list or counts), ``replaces`` (it does
neither), ``unknown`` (a value reading cannot follow).

The table (``spec/diff/patches_census.json``) has a row per (target, owner,
name): the libraries doing it, what each replacement does, one place per
library. ``generators/patches.py`` reads it.
"""

from __future__ import annotations

import argparse
import ast
import inspect
import json
import pathlib
import sys
from collections import defaultdict
from dataclasses import dataclass
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import overrides_census as oc  # noqa: E402
import private_census as pc  # noqa: E402
from private_names import Names  # noqa: E402
from private_reader import Reader  # noqa: E402
from private_types import Kind, Typer  # noqa: E402

#: What a replacement may do, from least to most informative.
EFFECTS = ("unknown", "replaces", "wraps", "records")
#: Names whose assignment is no replacement of behaviour (protocol
#: attributes matplotlib's own code sets on objects).
NOT_METHODS = {"__doc__", "__name__", "__qualname__", "__module__", "__wrapped__", "__signature__"}


@dataclass(frozen=True)
class Patch:
    """One replacement: of ``name`` of ``owner`` (a class or module
    qualname; None for an object reading cannot follow), on a ``target``
    of that kind, by ``library``, doing ``effect``, at ``where``."""

    name: str
    owner: str | None
    candidates: tuple[str, ...]
    target: str
    effect: str
    library: str
    where: str


class Callables:
    """matplotlib's methods and functions, by name."""

    def __init__(self, names: Names) -> None:
        #: method name -> qualnames of the classes defining it
        self.methods: dict[str, set[str]] = defaultdict(set)
        for qualname, cls in names.classes.items():
            for member, value in vars(cls).items():
                if member in NOT_METHODS:
                    continue
                if inspect.isfunction(value) or isinstance(value, (staticmethod, classmethod)):
                    self.methods[member].add(qualname)
        #: method name -> qualnames of the classes of the objects a figure
        #: is made of (artists, canvases and managers, timers, transforms,
        #: tick helpers, colormaps and norms) defining it
        roots = _figure_classes()
        self.object_methods: dict[str, set[str]] = {
            name: kept for name, owners in self.methods.items()
            if (kept := {q for q in owners if issubclass(names.classes[q], roots)})}
        #: module -> its functions
        self.functions: dict[str, set[str]] = {
            module: {name for name, value in vars(loaded).items()
                     if inspect.isfunction(value) and getattr(value, "__module__", None) == module}
            for module, loaded in names.modules.items()}
        self.classes = names.classes

    def defining(self, classes: tuple[type, ...], name: str) -> list[str]:
        """The qualnames of matplotlib's classes among ``classes`` and their
        bases defining the method ``name``."""
        owners = set()
        for cls in classes:
            for klass in cls.__mro__:
                qualname = f"{klass.__module__}.{klass.__qualname__}"
                if qualname in self.methods.get(name, ()):
                    owners.add(qualname)
                    break
        return sorted(owners)


def _figure_classes() -> tuple[type, ...]:
    """The bases of the classes of matplotlib's objects a figure is made
    of, whose methods a library replaces on one object."""
    from matplotlib import artist, backend_bases, colors, ticker, transforms
    return (artist.Artist, backend_bases.FigureCanvasBase, backend_bases.FigureManagerBase,
            backend_bases.TimerBase, backend_bases.RendererBase, transforms.TransformNode,
            ticker.TickHelper, colors.Colormap, colors.Normalize)


class PatchReader(Reader):
    """The private reader, recording replacements instead of private
    names' uses."""

    def __init__(self, census: oc.Census, names: Names, typer: Typer, callables: Callables,
                 own: set[str], record: Any) -> None:
        super().__init__(census, names, typer, own, set(), lambda use: None)
        self.callables = callables
        self.found = record
        self.functions: dict[str, ast.AST] = {}
        self.kept: set[str] = set()

    def read(self, module: oc.Module) -> None:
        """Record the replacements of ``module``."""
        self.functions = {node.name: node for node in ast.walk(module.tree)
                          if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))}
        # Names bound to an attribute's value somewhere (``orig =
        # Axes.plot``): a replacement calling one wraps it.
        self.kept = {f"{target.id}:{node.value.attr}" for node in ast.walk(module.tree)
                     if isinstance(node, ast.Assign) and isinstance(node.value, ast.Attribute)
                     for target in node.targets if isinstance(target, ast.Name)}
        super().read(module)

    # Recording -----------------------------------------------------------

    def visit_Assign(self, node: ast.Assign) -> None:
        if self.recording:
            for target in node.targets:
                if isinstance(target, ast.Attribute):
                    self._replaced(node, target.value, target.attr, node.value)
        super().visit_Assign(node)

    def visit_Call(self, node: ast.Call) -> None:
        if self.recording:
            function = node.func
            spelled = (len(node.args) >= 2 and isinstance(node.args[1], ast.Constant)
                       and isinstance(node.args[1].value, str))
            if isinstance(function, ast.Name) and function.id == "setattr" and len(node.args) == 3 and spelled:
                self._replaced(node, node.args[0], node.args[1].value, node.args[2])
            elif (isinstance(function, ast.Attribute) and function.attr == "object" and spelled
                  and oc.dotted_name(function.value) in ("patch", "mock.patch", "unittest.mock.patch")):
                self._replaced(node, node.args[0], node.args[1].value,
                               node.args[2] if len(node.args) > 2 else None)
        super().visit_Call(node)

    def _replaced(self, node: ast.AST, receiver_node: ast.expr, name: str, value: ast.expr | None) -> None:
        if name in NOT_METHODS or self.module is None:
            return
        receiver = self.kind(receiver_node)
        effect = self._effect(value, name)
        if receiver is not None and receiver.what == "module":
            if name in self.callables.functions.get(receiver.module, ()):
                self._emit_patch(node, name, receiver.module, (), "module", effect)
            return
        if receiver is not None and receiver.what in ("instance", "class"):
            for owner in self.callables.defining(receiver.classes, name):
                self._emit_patch(node, name, owner, (), receiver.what, effect)
            return
        if receiver is not None or not self.imports_matplotlib:
            return
        if isinstance(receiver_node, ast.Name) and receiver_node.id in ("self", "cls"):
            # The library's own object (a class not deriving from
            # matplotlib's, whose ``self`` reading would follow otherwise).
            return
        # An object reading cannot follow: a replacement only where the
        # value is code, the name is a method of matplotlib's objects a
        # figure is made of, and the library's own classes do not define
        # it (unless the replacement calls what it replaces, kept from the
        # object: ultraplot's ``axis._update_ticks``).
        if name in self.own_names and effect not in ("wraps", "records"):
            return
        definers = self.callables.object_methods.get(name)
        if definers and effect != "unknown" and not name.startswith("__"):
            self._emit_patch(node, name, None, tuple(sorted(definers)), "instance", effect)

    def _emit_patch(self, node: ast.AST, name: str, owner: str | None, candidates: tuple[str, ...],
                    target: str, effect: str) -> None:
        self.found(Patch(name, owner, candidates, target, effect, self.module.library,
                         f"{self.module.name}:{getattr(node, 'lineno', 0)}"))

    # What the replacement does --------------------------------------------

    def _code_of(self, value: ast.expr | None) -> ast.AST | None:
        """The function a value assigned is: a lambda, a function of the
        module, the function ``functools.partial`` or ``types.MethodType``
        bind."""
        if isinstance(value, ast.Lambda):
            return value
        if isinstance(value, ast.Name):
            return self.functions.get(value.id)
        if isinstance(value, ast.Call):
            called = oc.dotted_name(value.func) or ""
            if called.split(".")[-1] in ("partial", "MethodType", "partialmethod") and value.args:
                return self._code_of(value.args[0])
        return None

    def _effect(self, value: ast.expr | None, name: str) -> str:
        code = self._code_of(value)
        if code is None:
            if isinstance(value, ast.Call) and any(
                    isinstance(arg, ast.Attribute) and arg.attr == name for arg in value.args):
                # A wrapper made of the original (``wrapped(Axes.plot)``).
                return "wraps"
            return "unknown"
        calls_original = records = False
        for node in ast.walk(code):
            if isinstance(node, ast.Call):
                function = node.func
                if isinstance(function, ast.Name) and f"{function.id}:{name}" in self.kept:
                    calls_original = True
                elif isinstance(function, ast.Attribute) and function.attr == name:
                    calls_original = True
                elif isinstance(function, ast.Attribute) and function.attr in ("append", "add", "extend"):
                    records = True
            elif isinstance(node, ast.AugAssign):
                records = True
        if calls_original:
            return "records" if records else "wraps"
        return "replaces"


class Tally:
    """The replacements found, by (target, owner, name)."""

    def __init__(self) -> None:
        self.effects: dict[tuple[str, str | None, str], dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
        self.where: dict[tuple[str, str | None, str], dict[str, str]] = defaultdict(dict)
        self.candidates: dict[tuple[str, str | None, str], tuple[str, ...]] = {}

    def record(self, patch: Patch) -> None:
        """Count one replacement."""
        key = (patch.target, patch.owner, patch.name)
        self.effects[key][patch.library].add(patch.effect)
        self.where[key].setdefault(patch.library, patch.where)
        if patch.owner is None:
            self.candidates[key] = patch.candidates

    def rows(self) -> list[dict[str, Any]]:
        """The table, the names most libraries replace first."""
        rows = []
        for (target, owner, name), by_library in self.effects.items():
            rows.append({
                "name": name,
                "owner": owner,
                "candidates": list(self.candidates.get((target, owner, name), ())),
                "target": target,
                "libraries": sorted(by_library),
                "count": len(by_library),
                "effects": {library: max(effects, key=EFFECTS.index)
                            for library, effects in sorted(by_library.items())},
                "where": dict(sorted(self.where[(target, owner, name)].items())),
            })
        by_name: dict[str, set[str]] = defaultdict(set)
        for row in rows:
            by_name[row["name"]] |= set(row["libraries"])
        for row in rows:
            row["name_count"] = len(by_name[row["name"]])
        rows.sort(key=lambda r: (-r["name_count"], r["name"], r["target"], r["owner"] or ""))
        return rows


def main() -> int:
    """Read the sources, write the table."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=pathlib.Path, default=HERE / "patches_census.json")
    parser.add_argument("sources", nargs="+", type=pathlib.Path)
    args = parser.parse_args()
    import matplotlib

    names = Names()
    census = oc.Census()
    installed = pathlib.Path(matplotlib.__file__).parent
    census.add_package(installed, "matplotlib", "matplotlib")
    census.add_package(installed.parent / "mpl_toolkits", "mpl_toolkits", "mpl_toolkits")
    for source in args.sources:
        if source.name == "site-packages":
            census.add_site_packages(source)
    for source in args.sources:
        if source.name != "site-packages":
            for corpus in sorted(p for p in source.iterdir() if p.is_dir()):
                if f"corpus:{corpus.name}" not in census.libraries:
                    census.add_corpus(corpus)
    pc._not_copies(census)
    own, _ = pc._library_names(census)
    callables = Callables(names)
    typer = Typer(names)
    tally = Tally()
    unread: list[str] = []
    for module in list(census.modules.values()):
        if module.library in ("matplotlib", "mpl_toolkits"):
            continue
        try:
            PatchReader(census, names, typer, callables, own[module.library], tally.record).read(module)
        except RecursionError:  # a generated module of deeply nested expressions
            unread.append(module.name)
    rows = tally.rows()
    patching = sorted({library for row in rows for library in row["libraries"]})
    table = {
        "matplotlib": matplotlib.__version__,
        "libraries_read": sorted(census.libraries),
        "libraries_patching": len(patching),
        "names": len({row["name"] for row in rows}),
        "unread_modules": unread,
        "rows": rows,
    }
    args.out.write_text(json.dumps(table, indent=1) + "\n", encoding="utf-8")
    print(f"{len(census.libraries)} libraries read, {len(patching)} replace {table['names']} names "
          f"of matplotlib's in {len(rows)} (target, owner, name) rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
