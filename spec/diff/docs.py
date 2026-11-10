"""matplotlib's signatures, docstrings and names in pltrs (Python).

    python docs.py [PATTERN...] [--failures] [--strict]
                   --root ROOT --out OUT --pltrs-python PYTHON

``cargo xtask diff --docs`` runs this in the reference venv (after
building pltrs (Python)'s extension). It

1. lists matplotlib 3.11's public callables (``api_index``: modules and
   classes as ``spec/api-audit/enumerate.py`` takes them, with the members
   of nested classes; functions, classes, methods and properties);
2. describes each under matplotlib and under pltrs (``docs_dump.py``, in
   each venv, pltrs through its alias, as programs importing matplotlib
   get it): its signature (the parameters' names, kinds, defaults by repr
   and order, a class' and its constructor's; a property's accessors; the
   kind of method, static or class method), its docstring after
   matplotlib's interpolation (a class' and its constructor's), its
   ``__module__`` and ``__qualname__`` (a property's accessors'), and for
   an `.Artist` class, what ``ArtistInspector`` reads of it (its
   properties, their aliases and accepted values: what ``setp``,
   ``kwdoc`` and ``set``'s docstring show);
3. compares the public names of matplotlib's modules, constants and
   re-exports included (``docs_names.py``);
4. writes ``OUT/docs.md``: the share matching overall and per module,
   signatures, docstrings, names and Artist properties apart, the
   callables of matplotlib's C++ extensions (whose signatures Python
   cannot read) apart too, then the differences grouped by cause, the
   biggest groups first, then the module-level names, and
   ``OUT/docs.json`` with every difference.

Differences that are intended are listed, each with its proof, in
``docs-allowlist.txt``; the report names them apart, and ``--strict``
does not count them.

A PATTERN keeps the callables whose keys (``module:Owner.name``) contain
it. ``--failures`` prints every difference, not only the counts;
``--strict`` fails when any remains.
"""

from __future__ import annotations

import argparse
import difflib
import inspect
import json
import os
import pathlib
import re
import subprocess
import sys
from collections import defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import api_index  # noqa: E402
import driver  # noqa: E402
from docs_names import NameComparison  # noqa: E402

#: The aspects compared, as the report names them.
ASPECTS = ("signature", "doc", "name", "properties")
#: Lines of a docstring difference the report shows.
DOC_CONTEXT = 6
#: The intended differences: ``<key regex> | <cause regex> | why`` lines.
ALLOWLIST = HERE / "docs-allowlist.txt"
#: What ``ArtistInspector`` reads, as ``docs_dump.py`` records it.
INSPECTED = ("setters", "aliases", "valid_values", "pprint_setters", "error")


