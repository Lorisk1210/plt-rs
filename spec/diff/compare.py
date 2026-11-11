"""Compares the records of one program under matplotlib and under pltrs.

``differences(program, mpl, pltrs, mpl_dir, pltrs_dir)`` lists every way
the run under pltrs (Python) answered otherwise than the run under
matplotlib, as ``Difference``s:

* ``type``: another class, or other public classes along its MRO;
* ``value``: another value (numbers within ``RTOL``/``ATOL``, ints,
  strings and booleans exactly, tuples apart from lists, numpy dtypes and
  shapes; the elements of masked arrays under their mask are not
  compared, their mask is);
* ``identity``: an object that is (or is not) one described before;
* ``calls``: a statement that ran another number of times;
* ``exception``: an exception of another class or message, a missing one,
  or one matplotlib does not raise (``crash``);
* ``warning``: a warning matplotlib does not issue, or one it issues that
  pltrs does not;
* ``stdout``: other printed output;
* ``image``: a figure missing, of another size, or drawn more than
  ``MAX_RMS`` away from matplotlib's; for a corpus' program, a file it
  writes missing or extra, or a PNG it writes more than ``MAX_RMS`` away.

Each difference has a ``cause``, the key the report groups it by: its kind,
the function or attribute the statement calls (``get_linewidth``), and
what differs (``int -> float``).
"""

from __future__ import annotations

import ast
import dataclasses
import math
import pathlib
import re
from collections import Counter
from typing import Any, Iterator

import numpy as np

#: Relative and absolute tolerance of numbers.
RTOL = 1e-6
ATOL = 1e-9
#: The largest RMS difference accepted between the images (the spec's).
MAX_RMS = 1.0
#: Functions whose results hold whatever memory held: never compared.
UNINITIALIZED = {"empty", "empty_like"}


@dataclasses.dataclass
class Difference:
    """One way pltrs answered otherwise than matplotlib."""

    program: str
    kind: str
    cause: str
    where: str
    detail: str
    source: str = ""
    line: int | None = None

    def as_json(self) -> dict[str, Any]:
        """The difference as JSON."""
        return dataclasses.asdict(self)


class Source:
    """The program's statements, to name what a site calls."""

    def __init__(self, path: pathlib.Path) -> None:
        try:
            self.text = path.read_text()
            self.tree = ast.parse(self.text)
        except (OSError, SyntaxError):
            self.text, self.tree = "", None
        self.lines = self.text.splitlines()
        self._nodes: dict[tuple[int, int], ast.AST] = {}
        self._in_functions: set[tuple[int, int]] = set()
        if self.tree is not None:
            for node in ast.walk(self.tree):
                if isinstance(node, ast.stmt):
                    self._nodes[(node.lineno, node.col_offset)] = node
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
                    for inner in ast.walk(node):
                        if inner is not node and isinstance(inner, ast.stmt):
                            self._in_functions.add((inner.lineno, inner.col_offset))

    def line(self, number: int | None) -> str:
        """The text of line *number* of the program, stripped."""
        if number is None or not 0 < number <= len(self.lines):
            return ""
        return self.lines[number - 1].strip()

    def in_function(self, site: str) -> bool:
        """Whether the statement at ``site`` is in a function: code the
        program hands matplotlib, which calls it as often as it needs."""
        line, column = (int(part) for part in site.split(":"))
        return (line, column) in self._in_functions

    def callee(self, site: str) -> str:
        """The name of what the statement at ``site`` calls or reads last."""
        line, column = (int(part) for part in site.split(":"))
        node = self._nodes.get((line, column))
        value = getattr(node, "value", None)
        return _callee(value) if value is not None else "?"


def _callee(node: ast.AST) -> str:
    while isinstance(node, (ast.Subscript, ast.Starred)):
        node = node.value
    if isinstance(node, ast.Call):
        func = node.func
        if isinstance(func, ast.Attribute):
            return func.attr
        if isinstance(func, ast.Name):
            return func.id
        return _callee(func)
    if isinstance(node, ast.Attribute):
        return node.attr
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, (ast.Tuple, ast.List)) and node.elts:
        return _callee(node.elts[0])
    if isinstance(node, ast.Compare):
        return _callee(node.left) + " " + type(node.ops[0]).__name__.lower()
    if isinstance(node, ast.BinOp):
        return _callee(node.left)
    return type(node).__name__.lower()


