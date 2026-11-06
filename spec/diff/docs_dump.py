"""What one implementation shows of matplotlib's public callables: their
signatures, docstrings and names, for ``docs.py``; and of the public names
of matplotlib's modules.

    python docs_dump.py API_JSONL OUT_JSON --names OUT_NAMES [--names-of MPL_NAMES]

``API_JSONL`` holds ``spec/api-audit/enumerate.py``'s records (taken under
matplotlib). Each callable among them (``api_index.CALLABLE_KINDS``) is
looked up in the implementation that ``import matplotlib`` gives (pltrs
through its alias, ``python/alias``, so that its names are those programs
importing matplotlib read), and described:

* ``found``: whether the implementation has it;
* ``form``: what its owner holds (``function``, ``class``, ``property``,
  ``staticmethod``, ``classmethod``, ``native`` for a function or class of
  a C or pybind11 extension, or the type of another callable object);
* ``signature``: ``inspect.signature``'s parameters, each ``[name, kind,
  default]`` (the default's repr, memory addresses left out; None
  without one), or None where Python cannot read one; a property's
  accessors instead (``getter``, ``setter``, ``deleter``);
* ``doc``: ``__doc__`` as importing left it (after matplotlib's
  interpolation); for a class, also its ``__init__``'s (``init_doc``);
  where it is None, ``inherited_doc`` (``init_inherited_doc``) is the
  docstring ``inspect.getdoc`` finds on a base class instead;
* ``name``: ``__module__`` and ``__qualname__``.

A class member is looked up along its class' MRO, as a program reads it,
so that a member inherited from a base or a mixin counts as the class';
a member of a private base (``_AxesBase``) along the MRO of the base's
first public user (``Axes``), pltrs's private classes being its own.

``OUT_NAMES`` describes the public names of matplotlib's modules: under
matplotlib, of every module of ``matplotlib``, ``mpl_toolkits`` and
``pylab`` that imports, but private ones and test packages (``tests``);
under pltrs, of the modules and names ``MPL_NAMES`` (matplotlib's
``OUT_NAMES``) lists, every one imported first, as walking matplotlib's
modules imports them. A module's public names are those ``dir`` lists
without a leading underscore, and its ``__all__``. Each name is described
by its value's type (``type(value).__name__``) and, for a module, the
module's name, for a value of its own repr that is hashable (a number, a
string, an enum member, dateutil's weekdays, a tuple of them), its repr.
"""

from __future__ import annotations

import argparse
import importlib
import inspect
import json
import os
import pathlib
import pkgutil
import re
import sys
import types
import warnings
from typing import Any

#: The kinds of ``enumerate.py``'s records that are callables.
CALLABLE_KINDS = ("function", "class", "method", "property", "classmethod", "staticmethod")
#: Callable objects of extension modules (C, pybind11).
NATIVE_TYPES = (types.BuiltinFunctionType, types.WrapperDescriptorType, types.MethodDescriptorType,
                types.ClassMethodDescriptorType, types.MethodWrapperType)
#: The forms of ``_form`` other than callable objects of another type.
STANDARD_FORMS = ("staticmethod", "classmethod", "property", "native", "class", "function")
ADDRESS = re.compile(r" at 0x[0-9a-fA-F]+")
#: The top-level packages and modules whose modules' names are described.
NAME_ROOTS = ("matplotlib", "mpl_toolkits", "pylab")
#: The longest repr of a value described by its repr.
LONGEST_REPR = 200


class Implementation:
    """The modules of the implementation ``import matplotlib`` gives."""

    def __init__(self) -> None:
        self._modules: dict[str, types.ModuleType | None] = {}

    def module(self, name: str) -> types.ModuleType | None:
        """matplotlib's module ``name`` in the implementation, if it has it."""
        if name not in self._modules:
            try:
                self._modules[name] = importlib.import_module(name)
            except ImportError:
                self._modules[name] = None
        return self._modules[name]

    def owner(self, module_name: str, qualname: str) -> type | None:
        """The class ``qualname`` of ``module_name``, if the implementation
        has it."""
        module = self.module(module_name)
        return _class_by_qualname(module, qualname) if module is not None else None


def _class_by_qualname(module: types.ModuleType, qualname: str) -> type | None:
    obj: Any = module
    for part in qualname.split("."):
        obj = getattr(obj, part, None)
    if inspect.isclass(obj):
        return obj
    return next((v for v in vars(module).values() if inspect.isclass(v) and v.__qualname__ == qualname), None)


def _is_native(obj: Any) -> bool:
    """Whether ``obj`` is a function or class of an extension module."""
    if isinstance(obj, NATIVE_TYPES) or type(obj).__name__ == "instancemethod":
        # (pybind11 holds its methods in instancemethod objects.)
        return True
    if inspect.isclass(obj):
        path = getattr(sys.modules.get(obj.__module__), "__file__", None)
        return path is not None and not path.endswith(".py")
    return False