def _dump(setup: driver.Setup, impl: str, api: pathlib.Path) -> tuple[dict[str, Any], dict[str, Any]]:
    """``docs_dump.py``'s descriptions under ``impl``: of the callables, and
    of the module-level names (pltrs's of those matplotlib's lists)."""
    out, names = (setup.out / "docs" / f"{impl}{suffix}.json" for suffix in ("", ".names"))
    python = setup.mpl_python if impl == "matplotlib" else setup.pltrs_python
    of = [] if impl == "matplotlib" else ["--names-of", str(setup.out / "docs" / "matplotlib.names.json")]
    env = setup.environment(impl)
    if impl == "pltrs":
        # pltrs as programs importing matplotlib get it: through its alias.
        env["PYTHONPATH"] = os.pathsep.join([str(setup.root / "python" / "alias"), env["PYTHONPATH"]])
    done = subprocess.run([python, str(HERE / "docs_dump.py"), str(api), str(out),
                           "--names", str(names), *of],
                          env=env, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit(f"describing the callables under {impl} failed:\n{done.stderr[-5000:]}")
    return json.loads(out.read_text()), json.loads(names.read_text())


def _parameters(params: list[list[Any]]) -> str:
    """A signature's parameters as Python writes them."""
    parts, keyword_only = [], False
    for index, (name, kind, default) in enumerate(params):
        if kind == "KEYWORD_ONLY" and not keyword_only:
            parts.append("*")
        keyword_only = keyword_only or kind in ("KEYWORD_ONLY", "VAR_POSITIONAL")
        prefix = {"VAR_POSITIONAL": "*", "VAR_KEYWORD": "**"}.get(kind, "")
        parts.append(prefix + name + ("" if default is None else f"={default}"))
        following = params[index + 1][1] if index + 1 < len(params) else None
        if kind == "POSITIONAL_ONLY" and following != "POSITIONAL_ONLY":
            parts.append("/")
    return "(" + ", ".join(parts) + ")"


def _default(default: str | None) -> str:
    """A parameter's default as the report shows it."""
    return "(none)" if default is None else default


def _signature_causes(mpl: list[list[Any]], ours: list[list[Any]]) -> tuple[list[str], list[str]]:
    """What differs between two signatures' parameters: the causes, and
    each parameter's difference."""
    theirs, mine = {p[0]: p for p in mpl}, {p[0]: p for p in ours}
    causes, details = [], []
    missing, extra = [n for n in theirs if n not in mine], [n for n in mine if n not in theirs]
    if missing:
        causes.append("missing parameter")
        details.append("missing: " + ", ".join(missing))
    if extra:
        causes.append("extra parameter")
        details.append("extra: " + ", ".join(extra))
    shared = [name for name in theirs if name in mine]
    kinds = [f"{n} {theirs[n][1]} -> {mine[n][1]}" for n in shared if theirs[n][1] != mine[n][1]]
    if kinds:
        causes.append("parameter kind")
        details.append("kind: " + ", ".join(kinds))
    defaults = [f"{n} {_default(theirs[n][2])} -> {_default(mine[n][2])}"
                for n in shared if theirs[n][2] != mine[n][2]]
    if defaults:
        causes.append("default")
        details.append("default: " + ", ".join(defaults))
    if shared != [name for name in mine if name in theirs]:
        causes.append("order")
        details.append("order: " + ", ".join(name for name in mine if name in theirs))
    return causes, details


def _compare_signature(mpl: dict[str, Any], ours: dict[str, Any]) -> tuple[str, str] | None:
    """The cause and the detail of a signature difference, or None (a
    native callable's form is pltrs's own: Python reimplements it): the
    form (function, static or class method, property), then the callable's
    parameters, then a class' constructor's."""
    if mpl["form"] != ours["form"] and mpl["form"] != "native":
        return "form", f"{mpl['form']} in matplotlib, {ours['form']} in pltrs"
    for field, what in (("signature", ""), ("init_signature", "constructor's ")):
        theirs, mine = mpl.get(field), ours.get(field)
        if theirs == mine or (field == "init_signature" and theirs is None):
            continue
        if mpl["form"] == "property":
            accessors = ", ".join(
                f"{name} {'yes' if theirs[name] else 'no'} -> {'yes' if mine[name] else 'no'}"
                for name in theirs if theirs[name] != mine[name])
            return "property accessors", accessors
        if mine is None:
            return (f"{what}unreadable",
                    f"matplotlib {_parameters(theirs)}, pltrs: Python cannot read the signature")
        causes, details = _signature_causes(theirs, mine)
        return ", ".join(what + cause for cause in causes), "\n".join(details)
    return None


def _doc_detail(mpl: str, ours: str) -> str:
    """The first lines where two docstrings part, as a unified diff."""
    lines = list(difflib.unified_diff(mpl.splitlines(), ours.splitlines(), "matplotlib", "pltrs",
                                      n=1, lineterm=""))
    return "\n".join(lines[2:2 + DOC_CONTEXT] + (["..."] if len(lines) > 2 + DOC_CONTEXT else []))


def _compare_doc(mpl: dict[str, Any], ours: dict[str, Any]) -> tuple[str, str] | None:
    """The cause and the detail of a docstring difference, or None (a
    native callable's as ``inspect.getdoc`` shows it: Python's compiler
    strips the indentation an extension keeps; where matplotlib's callable
    has none of its own, the base class' that ``inspect.getdoc`` shows)."""
    for field, what in (("doc", ""), ("init_doc", "constructor's ")):
        theirs, mine = mpl.get(field) or "", ours.get(field) or ""
        inherited = mpl.get(field.replace("doc", "inherited_doc"))
        if mpl.get(field) is None and inherited is not None:
            # matplotlib's own has none: ``help`` shows its base class'.
            theirs, mine = inherited, ours.get(field.replace("doc", "inherited_doc")) or inspect.cleandoc(mine)
        elif mpl["form"] == "native":
            theirs, mine = inspect.cleandoc(theirs), inspect.cleandoc(mine)
        if theirs == mine:
            continue
        if not mine:
            return f"{what}docstring missing", theirs.strip().splitlines()[0] if theirs.strip() else ""
        if not theirs:
            return f"{what}docstring where matplotlib has none", mine.strip().splitlines()[0]
        first = theirs.strip().splitlines()[:1] == mine.strip().splitlines()[:1]
        return (f"{what}docstring differs" + (" after the first line" if first else ""),
                _doc_detail(theirs, mine))
    return None


def _compare_name(mpl: dict[str, Any], ours: dict[str, Any]) -> tuple[str, str] | None:
    """The cause and the detail of a ``__module__``/``__qualname__``
    difference, a property's getter's then its setter's and deleter's, or
    None (pybind11 gives its methods qualnames of its own)."""
    accessors = mpl.get("accessor_names") or {}
    for what, theirs, mine in [("", mpl.get("name"), ours.get("name")),
                               *((f"{accessor}'s ", name, (ours.get("accessor_names") or {}).get(accessor))
                                 for accessor, name in accessors.items())]:
        if theirs == mine or theirs is None or "pybind11" in theirs[1]:
            continue
        mine = mine or ["?", "?"]
        cause = "__module__" if theirs[0] != mine[0] else "__qualname__"
        if theirs[0] != mine[0] and theirs[1] != mine[1]:
            cause = "__module__ and __qualname__"
        return what + cause, f"{'.'.join(theirs)} -> {'.'.join(mine)}"
    return None


def _compare_properties(mpl: dict[str, Any], ours: dict[str, Any]) -> tuple[str, str] | None:
    """The cause and the detail of a difference in what ``ArtistInspector``
    reads of an Artist class, or None."""
    theirs, mine = mpl["properties"], ours.get("properties") or {}
    for part in INSPECTED:
        if theirs.get(part) == mine.get(part):
            continue
        if part == "pprint_setters":
            return part, _doc_detail("\n".join(theirs[part]), "\n".join(mine.get(part) or []))
        if isinstance(theirs.get(part), dict) and isinstance(mine.get(part), dict):
            keys = sorted(set(theirs[part]) | set(mine[part]))
            return part, "\n".join(f"{key}: {theirs[part].get(key)!r} -> {mine[part].get(key)!r}"
                                   for key in keys if theirs[part].get(key) != mine[part].get(key))
        return part, f"{theirs.get(part)!r} -> {mine.get(part)!r}"
    return None


COMPARERS = {"signature": _compare_signature, "doc": _compare_doc, "name": _compare_name,
             "properties": _compare_properties}


def _compared(aspect: str, mpl: dict[str, Any]) -> bool:
    """Whether *aspect* of matplotlib's callable is compared: not the
    signature of a native one Python cannot read, and the Artist
    properties of Artist classes only."""
    if aspect == "signature":
        return not (mpl["form"] == "native" and mpl["signature"] is None)
    if aspect == "properties":
        return "properties" in mpl
    return True


class Allowlist:
    """The intended differences of ``docs-allowlist.txt``: each line
    ``<key regex> | <cause regex> | why``, matched (``re.search``) against
    a difference's key (``module:Owner.name``) and its cause as the report
    heads its group (``doc: docstring differs``, ``names: missing``)."""

    def __init__(self, path: pathlib.Path) -> None:
        self.rules: list[tuple[re.Pattern[str], re.Pattern[str], str]] = []
        for line in path.read_text().splitlines() if path.exists() else []:
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            key, cause, why = (part.strip() for part in line.split(" | ", 2))
            self.rules.append((re.compile(key), re.compile(cause), why))

    def reason(self, key: str, cause: str) -> str | None:
        """Why the difference *cause* of *key* is intended, None if not."""
        return next((why for k, c, why in self.rules if k.search(key) and c.search(cause)), None)


class Comparison:
    """The callables of both implementations, compared."""

    def __init__(self, mpl: dict[str, Any], ours: dict[str, Any], patterns: list[str],
                 allowlist: Allowlist) -> None:
        #: key -> aspect -> True (matches), False (differs) or None (not compared)
        self.outcomes: dict[str, dict[str, bool | None]] = {}
        #: (aspect, cause) -> [(key, detail)]
        self.differences: dict[tuple[str, str], list[tuple[str, str]]] = defaultdict(list)
        #: The intended differences: [(key, "aspect: cause", why)]
        self.allowed: list[tuple[str, str, str]] = []
        self.native: set[str] = set()
        self.missing: list[str] = []
        for key, theirs in mpl.items():
            if not theirs["found"] or (patterns and not any(p in key for p in patterns)):
                continue
            mine = ours.get(key, {"found": False})
            if theirs["form"] == "native":
                self.native.add(key)
            if not mine["found"]:
                self.missing.append(key)
                self.outcomes[key] = {aspect: False if _compared(aspect, theirs) else None
                                      for aspect in ASPECTS}
                continue
            outcome = {}
            for aspect in ASPECTS:
                if not _compared(aspect, theirs):
                    outcome[aspect] = None
                    continue
                difference = COMPARERS[aspect](theirs, mine)
                outcome[aspect] = difference is None
                if difference is None:
                    continue
                why = allowlist.reason(key, f"{aspect}: {difference[0]}")
                if why is not None:
                    self.allowed.append((key, f"{aspect}: {difference[0]}", why))
                else:
                    self.differences[(aspect, difference[0])].append((key, difference[1]))
            self.outcomes[key] = outcome

    def share(self, keys: list[str], aspect: str) -> str:
        """How many of ``keys`` match in ``aspect``, of those compared."""
        compared = [self.outcomes[k][aspect] for k in keys if self.outcomes[k][aspect] is not None]
        if not compared:
            return "-"
        return f"{sum(compared)}/{len(compared)} ({100 * sum(compared) / len(compared):.1f}%)"

    def count(self) -> int:
        """The differences, the missing callables included, but the
        intended ones."""
        return len(self.missing) + sum(map(len, self.differences.values()))

    def report(self, failures: bool, names: NameComparison) -> tuple[str, str]:
        """The Markdown report, with every difference and the comparison
        of the module-level *names*, and what to print: the same with
        ``failures``, else only the groups' counts."""
        python = [k for k in self.outcomes if k not in self.native]
        lines = [
            "# matplotlib's signatures, docstrings and names in pltrs (Python)",
            "",
            "Written by `cargo xtask diff --docs` (`spec/diff/docs.py`). Signatures compare the parameters' "
            "names, kinds, defaults (by repr) and order (a property's accessors); docstrings are compared as "
            "importing leaves them, after matplotlib's interpolation (a class' with its constructor's); "
            "names are `__module__` and `__qualname__`, with pltrs imported as matplotlib (through `python/alias`).",
            "",
            f"* Callables: {len(self.outcomes)}, {len(self.missing)} of them missing in pltrs",
            f"* Signatures matching: {self.share(python, 'signature')}",
            f"* Docstrings matching: {self.share(python, 'doc')}",
            f"* Names matching: {self.share(python, 'name')}",
            f"* Artist properties (`ArtistInspector`) matching: {self.share(python, 'properties')}",
            f"* matplotlib's C++ extensions (pybind11): {len(self.native)} callables, docstrings matching "
            f"{self.share(sorted(self.native), 'doc')}, signatures {self.share(sorted(self.native), 'signature')}",
            names.summary_line(),
            f"* Intended differences (`docs-allowlist.txt`): {len(self.allowed) + len(names.allowed)}",
            "",
            "## By module",
            "",
            "| module | callables | signatures | docstrings | names | Artist properties |",
            "|---|---|---|---|---|---|",
        ]
        modules: dict[str, list[str]] = defaultdict(list)
        for key in self.outcomes:
            modules[key.split(":")[0]].append(key)
        for module, keys in sorted(modules.items(), key=lambda item: -len(item[1])):
            lines.append(f"| {module} | {len(keys)} | {self.share(keys, 'signature')} | "
                         f"{self.share(keys, 'doc')} | {self.share(keys, 'name')} | "
                         f"{self.share(keys, 'properties')} |")
        groups = sorted(self.differences.items(), key=lambda item: (ASPECTS.index(item[0][0]), -len(item[1])))
        lines += ["", "## Differences", ""]
        if self.missing:
            lines += [f"### missing in pltrs ({len(self.missing)})", "", *(f"* `{k}`" for k in self.missing), ""]
        summary = list(lines)
        for (aspect, cause), items in groups:
            heading = f"### {aspect}: {cause} ({len(items)})"
            summary.append(heading)
            lines += [heading, ""]
            for key, detail in items:
                lines.append(f"* `{key}`")
                if detail:
                    lines += ["", "  ```", *("  " + line for line in detail.splitlines()), "  ```", ""]
            lines.append("")
        named, named_summary = names.report()
        lines += named
        summary += named_summary
        allowed = self.allowed + names.allowed
        if allowed:
            intended = ["", f"## Intended differences ({len(allowed)})", "",
                        *(f"* `{key}` ({cause}): {why}" for key, cause, why in allowed)]
            lines += intended
            summary += intended
        return "\n".join(lines) + "\n", "\n".join(lines if failures else summary) + "\n"

    def as_json(self) -> dict[str, Any]:
        """Every callable's outcomes and every difference."""
        return {"outcomes": self.outcomes, "missing": self.missing, "native": sorted(self.native),
                "differences": [{"aspect": aspect, "cause": cause, "key": key, "detail": detail}
                                for (aspect, cause), items in self.differences.items() for key, detail in items],
                "allowed": [{"key": key, "cause": cause, "why": why} for key, cause, why in self.allowed]}


def main() -> int:
    """Describe the callables under both implementations and compare them."""
    parser = argparse.ArgumentParser()
    parser.add_argument("patterns", nargs="*")
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--pltrs-python", required=True)
    parser.add_argument("--failures", action="store_true")
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_intermixed_args()
    setup = driver.Setup(argparse.Namespace(root=args.root, out=args.out, gallery=args.root,
                                            pltrs_python=args.pltrs_python, mpl_python=None))
    (setup.out / "docs").mkdir(parents=True, exist_ok=True)
    api = setup.out / "docs" / "api.jsonl"
    api.write_text("".join(json.dumps(r) + "\n"
                           for r in api_index.enumerate_api(sys.executable, nested=True)))
    (mpl, mpl_names), (ours, our_names) = _dump(setup, "matplotlib", api), _dump(setup, "pltrs", api)
    allowlist = Allowlist(ALLOWLIST)
    comparison = Comparison(mpl, ours, args.patterns, allowlist)
    names = NameComparison(mpl_names, our_names, args.patterns, allowlist)
    report, printed = comparison.report(args.failures, names)
    (setup.out / "docs.md").write_text(report)
    (setup.out / "docs.json").write_text(json.dumps(
        {**comparison.as_json(), "module_names": names.as_json()}, indent=1))
    print(printed, end="")
    differences = comparison.count() + names.count()
    if args.strict and differences:
        print(f"{differences} differences (see {setup.out / 'docs.md'})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