def _numbers_close(a: Any, b: Any) -> bool:
    if isinstance(a, (str, dict)) or isinstance(b, (str, dict)):
        # Strings, and the descriptions of what is no plain number (the
        # elements of a complex or structured array's summary).
        return a == b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(_numbers_close(x, y) for x, y in zip(a, b))
    if isinstance(a, bool) or isinstance(b, bool) or isinstance(a, int) and isinstance(b, int):
        return a == b
    if a is None or b is None:
        return a is b
    return math.isclose(a, b, rel_tol=RTOL, abs_tol=ATOL)


class _Walk:
    """Walks two descriptions side by side, yielding what differs as
    ``(kind, detail key, where, detail)``."""

    def __init__(self) -> None:
        self.firsts: dict[int, int] = {}
        self.identities: list[tuple[str, dict, dict]] = []

    def walk(self, a: dict, b: dict, where: str) -> Iterator[tuple[str, str, str, str]]:
        """What differs between the descriptions *a* (matplotlib's) and *b*."""
        if a.get("k") != b.get("k"):
            yield "type", f"{_kind(a)} -> {_kind(b)}", where, f"{_short(a)} != {_short(b)}"
            return
        kind = a["k"]
        if kind in ("int", "bool", "str", "complex", "float"):
            if not _numbers_close(a["v"], b["v"]):
                yield "value", kind, where, f"{a['v']!r} != {b['v']!r}"
        elif kind == "npscalar":
            if a["dtype"] != b["dtype"]:
                yield "type", f"numpy.{a['dtype']} -> numpy.{b['dtype']}", where, ""
            elif not _numbers_close(a["v"], b["v"]):
                yield "value", "numpy scalar", where, f"{a['v']!r} != {b['v']!r}"
        elif kind == "ndarray":
            yield from self._array(a, b, where)
        elif kind in ("tuple", "list", "set", "frozenset"):
            yield from self._sequence(a, b, where)
        elif kind == "dict":
            yield from self._dict(a, b, where)
        elif kind in ("class", "module", "callable"):
            if a["name"] != b["name"]:
                yield "value", kind, where, f"{a['name']} != {b['name']}"
        elif kind == "obj":
            yield from self._object(a, b, where)
        elif kind in ("bytes", "bytearray"):
            if a["len"] != b["len"]:
                yield "value", "bytes length", where, f"{a['len']} != {b['len']}"
        elif kind == "undescribable":
            if a["type"] != b["type"]:
                yield "type", f"{a['type']} -> {b['type']}", where, ""

    def _array(self, a: dict, b: dict, where: str) -> Iterator[tuple[str, str, str, str]]:
        if a["dtype"] != b["dtype"]:
            yield "type", f"dtype {a['dtype']} -> {b['dtype']}", where, ""
        if a["type"] != b["type"]:
            yield "type", f"{a['type']} -> {b['type']}", where, ""
        if a["shape"] != b["shape"]:
            yield "value", "array shape", where, f"{a['shape']} != {b['shape']}"
            return
        if ("mask" in a) != ("mask" in b):
            yield "type", "masked" if "mask" in a else "not masked", where, ""
        elif "mask" in a and a["mask"] != b["mask"]:
            yield "value", "array mask", where, ""
        if a["dtype"] == b["dtype"] or (a["dtype"][0] in "iuf" and b["dtype"][0] in "iuf"):
            av, bv = _unmasked(a), _unmasked(b)
            if isinstance(av, dict) and isinstance(bv, dict):
                keys = ("head", "sum", "min", "max", "nonfinite")
                bad = [k for k in keys if not _numbers_close(av.get(k), bv.get(k))]
                if bad:
                    yield "value", "array values", where, f"summary {', '.join(bad)} differ"
            elif isinstance(av, list) and isinstance(bv, list) and av and isinstance(av[0], dict):
                for index, (x, y) in enumerate(zip(av, bv)):
                    yield from self.walk(x, y, f"{where}[{index}]")
            elif not _arrays_close(av, bv):
                yield "value", "array values", where, _array_detail(av, bv)

    def _sequence(self, a: dict, b: dict, where: str) -> Iterator[tuple[str, str, str, str]]:
        if a["len"] != b["len"]:
            yield "value", f"{a['k']} length", where, f"{a['len']} != {b['len']}"
            return
        for index, (x, y) in enumerate(zip(a.get("items", []), b.get("items", []))):
            yield from self.walk(x, y, f"{where}[{index}]")

    def _dict(self, a: dict, b: dict, where: str) -> Iterator[tuple[str, str, str, str]]:
        keys_a = [_key(k) for k, _ in a.get("items", [])]
        keys_b = [_key(k) for k, _ in b.get("items", [])]
        if sorted(keys_a) != sorted(keys_b):
            missing = sorted(set(keys_a) - set(keys_b))[:5]
            extra = sorted(set(keys_b) - set(keys_a))[:5]
            yield "value", "dict keys", where, f"missing {missing}, extra {extra}"
            return
        values_b = {_key(k): v for k, v in b.get("items", [])}
        for k, v in a.get("items", []):
            yield from self.walk(v, values_b[_key(k)], f"{where}[{_key(k)}]")

    def _object(self, a: dict, b: dict, where: str) -> Iterator[tuple[str, str, str, str]]:
        if a["type"] != b["type"]:
            if a["type"].rsplit(".", 1)[-1] == b["type"].rsplit(".", 1)[-1]:
                yield "type", f"module of {a['type']} -> {b['type']}", where, ""
            else:
                yield "type", f"{a['type']} -> {b['type']}", where, ""
                return
        else:
            # Classes pltrs adds along the MRO (its mixins) change no
            # `isinstance` a program makes; a class of matplotlib's missing
            # does.
            missing = [c for c in a["mro"] if c not in b["mro"]]
            if missing:
                yield "type", f"MRO of {a['type']}: missing {missing}", where, ""
        if "first" in a and "first" in b:
            self.firsts[a["first"]] = b["first"]
        else:
            self.identities.append((where, a, b))
        for field in ("vertices", "codes", "points", "matrix"):
            if field in a and field in b:
                yield from self.walk(a[field], b[field], f"{where}.{field}")
            elif field in a:
                yield "value", f"{field} missing", where, ""

    def identity(self) -> Iterator[tuple[str, str, str, str]]:
        """What differs in identity, once every first description is paired."""
        for where, a, b in self.identities:
            if "same_as" in a and "same_as" not in b:
                yield "identity", "a new object where matplotlib returns the same", where, ""
            elif "same_as" in b and "same_as" not in a:
                yield "identity", "the same object where matplotlib returns a new one", where, ""
            elif "same_as" in a and self.firsts.get(a["same_as"], b["same_as"]) != b["same_as"]:
                yield "identity", "another object than matplotlib's", where, ""


