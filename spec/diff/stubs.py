"""matplotlib's type stubs held against pltrs (Python) at run time.

    python stubs.py [PATTERN...] [--failures] [--strict]
                    --root ROOT --out OUT --sdist SDIST
                    --pltrs-python PYTHON --mpl-python PYTHON

``cargo xtask diff --types`` runs this, with two venvs holding mypy as
matplotlib's CI has it (``python/requirements-types.txt``): one with real
matplotlib, one without. matplotlib checks its stubs against its run time
with ``mypy.stubtest`` (``tools/stubtest.py``, ``ci/mypy-stubtest-allowlist.txt``);
this runs the same check twice:

1. against real matplotlib, with its allowlist and the lines its CI script
   derives from its sources (`derived_allowlist`): the reference, matplotlib's
   own errors (it runs on another Python than its CI's, with its test
   modules installed);
2. against pltrs (Python), with the stubs it ships (``python/pltrs/*.pyi``,
   matplotlib's) and the same allowlists, its module names ``pltrs.*``.

Every error of the second run the first does not have is a difference
between pltrs and matplotlib's public API as its maintainers describe it:
a name missing at run time, a value of another type, a property where the
stub has an attribute, a signature ``inspect`` reads otherwise than the
stub. ``OUT/types.md`` counts them by kind and by module, the private
names apart (``cargo xtask diff --private``'s), and
``OUT/types.json`` holds every one. Differences that are intended are
listed with their reasons in ``types-allowlist.txt``; ``--strict`` fails
on any other. A PATTERN keeps the errors whose names contain it;
``--failures`` prints every error, not only the counts.
"""

from __future__ import annotations

import argparse
import ast
import json
import os
import pathlib
import re
import subprocess
import sys
import tomllib
from collections import Counter, defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import driver  # noqa: E402

ALLOWLIST = HERE / "types-allowlist.txt"

#: What the installed matplotlib has that its CI's build does not install,
#: added to its mypy configuration's ``exclude``.
INSTALLED_TESTS = ".*/matplotlib/tests/"

#: The kinds of error, by the start of stubtest's message after the name;
#: the rest is the error's detail.
KINDS = (
    ("is not present at runtime", "missing at run time"),
    ("is not present in stub", "missing in the stub"),
    ("failed to import", "fails to import"),
    ("failed to find stubs", "no stub"),
    ("variable differs from runtime type", "value of another type"),
    ("is inconsistent, runtime does not have parameter", "parameter missing at run time"),
    ("is inconsistent, stub does not have parameter", "parameter missing in the stub"),
    ("is inconsistent, runtime does not have", "parameter missing at run time"),
    ("is inconsistent, stub does not have", "parameter missing in the stub"),
    ("is inconsistent, stub parameter", "parameter otherwise"),
    ("is inconsistent, runtime parameter", "parameter otherwise"),
    ("is inconsistent, runtime is a property", "kind: property at run time"),
    ("is inconsistent, runtime is not a property", "kind: no property at run time"),
    ("is inconsistent, ", "inconsistent"),
    ("is read-only at runtime", "read-only at run time"),
    ("is not a ", "kind: other at run time"),
    ("is not ", "kind: other at run time"),
)


def kind_of(message: str) -> str:
    """The kind of the stubtest error *message* (what follows the name)."""
    return next((kind for start, kind in KINDS if message.startswith(start)), "other")


