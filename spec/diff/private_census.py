"""The census of matplotlib's private names that other libraries use.

    python private_census.py [--out FILE] SOURCE...

Runs under matplotlib (the reference venv), so that it can be redone for
the next matplotlib. The SOURCEs are read as ``overrides_census.py``
reads them (whose reader this shares): ``site-packages`` directories,
each top-level package mentioning matplotlib a library (the first source
having it counts), and trees of programs (the corpora fetched into
``target/diff/corpora``, each the library ``corpus:<name>``). A directory
holding a copy of matplotlib's own code (``matplotlib``, ``mpl_toolkits``,
``vendor`` and ``_vendor`` inside a library) and the libraries' tests are
not read.

matplotlib's private names (`private_names.Names`: one leading
underscore, defined by a class of matplotlib's or assigned on its
``self``, or at module level of matplotlib's modules, and every name of a
private module) are looked for in every module of the libraries with
``ast`` (`private_reader.Reader`, which says how receivers are followed).

The table (``spec/diff/private_census.json``) has a row per (owner, name)
the libraries use: the owner is the class or module of matplotlib's
defining the name (None where the receiver is unknown: ``candidates``
then names the classes defining it), its kind (attribute, method,
property, class; module function, module class, module constant,
module), how the libraries use it (read, called, written, deleted,
overridden in a subclass) and which, and one place per library.
``private.py`` (``cargo xtask diff --private``) measures pltrs against it.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from collections import defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import overrides_census as oc  # noqa: E402
from private_names import Names, _self_assigned, is_private  # noqa: E402
from private_reader import Reader, Use  # noqa: E402
from private_types import Typer  # noqa: E402

#: The uses, in the order the table lists them.
USES = ("read", "called", "written", "deleted", "overridden")
#: Directories of a library holding copies of matplotlib's code.
COPIES = {"matplotlib", "mpl_toolkits", "vendor", "_vendor", "vendored", "_vendored"}
#: Top-level packages of a ``site-packages`` that are no library (a
#: wheel's stray test or documentation directory).
NO_LIBRARIES = {"tests", "testing", "test", "doc", "docs", "examples", "benchmarks"}


class Tally:
    """The uses recorded, by (name, owner)."""

    def __init__(self, names: Names) -> None:
        self.names = names
        self.uses: dict[tuple[str, str | None], dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
        self.where: dict[tuple[str, str | None], dict[str, str]] = defaultdict(dict)
        self.candidates: dict[tuple[str, str | None], tuple[str, ...]] = {}

    def record(self, use: Use) -> None:
        """Count one use."""
        key = (use.name, use.owner)
        self.uses[key][use.use].add(use.library)
        self.where[key].setdefault(use.library, use.where)
        if use.owner is None:
            self.candidates[key] = use.candidates

    def kind(self, name: str, owner: str | None) -> str:
        """The kind of the name: its owner's, or its candidates' (joined
        where they differ)."""
        if owner in self.names.own:
            return self.names.kind(owner, name)
        if owner is not None:
            return self.names.module_names[owner][name]
        return "/".join(sorted({self.names.kind(c, name) for c in self.candidates[(name, owner)]}))

    def rows(self) -> list[dict[str, Any]]:
        """The table, the names most libraries use first."""
        by_name: dict[str, set[str]] = defaultdict(set)
        for (name, _), uses in self.uses.items():
            for libraries in uses.values():
                by_name[name] |= libraries
        rows = []
        for (name, owner), uses in self.uses.items():
            libraries = sorted(set().union(*uses.values()))
            rows.append({
                "name": name,
                "owner": owner,
                "candidates": list(self.candidates.get((name, owner), ())),
                "kind": self.kind(name, owner),
                "uses": {use: sorted(uses[use]) for use in USES if uses.get(use)},
                "libraries": libraries,
                "count": len(libraries),
                "name_count": len(by_name[name]),
                "where": dict(sorted(self.where[(name, owner)].items())),
            })
        rows.sort(key=lambda r: (-r["name_count"], r["name"], r["owner"] is None, r["owner"] or ""))
        return rows


def _library_names(census: oc.Census) -> tuple[dict[str, set[str]], set[str]]:
    """The private names each library's own classes (those not deriving
    from matplotlib's) define, and all of them together."""
    own: dict[str, set[str]] = defaultdict(set)
    for module in census.modules.values():
        if module.library in ("matplotlib", "mpl_toolkits"):
            continue
        for node in (n for n in oc.ast.walk(module.tree) if isinstance(n, oc.ast.ClassDef)):
            if census.matplotlib_ancestors(module, node):
                continue
            own[module.library] |= {m for m, _ in oc._members(node) if is_private(m)} | _self_assigned(node)
    return own, set().union(*own.values()) if own else set()


def _not_copies(census: oc.Census) -> None:
    """Drop the modules of a library that are copies of matplotlib's code,
    and the packages that are no library."""
    census.libraries = [library for library in census.libraries if library not in NO_LIBRARIES]
    for name in [n for n, m in census.modules.items()
                 if m.library not in ("matplotlib", "mpl_toolkits") and not m.library.startswith("corpus:")]:
        parts = name.split(".")[1:-1]
        if COPIES & set(parts) or census.modules[name].library in NO_LIBRARIES:
            del census.modules[name]


def main() -> int:
    """Read the sources, write the table."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=pathlib.Path, default=HERE / "private_census.json")
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
    _not_copies(census)
    own, foreign = _library_names(census)
    tally = Tally(names)
    typer = Typer(names)
    unread: list[str] = []
    for module in list(census.modules.values()):
        if module.library in ("matplotlib", "mpl_toolkits"):
            continue
        try:
            Reader(census, names, typer, own[module.library], foreign, tally.record).read(module)
        except RecursionError:  # a generated module of deeply nested expressions
            unread.append(module.name)
    rows = tally.rows()
    using = sorted({library for row in rows for library in row["libraries"]})
    table = {
        "matplotlib": matplotlib.__version__,
        "libraries_read": sorted(census.libraries),
        "libraries_using": len(using),
        "names": len({row["name"] for row in rows}),
        "unimportable_modules": names.unimportable,
        "unread_modules": unread,
        "rows": rows,
    }
    args.out.write_text(json.dumps(table, indent=1) + "\n", encoding="utf-8")
    print(f"{len(census.libraries)} libraries read, {len(using)} use {table['names']} private names "
          f"of matplotlib's in {len(rows)} (owner, name) rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
