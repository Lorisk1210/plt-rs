"""matplotlib's public callables, as ``spec/api-audit/enumerate.py`` lists
them, resolved to the objects they are: what the coverage of the
differential tester (``coverage.py``, ``reach.py``) counts.

The enumeration takes matplotlib's modules and classes as the API audit
does (``enumerate.py`` writes one JSON record per name). Of its records,
the callables count: functions, classes, methods, properties, class and
static methods, and the members of the private bases of public classes.
Each callable has a key, ``module:Owner.name`` (``module:name`` at module
level), and the parameters it can be given by keyword: every named
parameter of a function or method but ``self``/``cls``, and those of a
class' constructor.

``Index`` resolves the callables to their objects, so that a run can tell
which of them a call reaches: by the object called (a function, a class, a
bound method's function), by the code of a property's accessors and of the
dunder methods Python calls implicitly, and by name along a class' MRO.
"""

from __future__ import annotations

import importlib
import inspect
import json
import pathlib
import subprocess
import sys
import types
import warnings
from dataclasses import dataclass, field
from typing import Any, Iterator

HERE = pathlib.Path(__file__).resolve().parent
ENUMERATE = HERE.parent / "api-audit" / "enumerate.py"

#: The kinds of ``enumerate.py``'s records that are callables.
CALLABLE_KINDS = ("function", "class", "method", "property", "classmethod", "staticmethod")
#: The kinds whose parameters count.
PARAMETER_KINDS = ("function", "class", "method", "classmethod", "staticmethod")
#: The parameter kinds a call can give by keyword.
KEYWORD_KINDS = (inspect.Parameter.POSITIONAL_OR_KEYWORD, inspect.Parameter.KEYWORD_ONLY)


def enumerate_api(python: str, nested: bool = False) -> list[dict[str, Any]]:
    """``enumerate.py``'s records, run under ``python`` (the reference venv);
    with *nested*, the members of nested classes too."""
    done = subprocess.run([python, "-I", str(ENUMERATE), *(["--nested"] if nested else [])],
                          capture_output=True, text=True, check=True)
    return [json.loads(line) for line in done.stdout.splitlines() if line.strip()]


@dataclass
class Callable:
    """One public callable of matplotlib."""

    module: str
    owner: str
    name: str
    kind: str
    deprecated: bool
    #: The parameters it can be given by keyword, in signature order.
    params: list[str] = field(default_factory=list)
    #: What it is: the function, class, property or descriptor its owner holds.
    obj: Any = None

    @property
    def key(self) -> str:
        """``module:Owner.name``, or ``module:name`` at module level."""
        return f"{self.module}:{self.owner + '.' if self.owner else ''}{self.name}"

    @property
    def group(self) -> str:
        """The module, or ``module:Owner`` for a class' members."""
        return f"{self.module}:{self.owner}" if self.owner else self.module

    def as_json(self) -> dict[str, Any]:
        """The callable without its object."""
        return {"key": self.key, "module": self.module, "owner": self.owner, "name": self.name,
                "kind": self.kind, "deprecated": self.deprecated, "params": self.params}


def _keyword_parameters(obj: Any, skip_first: bool) -> list[str]:
    """The parameters of ``obj`` a call can give by keyword (none if it has
    no signature Python can read)."""
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            signature = inspect.signature(obj)
    except (TypeError, ValueError):
        return []
    parameters = list(signature.parameters.values())
    if skip_first and parameters and parameters[0].kind is not inspect.Parameter.KEYWORD_ONLY:
        parameters = parameters[1:]
    return [p.name for p in parameters if p.kind in KEYWORD_KINDS and p.name not in ("self", "cls")]


def _class_by_qualname(module: types.ModuleType, qualname: str) -> type | None:
    obj: Any = module
    for part in qualname.split("."):
        obj = getattr(obj, part, None)
    if inspect.isclass(obj):
        return obj
    return next((v for v in vars(module).values() if inspect.isclass(v) and v.__qualname__ == qualname), None)


