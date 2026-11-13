"""Writes the table of matplotlib's plain fields into
``python/pltrs/_artist_private.py`` (between its lines `BEGIN` and `END`).

    python private_fields.py [--census CENSUS] [--pltrs PACKAGE] [--out FILE]

(a table inside the module rather than a module of its own, so that
importing pltrs loads no module more).

Runs under matplotlib (the reference venv), so that it can be redone for
the next matplotlib. A *plain field* is a private attribute of a class of
matplotlib's that a public getter of the class returns as it is
(``def get_text(self): return self._text``): its value is the getter's,
whatever the class keeps it as. pltrs keeps most of these in pltrs
(Rust), so its objects have no such attribute; ``_artist_private.py``
answers a read of one through pltrs's own getter of the class defining it
(the table's ``getter``).

Where the census (``private_census.json``) shows libraries writing the
field, and matplotlib's class has the public setter of the same name
(``set_text``), the table names the setter too, and pltrs's class gets a
descriptor routing the assignment to it (pltrs's own setter, so that the
next draw, getter and pickle see the value, as they see matplotlib's
field). A field pltrs's own code assigns on its objects (``self._x =``
anywhere in ``PACKAGE``) is left out of that: the descriptor would take
pltrs's own assignments.

The table is keyed by the class's module (without its top-level package,
but for the toolkits) and qualname (``lines.Line2D``,
``mpl_toolkits.mplot3d.art3d.Line3D``), as pltrs's classes are.
"""

from __future__ import annotations

import argparse
import ast
import inspect
import json
import pathlib
import pprint
import sys
import textwrap

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import overrides_census as oc  # noqa: E402
from private_names import Names, _self_assigned, is_private  # noqa: E402

#: The lines the table sits between in ``_artist_private.py``.
BEGIN = "# matplotlib's plain fields, written by spec/diff/private_fields.py from matplotlib {version}:"
END = "# (end of the plain fields)"


def plain_getters(cls: type) -> dict[str, str]:
    """``{field: getter}`` of the getters ``cls``'s own body defines that
    return a private attribute as it is."""
    try:
        source = textwrap.dedent(inspect.getsource(cls))
    except (OSError, TypeError):
        return {}
    node = ast.parse(source).body[0]
    found = {}
    for item in getattr(node, "body", []):
        if not isinstance(item, ast.FunctionDef) or not item.name.startswith("get_"):
            continue
        body = [s for s in item.body if not (isinstance(s, ast.Expr) and isinstance(s.value, ast.Constant))]
        if len(body) != 1 or not isinstance(body[0], ast.Return):
            continue
        value = body[0].value
        if (isinstance(value, ast.Attribute) and isinstance(value.value, ast.Name)
                and value.value.id == "self" and is_private(value.attr)):
            found[value.attr] = item.name
    return found


class PltrsAssignments:
    """The private names pltrs's code assigns on objects of a class of its
    own: on ``self`` in the class or its bases (read as GEN5's census reads
    libraries), or on another object anywhere (``line._x = ...``)."""

    def __init__(self, package: pathlib.Path) -> None:
        self.census = oc.Census()
        self.census.add_package(package, "pltrs", "pltrs")
        self.elsewhere: set[str] = set()
        for module in self.census.modules.values():
            for node in ast.walk(module.tree):
                targets = node.targets if isinstance(node, ast.Assign) else (
                    [node.target] if isinstance(node, (ast.AugAssign, ast.AnnAssign)) else [])
                for target in targets:
                    if (isinstance(target, ast.Attribute) and is_private(target.attr)
                            and not (isinstance(target.value, ast.Name) and target.value.id in ("self", "cls"))):
                        self.elsewhere.add(target.attr)
                if (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "setattr"
                        and len(node.args) > 1 and isinstance(node.args[1], ast.Constant)):
                    self.elsewhere.add(str(node.args[1].value))

    def of(self, key: str) -> set[str] | None:
        """The names assigned on objects of pltrs's class ``key``, None
        when pltrs has no such class."""
        module, _, qualname = key.rpartition(".")
        found = self.census.resolve(f"pltrs.{module}.{qualname}")
        if not found or isinstance(found[0], type):
            return None
        names = set(self.elsewhere)
        pending, seen = [found[0]], set()
        while pending:
            module_, node = pending.pop()
            if id(node) in seen:
                continue
            seen.add(id(node))
            names |= _self_assigned(node)
            pending.extend(base for base in self.census.bases(module_, node) if not isinstance(base, type))
        return names


def module_key(module: str) -> str:
    """A module as the table names it: matplotlib's without the package
    name (pltrs's are named apart), the toolkits' as they are."""
    return module if module.startswith("mpl_toolkits.") else module.partition(".")[2]


def main() -> int:
    """Find the fields, write the table."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--census", type=pathlib.Path, default=HERE / "private_census.json")
    parser.add_argument("--pltrs", type=pathlib.Path, default=HERE.parent.parent / "python" / "pltrs")
    parser.add_argument("--out", type=pathlib.Path)
    args = parser.parse_args()
    out = args.out or args.pltrs / "_artist_private.py"
    import matplotlib

    names = Names()
    census = json.loads(args.census.read_text())["rows"]
    written = {(row["owner"], row["name"]) for row in census if "written" in row["uses"]}
    written |= {(cls, row["name"]) for row in census if "written" in row["uses"] for cls in row["candidates"]}
    assignments = PltrsAssignments(args.pltrs)
    table: dict[str, dict[str, tuple[str, str | None]]] = {}
    for qualified, cls in sorted(names.classes.items()):
        key = f"{module_key(cls.__module__)}.{cls.__qualname__}"
        assigned = assignments.of(key)
        for field, getter in sorted(plain_getters(cls).items()):
            setter = "set_" + getter[len("get_"):]
            writable = ((qualified, field) in written and callable(getattr(cls, setter, None))
                        and assigned is not None and field not in assigned)
            table.setdefault(key, {})[field] = (getter, setter if writable else None)
    text = out.read_text(encoding="utf-8")
    start = text.index(BEGIN.partition("{version}")[0])
    end = text.index(END, start)
    block = (BEGIN.format(version=matplotlib.__version__) + "\n#: class -> {field: (getter, setter or None)}\n"
             + "FIELDS = " + pprint.pformat(table, width=96, sort_dicts=True) + "\n")
    out.write_text(text[:start] + block + text[end:], encoding="utf-8")
    fields = sum(map(len, table.values()))
    setters = sum(setter is not None for entry in table.values() for _, setter in entry.values())
    print(f"{fields} fields of {len(table)} classes, {setters} of them assignable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
