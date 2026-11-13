"""The objects of a figure a library meets, by path, and their values.

Imported by ``private_dump.py``, under either implementation. `walk`
reaches the objects of a figure the way programs do, through public
accessors only (an artist's children, an Axes' axes, a figure's canvas,
an axis' locators and formatters, an artist's transform and path, a
mappable's colormap, norm and colorbar...), and names each by the path it
was reached by (``fig.get_children()[1].xaxis``), so that the same path
names the same object under both implementations. `describe` turns a
value into what is compared: plain data as it is, matplotlib's value
types by their public state, an object met on the walk by its path.
"""

from __future__ import annotations

import inspect
import math
import re
from typing import Any, Callable, Iterator

import numpy as np

#: Public attributes holding matplotlib's objects.
ATTRIBUTES = ("canvas", "xaxis", "yaxis", "zaxis", "patch", "legend_", "colorbar", "norm", "cmap", "callbacks",
              "transData", "transAxes", "transFigure", "bbox", "viewLim", "dataLim", "dpi_scale_trans",
              "offsetText", "label", "mappable", "ax", "outline", "solids", "dividers", "cax", "inset_ax")
#: Public getters returning matplotlib's objects.
GETTERS = ("get_children", "get_major_locator", "get_minor_locator", "get_major_formatter",
           "get_minor_formatter", "get_transform", "get_path", "get_subplotspec", "get_gridspec",
           "get_layout_engine", "get_fontproperties", "get_legend", "get_major_ticks", "get_offset_text",
           "get_renderer", "get_frame", "get_texts", "get_lines", "get_patches", "get_title",
           "get_clip_path", "get_bbox_patch", "get_xaxis_transform")
#: Objects kept per class and per figure state, and in all per state.
PER_CLASS = 3
PER_STATE = 400
ADDRESS = re.compile(r" at 0x[0-9a-fA-F]+")
#: Significant digits floats are compared to.
DIGITS = 10


def is_ours(value: Any) -> bool:
    """Whether ``value`` is an object of matplotlib's (or pltrs's)."""
    module = type(value).__module__ or ""
    return module.split(".")[0] in ("matplotlib", "mpl_toolkits", "pltrs")


def _reached(obj: Any) -> Iterator[tuple[str, Any]]:
    """``(path step, object)`` for what ``obj``'s public accessors give."""
    for name in ATTRIBUTES:
        try:
            value = getattr(obj, name, None) if name in _own_attributes(obj) else None
        except Exception:  # noqa: BLE001 - a property refusing in this state
            continue
        if value is not None:
            yield f".{name}", value
    for name in GETTERS:
        method = getattr(type(obj), name, None)
        if method is None:
            continue
        try:
            value = getattr(obj, name)()
        except Exception:  # noqa: BLE001 - a getter needing what this state lacks
            continue
        if isinstance(value, (list, tuple)):
            for index, item in enumerate(value[:PER_CLASS * 4]):
                yield f".{name}()[{index}]", item
        elif value is not None:
            yield f".{name}()", value


def _own_attributes(obj: Any) -> set[str]:
    """The `ATTRIBUTES` an object has, without running its properties'
    getters for those it lacks."""
    return {name for name in ATTRIBUTES if hasattr(type(obj), name) or name in getattr(obj, "__dict__", {})}


def walk(roots: dict[str, Any]) -> dict[str, Any]:
    """Every object of matplotlib's reached from ``roots``, by path,
    breadth first, at most `PER_CLASS` per class (and its first reached
    object of each class whatever the limit)."""
    found: dict[str, Any] = {}
    seen: set[int] = set()
    per_class: dict[str, int] = {}
    queue = list(roots.items())
    while queue and len(found) < PER_STATE:
        path, obj = queue.pop(0)
        if id(obj) in seen or not is_ours(obj):
            continue
        seen.add(id(obj))
        qualname = type(obj).__qualname__
        if per_class.get(qualname, 0) >= PER_CLASS:
            continue
        per_class[qualname] = per_class.get(qualname, 0) + 1
        found[path] = obj
        queue.extend((path + step, value) for step, value in _reached(obj))
    return found


def _number(value: float) -> Any:
    if math.isnan(value):
        return "nan"
    if math.isinf(value):
        return "inf" if value > 0 else "-inf"
    return float(f"{value:.{DIGITS}g}")


def describe(value: Any, paths: dict[int, str], depth: int = 0) -> Any:
    """What is compared of ``value``: ``[type, state]``, the state plain
    data, or None where nothing of it is compared but its type."""
    kind = type(value).__qualname__
    if id(value) in paths and not isinstance(value, (int, float, str, bool, type(None))):
        return [kind, {"is": paths[id(value)]}]
    if value is None or isinstance(value, (bool, str)):
        return [kind, value]
    if isinstance(value, (int, np.integer)):
        return [kind, int(value)]
    if isinstance(value, (float, np.floating)):
        return [kind, _number(float(value))]
    if depth > 3:
        return [kind, None]
    if isinstance(value, (list, tuple)):
        return [kind, [describe(item, paths, depth + 1) for item in value[:32]] + (["..."] if len(value) > 32 else [])]
    if isinstance(value, dict):
        items = sorted(value.items(), key=lambda item: repr(item[0]))[:32]
        return [kind, [[repr(k), describe(v, paths, depth + 1)] for k, v in items]]
    if isinstance(value, np.ndarray):
        if value.dtype.kind in "biuf" and value.size <= 256:
            flat = [_number(float(v)) if value.dtype.kind == "f" else int(v) for v in value.ravel()]
            return [kind, {"dtype": value.dtype.kind, "shape": list(value.shape), "values": flat}]
        return [kind, {"dtype": value.dtype.kind, "shape": list(value.shape)}]
    state = _state(value, paths, depth)
    if state is not None:
        return [kind, state]
    if callable(value) and not isinstance(value, type):
        return [kind, {"signature": signature(value)}]
    if isinstance(value, type):
        return [kind, {"class": value.__qualname__}]
    return [kind, None]


#: The public state matplotlib's value types are compared by.
STATES: tuple[tuple[str, Callable[[Any], Any]], ...] = (
    ("matrix", lambda v: v.get_matrix() if getattr(v, "is_affine", False) else None),
    ("bounds", lambda v: v.bounds if type(v).__name__.endswith("Bbox") else None),
    ("vertices", lambda v: (v.vertices, v.codes) if type(v).__name__ == "Path" else None),
    ("colormap", lambda v: (v.name, v.N) if hasattr(v, "N") and hasattr(v, "_init") else None),
    ("norm", lambda v: (v.vmin, v.vmax) if hasattr(v, "vmin") and hasattr(v, "autoscale") else None),
    ("font", lambda v: v.get_fontconfig_pattern() if hasattr(v, "get_fontconfig_pattern") else None),
)


def _state(value: Any, paths: dict[int, str], depth: int) -> Any:
    if not is_ours(value):
        return None
    for name, read in STATES:
        try:
            state = read(value)
        except Exception:  # noqa: BLE001 - a value that cannot say in this state
            continue
        if state is not None:
            return {name: describe(state, paths, depth + 1)}
    return None


def signature(value: Any) -> Any:
    """A callable's parameters, ``[name, kind, default repr]`` each."""
    try:
        parameters = inspect.signature(value).parameters.values()
    except (TypeError, ValueError):
        return None
    return [[p.name, p.kind.name, None if p.default is inspect.Parameter.empty
             else ADDRESS.sub("", repr(p.default))] for p in parameters]
