"""What the generators share: the programs' header, literals of values,
and how matplotlib's getters and setters are found."""

from __future__ import annotations

import inspect
import math
import warnings
from typing import Any, Callable, Iterator

import numpy as np

#: The first statements of every generated program.
HEADER = [
    "import numpy as np",
    "import matplotlib",
    "import matplotlib.pyplot as plt",
    "from matplotlib import colors, ticker, transforms, patches, lines, text, collections, path, scale",
]

#: Data the programs share, as statements defining it.
DATA = [
    "x = np.linspace(0, 2 * np.pi, 7)",
    "y = np.sin(x)",
    "z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))",
    "values = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])",
]

#: Getters a program never calls: they need what a program does not have
#: (an event, a GUI), or matplotlib marks them as internal.
SKIPPED_METHODS = {
    "get_cursor_data", "get_window_extent_with_renderer", "get_default_filename",
    "get_supported_filetypes", "get_supported_filetypes_grouped", "get_renderer_cache",
    "get_window_title", "get_width_height", "get_tk_widget", "is_saving", "get_shared_x_axes",
    "get_shared_y_axes", "get_layout_engine", "get_default_bbox_extra_artists", "get_renderer",
}


def literal(value: Any) -> str | None:
    """Python source evaluating to ``value``, or None if it has none."""
    if value is None or isinstance(value, (bool, str)):
        return repr(value)
    if isinstance(value, (int, np.integer)) and not isinstance(value, bool):
        return repr(int(value))
    if isinstance(value, (float, np.floating)):
        f = float(value)
        return f"float('{f}')" if not math.isfinite(f) else repr(f)
    if isinstance(value, tuple):
        items = [literal(v) for v in value]
        if None in items:
            return None
        return "(" + ", ".join(items) + ("," if len(items) == 1 else "") + ")"
    if isinstance(value, list):
        items = [literal(v) for v in value]
        return None if None in items else "[" + ", ".join(items) + "]"
    if isinstance(value, np.ndarray) and value.dtype.kind in "biuf" and value.size <= 64:
        items = literal(value.tolist())
        return None if items is None else f"np.array({items}, dtype='{value.dtype}')"
    return None


def _required_parameters(method: Callable) -> int | None:
    try:
        signature = inspect.signature(method)
    except (TypeError, ValueError):
        return None
    return sum(
        1 for p in signature.parameters.values()
        if p.default is inspect.Parameter.empty
        and p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
    )


def getters(obj: Any) -> list[str]:
    """The names of ``obj``'s getters callable without arguments."""
    names = []
    for name in sorted(dir(obj)):
        if not name.startswith(("get_", "is_", "has_")) or name in SKIPPED_METHODS:
            continue
        try:
            method = getattr(obj, name)
        except Exception:  # noqa: BLE001 - a property that fails is no getter
            continue
        if callable(method) and _required_parameters(method) == 0:
            names.append(name)
    return names


def attributes(obj: Any) -> list[str]:
    """The names of ``obj``'s public attributes that are not methods."""
    names = []
    cls = type(obj)
    for name in sorted(dir(obj)):
        if name.startswith("_"):
            continue
        attr = inspect.getattr_static(obj, name, None)
        if attr is None and name not in getattr(obj, "__dict__", {}):
            continue
        if isinstance(inspect.getattr_static(cls, name, None), (staticmethod, classmethod)):
            continue
        if callable(attr) and not isinstance(attr, property):
            continue
        names.append(name)
    return names


def setters(obj: Any) -> list[str]:
    """The property names ``obj`` has a ``set_<name>`` of, aliases left
    out (``set_lw`` is ``set_linewidth``)."""
    names = []
    for name in sorted(dir(obj)):
        method = getattr(obj, name, None) if name.startswith("set_") else None
        if callable(method) and not (method.__doc__ or "").lstrip().startswith("Alias for"):
            names.append(name[4:])
    return names


def probe_object(variable: str, obj: Any, *, attrs: bool = True) -> Iterator[str]:
    """Statements calling every getter of ``obj`` (named ``variable`` in
    the program) and reading its attributes."""
    for name in getters(obj):
        yield f"{variable}.{name}()"
    if attrs:
        for name in attributes(obj):
            yield f"{variable}.{name}"


def namespace(statements: list[str]) -> dict[str, Any]:
    """The namespace the statements leave under matplotlib (run here)."""
    import matplotlib.pyplot as plt

    plt.close("all")
    scope: dict[str, Any] = {}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        exec("\n".join(statements), scope)  # noqa: S102 - the generator's own code
    return scope