def _unmasked(description: dict) -> Any:
    """The elements of an array description a program can read: a masked
    array's data under its mask is whatever its computation left there
    (numpy's masked operations leave it unspecified), so masked elements
    read as None."""
    values, mask = description["v"], description.get("mask")
    if isinstance(values, list) and isinstance(mask, list) and len(mask) == len(values):
        return [None if masked else value for value, masked in zip(values, mask)]
    return values


def _key(description: dict) -> str:
    """A dict key as text, the same for equal keys under both (a tuple's
    as its items', such as a table's ``(row, column)`` keys)."""
    if description.get("k") == "tuple":
        return "(" + ", ".join(_key(item) for item in description.get("items", [])) + ")"
    return repr(description.get("v", description.get("name", description.get("type"))))


def _kind(description: dict) -> str:
    kind = description.get("k")
    if kind == "obj":
        return description["type"]
    if kind == "ndarray":
        return f"ndarray[{description['dtype']}]"
    if kind == "npscalar":
        return f"numpy.{description['dtype']}"
    return str(kind)


def _short(description: dict) -> str:
    text = repr(description.get("v", description.get("type", description.get("items", ""))))
    return text[:80]


def _as_array(values: list) -> np.ndarray:
    def number(x: Any) -> Any:
        """*x* of a description as a number (a masked element as NaN)."""
        if x is None:
            return math.nan
        if isinstance(x, str):
            return {"nan": math.nan, "inf": math.inf, "-inf": -math.inf}.get(x, math.nan)
        if isinstance(x, list):
            return complex(number(x[0]), number(x[1]))
        return x
    return np.asarray([number(x) for x in values])