def derived_allowlist(sdist: pathlib.Path, existing: list[re.Pattern[str]]) -> list[str]:
    """
    The allowlist lines matplotlib's ``tools/stubtest.py`` derives from its
    sources: the functions ``_api.delete_parameter`` decorates (it adds a
    sentinel default the stubs leave out) and the alias accessors
    ``_api.define_aliases`` makes, unless *existing* allows them already.
    (matplotlib's logic, from its ``tools/stubtest.py``.)
    """
    lib = sdist / "lib"
    lines: list[str] = []

    def allowed(parts: list[str]) -> bool:
        candidates = [".".join(parts[:end]) for end in range(1, len(parts))]
        return any(rule.fullmatch(path) for rule in existing for path in candidates)

    def walk(node: ast.AST, context: list[str]) -> None:
        for child in ast.iter_child_nodes(node):
            if isinstance(child, ast.FunctionDef):
                deleted = [d for d in child.decorator_list if "delete_parameter" in ast.unparse(d)]
                if deleted:
                    name = deleted[0].args[1].value  # type: ignore[attr-defined]
                    variadic = [a.arg for a in (child.args.vararg, child.args.kwarg) if a]
                    parts = [*context, child.name]
                    if name not in variadic and not allowed(parts):
                        lines.append("\\.".join(parts))
            elif isinstance(child, ast.ClassDef):
                parts = [*context, child.name]
                for decorator in child.decorator_list:
                    if "define_aliases" in ast.unparse(decorator):
                        aliases = ast.literal_eval(decorator.args[0])  # type: ignore[attr-defined]
                        for alias in (a for names in aliases.values() for a in names):
                            if not (allowed([*parts, f"get_{alias}"])
                                    and allowed([*parts, f"set_{alias}"])):
                                lines.append("\\.".join([*parts, f"[gs]et_{alias}"]))
                walk(child, parts)

    for path in sorted((lib / "matplotlib").rglob("*.py")):
        walk(ast.parse(path.read_text(encoding="utf-8")),
             list(path.with_suffix("").relative_to(lib).parts))
    return lines


def allowlist_rules(path: pathlib.Path) -> list[str]:
    """The rules of the stubtest allowlist at *path*, comments removed."""
    rules = []
    for line in path.read_text(encoding="utf-8").splitlines():
        rule = line.partition("#")[0].strip()
        if rule:
            rules.append(rule)
    return rules


def mypy_configuration(sdist: pathlib.Path, package: str) -> str:
    """matplotlib's mypy configuration (its ``pyproject.toml``'s
    ``[tool.mypy]``) for *package*, as ``mypy.ini`` text, excluding the
    test modules the installed package has; for pltrs, mypy's own errors in
    its sources are not reported, as what is measured is the stubs against
    the run time."""
    settings = tomllib.loads((sdist / "pyproject.toml").read_text(encoding="utf-8"))["tool"]["mypy"]
    exclude = [rule.replace("/matplotlib/", f"/{package}/")
               for rule in (*settings["exclude"], INSTALLED_TESTS)]
    lines = ["[mypy]",
             f"ignore_missing_imports = {settings['ignore_missing_imports']}",
             f"enable_error_code = {', '.join(settings['enable_error_code'])}",
             f"follow_imports = {settings['follow_imports']}",
             f"warn_unreachable = {settings['warn_unreachable']}",
             "exclude = (?x)(" + " | ".join(exclude) + ")"]
    if package != "matplotlib":
        lines += [f"[mypy-{package}.*]", "ignore_errors = True"]
    return "\n".join(lines) + "\n"


class Run:
    """One run of stubtest over *package* in *python*'s venv."""

    def __init__(self, setup: driver.Setup, sdist: pathlib.Path, impl: str) -> None:
        self.impl = impl
        self.package = "matplotlib" if impl == "matplotlib" else "pltrs"
        self.python = setup.mpl_python if impl == "matplotlib" else setup.pltrs_python
        self.work = setup.out / "types" / impl
        self.work.mkdir(parents=True, exist_ok=True)
        self.environment = setup.environment(impl)
        self.sdist = sdist

    def allowlists(self, rules: list[str]) -> pathlib.Path:
        """The allowlist file of this run: *rules* (matplotlib's) for its
        package."""
        path = self.work / "allowlist.txt"
        rules = [re.sub(r"^matplotlib\\\.", f"{self.package}\\.", rule) for rule in rules]
        path.write_text("".join(rule + "\n" for rule in rules))
        return path

    def errors(self, rules: list[str]) -> list[dict[str, str]]:
        """stubtest's errors, each its name (with matplotlib's package
        name), message, module and the whole text stubtest printed."""
        config = self.work / "mypy.ini"
        config.write_text(mypy_configuration(self.sdist, self.package))
        command = [self.python, "-m", "mypy.stubtest", f"--mypy-config-file={config}",
                   "--ignore-disjoint-bases", f"--allowlist={self.allowlists(rules)}",
                   self.package]
        done = subprocess.run(command, cwd=self.work, env=self.environment,
                              capture_output=True, text=True)
        (self.work / "stubtest.txt").write_text(done.stdout + done.stderr)
        if "not checking stubs due to mypy build errors" in done.stdout:
            raise RuntimeError(f"stubtest ({self.impl}) could not build the stubs:\n{done.stdout}")
        if done.returncode not in (0, 1):
            raise RuntimeError(f"stubtest ({self.impl}) failed:\n{done.stdout}{done.stderr}")
        return [error for block in re.split(r"\n(?=error: )", done.stdout)
                if (error := self.parse(block)) is not None]

    def parse(self, block: str) -> dict[str, str] | None:
        """One error of stubtest's output, or None for what is no error."""
        found = re.match(r"error: (\S+) (.*)", block)
        if found is None:
            return None
        name, message = found.groups()
        name = re.sub(rf"^{self.package}\b", "matplotlib", name)
        stub = re.search(r"^Stub: in file (\S+?)(?::\d+)?$", block, re.MULTILINE)
        module = module_of(name, stub.group(1) if stub else None, self.package)
        text = re.sub(r"\n\nFound \d+ errors?.*", "", block.strip(), flags=re.DOTALL)
        return {"name": name, "message": message.replace(f"{self.package}.", "matplotlib."),
                "module": module, "text": text,
                "stubbed": stub is None or stub.group(1).endswith(".pyi")}


