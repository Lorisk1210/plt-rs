"""Every public callable of matplotlib called with what users get wrong.

The subjects and callables are the ``api`` generator's (``api.subjects``:
each module's functions and classes, each object's members). Per
callable, the program starts from the ``api`` generator's first call
(``api.argument_variants``) and changes one parameter at a time to each
of the values ``hostile_values.variants`` gives its representative value
(the call's, else ``values.VALUES``', else the parameter's default):
other types, None, empty and single-element data, NaN and inf, negative
and zero numbers, each string choice matplotlib checks the parameter
against (``kinds.Checks``) and one it does not. Then the callable is
given an unknown keyword, called without its required arguments and
with one positional argument too many, and, where the call passes two
equally long data, with one of them shorter. A setter is followed by its
getter, a settable property is set and read, and the subject's figure
is drawn after each value (where matplotlib uses what its setters keep
unchecked), then the value is set back to the representative one, so
that the next statements start from a figure that draws. A keyword of ``set`` and a setter's value without a
value of the tables' take the value the property's setter is given, else
what its getter reads. Special methods are called by name
(``obj.__getitem__(key)``); members undoing the subject (``remove``,
``clear``) run last, each on the subject made anew. On an ``Axes``, each callable's calls are followed by a
draw of the figure and the ``Axes`` cleared.

The artists' subjects also set each alias of their properties (``lw``,
``c``), read it through the alias' getter, and give an alias together
with the property it is an alias of.

Callables that wait, start event loops, switch backends, run TeX or
change the global configuration are left out (``hostile_tables``): their
hostile calls would test the machine, not the library. So are the values
matplotlib itself never returns from (``NEVER_RETURNS``). A subject's statements are split into programs of about
``STATEMENTS``; each ends drawing and saving its figures (``api.ENDING``).
"""

from __future__ import annotations

import inspect
import warnings
from typing import Any, Iterator

import api_index
import kinds

from . import api, artists, common, hostile_values
from .hostile_calls import POSITIONAL, Arguments
from .hostile_tables import DUNDERS_LEFT_OUT, LEFT_OUT, MODULES_LEFT_OUT, NEVER_RETURNS, REQUIRED, SMALL
from .subjects import PRELUDE

#: Statements per program, about.
STATEMENTS = 120
#: The keyword no callable has.
UNKNOWN = "nonexistent_kw"
#: What follows each value given a setter or a property: the subject's
#: figure drawn, where matplotlib uses what its setters keep unchecked.
DRAWN = "fig.canvas.draw()"
#: The value of the program's own lines the representative values fail on.
_FAILED = object()


def _left_out(callable_: api_index.Callable) -> bool:
    return (callable_.name in LEFT_OUT or callable_.name in DUNDERS_LEFT_OUT
            or callable_.module.startswith(MODULES_LEFT_OUT)
            or callable_.kind == "property" and callable_.obj.fset is None)


def _evaluate(text: str, namespace: dict[str, Any]) -> Any:
    """What ``text`` evaluates to in the subject's program (``_FAILED`` if
    it fails)."""
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            return eval(text, namespace)  # noqa: S307 - the generator's own values
    except Exception:  # noqa: BLE001 - a value only the program can make
        return _FAILED


def _named(value: Any) -> str | None:
    """The source naming a module-level function or class (a default such
    as ``cbook.flatten``'s ``scalarp``), importing its module."""
    module, qualname = getattr(value, "__module__", None), getattr(value, "__qualname__", "")
    if not module or not callable(value) or "<" in qualname:
        return None
    return f"__import__({module!r}, fromlist=['_']).{qualname}"


def _data_length(value: Any) -> int | None:
    import numpy as np

    if isinstance(value, (list, tuple, np.ndarray)) and not isinstance(value, str):
        try:
            array = np.asarray(value, dtype=float)
        except (TypeError, ValueError):
            return None
        return len(array) if array.ndim == 1 and len(array) > 1 else None
    return None