class Index:
    """The callables of ``records`` with their objects, and how to find the
    callables a call reaches."""

    def __init__(self, records: list[dict[str, Any]]) -> None:
        self.callables: list[Callable] = []
        #: id of an object a program calls -> the callables it is
        self.by_object: dict[int, list[Callable]] = {}
        #: code of a property accessor or dunder method -> its callables
        self.by_code: dict[types.CodeType, list[Callable]] = {}
        #: (id of the owner class, name) -> the callable
        self.by_member: dict[tuple[int, str], Callable] = {}
        #: id of a public class -> the class' callable
        self.classes: dict[int, Callable] = {}
        #: the objects the ids are of, kept alive
        self._keep: list[Any] = []
        private_users = {r["name"]: (r["module"], r["bases"][0]) for r in records if r["kind"] == "private_base"}
        owners: dict[tuple[str, str], type | None] = {}
        for record in records:
            if record["kind"] not in CALLABLE_KINDS:
                continue
            key = (record["module"], record["owner"])
            if record["owner"] and key not in owners:
                owners[key] = self._owner(record["module"], record["owner"], private_users)
            callable_ = self._resolve(record, owners.get(key))
            if callable_ is not None:
                self.callables.append(callable_)

    @staticmethod
    def _owner(module_name: str, owner: str, private_users: dict[str, tuple[str, str]]) -> type | None:
        module = importlib.import_module(module_name)
        cls = _class_by_qualname(module, owner)
        if cls is None and owner in private_users:
            user_module, user = private_users[owner]
            user_class = _class_by_qualname(importlib.import_module(user_module), user)
            cls = next((c for c in getattr(user_class, "__mro__", ()) if c.__qualname__ == owner), None)
        return cls

    def _resolve(self, record: dict[str, Any], owner: type | None) -> Callable | None:
        kind = record["kind"]
        if record["owner"]:
            if owner is None or record["name"] not in vars(owner):
                return None
            obj = vars(owner)[record["name"]]
        else:
            obj = getattr(importlib.import_module(record["module"]), record["name"], None)
            if obj is None:
                return None
        target = obj.__func__ if kind in ("classmethod", "staticmethod") else obj
        params = _keyword_parameters(target, skip_first=kind in ("method", "classmethod")) \
            if kind in PARAMETER_KINDS else []
        callable_ = Callable(record["module"], record["owner"], record["name"], kind,
                             bool(record["deprecated"]), params, obj)
        self._register(callable_, owner)
        return callable_

    def _add(self, table: dict[Any, list[Callable]], key: Any, callable_: Callable) -> None:
        table.setdefault(key, []).append(callable_)

    def _register(self, callable_: Callable, owner: type | None) -> None:
        obj = callable_.obj
        self._keep.append(obj)
        if owner is not None:
            self.by_member[(id(owner), callable_.name)] = callable_
        if callable_.kind == "class":
            self.classes[id(obj)] = callable_
            self._add(self.by_object, id(obj), callable_)
            init = vars(obj).get("__init__")
            if init is not None:
                self._keep.append(init)
                self._add(self.by_object, id(init), callable_)
            return
        if callable_.kind == "property":
            for accessor in (obj.fget, obj.fset, obj.fdel):
                code = getattr(inspect.unwrap(accessor), "__code__", None) if accessor else None
                if code is not None:
                    self._add(self.by_code, code, callable_)
            return
        self._add(self.by_object, id(obj), callable_)
        inner = getattr(obj, "__func__", None)
        if inner is not None:
            self._keep.append(inner)
            self._add(self.by_object, id(inner), callable_)
        if callable_.name.startswith("__") and callable_.name.endswith("__"):
            code = getattr(inspect.unwrap(inner or obj), "__code__", None)
            if code is not None:
                self._add(self.by_code, code, callable_)

    def member(self, cls: type, name: str) -> Callable | None:
        """The callable ``cls.name`` finds along ``cls``' MRO, if public."""
        for base in cls.__mro__:
            if name in vars(base):
                return self.by_member.get((id(base), name))
        return None

    def groups(self) -> Iterator[tuple[str, list[Callable]]]:
        """The callables by group (module, or a class' members), in order."""
        groups: dict[str, list[Callable]] = {}
        for callable_ in self.callables:
            groups.setdefault(callable_.group, []).append(callable_)
        yield from groups.items()


def load(path: pathlib.Path) -> Index:
    """The index of the records saved at ``path`` (JSON lines)."""
    import matplotlib

    matplotlib.use("agg")
    records = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return Index(records)


if __name__ == "__main__":
    index = load(pathlib.Path(sys.argv[1]))
    print(len(index.callables), "callables,", sum(len(c.params) for c in index.callables), "parameters")
