"""The census of matplotlib's members that other libraries override.

    python overrides_census.py [--out FILE] SOURCE...

Runs under matplotlib (the reference venv). Each SOURCE is a
``site-packages`` directory or a tree of programs (a corpus of
``spec/diff/corpora.toml`` as fetched into ``target/diff/corpora``): every
top-level package of a ``site-packages`` that mentions matplotlib is a
library (``mpl_toolkits.<name>`` packages each one), the first of the
sources having it counts; every corpus is a library ``corpus:<name>``
(its files under a directory named as a library already counted are that
library's, and left out). matplotlib itself (its installed package, but
its tests) counts as the library ``matplotlib``, its toolkits as
``mpl_toolkits``: the hooks matplotlib's own subclasses rely on.

The sources are read with ``ast``, never imported. A class's bases are
resolved through the imports and assignments of its module, through the
library's own classes (and other libraries'), to matplotlib's classes,
which are imported here. A class whose bases reach one of matplotlib's
contributes each method, property and class attribute it defines that
the nearest of matplotlib's classes among its ancestors defines too
(``object``'s members left out).

The table (``spec/diff/overrides_census.json``) has a row per
(matplotlib class, member): its kind, the class of matplotlib's MRO
defining it, how many libraries override it and which, and the classes
that do. ``generators/overrides.py`` reads it.
"""

from __future__ import annotations

import argparse
import ast
import importlib
import json
import pathlib
import sys
from collections import defaultdict
from typing import Any, Iterator

#: Directories never read: tests of the libraries, caches, build trees.
SKIPPED_DIRS = {"tests", "test", "testing", "__pycache__", "build", ".git", "node_modules", "_vendor", "vendor"}
#: The top-level packages that are matplotlib, or are no library.
NOT_LIBRARIES = {"matplotlib", "pltrs", "pylab", "_distutils_hack", "setuptools", "pip", "pkg_resources"}
#: matplotlib's own toolkits (the others in ``mpl_toolkits`` are libraries).
OWN_TOOLKITS = {"axes_grid1", "axisartist", "mplot3d"}
#: The members no override of which is a hook.
OBJECT_MEMBERS = set(dir(object)) | {"__dict__", "__weakref__", "__module__", "__qualname__", "__doc__",
                                     "__annotations__", "__slots__", "__firstlineno__", "__static_attributes__"}


class Module:
    """One module's classes and the names it binds."""

    def __init__(self, name: str, library: str, tree: ast.Module, is_package: bool) -> None:
        self.name = name
        self.library = library
        self.package = name if is_package else name.rpartition(".")[0]
        self.bindings: dict[str, str] = {}
        self.classes: dict[str, ast.ClassDef] = {}
        #: The module's source, as `private_census.py` reads it again.
        self.tree = tree
        for node in tree.body:
            self._bind(node)

    def _absolute(self, module: str | None, level: int) -> str:
        if level == 0:
            return module or ""
        parts = self.package.split(".") if self.package else []
        base = parts[: len(parts) - (level - 1)] if level > 1 else parts
        return ".".join([*base, *([module] if module else [])])

    def _bind(self, node: ast.stmt) -> None:
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.asname:
                    self.bindings[alias.asname] = alias.name
                else:
                    top = alias.name.split(".")[0]
                    self.bindings[top] = top
        elif isinstance(node, ast.ImportFrom):
            source = self._absolute(node.module, node.level)
            for alias in node.names:
                if alias.name != "*":
                    self.bindings[alias.asname or alias.name] = f"{source}.{alias.name}"
        elif isinstance(node, ast.ClassDef):
            self.classes[node.name] = node
            self.bindings[node.name] = f"{self.name}.{node.name}"
        elif isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            dotted = dotted_name(node.value)
            if dotted is not None:
                self.bindings[node.targets[0].id] = self.resolve_local(dotted) or dotted
        elif isinstance(node, (ast.If, ast.Try)):
            for child in [*node.body, *getattr(node, "orelse", []), *getattr(node, "finalbody", []),
                          *(s for h in getattr(node, "handlers", []) for s in h.body)]:
                self._bind(child)

    def resolve_local(self, dotted: str) -> str | None:
        """``dotted`` (``mpatches.Patch``) as an absolute dotted name."""
        head, _, rest = dotted.partition(".")
        bound = self.bindings.get(head)
        if bound is None:
            return None
        return f"{bound}.{rest}" if rest else bound


