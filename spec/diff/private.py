"""matplotlib's private names that libraries use, in pltrs (Python).

    python private.py [PATTERN...] [--failures] [--strict]
                      --root ROOT --out OUT --pltrs-python PYTHON

``cargo xtask diff --private`` runs this in the reference venv (after
building pltrs (Python)'s extension). For every row of the census
(``private_census.json``, written by ``private_census.py``: a private
name of matplotlib's that other libraries read, call, write, delete or
override, with the class or module of matplotlib's defining it) it

1. describes the name under matplotlib (``private_dump.py``) wherever the
   generators' scenes and objects made alone hold an instance of the
   class: present or not, its type and its value (plain data as it is,
   matplotlib's value types by their public state, objects of the figure
   by which object they are, callables by their signature), in each of
   the figure's states (as built, drawn, unpickled, every Axes cleared);
   module names on their module;
2. describes the same names at the same places under pltrs, imported
   through its alias as programs importing matplotlib get it;
3. for the names the census shows written, assigns another value (one
   the name has elsewhere under matplotlib, or the value changed) under
   both, and compares what follows: the name read back, the public
   getter matplotlib's class has for it, the name once the figure is
   unpickled, and the figure drawn again (RMS ≤ 1);
4. writes ``OUT/private.md``: the share of the census' rows matching,
   overall, per class or module and by the number of libraries using
   them, the differences grouped by cause (the names more libraries use
   first), the rows no scene reaches, and ``OUT/private.json``.

Differences that are intended are listed, each with its proof, in
``private-allowlist.txt`` (``<owner:name regex> | <cause regex> | why``);
the report names them apart, and ``--strict`` does not count them.

A PATTERN keeps the rows whose keys (``owner:name``) contain it.
``--failures`` prints every difference; ``--strict`` fails while any
remains.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import pathlib
import subprocess
import sys
from collections import Counter, defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare  # noqa: E402
import driver  # noqa: E402
from docs import Allowlist  # noqa: E402

CENSUS = HERE / "private_census.json"
ALLOWLIST = HERE / "private-allowlist.txt"
#: The receiver of the census' rows whose receiver is unknown.
UNKNOWN = "(receiver unknown)"
#: The cause of a place matplotlib's walk reaches and pltrs's does not.
UNREACHED = "object not reached"


def row_key(row: dict[str, Any]) -> str:
    """A census row's key: ``owner:name``."""
    return f"{row['owner'] or UNKNOWN}:{row['name']}"


def _dump(setup: driver.Setup, impl: str, out: pathlib.Path, *extra: str) -> dict[str, Any]:
    """``private_dump.py``'s output under ``impl``."""
    python = setup.mpl_python if impl == "matplotlib" else setup.pltrs_python
    env = setup.environment(impl)
    if impl == "pltrs":
        env["PYTHONPATH"] = os.pathsep.join([str(setup.root / "python" / "alias"), env["PYTHONPATH"]])
    done = subprocess.run([python, str(HERE / "private_dump.py"), str(CENSUS), str(out), *extra],
                          env=env, capture_output=True, text=True, cwd=out.parent)
    if done.returncode != 0:
        raise SystemExit(f"describing the private names under {impl} failed:\n{done.stderr[-5000:]}")
    return json.loads(out.read_text())


# Comparing descriptions --------------------------------------------------


def same(mpl: Any, ours: Any) -> bool:
    """Whether two descriptions (`private_walk.describe`) agree: numbers
    within `compare`'s tolerances."""
    if isinstance(mpl, float) and isinstance(ours, (int, float)) and not isinstance(ours, bool):
        return math.isclose(mpl, ours, rel_tol=compare.RTOL, abs_tol=compare.ATOL)
    if isinstance(mpl, list) and isinstance(ours, list):
        return len(mpl) == len(ours) and all(same(a, b) for a, b in zip(mpl, ours))
    if isinstance(mpl, dict) and isinstance(ours, dict):
        return mpl.keys() == ours.keys() and all(same(mpl[k], ours[k]) for k in mpl)
    return mpl == ours


def _short(description: Any, width: int = 100) -> str:
    text = json.dumps(description)
    return text if len(text) <= width else text[: width - 3] + "..."


