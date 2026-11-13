"""Recording of goldens: the common ones and each platform's.

The goldens of ``spec/goldens`` are what matplotlib computes on Linux
x86-64. matplotlib does not compute the same everywhere: its wheel for
arm64 macOS fuses products into sums, and each platform has its own maths
library. Most of that stays within what pltrs is allowed against a golden
(``pltrs-spec``: values within a tolerance, images within an RMS of 1,
documents of the same structure), so that the common goldens hold there
too. Where matplotlib on another platform would itself fail that check
against the common golden (other contour vertices, another triangulation,
another order of equal values), its value, image or document is that
platform's golden, in
``spec/goldens/platforms/<target>/`` under the same relative path: only
what fails is kept there, and pltrs is held to the platform's golden where
there is one and to the common one otherwise. Platform goldens are written
by ``spec/mpl-reference --update-goldens`` run on that platform, which
never touches the common ones.

The processor is such a platform too. The common goldens are recorded
with AVX-512, where numpy runs the ``X86_V4`` copy of its loops; with AVX2
alone (``X86_V3``) or without it (the baseline, ``X86_V2``) its sort leaves
equal values in another order, and a triple cannot say which processor a
machine has. So ``spec/goldens/platforms/x86_64-v3`` and ``x86_64-v2`` hold
what matplotlib computes at those levels that fails for the common golden,
whatever the system, and a machine is held to the first golden recorded
of: its target's, its level's, the common one. A level's goldens are
recorded on an AVX-512 machine too, with numpy held to the level
(``NPY_DISABLE_CPU_FEATURES``, and ``OPENBLAS_CORETYPE`` for the kernels
of numpy's BLAS, which follow the processor the same way):
``cargo xtask spec --level v3`` checks pltrs against them there.
"""

from __future__ import annotations

import json
import math
import pathlib
import platform
import sys
from typing import Any, Callable

import numpy as np

SPEC_ROOT = pathlib.Path(__file__).resolve().parents[2]
GOLDEN_ROOT = SPEC_ROOT / "goldens"
PROBE_ROOT = GOLDEN_ROOT / "probes"


def _platform() -> str | None:
    """The target whose goldens this machine records, ``None`` for the
    common ones (Linux x86-64, and every platform without goldens of its
    own so far). Keep in step with ``PLATFORM`` in ``pltrs-spec``."""
    machine = {"arm64": "aarch64", "amd64": "x86_64"}.get(platform.machine().lower(),
                                                         platform.machine().lower())
    if sys.platform == "darwin" and machine in ("aarch64", "x86_64"):
        return f"{machine}-apple-darwin"
    return None


def _level() -> str | None:
    """The processor level whose goldens this machine records, ``None``
    where numpy runs as the common goldens were recorded (``X86_V4``) and
    off x86-64. Keep in step with ``level_platform`` in ``pltrs-spec``."""
    if platform.machine().lower() not in ("x86_64", "amd64"):
        return None
    from numpy._core._multiarray_umath import __cpu_features__ as features
    if features.get("X86_V4"):
        return None
    return "x86_64-v3" if features.get("X86_V3") else "x86_64-v2"


#: The directory name of this machine's platform goldens, if it has any.
PLATFORM = _platform()
#: The directory name of the goldens of this machine's processor level.
LEVEL = _level()
#: The directories this machine's goldens are looked up in before the
#: common ones, the most specific first.
_ROOTS = [GOLDEN_ROOT / "platforms" / name for name in (PLATFORM, LEVEL) if name]
#: Where this machine records what it computes otherwise, ``None`` where
#: the common goldens are recorded.
OWN_ROOT = _ROOTS[0] if _ROOTS else None
#: Whether this machine records the common goldens: Linux x86-64 with
#: AVX-512, numpy unrestricted.
RECORDS_COMMON = OWN_ROOT is None


def _first(relative: str | pathlib.Path, roots: list[pathlib.Path]) -> pathlib.Path:
    """The golden ``relative`` in the first of ``roots`` holding one, the
    common one otherwise."""
    return next((root / relative for root in roots if (root / relative).exists()),
                GOLDEN_ROOT / relative)


def golden_path(relative: str | pathlib.Path) -> pathlib.Path:
    """The golden ``relative`` (below ``spec/goldens``) this machine is
    held to: its platform's where one is recorded, else its processor
    level's, else the common one."""
    return _first(relative, _ROOTS)