def dotted_name(node: ast.expr) -> str | None:
    """The dotted name ``node`` spells, if it spells one."""
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        inner = dotted_name(node.value)
        return None if inner is None else f"{inner}.{node.attr}"
    if isinstance(node, ast.Subscript):  # Generic[...] and the like
        return dotted_name(node.value)
    return None


def _parse(path: pathlib.Path, source: str | None = None) -> ast.Module | None:
    try:
        text = source if source is not None else path.read_text(encoding="utf-8", errors="replace")
        return ast.parse(text, str(path))
    except (SyntaxError, ValueError, RecursionError):
        return None


def _notebook_source(path: pathlib.Path) -> str:
    """A notebook's code cells as one script, magics and shell lines out."""
    try:
        cells = json.loads(path.read_text(encoding="utf-8")).get("cells", [])
    except (ValueError, UnicodeDecodeError):
        return ""
    lines = []
    for cell in cells:
        if cell.get("cell_type") != "code":
            continue
        source = cell.get("source", "")
        text = "".join(source) if isinstance(source, list) else source
        for line in text.splitlines():
            stripped = line.lstrip()
            lines.append("" if stripped.startswith(("%", "!", "?")) else line)
        lines.append("")
    return "\n".join(lines)


def _files(root: pathlib.Path, skipped: set[str]) -> Iterator[pathlib.Path]:
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).parts
        if any(part in SKIPPED_DIRS or part in skipped or part.startswith(".") for part in relative[:-1]):
            continue
        if path.suffix in (".py", ".ipynb") and path.is_file():
            yield path


def _mentions_matplotlib(root: pathlib.Path) -> bool:
    if root.is_file():
        return "matplotlib" in root.read_text(encoding="utf-8", errors="replace")
    for path in _files(root, set()):
        if path.suffix == ".py" and "matplotlib" in path.read_text(encoding="utf-8", errors="replace"):
            return True
    return False