def probe_difference(mpl: dict[str, Any], ours: dict[str, Any] | None) -> tuple[str, str] | None:
    """The cause and detail of a difference between two probes of a name
    (`private_dump._probe`), or None. A name matplotlib does not have in
    a state (its property refusing there) is not compared."""
    if not mpl.get("present"):
        return None
    if ours is None or ours.get("missing_object"):
        return UNREACHED, "pltrs reaches no object at this path"
    if not ours.get("present"):
        error = ours.get("error") or {}
        return f"missing ({error.get('raises', '?')})", error.get("message", "")
    theirs, mine = mpl["value"], ours["value"]
    if theirs[0] != mine[0]:
        return f"type {theirs[0]} -> {mine[0]}", f"{_short(theirs)} -> {_short(mine)}"
    if not same(theirs[1], mine[1]):
        what = "signature" if isinstance(theirs[1], dict) and "signature" in theirs[1] else "value"
        if isinstance(theirs[1], dict) and "is" in theirs[1]:
            what = "identity"
        return what, f"{_short(theirs[1])} -> {_short(mine[1])}"
    return None


# Planning the writes -------------------------------------------------------


def literal(description: Any) -> str | None:
    """Python source of a described plain value, or None."""
    kind, state = description
    if isinstance(state, bool) or state is None or kind == "str":
        return repr(state)
    if isinstance(state, int):
        return repr(state)
    if isinstance(state, float) or state in ("nan", "inf", "-inf"):
        return f"float({str(state)!r})"
    if kind in ("list", "tuple") and isinstance(state, list):
        items = [literal(item) for item in state]
        if None in items or "..." in state:
            return None
        return ("[" + ", ".join(items) + "]") if kind == "list" else ("(" + ", ".join(items) + ",)")
    if kind == "ndarray" and isinstance(state, dict) and "values" in state:
        return f"np.array({state['values']!r}).reshape({tuple(state['shape'])!r})"
    return None


def _changed(description: Any) -> str | None:
    """Python source of a value of the same type, other than the described
    one (for numbers and booleans)."""
    kind, state = description
    if isinstance(state, bool):
        return repr(not state)
    if isinstance(state, int):
        return repr(state + 1)
    if isinstance(state, float):
        return repr(state * 0.5 + 1)
    return None


def plan_writes(rows: list[dict[str, Any]], mpl: dict[str, Any]) -> list[dict[str, Any]]:
    """One assignment per census row the libraries write: on the first
    object of a drawn scene holding the name, of another value the name
    has under matplotlib (another object of the figure for a name holding
    one), else the value changed."""
    writes = []
    for row in rows:
        if "written" not in row["uses"]:
            continue
        probes = [(key, record) for key, record in _row_probes(row, mpl) if "@drawn|" in key]
        for key, record in probes:
            probe = record[row["name"]]
            if not probe.get("present"):
                continue
            scene, path = key.split("@", 1)[0], key.split("|", 1)[1]
            write = {"id": str(len(writes)), "row": row_key(row), "scene": scene, "path": path,
                     "name": row["name"], "getter": record.get("@getters", {}).get(row["name"])}
            other = _other_value(row["name"], probe["value"], key, probes)
            if other is None:
                continue
            write.update(other)
            writes.append(write)
            break
    return writes


def _other_value(name: str, value: Any, key: str, probes: list[tuple[str, dict[str, Any]]]) -> dict[str, str] | None:
    state = value[1]
    if isinstance(state, dict) and "is" in state:
        prefix = key.split("|", 1)[0]
        for other_key, record in probes:
            other = record[name].get("value") or [None, None]
            if (other_key.split("|", 1)[0] == prefix and isinstance(other[1], dict)
                    and other[1].get("is") not in (None, state["is"])):
                return {"value_path": other[1]["is"]}
        return None
    own = literal(value)
    if own is None:
        return None
    for _, record in probes:
        other = record[name].get("value")
        source = literal(other) if other and other[0] == value[0] else None
        if source is not None and source != own:
            return {"literal": source}
    changed = _changed(value)
    return {"literal": changed} if changed else None


def write_difference(mpl: dict[str, Any], ours: dict[str, Any], images: tuple[pathlib.Path, pathlib.Path]) \
        -> tuple[str, str] | None:
    """The cause and detail of a difference in what follows an assignment."""
    for stage in ("setup", "assign"):
        if stage in ours and stage not in mpl:
            return f"write: {stage} fails", ours[stage].get("message", "")
    if "setup" in mpl or "assign" in mpl:
        return None
    for part in ("read_back", "unpickled"):
        difference = probe_difference(mpl.get(part) or {}, ours.get(part))
        if difference is not None:
            return f"write: {part.replace('_', ' ')} {difference[0]}", difference[1]
    if "getter" in mpl and not same(mpl["getter"], ours.get("getter")):
        return "write: getter", f"{_short(mpl['getter'])} -> {_short(ours.get('getter'))}"
    if mpl.get("drawn") is True:
        if ours.get("drawn") is not True:
            return "write: draw fails", _short(ours.get("drawn"))
        return _drawn_difference(*images)
    return None


