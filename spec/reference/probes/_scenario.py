"""Scenarios ported from matplotlib's own test suite, replayed as lists of
method calls (shared by the ``*_behaviour`` probe areas).

An op ``<area>.run`` takes ``{case, steps, figure?, rc?, style?}`` and
returns one entry per step: the converted result of query steps, ``null``
for the others. ``case`` names the upstream test the scenario mirrors
(``test_<name>``, with a ``/variant`` suffix for parametrisations). The
whole op runs in ``Figure(**figure)`` (default ``Figure()``; ``figsize``
and ``layout`` are used) with an Agg canvas, inside
``style.context(style)`` and ``rc_context(rc)`` when given.

Steps
-----
A step is ``[target, method, arg, ...]``; a trailing JSON object holds
keyword arguments. ``target`` is ``"fig"``, an integer (the axes numbered
in creation order, counting the axes every step adds to ``fig.axes``;
removed axes keep their number) or ``"@k"`` (the object step ``k``
returned; for a list, its first item). ``method`` may be a dotted path:
callables along the path are called without arguments, and a list along
the path maps the rest of the path over its items
(``"xaxis.set_inverted"``, ``"get_shared_x_axes.joined"``,
``"get_texts.get_color"``).

* A callable is called. Its result is recorded when a component of the
  path starts with ``get_``.
* An attribute without arguments is read and recorded (``dataLim``,
  ``subplotpars.left``); with one argument it is assigned
  (``["use_sticky_edges", false]``).
* ``["fig", "draw"]`` draws the canvas.

Arguments: ``"@k"`` is the object of step ``k`` and ``"@axN"`` axes ``N``
(also inside lists); ``{"new": name, "args": [...]}`` constructs one of
``CONSTRUCTORS``: a ``LogNorm`` (autoscaling without arguments) or
``Normalize`` from ``[vmin, vmax]``, a ``MultipleLocator``, a ``PathPatch``
from ``[vertices, codes]``, a ``SubplotSpec`` ``GridSpec(nrows,
ncols)[r0:r1, c0:c1]`` from ``[nrows, ncols, [r0, r1], [c0, c1]]`` or a
``tuple``. The methods of a colorbar's axes are steps on the colorbar with
the method ``"ax.<method>"``.

Recorded values: sequences become lists, a ``Bbox`` becomes ``[[x0, y0],
[x1, y1]]``, a ``Text`` its string, an axes its number, an artist a step
returned that step's reference ``"@k"``; the results of getters named
``get_...color`` become RGBA lists.
"""

import contextlib

import matplotlib as mpl
import matplotlib.colors as mcolors
import matplotlib.ticker as mticker
import numpy as np
from matplotlib.artist import Artist
from matplotlib.axes import Axes
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.gridspec import GridSpec
from matplotlib.patches import PathPatch
from matplotlib.path import Path
from matplotlib.text import Text
from matplotlib.transforms import BboxBase

CONSTRUCTORS = {
    "LogNorm": mcolors.LogNorm,
    "Normalize": mcolors.Normalize,
    "MultipleLocator": mticker.MultipleLocator,
    "PathPatch": lambda vertices, codes: PathPatch(Path(vertices, codes)),
    "SubplotSpec": lambda nrows, ncols, rows, cols: (
        GridSpec(nrows, ncols)[rows[0]:rows[1], cols[0]:cols[1]]),
    "tuple": lambda *items: items,
}


class Scene:
    """The figure of one scenario, its numbered axes and step results."""

    def __init__(self, figure):
        self.fig = Figure(**(figure or {}))
        FigureCanvasAgg(self.fig)
        self.axes = []
        self.objects = []

    def track_axes(self):
        self.axes.extend(ax for ax in self.fig.axes if not any(ax is a for a in self.axes))

    def target(self, target):
        if target == "fig":
            return self.fig
        if isinstance(target, int):
            return self.axes[target]
        return self.reference(target)

    def reference(self, ref):
        if ref.startswith("@ax"):
            return self.axes[int(ref[3:])]
        obj = self.objects[int(ref[1:])]
        return obj[0] if isinstance(obj, list) else obj

    def argument(self, value):
        if isinstance(value, str) and value.startswith("@"):
            return self.reference(value)
        if isinstance(value, dict) and "new" in value:
            return CONSTRUCTORS[value["new"]](*value.get("args", []))
        if isinstance(value, list):
            return [self.argument(v) for v in value]
        return value

    def step_of(self, artist):
        for k, obj in enumerate(self.objects):
            if obj is artist or (isinstance(obj, list) and obj and obj[0] is artist):
                return f"@{k}"
        raise TypeError(f"cannot record {type(artist).__name__}")

    def jsonable(self, value):
        if isinstance(value, BboxBase):
            return self.jsonable(value.get_points())
        if isinstance(value, Text):
            return value.get_text()
        if isinstance(value, Axes):
            return next(i for i, ax in enumerate(self.axes) if ax is value)
        if isinstance(value, Artist):
            return self.step_of(value)
        if isinstance(value, np.ndarray):
            return self.jsonable(value.tolist())
        if isinstance(value, np.generic):
            return value.item()
        if isinstance(value, (list, tuple)):
            return [self.jsonable(v) for v in value]
        if isinstance(value, dict):
            return {k: self.jsonable(v) for k, v in value.items()}
        if value is None or isinstance(value, (bool, int, float, str)):
            return value
        raise TypeError(f"cannot record {type(value).__name__}")

    def resolve(self, owner, path, args, kwargs):
        """Follows ``path`` from ``owner``; returns ``(result, recorded)``."""
        part, *rest = path
        if isinstance(owner, list):
            results = [self.resolve(item, path, args, kwargs)[0] for item in owner]
            return results, part.startswith("get_") or any(p.startswith("get_") for p in rest)
        attr = getattr(owner, part)
        if rest:
            value = attr() if callable(attr) else attr
            result, recorded = self.resolve(value, rest, args, kwargs)
            return result, recorded or part.startswith("get_")
        if callable(attr):
            result = attr(*args, **kwargs)
            if part.startswith("get_") and part.endswith("color"):
                result = [mcolors.to_rgba(c) for c in result] if isinstance(
                    result, list) else mcolors.to_rgba(result)
            return result, part.startswith("get_")
        if args:
            setattr(owner, part, args[0])
            return None, False
        return attr, True

    def step(self, step):
        target, method, *args = step
        kwargs = args.pop() if args and isinstance(args[-1], dict) and "new" not in args[-1] else {}
        args = [self.argument(a) for a in args]
        kwargs = {k: self.argument(v) for k, v in kwargs.items()}
        if target == "fig" and method == "draw":
            self.fig.canvas.draw()
            self.objects.append(None)
            return None
        result, recorded = self.resolve(self.target(target), method.split("."), args, kwargs)
        self.objects.append(result)
        self.track_axes()
        return self.jsonable(result) if recorded else None


def run(case, steps, figure=None, rc=None, style=None):
    with contextlib.ExitStack() as stack:
        if style is not None:
            stack.enter_context(mpl.style.context(style))
        stack.enter_context(mpl.rc_context(rc or {}))
        scene = Scene(figure)
        return [scene.step(step) for step in steps]


def recorder(probes):
    """A ``case(name, steps, **options)`` function recording into ``probes``."""
    op = f"{probes.area}.run"

    def case(name, steps, tol=None, **options):
        probes.probe(op, run, tol=tol, case=name, steps=steps,
                     **{k: v for k, v in options.items() if v is not None})

    return case