class Census:
    """The modules read, and the classes resolved."""

    def __init__(self) -> None:
        self.modules: dict[str, Module] = {}
        self.libraries: list[str] = []
        self._resolving: set[str] = set()
        self._resolved: dict[str, list[Any]] = {}

    # Reading -------------------------------------------------------------

    def add_package(self, root: pathlib.Path, module: str, library: str) -> None:
        """Read the package or module at ``root``, importable as ``module``."""
        if root.is_file():
            tree = _parse(root)
            if tree is not None:
                self.modules[module] = Module(module, library, tree, False)
            return
        for path in _files(root, set()):
            if path.suffix != ".py":
                continue
            parts = path.relative_to(root).with_suffix("").parts
            is_package = parts[-1] == "__init__"
            name = ".".join([module, *(parts[:-1] if is_package else parts)])
            tree = _parse(path)
            if tree is not None:
                self.modules[name] = Module(name, library, tree, is_package)

    def add_site_packages(self, site: pathlib.Path) -> None:
        """Every library of ``site`` not read before."""
        known = {m.split(".")[0] if not m.startswith("mpl_toolkits.") else ".".join(m.split(".")[:2])
                 for m in self.modules}
        entries = []
        for entry in sorted(site.iterdir()):
            name = entry.stem if entry.suffix == ".py" else entry.name
            if entry.name == "mpl_toolkits" and entry.is_dir():
                for sub in sorted(entry.iterdir()):
                    if sub.is_dir() and sub.name not in OWN_TOOLKITS and not sub.name.startswith("_"):
                        entries.append((sub, f"mpl_toolkits.{sub.name}"))
                continue
            if (name.startswith(("_", ".")) or "." in name or name in NOT_LIBRARIES
                    or not (entry.is_dir() or entry.suffix == ".py")):
                continue
            entries.append((entry, name))
        for entry, name in entries:
            if name in known or not _mentions_matplotlib(entry):
                continue
            self.libraries.append(name)
            self.add_package(entry, name, name)

    def add_corpus(self, root: pathlib.Path) -> None:
        """The programs of a corpus, as the library ``corpus:<name>``."""
        library = f"corpus:{root.name}"
        skipped = {name.split(".")[0] for name in self.libraries} | {"matplotlib", "mpl_toolkits"}
        found = False
        for path in _files(root, skipped):
            source = _notebook_source(path) if path.suffix == ".ipynb" else None
            tree = _parse(path, source)
            if tree is None:
                continue
            relative = ".".join(path.relative_to(root).with_suffix("").parts).replace("-", "_")
            name = f"corpus.{root.name.replace('-', '_')}.{relative}"
            self.modules[name] = Module(name, library, tree, False)
            found = True
        if found:
            self.libraries.append(library)

    # Resolving -----------------------------------------------------------

    def _split(self, dotted: str) -> tuple[Module, str] | None:
        """The read module ``dotted`` starts with, and the rest."""
        parts = dotted.split(".")
        for cut in range(len(parts) - 1, 0, -1):
            module = self.modules.get(".".join(parts[:cut]))
            if module is not None:
                return module, ".".join(parts[cut:])
        return None

    def resolve(self, dotted: str) -> list[Any]:
        """What ``dotted`` names: ``[class]`` for one of matplotlib's,
        ``[(module, ClassDef)]`` for a read class, ``[]`` if unknown."""
        if dotted in self._resolved:
            return self._resolved[dotted]
        if dotted in self._resolving:
            return []
        self._resolving.add(dotted)
        try:
            found = self._resolve(dotted)
        finally:
            self._resolving.discard(dotted)
        self._resolved[dotted] = found
        return found

    def _resolve(self, dotted: str) -> list[Any]:
        top = dotted.split(".")[0]
        split = self._split(dotted)
        if split is not None and top not in ("matplotlib", "mpl_toolkits"):
            module, rest = split
            head, _, tail = rest.partition(".")
            if not tail and head in module.classes:
                return [(module, module.classes[head])]
            bound = module.bindings.get(head)
            if bound is not None and bound != dotted:
                return self.resolve(f"{bound}.{tail}" if tail else bound)
            return []
        if top in ("matplotlib", "mpl_toolkits"):
            if split is not None and split[0].library in ("matplotlib", "mpl_toolkits"):
                found = runtime_class(dotted)
                return [found] if found is not None else []
            if split is not None:  # a library in the mpl_toolkits namespace
                module, rest = split
                if rest in module.classes:
                    return [(module, module.classes[rest])]
                bound = module.bindings.get(rest.partition(".")[0])
                return self.resolve(bound) if bound and bound != dotted else []
            found = runtime_class(dotted)
            return [found] if found is not None else []
        return []

    def bases(self, module: Module, node: ast.ClassDef) -> list[Any]:
        """The resolved bases of a read class."""
        found = []
        for base in node.bases:
            dotted = dotted_name(base)
            if dotted is None:
                continue
            absolute = module.resolve_local(dotted)
            if absolute is None and dotted in module.classes:
                absolute = f"{module.name}.{dotted}"
            if absolute is not None:
                found.extend(self.resolve(absolute))
        return found

    def matplotlib_ancestors(self, module: Module, node: ast.ClassDef, seen: set[int] | None = None) -> list[type]:
        """The nearest of matplotlib's classes among a read class's
        ancestors, in base order."""
        seen = set() if seen is None else seen
        if id(node) in seen:
            return []
        seen.add(id(node))
        found: list[type] = []
        for base in self.bases(module, node):
            if isinstance(base, type):
                found.append(base)
            else:
                found.extend(self.matplotlib_ancestors(*base, seen))
        return list(dict.fromkeys(found))


def runtime_class(dotted: str) -> type | None:
    """matplotlib's class ``dotted`` names, imported."""
    parts = dotted.split(".")
    for cut in range(len(parts) - 1, 0, -1):
        try:
            value: Any = importlib.import_module(".".join(parts[:cut]))
        except Exception:  # noqa: BLE001 - not a module, or one failing to import
            continue
        try:
            for attribute in parts[cut:]:
                value = getattr(value, attribute)
        except AttributeError:
            return None
        return value if isinstance(value, type) and value.__module__.startswith(("matplotlib", "mpl_toolkits")) \
            else None
    return None


def public_name(cls: type) -> str:
    """``cls`` by the shortest public module exposing it."""
    parts = cls.__module__.split(".")
    for cut in range(1, len(parts) + 1):
        candidate = parts[:cut]
        if any(part.startswith("_") for part in candidate):
            break
        try:
            if getattr(importlib.import_module(".".join(candidate)), cls.__qualname__, None) is cls:
                return f"{'.'.join(candidate)}.{cls.__qualname__}"
        except Exception:  # noqa: BLE001 - a module failing to import exposes nothing
            continue
    return f"{cls.__module__}.{cls.__qualname__}"


