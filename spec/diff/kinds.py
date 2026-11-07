"""The kinds of values the differential tester's programs give matplotlib's
parameters, and what matplotlib checks those parameters against: what
the coverage (``coverage.py``) counts beyond the parameters themselves.

A value's kinds (``of``) are its type (``None``, ``bool``, ``int``,
``float``, ``str``, ``list``, ``ndarray[f]`` by the array's dtype kind,
``masked``, ``Series``, ``datetime``, ``callable``, ...) and its edges:
``empty`` and ``single`` for sized values, ``negative``, ``zero``,
``nan`` and ``inf`` for numbers and numeric data.

``Checks`` reads, from the source of each public callable
(``api_index``), the parameters it hands matplotlib's ``_api`` choice
checks itself (``check_in_list``, ``getitem_checked``) and their string
choices. A traced run (``reach.py``) adds the choices the checks deeper
down see given to a parameter of the program's call.
"""

from __future__ import annotations

import ast
import datetime
import inspect
import math
import numbers
import sys
import textwrap
import types
import warnings
from typing import Any, Iterable

#: The ``_api`` checks naming a parameter's choices.
CHOICE_CHECKS = ("check_in_list", "getitem_checked")
#: Elements of data looked at for NaN and inf; larger data is not.
MAX_SCANNED = 100_000
#: A parameter's kinds given as edges of its numbers and data.
EDGES = ("empty", "single", "negative", "zero", "nan", "inf")


def _type_kind(value: Any) -> str:
    """The kind naming ``value``'s type."""
    if value is None:
        return "None"
    module = type(value).__module__.split(".")[0]
    if module == "pandas":
        return type(value).__name__ if type(value).__name__ in ("Series", "DataFrame", "Index") else "pandas"
    if isinstance(value, bool) or type(value).__name__ == "bool":
        return "bool"
    if module == "numpy":
        import numpy as np

        if isinstance(value, np.ma.MaskedArray):
            return "masked"
        if isinstance(value, np.ndarray):
            return f"ndarray[{value.dtype.kind}]"
        if isinstance(value, np.datetime64):
            return "datetime"
        if isinstance(value, np.timedelta64):
            return "timedelta"
    if isinstance(value, numbers.Integral):
        return "int"
    if isinstance(value, numbers.Real):
        return "float"
    if isinstance(value, numbers.Complex):
        return "complex"
    if isinstance(value, (datetime.datetime, datetime.date)):
        return "datetime"
    if isinstance(value, datetime.timedelta):
        return "timedelta"
    for kind in (str, bytes, list, tuple, dict, set, frozenset, range):
        if isinstance(value, kind):
            return kind.__name__
    if isinstance(value, type):
        return "class"
    if isinstance(value, (types.FunctionType, types.BuiltinFunctionType, types.MethodType)):
        return "callable"
    return "object"


def _number_edges(number: float) -> list[str]:
    if math.isnan(number):
        return ["nan"]
    if math.isinf(number):
        return ["inf", "negative"] if number < 0 else ["inf"]
    if number == 0:
        return ["zero"]
    return ["negative"] if number < 0 else []


def _data_edges(value: Any) -> list[str]:
    """``nan`` and ``inf`` when numeric data holds them."""
    import numpy as np

    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            if isinstance(value, np.ndarray) and value.dtype.kind not in "biuf":
                return []
            array = np.asarray(value)
            if array.size > MAX_SCANNED or array.dtype.kind not in "biuf":
                return []
            array = np.asarray(array, dtype=float)
            return [edge for edge, found in (("nan", np.isnan(array).any()), ("inf", np.isinf(array).any()))
                    if found]
    except Exception:  # noqa: BLE001 - data numpy cannot read has no numeric edges
        return []


def of(value: Any) -> list[str]:
    """The kinds of ``value``: its type's, then its edges."""
    kind = _type_kind(value)
    found = [kind]
    if kind in ("int", "float"):
        try:
            found += _number_edges(float(value))
        except (TypeError, ValueError, OverflowError):
            pass
    elif kind not in ("None", "bool", "class", "callable", "object", "complex", "dict"):
        try:
            size = len(value)
        except Exception:  # noqa: BLE001 - an unsized value has no size edges
            size = None
        if size == 0:
            found.append("empty")
        elif size == 1 and kind not in ("str", "bytes"):
            found.append("single")
        if kind not in ("str", "bytes", "datetime", "timedelta") and size:
            found += _data_edges(value)
    return found


def choices_of(values: Any) -> list[str]:
    """The string choices of a check's first argument (a mapping's keys)."""
    try:
        return sorted({v for v in values if isinstance(v, str)})
    except TypeError:
        return []


def _function_of(callable_: Any) -> Any:
    """The Python function whose source a callable's checks are in."""
    obj = callable_.obj
    if callable_.kind == "class":
        obj = vars(obj).get("__init__")
    elif callable_.kind in ("classmethod", "staticmethod"):
        obj = obj.__func__
    obj = inspect.unwrap(obj) if obj is not None else None
    return obj if isinstance(obj, types.FunctionType) else None


def _owner_class(function: types.FunctionType) -> Any:
    """The class a method is defined in, found by its qualified name."""
    owner: Any = sys.modules.get(function.__module__)
    for part in function.__qualname__.split(".")[:-1]:
        owner = getattr(owner, part, None)
    return owner if isinstance(owner, type) else None


def _checked_parameter(expression: ast.expr, parameters: set[str]) -> str | None:
    """The parameter a check's keyword value is (``p``, ``p.lower()``,
    ``self.p``), if it is one."""
    for node in ast.walk(expression):
        if isinstance(node, ast.Name) and node.id in parameters:
            return node.id
        if (isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == "self"
                and node.attr in parameters):
            return node.attr
    return None


class Checks:
    """The choices the public callables check their parameters against,
    by callable key and parameter."""

    def __init__(self, callables: Iterable[Any]) -> None:
        self.choices: dict[str, dict[str, set[str]]] = {}
        for callable_ in callables:
            if callable_.params:
                self._scan(callable_)

    def _scan(self, callable_: Any) -> None:
        function = _function_of(callable_)
        if function is None:
            return
        try:
            tree = ast.parse(textwrap.dedent(inspect.getsource(function)))
        except (OSError, TypeError, SyntaxError):
            return
        parameters = set(callable_.params)
        owner = _owner_class(function)
        scope = {"self": owner, "cls": owner}
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not node.args:
                continue
            name = node.func.attr if isinstance(node.func, ast.Attribute) else getattr(node.func, "id", None)
            if name not in CHOICE_CHECKS:
                continue
            checked = [_checked_parameter(k.value, parameters) for k in node.keywords
                       if k.arg and not k.arg.startswith("_")]
            checked = [parameter for parameter in checked if parameter is not None]
            if not checked:
                continue
            try:
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore")
                    first = eval(compile(ast.Expression(node.args[0]), "<check>", "eval"),  # noqa: S307
                                 function.__globals__, scope)
            except Exception:  # noqa: BLE001 - a check of what only a call knows is left out
                continue
            found = choices_of(first)
            for parameter in checked if found else ():
                self.add_choices(callable_.key, parameter, found)

    def add_choices(self, key: str, parameter: str, choices: Iterable[str]) -> None:
        """Add the choices a run saw checked for ``parameter`` of ``key``."""
        self.choices.setdefault(key, {}).setdefault(parameter, set()).update(choices)