def _drawn_difference(mpl: pathlib.Path, ours: pathlib.Path) -> tuple[str, str] | None:
    """Whether an assignment changed the figure drawn under both
    implementations alike: under both or under neither, and, where it
    did, no further from matplotlib's than before (images of a scene
    differing a little already, at the small size drawn)."""
    def before(path: pathlib.Path) -> pathlib.Path:
        return path.with_name(path.stem + ".before.png")
    changed = [path.read_bytes() != before(path).read_bytes() for path in (mpl, ours)]
    if changed[0] != changed[1]:
        return "write: drawn otherwise", ("the figure changes under matplotlib only" if changed[0]
                                          else "the figure changes under pltrs only")
    if changed[0]:
        after, problem = compare.image_rms(mpl, ours)
        was, _ = compare.image_rms(before(mpl), before(ours))
        if after is None or after > (was or 0.0) + compare.MAX_RMS:
            return "write: drawn otherwise", problem or f"RMS {after:.2f} after, {was or 0.0:.2f} before"
    return None


# Comparing ---------------------------------------------------------------


def _row_probes(row: dict[str, Any], dump: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    """The objects of matplotlib's dump the census row is described on."""
    classes = {row["owner"]} if row["owner"] else set(row["candidates"])
    return [(key, record) for key, record in dump["objects"].items()
            if row["name"] in record and classes & set(record.get("@census", ()))]


class Comparison:
    """The census' rows, compared."""

    def __init__(self, rows: list[dict[str, Any]], mpl: dict[str, Any], ours: dict[str, Any],
                 writes: tuple[list[dict[str, Any]], dict[str, Any], dict[str, Any], pathlib.Path],
                 patterns: list[str], allowlist: Allowlist) -> None:
        self.rows = [row for row in rows if not patterns or any(p in row_key(row) for p in patterns)]
        #: row key -> "match", "unreached" or the first cause
        self.outcome: dict[str, str] = {}
        #: cause -> [(row key, where, detail)]
        self.differences: dict[str, list[tuple[str, str, str]]] = defaultdict(list)
        self.allowed: list[tuple[str, str, str]] = []
        planned, mpl_writes, our_writes, images = writes
        by_row = {write["row"]: write for write in planned}
        for row in self.rows:
            key = row_key(row)
            found = self._compare_row(row, mpl, ours)
            write = by_row.get(key)
            if write is not None:
                pair = (images / "matplotlib" / f"{write['id']}.png", images / "pltrs" / f"{write['id']}.png")
                difference = write_difference(mpl_writes[write["id"]], our_writes.get(write["id"], {}), pair)
                if difference is not None:
                    found = [*(found or []), (difference[0], f"{write['scene']}|{write['path']}", difference[1])]
            if found is None:
                self.outcome[key] = "unreached"
                continue
            causes = []
            for cause, where, detail in found:
                why = allowlist.reason(key, cause)
                if why is not None:
                    self.allowed.append((key, cause, why))
                else:
                    causes.append(cause)
                    self.differences[cause].append((key, where, detail))
            self.outcome[key] = Counter(causes).most_common(1)[0][0] if causes else "match"

    @staticmethod
    def _compare_row(row: dict[str, Any], mpl: dict[str, Any], ours: dict[str, Any]) \
            -> list[tuple[str, str, str]] | None:
        """The row's differences, one per cause; None where nothing holds
        the name under matplotlib."""
        if row["owner"] and row["kind"].startswith("module"):
            where = f"{row['owner']}:{row['name']}"
            pairs = [(where, mpl["modules"].get(where, {}), ours["modules"].get(where))]
        else:
            pairs = [(key, record[row["name"]], (ours["objects"].get(key) or {}).get(row["name"]))
                     for key, record in _row_probes(row, mpl)]
        pairs = [pair for pair in pairs if pair[1].get("present")]
        if not pairs:
            return None
        found: dict[str, tuple[str, str, str]] = {}
        unreached = 0
        for where, theirs, mine in pairs:
            difference = probe_difference(theirs, mine)
            unreached += difference is not None and difference[0] == UNREACHED
            if difference is not None and difference[0] not in found:
                found[difference[0]] = (difference[0], where, difference[1])
        if unreached < len(pairs):
            # A place pltrs's walk reaches by another path (children in
            # another order) says nothing of the name where others do.
            found.pop(UNREACHED, None)
        return list(found.values())

    def count(self) -> int:
        """The rows differing, but for intended differences."""
        return sum(outcome not in ("match", "unreached") for outcome in self.outcome.values())

    def share(self, keys: list[str]) -> str:
        """How many of ``keys`` match, of those reached."""
        reached = [k for k in keys if self.outcome[k] != "unreached"]
        matching = sum(self.outcome[k] == "match" for k in reached)
        if not reached:
            return f"- ({len(keys)} unreached)"
        return f"{matching}/{len(reached)} ({100 * matching / len(reached):.1f}%), {len(keys) - len(reached)} unreached"

    def report(self, failures: bool) -> tuple[str, str]:
        """The Markdown report, and what to print (the same with
        ``failures``, else without the details)."""
        keys = [row_key(row) for row in self.rows]
        libraries = {row_key(row): row["name_count"] for row in self.rows}
        lines = [
            "# matplotlib's private names in pltrs (Python)",
            "",
            "Written by `cargo xtask diff --private` (`spec/diff/private.py`) from the census "
            "`spec/diff/private_census.json`: each private name other libraries use, described where the "
            "generators' scenes hold an instance of its class (built, drawn, unpickled, cleared) and on objects "
            "made alone; names the libraries write are assigned under both implementations.",
            "",
            f"* Census rows: {len(keys)}; matching {self.share(keys)}",
            f"* Rows of names two or more libraries use: {self.share([k for k in keys if libraries[k] >= 2])}",
            f"* Rows differing: {self.count()}",
            f"* Intended differences (`private-allowlist.txt`): {len(self.allowed)}",
            "",
            "## By owner",
            "",
            "| owner | rows | matching |",
            "|---|---|---|",
        ]
        owners: dict[str, list[str]] = defaultdict(list)
        for key in keys:
            owners[key.rsplit(":", 1)[0]].append(key)
        for owner, owned in sorted(owners.items(), key=lambda item: (-len(item[1]), item[0])):
            lines.append(f"| {owner} | {len(owned)} | {self.share(owned)} |")
        lines += ["", "## Differences", ""]
        summary = list(lines)
        groups = sorted(self.differences.items(),
                        key=lambda item: (-max(libraries[k] for k, _, _ in item[1]), -len(item[1])))
        for cause, items in groups:
            heading = f"### {cause} ({len(items)})"
            summary.append(heading)
            lines += [heading, ""]
            for key, where, detail in sorted(items, key=lambda item: -libraries[item[0]]):
                lines.append(f"* `{key}` ({libraries[key]} libraries) at `{where}`")
                if detail:
                    lines += ["", "  ```", *("  " + line for line in detail.splitlines()), "  ```", ""]
            lines.append("")
        unreached = [k for k in keys if self.outcome[k] == "unreached"]
        tail = ["", f"## Not reached by any scene ({len(unreached)})", "",
                *(f"* `{k}` ({libraries[k]} libraries)" for k in sorted(unreached, key=lambda k: -libraries[k]))]
        if self.allowed:
            tail += ["", f"## Intended differences ({len(self.allowed)})", "",
                     *(f"* `{key}` ({cause}): {why}" for key, cause, why in self.allowed)]
        lines += tail
        summary += tail
        return "\n".join(lines) + "\n", "\n".join(lines if failures else summary) + "\n"

    def as_json(self) -> dict[str, Any]:
        """Every row's outcome and every difference."""
        return {"outcomes": self.outcome,
                "differences": [{"cause": cause, "key": key, "where": where, "detail": detail}
                                for cause, items in self.differences.items() for key, where, detail in items],
                "allowed": [{"key": key, "cause": cause, "why": why} for key, cause, why in self.allowed]}


def main() -> int:
    """Describe the census' names under both implementations and compare."""
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
    out = setup.out / "private"
    out.mkdir(parents=True, exist_ok=True)
    rows = json.loads(CENSUS.read_text())["rows"]
    mpl = _dump(setup, "matplotlib", out / "matplotlib.json")
    ours = _dump(setup, "pltrs", out / "pltrs.json", "--plan", str(out / "matplotlib.json"))
    planned = plan_writes(rows, mpl)
    (out / "writes.json").write_text(json.dumps(planned, indent=1))
    written = [_dump(setup, impl, out / f"{impl}.writes.json", "--writes", str(out / "writes.json"),
                     "--images", str(out / "images" / impl)) for impl in ("matplotlib", "pltrs")]
    comparison = Comparison(rows, mpl, ours, (planned, written[0], written[1], out / "images"),
                            args.patterns, Allowlist(ALLOWLIST))
    report, printed = comparison.report(args.failures)
    (setup.out / "private.md").write_text(report)
    (setup.out / "private.json").write_text(json.dumps(comparison.as_json(), indent=1))
    print(printed, end="")
    if args.strict and comparison.count():
        print(f"{comparison.count()} rows differ (see {setup.out / 'private.md'})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