def _members(node: ast.ClassDef) -> Iterator[tuple[str, str]]:
    """``(name, kind)`` of what a class body defines."""
    for item in node.body:
        if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
            kind = "method"
            for decorator in item.decorator_list:
                name = dotted_name(decorator) or ""
                if name in ("property", "functools.cached_property", "cached_property") or name.endswith(
                        (".setter", ".getter", ".deleter")):
                    kind = "property"
                elif name in ("staticmethod", "classmethod"):
                    kind = name
            yield item.name, kind
        elif isinstance(item, ast.Assign):
            for target in item.targets:
                if isinstance(target, ast.Name):
                    yield target.id, "attribute"
        elif isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name) and item.value is not None:
            yield item.target.id, "attribute"


def _defining(cls: type, member: str) -> type | None:
    for klass in cls.__mro__:
        if klass is object:
            return None
        if member in vars(klass):
            return klass
    return None


def tabulate(census: Census) -> list[dict[str, Any]]:
    """The census' rows, most libraries first."""
    rows: dict[tuple[str, str], dict[str, Any]] = {}
    libraries: dict[tuple[str, str], set[str]] = defaultdict(set)
    for module in list(census.modules.values()):
        for node in module.classes.values():
            ancestors = census.matplotlib_ancestors(module, node)
            if not ancestors:
                continue
            for member, kind in dict(_members(node)).items():
                if member in OBJECT_MEMBERS and member not in ("__init__", "__call__", "__repr__", "__str__",
                                                               "__eq__", "__hash__", "__getstate__",
                                                               "__setstate__", "__reduce__", "__init_subclass__"):
                    continue
                for ancestor in ancestors:
                    defining = _defining(ancestor, member)
                    if defining is None:
                        continue
                    key = (public_name(ancestor), member)
                    row = rows.setdefault(key, {
                        "class": key[0], "member": member, "kind": _kind(defining, member, kind),
                        "defined_in": public_name(defining), "classes": []})
                    row["classes"].append(f"{module.name}.{node.name}")
                    libraries[key].add(module.library)
                    break
    table = []
    for key, row in rows.items():
        row["libraries"] = sorted(libraries[key])
        row["count"] = len(row["libraries"])
        row["classes"] = sorted(set(row["classes"]))
        table.append(row)
    table.sort(key=lambda r: (-r["count"], r["class"], r["member"]))
    return [{k: row[k] for k in ("class", "member", "kind", "defined_in", "count", "libraries", "classes")}
            for row in table]


def _kind(defining: type, member: str, given: str) -> str:
    """The member's kind in matplotlib's class (the override's if plain)."""
    value = vars(defining)[member]
    if isinstance(value, property) or type(value).__name__ in ("cached_property", "_classproperty"):
        return "property"
    if isinstance(value, staticmethod):
        return "staticmethod"
    if isinstance(value, classmethod):
        return "classmethod"
    if callable(value):
        return "method"
    return "attribute" if given in ("attribute", "method") else given


def main() -> int:
    """Read the sources, write the table."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=pathlib.Path,
                        default=pathlib.Path(__file__).resolve().parent / "overrides_census.json")
    parser.add_argument("sources", nargs="+", type=pathlib.Path)
    args = parser.parse_args()
    import matplotlib

    census = Census()
    installed = pathlib.Path(matplotlib.__file__).parent
    census.add_package(installed, "matplotlib", "matplotlib")
    census.add_package(installed.parent / "mpl_toolkits", "mpl_toolkits", "mpl_toolkits")
    census.libraries += ["matplotlib", "mpl_toolkits"]
    for source in args.sources:
        if source.name == "site-packages":
            census.add_site_packages(source)
    for source in args.sources:
        if source.name != "site-packages":
            for corpus in sorted(p for p in source.iterdir() if p.is_dir()):
                if f"corpus:{corpus.name}" not in census.libraries:
                    census.add_corpus(corpus)
    rows = tabulate(census)
    overriding = sorted({lib for row in rows for lib in row["libraries"]})
    table = {
        "libraries_read": len(census.libraries),
        "libraries_overriding": len(overriding),
        "classes": len({c for row in rows for c in row["classes"]}),
        "rows": rows,
    }
    args.out.write_text(json.dumps(table, indent=1) + "\n", encoding="utf-8")
    print(f"{len(census.libraries)} libraries read, {len(overriding)} override; {len(rows)} (class, member) rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
