"""matplotlib's private names, and the types its annotations give.

Imported by ``private_census.py``, under matplotlib (the reference venv).
`Names` walks matplotlib's and its toolkits' modules (their tests left
out), importing each, and records

* per class of matplotlib's: the private names (one leading underscore,
  not a dunder) its body defines, and those its methods assign on
  ``self`` (``self._x = ...``, read from the source with ``ast``), each
  with its kind (attribute, method, property, class);
* per module: its private module-level names (module function, class,
  constant, or a private submodule), and every name of a private module
  (``matplotlib._api.check_in_list``);
* the types matplotlib's annotations give (the ``.pyi`` stubs beside a
  module, else the module's own annotations): of a class' attributes and
  properties, and what its methods and the module's functions return, so
  that the reader can follow ``ax.xaxis`` or ``fig.add_subplot()`` to the
  class whose private name a library reaches.
"""

from __future__ import annotations

import ast
import importlib
import inspect
import pathlib
import pkgutil
import types
from typing import Any, Iterator

import overrides_census as oc

#: The packages read, and the subpackages left out (matplotlib's tests).
PACKAGES = ("matplotlib", "mpl_toolkits")
SKIPPED_PARTS = {"tests"}


#: Names with a leading underscore that are protocols, not private state:
#: named tuples' API.
PROTOCOL_NAMES = {"_asdict", "_replace", "_make", "_fields", "_field_defaults"}


def is_private(name: str) -> bool:
    """Whether ``name`` is private: one leading underscore, no dunder, no
    ``_sunder_`` name (IPython's ``_repr_html_``, enum's ``_value_``) nor
    named tuples' API."""
    return (name.startswith("_") and not name.startswith("__") and len(name) > 2
            and not name.endswith("_") and name not in PROTOCOL_NAMES)


def member_kind(value: Any) -> str:
    """The kind of a class member."""
    if isinstance(value, property) or type(value).__name__ in ("cached_property", "_classproperty"):
        return "property"
    if isinstance(value, (staticmethod, classmethod, types.FunctionType, types.BuiltinFunctionType,
                          types.MethodDescriptorType)) or inspect.isroutine(value):
        return "method"
    if isinstance(value, type):
        return "class"
    return "attribute"


def module_kind(value: Any) -> str:
    """The kind of a module-level name."""
    if isinstance(value, types.ModuleType):
        return "module"
    if isinstance(value, type):
        return "module class"
    if callable(value):
        return "module function"
    return "module constant"


def qualified(cls: type) -> str:
    """``module.Qualname`` of a class."""
    return f"{cls.__module__}.{cls.__qualname__}"


def _self_assigned(node: ast.ClassDef) -> set[str]:
    """The private names the methods of a class body assign on their first
    parameter."""
    found = set()
    for item in node.body:
        if not isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)) or not item.args.args:
            continue
        me = item.args.args[0].arg
        for sub in ast.walk(item):
            targets: list[ast.expr] = []
            if isinstance(sub, ast.Assign):
                targets = [t for target in sub.targets for t in _flatten(target)]
            elif isinstance(sub, (ast.AnnAssign, ast.AugAssign)):
                targets = [sub.target]
            for target in targets:
                if (isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name)
                        and target.value.id == me and is_private(target.attr)):
                    found.add(target.attr)
    return found


def _flatten(target: ast.expr) -> Iterator[ast.expr]:
    if isinstance(target, (ast.Tuple, ast.List)):
        for element in target.elts:
            yield from _flatten(element)
    elif isinstance(target, ast.Starred):
        yield from _flatten(target.value)
    else:
        yield target


def _class_nodes(tree: ast.Module) -> dict[str, ast.ClassDef]:
    """The classes of a module's source by qualname (nested ones too)."""
    found: dict[str, ast.ClassDef] = {}

    def visit(body: list[ast.stmt], prefix: str) -> None:
        for node in body:
            if isinstance(node, ast.ClassDef):
                found[prefix + node.name] = node
                visit(node.body, f"{prefix}{node.name}.")
            elif isinstance(node, (ast.If, ast.Try)):
                visit([*node.body, *node.orelse, *getattr(node, "finalbody", []),
                       *(s for h in getattr(node, "handlers", []) for s in h.body)], prefix)

    visit(tree.body, "")
    return found


