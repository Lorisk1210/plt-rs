"""What a library's expression is, as far as static reading can tell.

Imported by ``private_reader.py``. A `Kind` is what an expression holds,
when it is one of matplotlib's: an instance of some of its classes, a
class itself, one of its modules, a sequence or mapping of such things
(``fig.axes``, ``plt.subplots()[1]``, ``ax.spines``), or a tuple of them.
`Typer.annotation` turns matplotlib's annotations into kinds, and
`Typer.returned` knows the factories whose annotations say ``Any``.
"""

from __future__ import annotations

import ast
import functools
import importlib
import types
from dataclasses import dataclass
from typing import Any

import overrides_census as oc
from private_names import Names


@dataclass(frozen=True)
class Kind:
    """What an expression holds: ``what`` is ``instance`` (``classes``),
    ``class`` (``classes``, one), ``module`` (``module``), ``function``
    (``module`` is its dotted name), ``sequence`` or ``mapping`` (of
    ``inner``), ``tuple`` (``items``), or ``foreign``: something that is
    not matplotlib's (another library's module or name, a literal)."""

    what: str
    classes: tuple[type, ...] = ()
    module: str = ""
    inner: Kind | None = None
    items: tuple[Kind | None, ...] = ()

    @staticmethod
    def instance(*classes: type) -> Kind:
        """An instance of any of ``classes``."""
        return Kind("instance", tuple(dict.fromkeys(classes)))

    def element(self) -> Kind | None:
        """What indexing or iterating this gives."""
        if self.what in ("sequence", "mapping"):
            return self.inner
        if self.what == "tuple":
            known = [item for item in self.items if item is not None]
            return known[0] if len(set(known)) == 1 and len(known) == len(self.items) else None
        return None


#: Factories returning a figure and an array of Axes (or a dict of them),
#: whose annotations say ``Any``.
FIGURE_AND_AXES = {"matplotlib.pyplot.subplots": "sequence", "matplotlib.pyplot.subplot_mosaic": "mapping"}
#: Methods of figures returning an array or dict of Axes.
AXES_OF_FIGURE = {"subplots": "sequence", "subplot_mosaic": "mapping"}
#: Attributes and methods of a sequence of Axes (numpy's) giving one again.
SEQUENCE_VIEWS = {"flat", "ravel", "flatten", "T", "reshape", "squeeze", "tolist", "transpose", "values"}
#: Container annotations whose subscript is what they hold.
CONTAINERS = {"list", "List", "Sequence", "MutableSequence", "Iterable", "Iterator", "Collection", "set",
              "frozenset", "ArrayLike", "ndarray", "Generator"}
MAPPINGS = {"dict", "Dict", "Mapping", "MutableMapping", "OrderedDict"}