class Choices:
    """The string choices of the public callables' parameters
    (``kinds.Checks``), and those a parameter takes from elsewhere."""

    #: Where a pyplot function's method is defined.
    WRAPPED = ("matplotlib.axes:Axes", "matplotlib.axes:_AxesBase", "matplotlib.figure:Figure",
               "matplotlib.figure:FigureBase")

    def __init__(self, callables: list[api_index.Callable]) -> None:
        self.checks = kinds.Checks(callables)
        self.by_name: dict[str, set[str]] = {}
        for by_parameter in self.checks.choices.values():
            for name, found in by_parameter.items():
                self.by_name.setdefault(name, set()).update(found)

    def of(self, callable_: api_index.Callable, name: str, cls: type) -> list[str]:
        """The choices of ``name`` of ``callable_`` (a member of ``cls``)."""
        own = self.checks.choices
        found = own.get(callable_.key, {}).get(name)
        if not found and callable_.module == "matplotlib.pyplot":
            found = next((own[key][name] for key in (f"{w}.{callable_.name}" for w in self.WRAPPED)
                          if name in own.get(key, {})), None)
        if not found:
            setter = api.index().member(cls, f"set_{name}")
            first = next(iter(setter.params), None) if setter is not None else None
            found = own.get(setter.key, {}).get(first) if setter is not None else None
        return sorted(found or self.by_name.get(name, ()))


