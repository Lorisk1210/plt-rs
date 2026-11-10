"""The public names of matplotlib's modules in pltrs (Python), compared for
``docs.py``.

``docs_dump.py`` describes, in each implementation, the public names of
matplotlib's modules (``--names``): every name ``dir`` lists without a
leading underscore, and ``__all__``, by its value's type and, for modules
and constants, its value. A name matches when pltrs's module has it, with
a value of the same type and, where one is compared, the same value: the
re-exports of numpy, dateutil, enum and the standard library programs
import from matplotlib's modules (``from matplotlib.dates import MO``),
constants, and the modules reachable as attributes (``matplotlib.cbook.np``).

A class or function of matplotlib's C++ extensions (pybind11's
``FT2Font``, ``image.resample``) matches a Python class or function of
pltrs's, as ``docs.py`` compares their callables: Python reimplements them,
so the type of the value (pybind11's metaclass, a builtin function) is
pybind11's, not matplotlib's.
"""

from __future__ import annotations

from collections import defaultdict
from typing import Any

#: The causes of a difference, in the order the report lists them.
CAUSES = ("module missing in pltrs", "missing", "type", "value")


def _shown(description: dict[str, Any]) -> str:
    """A name's description as the report shows it."""
    value = description.get("value")
    return description["type"] + ("" if value is None else f" {value}")


def _difference(theirs: dict[str, Any], mine: dict[str, Any] | None) -> tuple[str, str] | None:
    """The cause and the detail of a name's difference, or None."""
    if mine is None or not mine["found"]:
        return "missing", _shown(theirs)
    reimplemented = theirs.get("native") and theirs.get("kind") == mine.get("kind")
    if theirs["type"] != mine["type"] and not reimplemented:
        return "type", f"{theirs['type']} -> {mine['type']}"
    if theirs["value"] != mine["value"]:
        return "value", f"{theirs['value']} -> {mine['value']}"
    return None


class NameComparison:
    """The public names of matplotlib's modules under both implementations,
    compared."""

    def __init__(self, mpl: dict[str, Any], ours: dict[str, Any], patterns: list[str],
                 allowlist: Any) -> None:
        #: module -> name -> whether it matches
        self.outcomes: dict[str, dict[str, bool]] = {}
        #: cause -> [(key, detail)]
        self.differences: dict[str, list[tuple[str, str]]] = defaultdict(list)
        #: The intended differences (*allowlist*'s): [(key, "names: cause", why)]
        self.allowed: list[tuple[str, str, str]] = []
        #: The modules pltrs lacks.
        self.missing_modules: list[str] = []
        for module, names in mpl.items():
            mine = ours.get(module)
            if mine is None and names:
                self.missing_modules.append(module)
            for name, theirs in (names or {}).items():
                key = f"{module}:{name}"
                if not theirs["found"] or (patterns and not any(p in key for p in patterns)):
                    continue
                difference = (("module missing in pltrs", "") if mine is None
                              else _difference(theirs, mine.get(name)))
                self.outcomes.setdefault(module, {})[name] = difference is None
                if difference is None:
                    continue
                why = allowlist.reason(key, f"names: {difference[0]}")
                if why is not None:
                    self.allowed.append((key, f"names: {difference[0]}", why))
                else:
                    self.differences[difference[0]].append((key, difference[1]))

    def share(self, modules: list[str] | None = None) -> str:
        """How many of the names of *modules* (all by default) match."""
        outcomes = [matched for module in (modules or self.outcomes)
                    for matched in self.outcomes[module].values()]
        if not outcomes:
            return "-"
        return f"{sum(outcomes)}/{len(outcomes)} ({100 * sum(outcomes) / len(outcomes):.1f}%)"

    def count(self) -> int:
        """The differences, but the intended ones."""
        return sum(map(len, self.differences.values()))

    def summary_line(self) -> str:
        """The report's line in the summary of ``docs.md``."""
        return (f"* Module-level names matching: {self.share()}, in {len(self.outcomes)} modules, "
                f"{len(self.missing_modules)} of them missing in pltrs")

    def report(self) -> tuple[list[str], list[str]]:
        """The report's section, with every difference, and its summary
        (the counts only)."""
        lines = ["", "## Module-level names", "",
                 "Every name `dir(module)` lists without a leading underscore, and `__all__`, of matplotlib's "
                 "public modules (`docs_names.py`): present in pltrs's module, with a value of the same type "
                 "(`type(value).__name__`) and, for modules and constants, the same value.", "",
                 "| module | names | matching |", "|---|---|---|"]
        for module, names in sorted(self.outcomes.items(), key=lambda item: (-len(item[1]), item[0])):
            if not all(names.values()):
                lines.append(f"| {module} | {len(names)} | {self.share([module])} |")
        lines += ["", f"All other modules' names match ({sum(all(n.values()) for n in self.outcomes.values())} "
                  "modules).", ""]
        summary = list(lines)
        for cause in CAUSES:
            items = self.differences.get(cause)
            if not items:
                continue
            heading = f"### names: {cause} ({len(items)})"
            summary.append(heading)
            lines += [heading, "", *(f"* `{key}`" + (f": {detail}" if detail else "") for key, detail in items), ""]
        return lines, summary

    def as_json(self) -> dict[str, Any]:
        """Every name's outcome and every difference."""
        return {"outcomes": self.outcomes,
                "differences": [{"cause": cause, "key": key, "detail": detail}
                                for cause, items in self.differences.items() for key, detail in items],
                "allowed": [{"key": key, "cause": cause, "why": why} for key, cause, why in self.allowed]}