def _arrays_close(a: list, b: list) -> bool:
    if len(a) != len(b):
        return False
    if not a:
        return True
    x, y = _as_array(a), _as_array(b)
    if x.dtype.kind == "b" or y.dtype.kind == "b":
        return bool(np.array_equal(x, y))
    return bool(np.allclose(x, y, rtol=RTOL, atol=ATOL, equal_nan=True))


def _array_detail(a: list, b: list) -> str:
    x, y = _as_array(a), _as_array(b)
    if x.dtype.kind in "biufc" and y.dtype.kind in "biufc" and x.size == y.size:
        with np.errstate(invalid="ignore"):
            delta = np.abs(x.astype(complex) - y.astype(complex))
        delta = np.where(np.isnan(delta), 0, delta)
        index = int(np.argmax(delta))
        return f"max |diff| {float(delta[index]):.3g} at {index}: {a[index]!r} != {b[index]!r}"
    return f"{str(a)[:60]} != {str(b)[:60]}"


def _compare_sites(program: str, source: Source, mpl: dict, pltrs: dict, crashed: bool) -> Iterator[Difference]:
    walker = _Walk()
    found: list[tuple[str, str, str, str, str]] = []
    # A statement that raised under either is an exception difference, not
    # one of calls.
    raised = {e["statement_line"] for e in mpl["errors"] + pltrs["errors"]}
    for site, entry in mpl["sites"].items():
        other = pltrs["sites"].get(site)
        callee = source.callee(site)
        calls = "callback" if source.in_function(site) else "calls"
        if other is None:
            if not crashed and int(site.split(":")[0]) not in raised:
                found.append((calls, callee, "never runs", site, f"runs {entry['count']} times under matplotlib"))
            continue
        if entry["count"] != other["count"]:
            if not crashed:
                found.append((calls, callee, "runs another number of times", site,
                              f"{entry['count']} != {other['count']}"))
            if calls == "callback":
                # The occurrences no longer correspond.
                continue
        if callee in UNINITIALIZED:
            continue
        for index, (a, b) in enumerate(zip(entry["values"], other["values"])):
            where = site if index == 0 else f"{site}#{index}"
            for kind, key, at, detail in walker.walk(a, b, where):
                found.append((kind, callee, key, at, detail))
    for site, entry in pltrs["sites"].items():
        if site not in mpl["sites"] and not crashed and int(site.split(":")[0]) not in raised:
            calls = "callback" if source.in_function(site) else "calls"
            found.append((calls, source.callee(site), "runs only under pltrs", site,
                          f"runs {entry['count']} times"))
    for kind, key, at, detail in walker.identity():
        found.append((kind, source.callee(at.split("#")[0].split("[")[0].split(".")[0]), key, at, detail))
    seen: set[tuple[str, str, str]] = set()
    for kind, callee, key, at, detail in found:
        site = at.split("#")[0].split("[")[0].split(".")[0]
        line = int(site.split(":")[0])
        # One difference per site and cause: a list of 30 lines of one
        # wrong type is one finding.
        if (site, kind, key) in seen:
            continue
        seen.add((site, kind, key))
        yield Difference(program, kind, f"{kind}[{callee}]: {key}", at, detail, source.line(line), line)


def _error_key(error: dict | None) -> str:
    if error is None:
        return "none"
    return error["type"].rsplit(".", 1)[-1]


def _compare_errors(program: str, source: Source, mpl: dict, pltrs: dict) -> Iterator[Difference]:
    by_statement = {}
    for side, record in (("mpl", mpl), ("pltrs", pltrs)):
        for error in record["errors"]:
            by_statement.setdefault(error["statement"], {})[side] = error
    for which in ("uncaught",):
        a, b = mpl.get(which), pltrs.get(which)
        if a or b:
            by_statement.setdefault("uncaught", {}).update({k: v for k, v in (("mpl", a), ("pltrs", b)) if v})
    for statement, pair in by_statement.items():
        a, b = pair.get("mpl"), pair.get("pltrs")
        line = (a or b).get("line") or (a or b).get("statement_line")
        callee = _line_callee(source, line)
        if a and not b:
            yield Difference(program, "exception", f"exception[{callee}]: missing {_error_key(a)}",
                             f"line {line}", f"{_error_key(a)}: {a['message'][:200]}", source.line(line), line)
        elif b and not a:
            missing = _MISSING_NAME.match(b["message"]) if _error_key(b) == "AttributeError" else None
            if missing:
                yield Difference(program, "missing", f"missing name: {missing['owner']}.{missing['name']}",
                                 f"line {line}", b["message"][:300], source.line(line), line)
            else:
                yield Difference(program, "exception", f"crash: {_error_key(b)}: {_message_key(b['message'])}",
                                 f"line {line}", f"{b['type']}: {b['message'][:300]}", source.line(line), line)
        elif a["type"] != b["type"]:
            yield Difference(program, "exception", f"exception[{callee}]: {_error_key(a)} -> {_error_key(b)}",
                             f"line {line}", f"{a['message'][:150]} | {b['message'][:150]}", source.line(line), line)
        elif a["message"] != b["message"]:
            yield Difference(program, "exception", f"exception message[{callee}]: {_error_key(a)}",
                             f"line {line}", f"{a['message'][:150]!r} != {b['message'][:150]!r}",
                             source.line(line), line)


