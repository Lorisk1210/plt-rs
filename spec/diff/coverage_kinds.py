"""The kinds of values the differential tester's programs give matplotlib's
parameters, for the coverage report (``coverage.py``).

The traced runs (``reach.py``) note, per callable and parameter, the kinds
of the values given (``kinds.of``), the strings given, the choices
matplotlib's checks saw, and the calls refused (an unknown keyword, the
wrong number of arguments). ``KindsCoverage`` adds them up over the
runs, with the choices read from the callables' source (``kinds.Checks``),
and counts, over the keyword parameters ``api_index`` lists:

* the kinds given (distinct callable, parameter and kind), the parameters
  given more than one type, and those given ``None``;
* of the parameters given numbers, those given negative numbers, zero,
  NaN and inf; of those given data (lists, arrays, pandas), those given
  empty and single-element data, data with NaN or inf, masked arrays,
  integer arrays and pandas objects;
* of the parameters with string choices (lists of at most
  ``MAX_CHOICES``: longer ones, the colormaps' names, are names rather
  than choices), the choices given, and the parameters given a string
  that is none of them;
* of the callables reached that take parameters, those given an unknown
  keyword and those given the wrong number of arguments;
* of the aliases of the artists' properties (``lw``, ``c``), those given
  as keywords.
"""

from __future__ import annotations

import inspect
import warnings
from collections import Counter
from typing import Any

import api_index
import kinds

#: Choices of longer lists are names (colormaps, fonts), not counted as choices.
MAX_CHOICES = 40
#: The kinds that are a value's type, not its edges.
DATA_TYPES = {"list", "tuple", "masked", "Series", "DataFrame", "Index"}
#: The kinds counted over the parameters given data.
DATA_EDGES = ("empty", "single", "nan", "inf", "masked", "ndarray[i]", "ndarray[b]", "Series", "DataFrame",
              "datetime64")
#: The kinds counted over the parameters given numbers.
NUMBER_EDGES = ("negative", "zero", "nan", "inf")


def _share(reached: int, total: int) -> str:
    return f"{reached}/{total} ({100 * reached / total:.1f}%)" if total else "-"


def _is_data(kind: str) -> bool:
    return kind in DATA_TYPES or kind.startswith("ndarray[")


def _defaults_none(callable_: api_index.Callable) -> set[str]:
    """The parameters of ``callable_`` whose default is None."""
    obj = callable_.obj
    target = obj.__func__ if callable_.kind in ("classmethod", "staticmethod") else obj
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            parameters = inspect.signature(target).parameters.values()
    except (TypeError, ValueError):
        return set()
    return {p.name for p in parameters if p.default is None}


def _merged(runs: list[dict[str, Any]], field: str) -> dict[str, dict[str, set[str]]]:
    merged: dict[str, dict[str, set[str]]] = {}
    for run in runs:
        for key, by_name in run.get(field, {}).items():
            for name, found in by_name.items():
                merged.setdefault(key, {}).setdefault(name, set()).update(found)
    return merged


def _aliases(index: api_index.Index) -> set[str]:
    """The aliases of the public artists' properties."""
    found: set[str] = set()
    for callable_ in index.callables:
        if callable_.kind == "class":
            found.update(getattr(callable_.obj, "_alias_to_prop", {}))
    return found