def module_of(name: str, stub: str | None, package: str) -> str:
    """The module of the error about *name*: the module of its stub file,
    or *name*'s parent where there is none."""
    if stub is None:
        return name.rpartition(".")[0] or name
    parts = pathlib.Path(stub).with_suffix("").parts
    inside = parts[len(parts) - parts[::-1].index(package):]
    return ".".join(("matplotlib", *inside)).removesuffix(".__init__")


def is_private(name: str) -> bool:
    """Whether *name* is private: its last part (``_x``, not ``__x__``),
    or a special method of a private class (``_Curve.__init__``). A public
    member of a private class or module is public (``_AxesBase.transData``
    is ``Axes.transData``)."""
    *owners, last = name.split(".")

    def private(part: str) -> bool:
        return part.startswith("_") and not part.endswith("__")

    return private(last) or (last.endswith("__") and bool(owners) and private(owners[-1]))


class Allowlist:
    """The intended differences of ``types-allowlist.txt``: each line
    ``<name regex> | <message regex> | why``, matched (``re.search``)
    against an error's name (``matplotlib.module.Owner.name``) and
    stubtest's message."""

    def __init__(self, path: pathlib.Path) -> None:
        self.rules: list[tuple[re.Pattern[str], re.Pattern[str], str]] = []
        for line in path.read_text().splitlines() if path.exists() else []:
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            name, message, why = (part.strip() for part in line.split(" | ", 2))
            self.rules.append((re.compile(name), re.compile(message), why))

    def reason(self, error: dict[str, str]) -> str | None:
        """Why *error* is intended, None if it is not."""
        return next((why for n, m, why in self.rules
                     if n.search(error["name"]) and m.search(error["message"])), None)