def record_golden(relative: str | pathlib.Path, data: bytes,
                  passes: Callable[[bytes, bytes], bool]) -> None:
    """Record what matplotlib produced here for the golden ``relative``.

    Where the common goldens are recorded, that is the common golden.
    Elsewhere it is a golden of this machine's own (``OWN_ROOT``) if it
    fails for the golden the machine is held to without it
    (``passes(held, data)`` is false: the check pltrs' output is put to),
    and no file at all if it passes: one left from an earlier run is then
    removed.
    """
    if OWN_ROOT is None:
        common = GOLDEN_ROOT / relative
        common.parent.mkdir(parents=True, exist_ok=True)
        common.write_bytes(data)
        return
    held = _first(relative, _ROOTS[1:])
    if not held.exists():
        raise AssertionError(f"{relative} has no common golden: record it on Linux x86-64 first")
    own = OWN_ROOT / relative
    if passes(held.read_bytes(), data):
        own.unlink(missing_ok=True)
    else:
        own.parent.mkdir(parents=True, exist_ok=True)
        own.write_bytes(data)


_REGISTRY: dict[str, "ProbeSet"] = {}


def to_jsonable(value: Any) -> Any:
    """Convert matplotlib/numpy results into plain JSON-compatible values.

    Non-finite floats are kept (they are written as ``NaN``/``Infinity``,
    which the pltrs JSON reader accepts).
    """
    if isinstance(value, np.ndarray):
        if value.dtype.kind == "M":
            return [str(v) for v in value.tolist()]
        return to_jsonable(value.tolist())
    if isinstance(value, np.generic):
        return to_jsonable(value.item())
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return 0.0 if value == 0 else value  # normalise -0.0
    if isinstance(value, dict):
        return {str(k): to_jsonable(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_jsonable(v) for v in value]
    if hasattr(value, "isoformat"):
        return value.isoformat()
    raise TypeError(f"cannot convert {type(value).__name__} to a golden value")


def _canonical(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


class ProbeSet:
    """Records ``op(args) -> result`` observations for one functional area.

    Every observation is produced by actually calling matplotlib, so the
    goldens can never drift from the reference implementation.
    """

    def __init__(self, area: str) -> None:
        if area in _REGISTRY:
            raise ValueError(f"probe area {area!r} registered twice")
        self.area = area
        self._probes: dict[str, dict[str, Any]] = {}
        _REGISTRY[area] = self

    def probe(
        self,
        op: str,
        fn: Callable[..., Any],
        /,
        *,
        tol: float | None = None,
        **args: Any,
    ) -> Any:
        """Evaluate ``fn(**args)`` with matplotlib and record the outcome.

        Exceptions are recorded as ``{"error": <ExceptionType>}`` and returned
        (not raised), so tests can assert that matplotlib rejects an input.
        ``tol`` widens the relative tolerance pltrs is held to.
        """
        try:
            result = fn(**args)
        except Exception as exc:  # noqa: BLE001 - errors are part of the spec
            self._store(op, args, {"error": type(exc).__name__}, tol)
            return exc
        self._store(op, args, {"value": to_jsonable(result)}, tol)
        return result

    def record(self, op: str, args: dict[str, Any], result: Any, *, tol: float | None = None) -> None:
        """Record an observation computed by the caller."""
        self._store(op, args, {"value": to_jsonable(result)}, tol)

    def _store(self, op: str, args: dict[str, Any], outcome: dict[str, Any], tol: float | None) -> None:
        args = to_jsonable(args)
        key = f"{op} {_canonical(args)}"
        entry: dict[str, Any] = {"op": op, "args": args, **outcome}
        if tol is not None:
            entry["tol"] = tol
        self._probes[key] = entry

    def document(self) -> dict[str, Any]:
        import matplotlib

        return {
            "area": self.area,
            "matplotlib": matplotlib.__version__,
            "probes": [self._probes[k] for k in sorted(self._probes)],
        }

    @property
    def path(self) -> pathlib.Path:
        return PROBE_ROOT / f"{self.area}.json"

    def __len__(self) -> int:
        return len(self._probes)


def all_probe_sets() -> list[ProbeSet]:
    return [_REGISTRY[k] for k in sorted(_REGISTRY)]


def probe_key(probe: dict[str, Any]) -> str:
    """What names a probe within its area: the operation and its arguments."""
    return f"{probe['op']} {json.dumps(probe['args'], sort_keys=True)}"


#: What ``pltrs-spec`` allows between a value of pltrs and the golden one
#: (``Tolerance::default`` in ``crates/pltrs-spec/src/compare.rs``, and its
#: ``Tolerance::relative`` for a probe with a ``tol``): keep in step.
RELATIVE_TOLERANCE = 1e-9
ABSOLUTE_TOLERANCE = 1e-12
ABSOLUTE_PER_RELATIVE = 1e-3


def _within(expected: Any, actual: Any, relative: float, absolute: float) -> bool:
    """``compare`` of ``pltrs-spec``: numbers within the tolerance (NaN
    with NaN, an infinity with itself), lists and objects item by item,
    anything else equal."""
    if isinstance(expected, bool) or isinstance(actual, bool):
        return expected is actual
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        expected, actual = float(expected), float(actual)
        if math.isnan(expected) or math.isnan(actual):
            return math.isnan(expected) and math.isnan(actual)
        if math.isinf(expected) or math.isinf(actual):
            return expected == actual
        return abs(expected - actual) <= absolute + relative * max(abs(expected), abs(actual))
    if isinstance(expected, list) and isinstance(actual, list):
        return len(expected) == len(actual) and all(
            _within(e, a, relative, absolute) for e, a in zip(expected, actual))
    if isinstance(expected, dict) and isinstance(actual, dict):
        return expected.keys() == actual.keys() and all(
            _within(expected[k], actual[k], relative, absolute) for k in expected)
    return type(expected) is type(actual) and expected == actual


def passes(golden: dict[str, Any], probe: dict[str, Any]) -> bool:
    """Whether the outcome of ``probe`` passes for the golden one as
    ``pltrs-spec`` judges an outcome of pltrs: a value within the golden's
    tolerance of its value, any refusal for a refusal."""
    if "error" in golden or "error" in probe:
        return "error" in golden and "error" in probe
    relative = golden.get("tol")
    if relative is None:
        relative, absolute = RELATIVE_TOLERANCE, ABSOLUTE_TOLERANCE
    else:
        absolute = relative * ABSOLUTE_PER_RELATIVE
    return _within(golden["value"], probe["value"], relative, absolute)


def agrees(golden: dict[str, Any], probe: dict[str, Any]) -> bool:
    """Whether matplotlib's ``probe`` here is what the golden one says:
    exactly where the common goldens are recorded, and as far as pltrs is
    held to it (``passes``) on a machine with goldens of its own."""
    return values_equal(golden, probe) if RECORDS_COMMON else passes(golden, probe)


class GoldenAnswers:
    """Which golden probe each probe of a run answers.

    That is the golden probe of its key. On a platform with goldens of its
    own a probe without one answers a golden probe of its operation, not
    answered by key, whose arguments are its own within the tolerance:
    matplotlib and numpy compute many arguments too (points on a circle,
    transformed vertices) and round them by platform. Of several such
    golden probes (arguments a hair apart, on purpose) it answers the first
    it agrees with.
    """

    def __init__(self, goldens: dict[str, dict[str, Any]], run: dict[str, dict[str, Any]]) -> None:
        self._goldens = goldens
        self._free: dict[str, list[str]] = {}
        if not RECORDS_COMMON:
            for key, golden in goldens.items():
                if key not in run:
                    self._free.setdefault(golden["op"], []).append(key)

    def key(self, probe: dict[str, Any]) -> str | None:
        """The key of the golden probe ``probe`` answers, if any; a golden
        probe is given to one probe only."""
        key = probe_key(probe)
        if key in self._goldens:
            return key
        free = self._free.get(probe["op"], [])
        near = [candidate for candidate in free
                if _within(self._goldens[candidate]["args"], probe["args"],
                           RELATIVE_TOLERANCE, ABSOLUTE_TOLERANCE)]
        if not near:
            return None
        chosen = next((candidate for candidate in near if agrees(self._goldens[candidate], probe)),
                      near[0])
        free.remove(chosen)
        return chosen


def _probes_under(area: str, roots: list[pathlib.Path]) -> dict[str, dict[str, Any]] | None:
    """The probes of ``area`` by key as the goldens of ``roots`` (the most
    specific first) change the common ones: each, from the least specific
    on, puts its probes in place of those they replace, drops those it
    lists as absent and adds its own. ``None`` without a common golden."""
    common = PROBE_ROOT / f"{area}.json"
    if not common.exists():
        return None
    probes = {probe_key(p): p for p in json.loads(common.read_text(encoding="utf-8"))["probes"]}
    for root in reversed(roots):
        own = root / "probes" / f"{area}.json"
        if own.exists():
            document = json.loads(own.read_text(encoding="utf-8"))
            for absent in document["absent"]:
                probes.pop(probe_key(absent), None)
            probes.update((probe_key(p), p) for p in document["probes"])
    return probes


def golden_probes(area: str) -> dict[str, dict[str, Any]] | None:
    """The probes of ``area`` this machine is held to, by key: the common
    golden's as its processor level's golden and then its platform's
    change them. ``None`` without a common golden."""
    return _probes_under(area, _ROOTS)


def record_probes(probes: ProbeSet) -> None:
    """Record the probes matplotlib evaluated here as the golden of their
    area: the whole area where the common goldens are recorded. Elsewhere
    this machine's own golden holds the probes that fail for the golden it
    is held to without it (``passes``; with them those evaluated with
    other arguments than any such probe) and, as absent, the held probes
    of the same operations that no probe here answers: evaluated with
    those other arguments. A held probe of an operation with nothing of
    its own here stays, as one matplotlib could not run for want of a
    program."""
    if OWN_ROOT is None:
        probes.path.parent.mkdir(parents=True, exist_ok=True)
        probes.path.write_text(dump_json(probes.document()), encoding="utf-8", newline="\n")
        return
    held = _probes_under(probes.area, _ROOTS[1:])
    if held is None:
        raise AssertionError(f"{probes.area} has no common golden: record it on Linux x86-64 first")
    own = OWN_ROOT / "probes" / f"{probes.area}.json"
    document = json.loads(dump_json(probes.document()))
    run = {probe_key(p): p for p in document["probes"]}
    answers = GoldenAnswers(held, run)
    failing, answered, replaced = [], set(), set()
    for key, probe in run.items():
        golden = answers.key(probe)
        if golden is not None and passes(held[golden], probe):
            answered.add(golden)
            continue
        failing.append(probe)
        if key not in held:
            replaced.add(probe["op"])
    absent = [{"op": p["op"], "args": p["args"]} for key, p in held.items()
              if key not in run and key not in answered and p["op"] in replaced]
    if not failing and not absent:
        own.unlink(missing_ok=True)
        return
    own.parent.mkdir(parents=True, exist_ok=True)
    own.write_text(dump_json({"area": probes.area, "matplotlib": document["matplotlib"],
                              "platform": OWN_ROOT.name, "probes": failing, "absent": absent}),
                   encoding="utf-8", newline="\n")


def dump_json(document: Any) -> str:
    """Serialise deterministically, keeping leaf arrays on one line.

    Probe files hold one probe per line so diffs stay reviewable.
    """
    return _format(document, 0) + "\n"


def _is_leaf(value: Any) -> bool:
    scalar = (str, int, float, bool, type(None))
    if isinstance(value, scalar):
        return True
    if isinstance(value, list):
        return all(isinstance(v, scalar) or (isinstance(v, list) and all(isinstance(w, scalar) for w in v)) for v in value)
    return False


def _format(value: Any, depth: int) -> str:
    compact = json.dumps(value, ensure_ascii=False, allow_nan=True, separators=(", ", ": "))
    if _is_leaf(value) or (depth >= 2 and len(compact) <= 160):
        return compact
    pad = " " * (depth + 1)
    if isinstance(value, dict):
        items = [f"{pad}{json.dumps(k)}: {_format(v, depth + 1)}" for k, v in value.items()]
        return "{\n" + ",\n".join(items) + "\n" + " " * depth + "}"
    items = [f"{pad}{_format(v, depth + 1)}" for v in value]
    return "[\n" + ",\n".join(items) + "\n" + " " * depth + "]"


def values_equal(a: Any, b: Any) -> bool:
    """Exact structural equality treating NaN as equal to NaN."""
    if isinstance(a, float) and isinstance(b, float):
        return (math.isnan(a) and math.isnan(b)) or a == b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(values_equal(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(values_equal(a[k], b[k]) for k in a)
    return a == b