#: An AttributeError for a name pltrs lacks: missing API, not a wrong answer.
_MISSING_NAME = re.compile(r"(?:type object |module )?'(?P<owner>[\w.]+)' (?:object )?has no attribute '(?P<name>\w+)'$")


def _message_key(message: str) -> str:
    """The first words of an error message, without the numbers that vary."""
    first = message.strip().splitlines()[0] if message.strip() else ""
    first = re.sub(r"\d+(\.\d+)?", "N", first)
    first = re.sub(r"'[^']*'", "'…'", first)
    return first[:90]


def _line_callee(source: Source, line: int | None) -> str:
    if source.tree is None or line is None:
        return "?"
    for node in ast.walk(source.tree):
        if isinstance(node, ast.stmt) and node.lineno == line and hasattr(node, "value") and node.value is not None:
            return _callee(node.value)
    return "?"


def _warning_key(warning: dict) -> tuple[str, str, Any]:
    return warning["category"], warning["message"], warning["line"]


def _warning_counts(record: dict) -> Counter:
    """How many times a run issued each warning: those it recorded and
    those it only counted (``trace.py``'s ``MAX_WARNINGS``)."""
    counts = Counter(_warning_key(w) for w in record["warnings"])
    for category, message, line, count in record["warnings_dropped"]:
        counts[category, message, line] += count
    return counts


def _compare_warnings(program: str, source: Source, mpl: dict, pltrs: dict) -> Iterator[Difference]:
    a, b = _warning_counts(mpl), _warning_counts(pltrs)
    for (category, message, line), count in (a - b).items():
        short = category.rsplit(".", 1)[-1]
        yield Difference(program, "warning", f"warning missing: {short}: {_message_key(message)}",
                         f"line {line}", f"{count}x {message[:300]}", source.line(line), line)
    for (category, message, line), count in (b - a).items():
        short = category.rsplit(".", 1)[-1]
        yield Difference(program, "warning", f"warning extra: {short}: {_message_key(message)}",
                         f"line {line}", f"{count}x {message[:300]}", source.line(line), line)


def _compare_stdout(program: str, mpl: dict, pltrs: dict) -> Iterator[Difference]:
    a, b = mpl["stdout"].splitlines(), pltrs["stdout"].splitlines()
    if a == b:
        return
    for index, (x, y) in enumerate(zip(a, b)):
        if x != y:
            yield Difference(program, "stdout", "stdout", f"line {index + 1} of the output", f"{x[:150]!r} != {y[:150]!r}")
            return
    yield Difference(program, "stdout", "stdout", "output length", f"{len(a)} != {len(b)} lines")


def image_rms(expected: pathlib.Path, actual: pathlib.Path) -> tuple[float | None, str]:
    """matplotlib's RMS between two PNGs (``testing.compare``'s), or None
    and why when they cannot be compared. Figures of any size are read
    (ultraplot's tables of colormaps exceed Pillow's limit of decompression
    bombs), and their squared differences summed a band of rows at a time."""
    from PIL import Image
    Image.MAX_IMAGE_PIXELS = None

    def load(path: pathlib.Path) -> np.ndarray:
        """The image as matplotlib's comparison reads it."""
        with Image.open(path) as image:
            if image.mode != "RGBA" or image.getextrema()[3][0] == 255:
                image = image.convert("RGB")
            return np.asarray(image).astype(np.int16)

    a, b = load(expected), load(actual)
    if a.shape != b.shape:
        return None, f"size {a.shape[1]}x{a.shape[0]} != {b.shape[1]}x{b.shape[0]}"
    band = 1024
    squares = sum(float((((a[i:i + band] - b[i:i + band]).astype(float)) ** 2).sum())
                  for i in range(0, a.shape[0], band))
    return float(np.sqrt(squares / a.size)), ""