class HostileSubject:
    """The hostile calls of one subject's callables."""

    def __init__(self, subject: api.Subject, choices: Choices) -> None:
        self.subject = subject
        self.choices = choices

    def _call(self, callable_: api_index.Callable, owner: str | None, arguments: str) -> str:
        if owner is None:
            return f"{self.subject.receiver}.{callable_.name}({arguments})"
        first = "" if callable_.kind in ("classmethod", "staticmethod", "class") else "obj"
        return f"{owner}.{callable_.name}({', '.join(a for a in (first, arguments) if a)})"

    def _representative(self, callable_: api_index.Callable, parameter: inspect.Parameter,
                        arguments: Arguments) -> str | None:
        """The value a call gives ``parameter`` of ``callable_`` when well
        made: the call's, the tables', a property's as its setter is called
        or its getter reads it, else the parameter's default."""
        given = arguments.value_of(parameter.name)
        if given is not None:
            return given
        found = api._values(callable_, parameter.name, self.subject.scope)
        if found:
            return found[0]
        prop = parameter.name if callable_.name == "set" else callable_.name[4:] if (
            callable_.name.startswith("set_")) else None
        found = self._property_value(prop) if prop else None
        if found is not None:
            return found
        if parameter.default is not inspect.Parameter.empty:
            return common.literal(parameter.default) or _named(parameter.default)
        return None

    def _property_value(self, prop: str) -> str | None:
        """A value of the subject's property ``prop``: what the ``api``
        generator gives its setter, else what its getter reads."""
        setter = api.index().member(self.subject.cls, f"set_{prop}")
        if setter is not None:
            for variant in api.argument_variants(setter, self.subject.scope)[:1]:
                try:
                    arguments = Arguments(variant, api._parameters(setter))
                except SyntaxError:
                    continue
                if arguments.positional_count() == 1 and len(arguments.items) == 1:
                    return arguments.items[0][1]
        if self.subject.receiver == "obj" and callable(getattr(self.subject.cls, f"get_{prop}", None)):
            return f"obj.get_{prop}()"
        return None

    def _choices(self, callable_: api_index.Callable, name: str) -> list[str]:
        """The string choices matplotlib checks ``name`` against: the
        callable's own, else those of the ``Axes`` or figure method a
        pyplot function wraps, of the property's setter, or of any
        callable's parameter of that name (``sides``, which the spectral
        methods hand ``mlab``)."""
        return self.choices.of(callable_, name, self.subject.cls)

    def _base(self, callable_: api_index.Callable, parameters: list[inspect.Parameter]) -> str | None:
        """The arguments of a well-made call: the ``api`` generator's
        first, else the required parameters' values (None if one has none)."""
        variants = api.argument_variants(callable_, self.subject.scope)
        if variants:
            return variants[0]
        parts = []
        for parameter in parameters:
            if parameter.default is not inspect.Parameter.empty or parameter.kind in (
                    inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD):
                continue
            found = (api._values(callable_, parameter.name, self.subject.scope)
                     or self.subject.scope.available(REQUIRED.get(parameter.name, [])))
            if not found:
                return None
            parts.append(found[0] if parameter.kind in POSITIONAL else f"{parameter.name}={found[0]}")
        return ", ".join(parts)

    def _getter(self, callable_: api_index.Callable) -> str | None:
        name = callable_.name
        getter = f"get_{name[4:]}" if name.startswith("set_") else None
        if getter is not None and callable(getattr(self.subject.cls, getter, None)):
            return f"{self.subject.receiver}.{getter}()"
        return None

    def _property(self, callable_: api_index.Callable) -> list[str]:
        receiver, name = self.subject.receiver, callable_.name
        found = api._values(callable_, name, self.subject.scope)
        current = found[0] if found else f"{receiver}.{name}"
        value = _evaluate(current, self.subject.namespace)
        if value is _FAILED:
            return []
        out = []
        restored = [f"{receiver}.{name} = {found[0]}"] if found else []
        for variant in hostile_values.variants(value, current, name, []):
            out += [f"{receiver}.{name} = {variant}", f"{receiver}.{name}", DRAWN, *restored]
        return out

    def _mismatched(self, callable_: api_index.Callable, owner: str | None, arguments: Arguments) -> list[str]:
        """The call with the second of two equally long data one shorter."""
        lengths = {}
        for name in arguments.bound:
            length = _data_length(_evaluate(arguments.value_of(name), self.subject.namespace))
            if length is not None:
                lengths[name] = length
        names = [n for n in lengths if list(lengths.values()).count(lengths[n]) > 1]
        if len(names) < 2:
            return []
        shorter = arguments.with_value(names[1], f"np.asarray({arguments.value_of(names[1])})[:-1]")
        return [self._call(callable_, owner, shorter)] if shorter is not None else []

    def statements(self, callable_: api_index.Callable, owner: str | None) -> list[str]:
        """The hostile calls of ``callable_``."""
        if callable_.kind == "property":
            return self._property(callable_) if owner is None and self.subject.receiver == "obj" else []
        parameters = api._parameters(callable_)
        base = self._base(callable_, parameters)
        required = [p for p in parameters if p.default is inspect.Parameter.empty
                    and p.kind not in (inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD)]
        if base is None:
            return []
        try:
            arguments = Arguments(base, parameters)
        except SyntaxError:
            return []
        small = SMALL.get(callable_.name)
        if small is not None and small[0] not in arguments.bound:
            arguments = Arguments(arguments.with_keyword(*small), parameters)
        getter = self._getter(callable_)
        setting = callable_.name == "set" or callable_.name.startswith("set_")
        out: list[str] = []

        def add(text: str | None, restored: str | None = None) -> None:
            if text is not None:
                out.append(self._call(callable_, owner, text))
                if getter:
                    out.append(getter)
                if setting:
                    out.append(DRAWN)
                    if restored is not None:
                        out.append(self._call(callable_, owner, restored))

        for parameter in parameters:
            if parameter.kind is inspect.Parameter.VAR_POSITIONAL or parameter.kind is inspect.Parameter.VAR_KEYWORD:
                continue
            representative = self._representative(callable_, parameter, arguments)
            if representative is None:
                continue
            value = _evaluate(representative, self.subject.namespace)
            choices = self._choices(callable_, parameter.name)
            given = (["None", "3", "'x'"] if value is _FAILED
                     else hostile_values.variants(value, representative, parameter.name, choices))
            given += [c for c in hostile_values.choice_values(choices) if c not in given]
            never = NEVER_RETURNS.get((callable_.name, parameter.name), set())
            restored = arguments.with_value(parameter.name, representative)
            for variant in given if "*" not in never else ():
                if variant in never:
                    continue
                add(arguments.with_value(parameter.name, variant), restored)
        if not arguments.starred:
            add(arguments.with_keyword(UNKNOWN, "1"))
        if required:
            add("")
        if not any(p.kind is inspect.Parameter.VAR_POSITIONAL for p in parameters):
            positional = sum(1 for p in parameters if p.kind in POSITIONAL)
            add(", ".join(["None"] * (positional + 1)))
        out += self._mismatched(callable_, owner, arguments)
        if setting:
            add(arguments.text())
        return out

    def aliases(self) -> list[str]:
        """Each alias of the object's properties set, read through its
        getter, and given with the property it is an alias of."""
        obj = self.subject.namespace.get("obj")
        aliases = getattr(type(obj), "_alias_to_prop", {}) if self.subject.receiver == "obj" else {}
        out = []
        for alias, prop in sorted(aliases.items()):
            values = artists.VALUES.get(prop, [])
            if not values or not callable(getattr(obj, f"get_{alias}", None)):
                continue
            out += [f"obj.set({alias}={values[0]})", f"obj.get_{alias}()", f"obj.set_{alias}({values[-1]})",
                    f"obj.get_{prop}()", f"obj.set({alias}={values[0]}, {prop}={values[-1]})"]
        return out

    def _drawn_and_cleared(self) -> list[str]:
        """What follows each callable's calls on an ``Axes``: its figure
        drawn, then the ``Axes`` cleared, so that each callable's artists
        are drawn on their own."""
        from matplotlib.axes import Axes

        if self.subject.receiver == "obj" and isinstance(self.subject.namespace.get("obj"), Axes):
            return ["obj.figure.canvas.draw()", "obj.cla()"]
        return []

    def programs(self) -> Iterator[tuple[str, list[str]]]:
        """The subject's programs, about ``STATEMENTS`` statements each."""
        after = self._drawn_and_cleared()
        taken = [(c, owner) for c, owner in self.subject.callables if not _left_out(c)]
        groups = [group + after if group else group
                  for group in (self.statements(c, owner) for c, owner in taken if c.name not in api.UNDOING)]
        groups.append(self.aliases())
        # Each callable undoing the subject runs last, on the subject made anew.
        for callable_, owner in taken:
            if callable_.name in api.UNDOING:
                undoing = self.statements(callable_, owner)
                groups.append(self.subject.setup + undoing if undoing else [])
        imports = sorted({f"import {c.module}" for c, owner in self.subject.callables
                          if owner is not None and not c.owner.startswith("_")})
        chunk: list[str] = []
        number = 0
        for group in [g for g in groups if g]:
            chunk += group
            if len(chunk) >= STATEMENTS:
                yield self._program(number, imports, chunk)
                chunk, number = [], number + 1
        if chunk:
            yield self._program(number, imports, chunk)

    def _program(self, number: int, imports: list[str], body: list[str]) -> tuple[str, list[str]]:
        return (f"{self.subject.name}_{number}",
                PRELUDE + ["import pandas as pd"] + imports + self.subject.setup + body + api.ENDING)


def programs() -> Iterator[tuple[str, list[str]]]:
    """The hostile programs of every subject."""
    choices = Choices(api.index().callables)
    named: set[str] = set()
    for subject in api.subjects():
        # A subject listed twice (once per table) is named apart.
        for name, statements in HostileSubject(subject, choices).programs():
            while name in named:
                name += "_again"
            named.add(name)
            yield name, statements