class Typer:
    """Kinds of matplotlib's annotations and factories."""

    def __init__(self, names: Names) -> None:
        self.names = names
        figure = importlib.import_module("matplotlib.figure")
        axes = importlib.import_module("matplotlib.axes")
        self.figure = Kind.instance(figure.Figure)
        self.axes = Kind.instance(axes.Axes)
        self._cache: dict[int, Kind | None] = {}

    def of_value(self, value: Any) -> Kind | None:
        """The kind of a runtime value of matplotlib's (a module, a class)."""
        if isinstance(value, types.ModuleType) and value.__name__.startswith(("matplotlib", "mpl_toolkits")):
            return Kind("module", module=value.__name__)
        if isinstance(value, type) and value.__module__.startswith(("matplotlib", "mpl_toolkits")):
            return Kind("class", (value,))
        if isinstance(value, (types.FunctionType, types.BuiltinFunctionType)) and (
                value.__module__ or "").startswith(("matplotlib", "mpl_toolkits")):
            return Kind("function", module=f"{value.__module__}.{value.__name__}")
        if type(value).__module__.startswith(("matplotlib", "mpl_toolkits")):
            return Kind.instance(type(value))
        return None

    @functools.cache  # noqa: B019 - one typer per census
    def of_dotted(self, dotted: str) -> Kind | None:
        """The kind of matplotlib's name ``dotted`` (``matplotlib.pyplot``,
        ``matplotlib.axes.Axes``)."""
        if not dotted.startswith(("matplotlib", "mpl_toolkits")):
            return None
        parts = dotted.split(".")
        for cut in range(len(parts), 0, -1):
            module = self.names.modules.get(".".join(parts[:cut]))
            if module is None:
                try:
                    module = importlib.import_module(".".join(parts[:cut]))
                except Exception:  # noqa: BLE001 - not a module
                    continue
            value: Any = module
            for part in parts[cut:]:
                value = getattr(value, part, None)
                if value is None:
                    return None
            return self.of_value(value)
        return None

    def annotation(self, node: ast.expr | None, reader: oc.Module) -> Kind | None:
        """The kind an annotation of matplotlib's names."""
        if node is None:
            return None
        key = id(node)
        if key not in self._cache:
            self._cache[key] = None  # recursion guard
            self._cache[key] = self._annotation(node, reader)
        return self._cache[key]

    def _annotation(self, node: ast.expr, reader: oc.Module) -> Kind | None:
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            try:
                return self.annotation(ast.parse(node.value, mode="eval").body, reader)
            except SyntaxError:
                return None
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.BitOr):
            return _union([self.annotation(node.left, reader), self.annotation(node.right, reader)])
        if isinstance(node, ast.Subscript):
            head = (oc.dotted_name(node.value) or "").rpartition(".")[2]
            args = node.slice.elts if isinstance(node.slice, ast.Tuple) else [node.slice]
            if head in ("Optional", "Union"):
                return _union([self.annotation(arg, reader) for arg in args])
            if head == "type":
                inner = self.annotation(args[0], reader)
                return Kind("class", inner.classes) if inner and inner.what == "instance" else None
            if head == "tuple":
                if len(args) == 2 and isinstance(args[1], ast.Constant) and args[1].value is Ellipsis:
                    inner = self.annotation(args[0], reader)
                    return Kind("sequence", inner=inner) if inner else None
                return Kind("tuple", items=tuple(self.annotation(arg, reader) for arg in args))
            if head in CONTAINERS:
                inner = self.annotation(args[0], reader)
                return Kind("sequence", inner=inner) if inner else None
            if head in MAPPINGS:
                inner = self.annotation(args[-1], reader)
                return Kind("mapping", inner=inner) if inner else None
            return None
        dotted = oc.dotted_name(node)
        if dotted is None:
            return None
        absolute = reader.resolve_local(dotted) or f"{reader.name}.{dotted}"
        kind = self.of_dotted(absolute)
        if kind is not None and kind.what == "class":
            return Kind.instance(*kind.classes)
        return None

    def member(self, cls: type, member: str) -> Kind | None:
        """What ``instance.member`` holds, by matplotlib's annotations (a
        method's: what calling it returns, see `returned`)."""
        found = self.names.member_annotation(cls, member)
        return self.annotation(*found) if found else None

    def returned(self, name: str, owner: Kind | None) -> Kind | None:
        """What calling ``owner.name`` returns (``owner`` a module, a class
        or an instance)."""
        if owner is not None and owner.what == "module":
            dotted = f"{owner.module}.{name}"
            if dotted in FIGURE_AND_AXES:
                inner = Kind(FIGURE_AND_AXES[dotted], inner=self.axes)
                return Kind("tuple", items=(self.figure, inner))
            found = self.names.function_annotation(owner.module, name)
            return self.annotation(*found) if found else None
        if owner is not None and owner.what == "instance":
            if name in AXES_OF_FIGURE and any(c.__name__.endswith("Figure") for c in owner.classes):
                return Kind(AXES_OF_FIGURE[name], inner=self.axes)
            return _union([self.member(cls, name) for cls in owner.classes])
        if owner is not None and owner.what == "class":
            return _union([self.member(cls, name) for cls in owner.classes])
        return None


def _union(kinds: list[Kind | None]) -> Kind | None:
    """The kind of either: instances of all the classes (None, a missing
    value, dropped), or the one kind when they agree."""
    known = [kind for kind in kinds if kind is not None]
    if not known:
        return None
    if all(kind.what == "instance" for kind in known):
        return Kind.instance(*(cls for kind in known for cls in kind.classes))
    return known[0] if all(kind == known[0] for kind in known) else None
