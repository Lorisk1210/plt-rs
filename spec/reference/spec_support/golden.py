"""Recording of probe goldens."""

from __future__ import annotations

import json
import math
import pathlib
from typing import Any, Callable

import numpy as np

SPEC_ROOT = pathlib.Path(__file__).resolve().parents[2]
GOLDEN_ROOT = SPEC_ROOT / "goldens"
PROBE_ROOT = GOLDEN_ROOT / "probes"

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