class Comparison:
    """pltrs' errors that matplotlib's run does not have, sorted out."""

    def __init__(self, reference: list[dict[str, str]], ours: list[dict[str, str]],
                 patterns: list[str], allowlist: Allowlist) -> None:
        own = {(e["name"], e["message"]) for e in reference}
        kept = [e for e in ours if not patterns or any(p in e["name"] for p in patterns)]
        # A module without a stub is read from its code, matplotlib's or
        # pltrs': checked against itself, it says nothing of matplotlib's API.
        self.unstubbed = [e for e in kept if not e["stubbed"]]
        kept = [e for e in kept if e["stubbed"]]
        self.reference = reference
        self.shared = [e for e in kept if (e["name"], e["message"]) in own]
        new = [{**e, "kind": kind_of(e["message"])} for e in kept
               if (e["name"], e["message"]) not in own]
        self.allowed = [(e, why) for e in new if (why := allowlist.reason(e)) is not None]
        rest = [e for e in new if allowlist.reason(e) is None]
        self.private = [e for e in rest if is_private(e["name"])]
        self.public = [e for e in rest if not is_private(e["name"])]

    def count(self) -> int:
        """The differences, but the intended ones."""
        return len(self.public) + len(self.private)

    def report(self, failures: bool) -> tuple[str, str]:
        """The Markdown report, with every error, and what to print: the
        same with ``failures``, else the counts."""
        lines = [
            "# matplotlib's type stubs against pltrs (Python)",
            "",
            "Written by `cargo xtask diff --types` (`spec/diff/stubs.py`): `mypy.stubtest` with "
            "matplotlib's stubs and allowlists, against real matplotlib (the reference) and against "
            "pltrs (Python), whose stubs are matplotlib's (`python/pltrs/*.pyi`).",
            "",
            f"* matplotlib's own errors (the reference run): {len(self.reference)}",
            f"* pltrs' errors the reference does not have: {len(self.public) + len(self.private)} "
            f"({len(self.public)} public, {len(self.private)} private)",
            f"* Intended differences (`types-allowlist.txt`): {len(self.allowed)}",
            f"* Errors in modules without a stub, whose code is read as their stub (not compared): "
            f"{len(self.unstubbed)}",
            "",
            "## By kind",
            "",
            "| kind | public | private |",
            "|---|---|---|",
        ]
        public = Counter(e["kind"] for e in self.public)
        private = Counter(e["kind"] for e in self.private)
        for kind, _ in (public + private).most_common():
            lines.append(f"| {kind} | {public[kind]} | {private[kind]} |")
        lines += ["", "## By module", "", "| module | public | private |", "|---|---|---|"]
        modules = Counter(e["module"] for e in self.public)
        private_modules = Counter(e["module"] for e in self.private)
        for module, _ in (modules + private_modules).most_common():
            lines.append(f"| {module} | {modules[module]} | {private_modules[module]} |")
        summary = list(lines)
        for title, errors in (("Differences", self.public),
                              ("Private names (`cargo xtask diff --private`)",
                               self.private)):
            groups: dict[str, list[dict[str, str]]] = defaultdict(list)
            for error in errors:
                groups[error["kind"]].append(error)
            lines += ["", f"## {title} ({len(errors)})", ""]
            summary += ["", f"## {title} ({len(errors)})", ""]
            for kind, items in sorted(groups.items(), key=lambda item: -len(item[1])):
                summary.append(f"* {kind}: {len(items)}")
                lines += [f"### {kind} ({len(items)})", ""]
                for error in sorted(items, key=lambda e: e["name"]):
                    lines += [f"* `{error['name']}` {error['message']}", "", "  ```",
                              *("  " + line for line in error["text"].splitlines()[1:]),
                              "  ```", ""]
        if self.reference:
            own = ["", f"## matplotlib's own errors ({len(self.reference)})", "",
                   *(f"* `{e['name']}` {e['message']}" for e in self.reference)]
            lines += own
            summary += own
        if self.allowed:
            intended = ["", f"## Intended differences ({len(self.allowed)})", "",
                        *(f"* `{e['name']}` {e['message']}: {why}" for e, why in self.allowed)]
            lines += intended
            summary += intended
        return "\n".join(lines) + "\n", "\n".join(lines if failures else summary) + "\n"

    def as_json(self) -> dict[str, Any]:
        """Every error, sorted out."""
        return {"reference": self.reference, "shared": self.shared,
                "unstubbed": self.unstubbed, "public": self.public,
                "private": self.private,
                "allowed": [{**e, "why": why} for e, why in self.allowed]}


def main() -> int:
    """Run stubtest under both implementations and compare."""
    parser = argparse.ArgumentParser()
    parser.add_argument("patterns", nargs="*")
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--sdist", required=True, type=pathlib.Path)
    parser.add_argument("--pltrs-python", required=True)
    parser.add_argument("--mpl-python", required=True)
    parser.add_argument("--failures", action="store_true")
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_intermixed_args()
    setup = driver.Setup(argparse.Namespace(root=args.root, out=args.out, gallery=args.root,
                                            pltrs_python=args.pltrs_python,
                                            mpl_python=args.mpl_python))
    os.makedirs(setup.out, exist_ok=True)
    rules = allowlist_rules(args.sdist / "ci" / "mypy-stubtest-allowlist.txt")
    rules += derived_allowlist(args.sdist, [re.compile(rule) for rule in rules])
    reference = Run(setup, args.sdist, "matplotlib").errors(rules)
    ours = Run(setup, args.sdist, "pltrs").errors(rules)
    comparison = Comparison(reference, ours, args.patterns, Allowlist(ALLOWLIST))
    report, printed = comparison.report(args.failures)
    (setup.out / "types.md").write_text(report)
    (setup.out / "types.json").write_text(json.dumps(comparison.as_json(), indent=1))
    print(printed, end="")
    if args.strict and comparison.count():
        print(f"{comparison.count()} differences (see {setup.out / 'types.md'})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