def _form(raw: Any) -> str:
    """What an owner holds, as ``docs.py`` compares it."""
    if isinstance(raw, staticmethod):
        return "staticmethod"
    if isinstance(raw, classmethod):
        return "classmethod"
    if isinstance(raw, property):
        return "property"
    if _is_native(raw):
        return "native"
    if inspect.isclass(raw):
        return "class"
    if inspect.isfunction(raw):
        return "function"
    return type(raw).__name__


class Describer:
    """Describes callables of one implementation."""

    def __init__(self, implementation: Implementation) -> None:
        self.implementation = implementation

    def _repr(self, value: Any) -> str:
        try:
            text = repr(value)
        except Exception as err:  # noqa: BLE001 - a broken repr is described, not raised
            text = f"<repr raised {type(err).__name__}>"
        return ADDRESS.sub("", text)

    def signature(self, obj: Any) -> list[list[Any]] | None:
        """The parameters of ``obj``, None where Python cannot read them."""
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                signature = inspect.signature(obj)
        except (TypeError, ValueError):
            return None
        return [[p.name, p.kind.name, None if p.default is p.empty else self._repr(p.default)]
                for p in signature.parameters.values()]

    def name(self, obj: Any) -> list[str] | None:
        """``__module__`` and ``__qualname__``."""
        module, qualname = getattr(obj, "__module__", None), getattr(obj, "__qualname__", None)
        if not isinstance(module, str) or not isinstance(qualname, str):
            return None
        return [module, qualname]

    def describe(self, raw: Any) -> dict[str, Any]:
        """What ``raw``, as its owner holds it, shows."""
        form = _form(raw)
        if form == "property":
            accessors = {"getter": raw.fget is not None, "setter": raw.fset is not None,
                         "deleter": raw.fdel is not None}
            return {"found": True, "form": form, "signature": accessors, "doc": raw.__doc__,
                    "name": self.name(raw.fget) if raw.fget is not None else None}
        target = raw.__func__ if form in ("staticmethod", "classmethod") else raw
        description = {"found": True, "form": form, "signature": self.signature(target),
                       "doc": getattr(target, "__doc__", None), "inherited_doc": _inherited_doc(target),
                       "name": self.name(target)}
        init = None
        if inspect.isclass(target):
            init = next((vars(c)["__init__"] for c in target.__mro__ if "__init__" in vars(c)), None)
            if init is not None and init is not object.__init__:
                description["init_doc"] = init.__doc__
                description["init_inherited_doc"] = _inherited_doc(init)
        if form == "native":
            self._read_pybind11(description, getattr(target, "__name__", ""), init is not None)
        return description

    @staticmethod
    def _read_pybind11(description: dict[str, Any], name: str, is_class: bool) -> None:
        """A native callable's signature from the line pybind11 puts first
        in its docstring (a class' constructor's, without ``self``), and
        its docstring without that line."""
        field, function = ("init_doc", "__init__") if is_class else ("doc", name)
        parsed = _pybind11_signature(description.get(field), function)
        if parsed is None:
            return
        params, rest = parsed
        description[field] = rest
        if description["signature"] is None:
            description["signature"] = params[1:] if is_class else params


def _inherited_doc(obj: Any) -> str | None:
    """For a callable without a docstring of its own, the one
    ``inspect.getdoc`` (and so ``help``) shows: its base class'."""
    if getattr(obj, "__doc__", None) is not None:
        return None
    try:
        return inspect.getdoc(obj)
    except Exception:  # noqa: BLE001 - an object Python cannot introspect has none
        return None


def _split_top_level(text: str) -> list[str]:
    """``text`` split at the commas outside brackets."""
    parts, depth, start = [], 0, 0
    for index, char in enumerate(text):
        if char in "([{<":
            depth += 1
        elif char in ")]}>":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(text[start:index])
            start = index + 1
    parts.append(text[start:])
    return [part.strip() for part in parts if part.strip()]


def _pybind11_signature(doc: str | None, name: str) -> tuple[list[list[Any]], str] | None:
    """The parameters of the signature line ``name(...) -> type`` that
    pybind11 starts the docstring with, and the docstring after it."""
    if not doc or not doc.startswith(name + "("):
        return None
    depth = 0
    for end, char in enumerate(doc[len(name):], len(name)):
        depth += char in "([{<"
        depth -= char in ")]}>"
        if depth == 0:
            break
    else:
        return None
    params, kind = [], "POSITIONAL_OR_KEYWORD"
    for part in _split_top_level(doc[len(name) + 1:end]):
        if part == "*":
            kind = "KEYWORD_ONLY"
            continue
        head, _, default = part.partition(" = ")
        params.append([head.partition(":")[0].strip(), kind, default.strip() or None])
    rest = doc[end + 1:].partition("\n")[2]
    return params, rest


def public_modules() -> list[str]:
    """matplotlib's public modules (no part of the name private, no test
    package) that import, by name."""
    names = []
    for root in NAME_ROOTS:
        package = importlib.import_module(root)
        names.append(root)
        if not hasattr(package, "__path__"):
            continue
        walked = pkgutil.walk_packages(package.__path__, f"{root}.", onerror=lambda name: None)
        names += [info.name for info in walked
                  if not any(part.startswith("_") or part == "tests"
                             for part in info.name.split("."))]
    imported = []
    for name in sorted(set(names)):
        try:
            importlib.import_module(name)
        except Exception:  # noqa: BLE001 - a module needing what is not installed is left out
            continue
        imported.append(name)
    return imported