class Annotations:
    """The types a module's annotations give: ``(class qualname, member)``
    or ``("module:" + module, function)`` -> (the annotation, the reader of
    the stub or module it is in)."""

    def __init__(self) -> None:
        self.members: dict[tuple[str, str], tuple[ast.expr, oc.Module]] = {}

    def read(self, module: str, tree: ast.Module) -> None:
        """Take the annotations of ``tree`` (the source of ``module``, or its
        stub)."""
        reader = oc.Module(module, "matplotlib", tree, False)
        for node in tree.body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.returns is not None:
                self.members.setdefault((f"module:{module}", node.name), (node.returns, reader))
        for qualname, node in _class_nodes(tree).items():
            key = f"{module}.{qualname}"
            for item in node.body:
                if isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
                    self.members.setdefault((key, item.target.id), (item.annotation, reader))
                elif isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)) and item.returns is not None:
                    self.members.setdefault((key, item.name), (item.returns, reader))
            for sub in ast.walk(node):
                # ``self._x: T = ...`` in a method.
                if (isinstance(sub, ast.AnnAssign) and isinstance(sub.target, ast.Attribute)
                        and isinstance(sub.target.value, ast.Name) and sub.target.value.id == "self"):
                    self.members.setdefault((key, sub.target.attr), (sub.annotation, reader))


class Names:
    """matplotlib's private names and the types of its members."""

    def __init__(self) -> None:
        #: class qualname -> the class
        self.classes: dict[str, type] = {}
        #: class qualname -> {private name: kind} the class itself defines
        self.own: dict[str, dict[str, str]] = {}
        #: private name -> the qualnames of the classes defining it
        self.definers: dict[str, set[str]] = {}
        #: module -> {name: kind}: its private names, or every name of a
        #: private module
        self.module_names: dict[str, dict[str, str]] = {}
        self.modules: dict[str, types.ModuleType] = {}
        self.unimportable: list[str] = []
        self.annotations = Annotations()
        for name, path in self._module_files():
            self._read_module(name, path)

    @staticmethod
    def _module_files() -> Iterator[tuple[str, pathlib.Path]]:
        for package in PACKAGES:
            root = pathlib.Path(importlib.import_module(package).__path__[0])
            for path in sorted(root.rglob("*.py")):
                parts = path.relative_to(root).with_suffix("").parts
                if SKIPPED_PARTS & set(parts):
                    continue
                parts = parts[:-1] if parts[-1] == "__init__" else parts
                yield ".".join([package, *parts]), path

    def _read_module(self, name: str, path: pathlib.Path) -> None:
        try:
            module = importlib.import_module(name)
        except Exception:  # noqa: BLE001 - a backend whose toolkit is missing
            self.unimportable.append(name)
            return
        self.modules[name] = module
        tree = oc._parse(path)
        if tree is None:
            return
        stub = path.with_suffix(".pyi")
        stub_tree = oc._parse(stub) if stub.exists() else None
        if stub_tree is not None:
            self.annotations.read(name, stub_tree)
        self.annotations.read(name, tree)
        private_module = any(is_private(part) for part in name.split("."))
        names = {}
        for attribute, value in vars(module).items():
            if attribute.startswith("__"):
                continue
            own = getattr(value, "__module__", name) == name or not isinstance(value, (type, types.FunctionType))
            if isinstance(value, types.ModuleType):
                own = value.__name__ == f"{name}.{attribute}"
            if (is_private(attribute) or private_module) and own:
                names[attribute] = module_kind(value)
        self.module_names[name] = names
        nodes = _class_nodes(tree)
        for qualname, node in nodes.items():
            cls = _lookup(module, qualname)
            if not isinstance(cls, type) or cls.__module__ != name:
                continue
            key = qualified(cls)
            own = {attribute: member_kind(value) for attribute, value in vars(cls).items()
                   if is_private(attribute)}
            for attribute in _self_assigned(node):
                own.setdefault(attribute, "attribute")
            self.classes[key] = cls
            self.own[key] = own
            for attribute in own:
                self.definers.setdefault(attribute, set()).add(key)

    def defining(self, cls: type, name: str) -> str | None:
        """The qualname of the class of ``cls``'s MRO defining the private
        ``name``, or None."""
        for klass in cls.__mro__:
            own = self.own.get(qualified(klass))
            if own is not None and name in own:
                return qualified(klass)
        return None

    def kind(self, owner: str, name: str) -> str:
        """The kind of ``owner``'s private ``name``."""
        return self.own[owner][name]

    def member_annotation(self, cls: type, member: str) -> tuple[ast.expr, oc.Module] | None:
        """The annotation of ``member`` along ``cls``'s MRO, if any."""
        for klass in cls.__mro__:
            found = self.annotations.members.get((qualified(klass), member))
            if found is not None:
                return found
        return None

    def function_annotation(self, module: str, function: str) -> tuple[ast.expr, oc.Module] | None:
        """The return annotation of the module function, if any."""
        return self.annotations.members.get((f"module:{module}", function))


def _lookup(module: types.ModuleType, qualname: str) -> Any:
    value: Any = module
    for part in qualname.split("."):
        value = getattr(value, part, None)
        if value is None:
            return None
    return value