class KindsCoverage:
    """The kinds of values the runs gave the parameters."""

    def __init__(self, index: api_index.Index, runs: list[dict[str, Any]], checks: kinds.Checks,
                 reached: set[str]) -> None:
        self.index = index
        self.reached = reached
        self.kinds = _merged(runs, "kinds")
        self.strings = _merged(runs, "strings")
        self.choices: dict[str, dict[str, set[str]]] = {
            key: {name: set(found) for name, found in by_name.items()} for key, by_name in checks.choices.items()}
        for key, by_name in _merged(runs, "choices").items():
            for name, found in by_name.items():
                self.choices.setdefault(key, {}).setdefault(name, set()).update(found)
        self.refused: dict[str, set[str]] = {}
        for run in runs:
            for key, why in run.get("refused", {}).items():
                self.refused.setdefault(key, set()).update(why)
        self.alias_names = _aliases(index)

    def _given(self, callable_: api_index.Callable) -> dict[str, set[str]]:
        """The kinds given each named parameter of ``callable_``."""
        by_name = self.kinds.get(callable_.key, {})
        return {name: by_name[name] for name in callable_.params if name in by_name}

    def choice_table(self, callable_: api_index.Callable) -> dict[str, tuple[set[str], set[str], bool]]:
        """Per parameter with choices: the choices, those given, and whether
        a string none of them was given."""
        table = {}
        for name, choices in self.choices.get(callable_.key, {}).items():
            if name not in callable_.params or len(choices) > MAX_CHOICES:
                continue
            strings = self.strings.get(callable_.key, {}).get(name, set())
            lowered = {c.lower() for c in choices}
            table[name] = (choices, {c for c in choices if c.lower() in {s.lower() for s in strings}},
                           any(s.lower() not in lowered for s in strings))
        return table

    def summary(self) -> list[str]:
        """The report's bullets of the kinds of values."""
        callables = self.index.callables
        params = sum(len(c.params) for c in callables)
        triples, several, given_count, nones, default_none, default_none_given = 0, 0, 0, 0, 0, 0
        numbers, data = Counter(), Counter()
        number_params = data_params = 0
        choice_params = choices = choices_given = invalid = 0
        takes, unknown, arity = 0, 0, 0
        for callable_ in callables:
            given = self._given(callable_)
            defaults = _defaults_none(callable_) & set(callable_.params)
            default_none += len(defaults)
            for name, found in given.items():
                given_count += 1
                triples += len(found)
                types = {k for k in found if k not in kinds.EDGES}
                several += len(types) > 1
                nones += "None" in found
                default_none_given += name in defaults and "None" in found
                if types & {"int", "float"}:
                    number_params += 1
                    numbers.update(edge for edge in NUMBER_EDGES if edge in found)
                if any(map(_is_data, types)):
                    data_params += 1
                    data.update(edge for edge in DATA_EDGES
                                if edge in found or (edge == "datetime64" and "ndarray[M]" in found))
            for _, (known, chosen, wrong) in self.choice_table(callable_).items():
                choice_params += 1
                choices += len(known)
                choices_given += len(chosen)
                invalid += wrong
            if callable_.params and callable_.key in self.reached:
                takes += 1
                why = self.refused.get(callable_.key, set())
                unknown += "unknown keyword" in why
                arity += bool(why & {"missing argument", "too many arguments", "argument given twice"})
        aliases_given = {name[2:] for by_name in self.kinds.values() for name in by_name
                         if name.startswith("**") and name[2:] in self.alias_names}
        return [
            f"* Kinds of values given (callable, parameter, kind): {triples}, "
            f"{triples / given_count if given_count else 0:.2f} per parameter given",
            f"* Parameters given values of more than one type: {_share(several, given_count)}",
            f"* Parameters given None: {_share(nones, params)}; of those defaulting to None: "
            f"{_share(default_none_given, default_none)}",
            f"* Parameters given numbers ({number_params}), given: "
            + ", ".join(f"{edge} {_share(numbers[edge], number_params)}" for edge in NUMBER_EDGES),
            f"* Parameters given data ({data_params}), given: "
            + ", ".join(f"{edge} {_share(data[edge], data_params)}" for edge in DATA_EDGES),
            f"* String choices (`_api.check_in_list`, `getitem_checked`; {choice_params} parameters) given: "
            f"{_share(choices_given, choices)}; parameters given a string none of them: "
            f"{_share(invalid, choice_params)}",
            f"* Callables taking parameters reached ({takes}), given an unknown keyword: {_share(unknown, takes)}, "
            f"the wrong number of arguments: {_share(arity, takes)}",
            f"* Aliases of properties given as keywords: {_share(len(aliases_given), len(self.alias_names))}",
        ]

    def kinds_table(self) -> list[str]:
        """The parameters given each kind, the most given first."""
        counts = Counter(kind for callable_ in self.index.callables
                         for found in self._given(callable_).values() for kind in found)
        lines = ["## Parameters given each kind of value", "", "| kind | parameters |", "|---|---|"]
        lines += [f"| {kind} | {count} |" for kind, count in counts.most_common()]
        return lines

    def missing_choices(self) -> list[str]:
        """The choices no program gave, per callable and parameter."""
        lines = ["## String choices not given", ""]
        for callable_ in self.index.callables:
            for name, (known, chosen, wrong) in self.choice_table(callable_).items():
                missing = sorted(known - chosen)
                if missing or not wrong:
                    shown = ", ".join(f"`{c}`" for c in missing)
                    lines.append(f"* `{callable_.key}` {name}: {shown}" + ("" if wrong else " (no invalid one)"))
        return lines

    def as_json(self, callable_: api_index.Callable) -> dict[str, Any]:
        """The kinds, choices and refusals of ``callable_``."""
        return {
            "kinds": {name: sorted(found) for name, found in sorted(self._given(callable_).items())},
            "choices": {name: {"known": sorted(known), "given": sorted(chosen), "invalid_given": wrong}
                        for name, (known, chosen, wrong) in sorted(self.choice_table(callable_).items())},
            "refused": sorted(self.refused.get(callable_.key, ())),
        }