def public_names(module: types.ModuleType) -> list[str]:
    """The public names of *module*: those ``dir`` lists without a leading
    underscore, and its ``__all__``."""
    listed = getattr(module, "__all__", ())
    return sorted({name for name in dir(module) if not name.startswith("_")}
                  | {name for name in listed if isinstance(name, str)})


class NameDescriber:
    """Describes the values of module-level names of one implementation."""

    def __init__(self, implementation: Implementation) -> None:
        self.implementation = implementation

    def value(self, obj: Any) -> str | None:
        """What a value is compared by beside its type: a module's name,
        else its repr for a hashable value with a repr of its own."""
        if isinstance(obj, types.ModuleType):
            return obj.__name__
        if inspect.isclass(obj) or inspect.isroutine(obj) or type(obj).__repr__ is object.__repr__:
            return None
        try:
            hash(obj)
            text = repr(obj)
        except Exception:  # noqa: BLE001 - an unhashable value, or a broken repr, has none
            return None
        if ADDRESS.search(text) or len(text) > LONGEST_REPR:
            return None
        return text

    def describe(self, module_name: str, names: list[str]) -> dict[str, Any] | None:
        """The names *names* of the module *module_name*, None without the
        module."""
        try:
            module = self.implementation.module(module_name)
        except Exception:  # noqa: BLE001 - a module failing to import is missing
            module = None
        if module is None:
            return None
        described = {}
        for name in names:
            try:
                obj = getattr(module, name)
            except Exception:  # noqa: BLE001 - a name failing to resolve is missing
                described[name] = {"found": False}
                continue
            described[name] = {"found": True, "type": type(obj).__name__, "value": self.value(obj)}
        return described


def describe_module_names(implementation: Implementation, of: dict[str, Any] | None) -> dict[str, Any]:
    """The public names of matplotlib's modules (see the module
    documentation): those *of* lists (matplotlib's description), or, for
    matplotlib itself, all."""
    describer = NameDescriber(implementation)
    if of is None:
        modules = {name: public_names(importlib.import_module(name)) for name in public_modules()}
    else:
        modules = {name: [n for n, d in names.items() if d["found"]] for name, names in of.items()}
        # Each module imported first, as walking matplotlib's imports them:
        # a package then has its submodules as attributes in both.
        for name in modules:
            try:
                implementation.module(name)
            except Exception:  # noqa: BLE001 - a module failing to import is missing
                pass
    return {module: describer.describe(module, names) for module, names in modules.items()}


def lookup(implementation: Implementation, record: dict[str, Any],
           private_users: dict[str, tuple[str, str]]) -> tuple[bool, Any]:
    """Whether the implementation has the record's callable, and what its
    owner holds."""
    if record["owner"]:
        # A member of a private base is read off its public user, as
        # programs read it (pltrs's private classes are its own).
        user = private_users.get(record["owner"], (record["module"], record["owner"]))
        owner = implementation.owner(*user)
        if owner is None:
            return False, None
        try:
            raw = inspect.getattr_static(owner, record["name"])
        except AttributeError:
            return False, None
        if _form(raw) not in STANDARD_FORMS and hasattr(type(raw), "__get__"):
            # A descriptor making a function when it is read (pltrs's
            # artists make their ``set`` so) counts as that function.
            made = raw.__get__(None, owner)
            if inspect.isfunction(made):
                return True, made
        return True, raw
    module = implementation.module(record["module"])
    if module is None or not hasattr(module, record["name"]):
        return False, None
    return True, getattr(module, record["name"])


def main() -> int:
    """Describe every callable of the records and write them."""
    parser = argparse.ArgumentParser()
    parser.add_argument("api")
    parser.add_argument("out")
    parser.add_argument("--names", required=True)
    parser.add_argument("--names-of")
    args = parser.parse_args()
    os.environ.setdefault("MPLBACKEND", "agg")
    warnings.simplefilter("ignore")
    records = [json.loads(line) for line in pathlib.Path(args.api).read_text().splitlines() if line.strip()]
    private_users = {r["name"]: (r["module"], r["bases"][0]) for r in records if r["kind"] == "private_base"}
    implementation = Implementation()
    describer = Describer(implementation)
    described = {}
    for record in records:
        if record["kind"] not in CALLABLE_KINDS:
            continue
        key = f"{record['module']}:{record['owner'] + '.' if record['owner'] else ''}{record['name']}"
        found, raw = lookup(implementation, record, private_users)
        described[key] = describer.describe(raw) if found else {"found": False}
        described[key]["kind"] = record["kind"]
    pathlib.Path(args.out).write_text(json.dumps(described, indent=0))
    of = json.loads(pathlib.Path(args.names_of).read_text()) if args.names_of else None
    names = describe_module_names(implementation, of)
    pathlib.Path(args.names).write_text(json.dumps(names, indent=0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