def _figures_by_key(mpl: dict, pltrs: dict) -> tuple[dict, dict]:
    """The figures of both runs keyed by number, or by label when the
    runs opened other numbers but each figure has a label of its own
    (``plt.subplots(num=label)``): one figure more under either then
    leaves the others paired."""
    def keyed(figures: list[dict], key: str) -> dict:
        return {f.get(key): f for f in figures}
    a, b = keyed(mpl["figures"], "number"), keyed(pltrs["figures"], "number")
    if sorted(a) != sorted(b):
        labels_a, labels_b = keyed(mpl["figures"], "label"), keyed(pltrs["figures"], "label")
        labelled = all(len(labels) == len(figures) and "" not in labels and None not in labels
                       for labels, figures in ((labels_a, a), (labels_b, b)))
        if labelled:
            return labels_a, labels_b
    return a, b


def _compare_figures(program: str, mpl: dict, pltrs: dict, mpl_dir: pathlib.Path,
                     pltrs_dir: pathlib.Path) -> Iterator[Difference]:
    a, b = _figures_by_key(mpl, pltrs)
    if sorted(a) != sorted(b):
        yield Difference(program, "image", "image: other figures open", "figures", f"{sorted(a)} != {sorted(b)}")
    for number in sorted(set(a) & set(b)):
        fa, fb = a[number], b[number]
        if "png" not in fa:
            continue
        if "png" not in fb:
            error = fb.get("error", {})
            yield Difference(program, "exception",
                             f"crash: {_error_key(error)}: {_message_key(error.get('message', ''))}",
                             f"savefig of figure {number}", error.get("message", "")[:300])
            continue
        rms, why = image_rms(mpl_dir / fa["png"], pltrs_dir / fb["png"])
        if rms is None:
            yield Difference(program, "image", "image: other size", f"figure {number}", why)
        elif rms > MAX_RMS:
            yield Difference(program, "image", "image: RMS > 1", f"figure {number}", f"RMS {rms:.3f}")


def _compare_written(program: str, mpl: dict, pltrs: dict, mpl_dir: pathlib.Path,
                     pltrs_dir: pathlib.Path) -> Iterator[Difference]:
    """The files a corpus' program wrote in its tree: the same ones, and
    the same pixels in each PNG."""
    a, b = set(mpl.get("written", [])), set(pltrs.get("written", []))
    if a != b:
        yield Difference(program, "image", "image: other files written", "files",
                         f"missing {sorted(a - b)[:5]}, extra {sorted(b - a)[:5]}")
    for name in sorted(a & b):
        if name.lower().endswith(".png"):
            try:
                rms, why = image_rms(mpl_dir / "tree" / name, pltrs_dir / "tree" / name)
            except OSError as err:
                yield Difference(program, "image", "image: written PNG unreadable", name, str(err)[:300])
                continue
            if rms is None:
                yield Difference(program, "image", "image: written PNG of other size", name, why)
            elif rms > MAX_RMS:
                yield Difference(program, "image", "image: written PNG RMS > 1", name, f"RMS {rms:.3f}")


def differences(program: str, path: pathlib.Path, mpl: dict, pltrs: dict, mpl_dir: pathlib.Path,
                pltrs_dir: pathlib.Path) -> list[Difference]:
    """Every difference of the run under pltrs from the run under matplotlib."""
    source = Source(path)
    crashed = bool(mpl.get("uncaught")) != bool(pltrs.get("uncaught")) or (
        mpl.get("uncaught") and pltrs.get("uncaught") and mpl["uncaught"]["line"] != pltrs["uncaught"]["line"])
    found = list(_compare_errors(program, source, mpl, pltrs))
    found += _compare_sites(program, source, mpl, pltrs, bool(crashed))
    found += _compare_warnings(program, source, mpl, pltrs)
    found += _compare_stdout(program, mpl, pltrs)
    found += _compare_figures(program, mpl, pltrs, mpl_dir, pltrs_dir)
    found += _compare_written(program, mpl, pltrs, mpl_dir, pltrs_dir)
    return found
