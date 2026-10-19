"""Contouring: ``Axes.contour``/``contourf``, ``Axes.tricontour``/``tricontourf``, ``clabel``.

Quad-grid contours use matplotlib's default ``contour.algorithm`` rcParam,
``'mpl2014'`` (contourpy 1.4, ``LineType.SeparateCode`` /
``FillType.OuterCode``).  No other algorithm is probed, so ``algorithm`` never
appears in args.  Triangular contours use matplotlib's own
``_tri.TriContourGenerator`` on an EXPLICIT triangulation (never Delaunay).

Every adapter builds a fresh ``Figure()`` (rc defaults: 6.4x4.8 in, 100 dpi)
with an Agg canvas and one ``fig.add_subplot()`` axes, unless stated otherwise.

=============================================================================
Common input args (quad ops ``Axes.contour`` / ``Axes.contourf``)
=============================================================================
* ``z``: 2D list (rows = y index i, columns = x index j).  Converted with
  ``np.asarray(z, float64)``; ``NaN`` entries are allowed (matplotlib masks them
  via ``ma.masked_invalid``).  If ``z_dtype == "bool"`` it is converted with
  ``np.asarray(z, bool)`` instead (bool data gets default levels ``[0.5]`` for
  lines / ``[0, 0.5, 1]`` for fills).
* ``mask`` (optional): 2D bool list, ``Z = np.ma.array(Z, mask=mask)``.
* ``x``, ``y`` (optional, both or neither): 1D (len Nx / Ny) or 2D (shape of z)
  lists; called as ``ax.contour(x, y, Z, **kw)``; without them
  ``ax.contour(Z, **kw)`` (then ``origin``/``extent`` place the grid).
* Passed through unchanged as keyword arguments when present: ``levels``
  (int or list), ``extend``, ``origin``, ``extent``, ``corner_mask``, ``vmin``,
  ``vmax``, ``nchunk``, ``colors`` (colour string or list of colour strings),
  ``algorithm``.
* ``locator`` (optional): ``{"name": "MaxNLocator", **kw}`` or
  ``{"name": "LogLocator", **kw}`` -> ``ticker.<name>(**kw)``.
* ``norm`` (optional): ``"log"`` -> ``colors.LogNorm()``.
* ``rc`` (optional): dict of rcParams applied with ``matplotlib.rc_context``
  around the call (e.g. ``image.origin``, ``contour.corner_mask``,
  ``contour.algorithm``, ``contour.negative_linestyle``, ``contour.linewidth``).

Tri ops (``Axes.tricontour`` / ``Axes.tricontourf``) take ``x``, ``y`` (1D
lists, one per point), ``triangles`` (list of [i, j, k] point indices),
optional ``tri_mask`` (one bool per triangle) and ``z`` (1D list, one per
point, NaN allowed for points not used by any triangle), and build
``Triangulation(x, y, triangles, mask=tri_mask)``, then call
``ax.tricontour(triangulation, Z, **kw)`` with the same keyword pass-through
as above (``levels``, ``extend``, ``vmin``, ``vmax``, ``colors``, ``locator``,
``norm``, ``rc``).  (The ``triangles=``/``mask=`` keywords of
``ax.tricontour(x, y, z, triangles=...)`` raise AttributeError in matplotlib
3.11.2 -- a matplotlib bug -- hence the explicit Triangulation.)

Level selection (matplotlib behaviour the goldens pin down):
* explicit list -> float64 array; ValueError if not strictly increasing, or if
  filled with fewer than 2 levels.  Lines accept ``[]`` (no levels).
* ``levels`` None -> N = 7; int N -> ``MaxNLocator(N + 1, min_n_ticks=1)``
  (or ``LogLocator(numticks=N)`` when log-scaled: ``norm="log"`` or a
  LogLocator) unless ``locator`` is given; ``tick_values(zmin, zmax)`` is then
  trimmed to keep at most one level below zmin and one above zmax (one fewer at
  the bottom for extend min/both, at the top for max/both; untrimmed if fewer
  than 3 would remain; never trimmed for a ``symmetric`` MaxNLocator).
* bool z without levels: ``[0.5]`` (lines) / ``[0, 0.5, 1]`` (fills).
* Unknown ``extend`` strings are NOT validated (behave like 'neither').
* No warning is emitted for constant fields in 3.11; the singular range is
  expanded by the locator (e.g. z == 1 -> levels [1 - 2.5e-14, 1, 1 + 2.5e-14]).

=============================================================================
Canonical geometry (the ``paths`` field) -- apply the SAME function in Rust
=============================================================================
``paths`` has one entry per level (line contours, ``len(levels)`` entries) or
per band (filled contours, ``len(levels) - 1`` entries, i.e. one per
``bands`` pair).  Each entry is the canonicalised form of
``cs.get_paths()[k]`` (a compound Path):

1. Every coordinate is normalised ``-0.0 -> 0.0``.  An empty path (no
   vertices; matplotlib emits ``Path(np.empty((0, 2)))`` with codes None) gives
   ``[]``.  A non-empty path with ``codes is None`` is treated as one open
   polyline.
2. Split into subpaths at every MOVETO (1) code.  A subpath is ``closed`` iff
   its last code is CLOSEPOLY (79); geometry is never consulted.  What the
   generators emit (all CLOSEPOLY vertices repeat their MOVETO vertex):
   * filled rings (mpl2014 and tri): always MOVETO, LINETO..., CLOSEPOLY.
   * mpl2014 lines: each traced line is MOVETO, LINETO...; its last code is
     turned into CLOSEPOLY iff the line has > 1 point and its last point is
     bit-identical to its first.  A loop's final point re-crosses the start
     edge with the edge's endpoints in the opposite order, and mpl2014's
     ``interp(p1, p2) = xy1*f + xy2*(1-f)``, ``f = (z2-level)/(z2-z1)`` is not
     symmetric in rounding, so some geometrically closed loops end 1 ulp away
     from their start and are emitted -- and canonicalised -- as OPEN
     polylines with their raw start vertex (12 such lines in this golden,
     e.g. ``Axes.contour`` peaks ``levels=4``, level index 1; the 'serial'
     algorithm would close them).  A faithful port of the mpl2014 arithmetic
     reproduces this.
   * TriContourGenerator lines: closed loops end with CLOSEPOLY; open lines
     have none.
   * Degenerate quirks when a level equals z exactly at an isolated extremum:
     mpl2014 emits a 2-point loop (MOVETO + CLOSEPOLY on the same point;
     canonical ``{"closed": true, "vertices": [[x, y]]}``);
     TriContourGenerator emits a lone MOVETO, in line AND filled output
     (canonical ``{"closed": false, "vertices": [[x, y]]}``), see tri case
     ``fan_level_at_vertices``.
3. Closed subpaths: drop the last (CLOSEPOLY) vertex, giving a cyclic vertex
   list; rotate it to start at its lexicographically smallest vertex (compare
   x, then y).  If that minimal vertex occurs more than once, start at the
   occurrence whose rotated sequence is lexicographically smallest.  The
   ring DIRECTION IS KEPT.  A ring may have a single vertex (see above).
4. Open subpaths: vertex order is KEPT (the direction is a guaranteed property).
5. Sort the subpaths of the level by key ``(vertices, closed)`` where
   ``vertices`` compares lexicographically as a sequence of (x, y) pairs (a
   proper prefix sorts first) and ``False < True``.  Exact float comparison, no
   rounding: the comparison against the golden is exact anyway, and sort keys
   only tie when two subpaths share their first vertex (they touch), in which
   case later vertices decide.
6. Each subpath is emitted as ``{"closed": bool, "vertices": [[x, y], ...]}``.

Consecutive duplicate vertices and collinear vertices are part of matplotlib's
output and are KEPT: mpl2014 emits a vertex twice where a line passes exactly
through a grid point (it lies on two quad edges), and filled rings that run
along the domain/mask boundary contain every boundary grid point (collinear).

Direction guarantees (verified by the hand-written tests below):
* Line contours: the region with z > level lies on the LEFT of the direction
  of travel.  Filled contours: outer boundaries are counter-clockwise, holes
  clockwise.  For tri contours this holds in data space.  For quad grids it
  holds in INDEX space (column j as abscissa, row i as ordinate); in data space
  it is reversed when the grid mapping (j, i) -> (x, y) is orientation
  reversing (e.g. x decreasing with j, or y decreasing with i such as
  ``origin='upper'``).

=============================================================================
Ops
=============================================================================
``Axes.contour`` / ``Axes.contourf`` (quad) and ``Axes.tricontour`` /
``Axes.tricontourf`` (tri) return a dict:
  * ``levels``: ``cs.levels`` (chosen levels, after auto-selection/trimming).
  * ``layers``: ``cs.layers`` (== levels for lines; band midpoints for fills,
    using the extended ``+-1e250`` sentinels for ``extend``; geometric means
    with ``1e-250``/``1e250`` sentinels when log-scaled).
  * ``zmin``, ``zmax``: ``cs.zmin``/``cs.zmax`` as floats (null if masked).
  * ``logscale``: ``cs.logscale``.
  * ``norm``: ``[cs.norm.vmin, cs.norm.vmax]``.
  * ``xlim``, ``ylim``: axes limits after the call (fresh axes, tight
    autoscale onto the grid extent).
  * ``bands`` (filled only): ``[[lower, upper], ...]`` from
    ``cs._get_lowers_and_uppers()`` -- the z intervals actually contoured
    (the lowest lower bound is decreased by 1, or multiplied by 0.99 when
    log-scaled, if it equals zmin, so the minimum is included).
  * ``colors``: RGBA per level (lines: ``get_edgecolor()``) or per band
    (fills: ``get_facecolor()``).
  * ``linewidths``, ``linestyles`` (lines only): ``get_linewidth()`` and
    ``get_linestyle()`` as ``[[offset, dashes-or-null], ...]``.
  * ``warnings``: ``[[category_name, message], ...]`` emitted during the call.
  * ``paths``: canonical geometry (above).

``ContourSet.get_paths`` -- RAW, uncanonicalised output.  Args: ``func``
(``"contour"``, ``"contourf"``, ``"tricontour"`` or ``"tricontourf"``) plus the
args of that op.  Returns ``[Path, ...]`` (one per level/band, shared Path
dict ``{"vertices", "codes"}``) exactly as matplotlib produced it.  This pins
mpl2014's subpath order and loop start vertices, which are implementation
details everywhere except ``clabel`` (automatic label placement walks the
subpaths in this order, starting each loop at its emitted first vertex).

``ContourSet.clabel`` -- label placement on line contours (tol 1e-9: the
results go through display transforms, their inverse, hypot/arctan2).
Builds ``Figure(figsize=figsize, dpi=dpi)`` with an Agg canvas (label widths
come from the Agg text layout) and ``ax = fig.add_axes(rect)``, calls
``ax.contour(x, y, z, levels=levels)``, then ``ax.set_xlim(*xlim)``,
``ax.set_ylim(*ylim)``, then ``cs.clabel(**clabel)``.
Args: ``figsize`` [w, h] in, ``dpi``, ``rect`` [left, bottom, w, h],
``xlim``, ``ylim``, ``x``, ``y`` (1D), ``z`` (2D), ``levels`` (contour
levels), and ``clabel``: dict of clabel kwargs (``levels`` subset,
``fontsize``, ``inline``, ``inline_spacing``, ``fmt``, ``rightside_up``,
``manual`` as a list of [x, y] data points).  ``fmt`` is a %-format string, or
a list of [level, text] pairs meaning the dict ``{level: text}``; absent means
the default ``ScalarFormatter(useOffset=False)`` applied to all labelled
levels at once.  Returns:
  * ``labels``: in creation order (levels ascending, then subpaths in the raw
    generator order -- see ``ContourSet.get_paths``; the order matters because
    a candidate position too close to an earlier label is skipped),
    ``{"level", "text", "display", "rotation"}``: ``level`` = the label's
    contour level, ``text`` = label string (default formatter output uses
    U+2212 for minus), ``display`` = ``cs.labelXYs`` entry, the label centre in
    display pixels (origin bottom-left; the Text's data position is
    ``transData.inverted()`` of it and is not recorded, to avoid comparing
    round-trip noise around 0), ``rotation`` = ``Text.get_rotation()``
    (degrees, normalised to [0, 360)).
  * ``label_widths``: per labelled level, label width in pixels used for
    placement (``cs._get_nth_label_width``).
  * ``paths``: canonical geometry of ``cs.get_paths()`` AFTER labelling (with
    ``inline`` the lines are cut around each label; cut loops become open
    polylines).
"""

from __future__ import annotations

import math
import warnings

import matplotlib
import matplotlib.colors as mcolors
import matplotlib.ticker as mticker
import numpy as np
import pytest
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.path import Path
from matplotlib.tri import Triangulation

from spec_support import ProbeSet

probes = ProbeSet("contour")


# ---------------------------------------------------------------------------
# Canonicalisation
# ---------------------------------------------------------------------------


def _nz(v: float) -> float:
    v = float(v)
    return 0.0 if v == 0 else v


def _rotate_min(ring: list[tuple[float, float]]) -> list[tuple[float, float]]:
    if not ring:
        return ring
    m = min(ring)
    best = None
    for i, p in enumerate(ring):
        if p == m:
            cand = ring[i:] + ring[:i]
            if best is None or cand < best:
                best = cand
    return best


def canon_path(path: Path) -> list[dict]:
    verts = np.asarray(path.vertices, dtype=float)
    if len(verts) == 0:
        return []
    codes = path.codes
    if codes is None:
        codes = np.full(len(verts), Path.LINETO)
        codes[0] = Path.MOVETO
    starts = list(np.flatnonzero(codes == Path.MOVETO))
    if not starts or starts[0] != 0:
        raise ValueError("path does not start with MOVETO")
    subpaths = []
    for s, e in zip(starts, starts[1:] + [len(codes)]):
        v = [(_nz(a), _nz(b)) for a, b in verts[s:e]]
        closed = bool(codes[e - 1] == Path.CLOSEPOLY)
        if closed:
            v = _rotate_min(v[:-1])
        subpaths.append((v, closed))
    subpaths.sort()
    return [{"closed": c, "vertices": [list(p) for p in v]} for v, c in subpaths]


def raw_path(path: Path) -> dict:
    return {
        "vertices": np.asarray(path.vertices, dtype=float).reshape(-1, 2),
        "codes": None if path.codes is None else [int(c) for c in path.codes],
    }


# ---------------------------------------------------------------------------
# Adapters
# ---------------------------------------------------------------------------

_PASS = ("levels", "extend", "origin", "extent", "corner_mask", "vmin", "vmax", "nchunk", "colors",
         "algorithm")


def _kwargs(args: dict) -> dict:
    kw = {k: args[k] for k in _PASS if k in args}
    if "locator" in args:
        spec = dict(args["locator"])
        kw["locator"] = getattr(mticker, spec.pop("name"))(**spec)
    if "norm" in args:
        if args["norm"] != "log":
            raise ValueError(f"unknown norm spec {args['norm']!r}")
        kw["norm"] = mcolors.LogNorm()
    return kw


def _fresh_axes(figsize=None, dpi=None, rect=None):
    fig = Figure(figsize=figsize, dpi=dpi)
    FigureCanvasAgg(fig)
    ax = fig.add_axes(rect) if rect is not None else fig.add_subplot()
    return fig, ax


def _quad_cs(ax, filled: bool, args: dict):
    z = args["z"]
    Z = np.asarray(z, dtype=bool if args.get("z_dtype") == "bool" else np.float64)
    if "mask" in args:
        Z = np.ma.array(Z, mask=np.asarray(args["mask"], dtype=bool))
    pos = (Z,)
    if "x" in args or "y" in args:
        pos = (np.asarray(args["x"], dtype=float), np.asarray(args["y"], dtype=float), Z)
    fn = ax.contourf if filled else ax.contour
    return fn(*pos, **_kwargs(args))


def _tri_cs(ax, filled: bool, args: dict):
    mask = args.get("tri_mask")
    tri = Triangulation(
        np.asarray(args["x"], dtype=float),
        np.asarray(args["y"], dtype=float),
        np.asarray(args["triangles"], dtype=np.int32),
        mask=None if mask is None else np.asarray(mask, dtype=bool),
    )
    Z = np.asarray(args["z"], dtype=np.float64)
    fn = ax.tricontourf if filled else ax.tricontour
    return fn(tri, Z, **_kwargs(args))


def _call(builder, filled: bool, args: dict):
    _, ax = _fresh_axes()
    with matplotlib.rc_context(args.get("rc", {})):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            cs = builder(ax, filled, args)
    return ax, cs, [[w.category.__name__, str(w.message)] for w in caught]


def _scalar(v):
    if v is None or v is np.ma.masked or np.ma.is_masked(v):
        return None
    return float(v)


def _summary(ax, cs, caught) -> dict:
    out = {
        "levels": np.asarray(cs.levels, dtype=float),
        "layers": np.asarray(cs.layers, dtype=float),
        "zmin": _scalar(cs.zmin),
        "zmax": _scalar(cs.zmax),
        "logscale": bool(cs.logscale),
        "norm": [_scalar(cs.norm.vmin), _scalar(cs.norm.vmax)],
        "xlim": list(ax.get_xlim()),
        "ylim": list(ax.get_ylim()),
    }
    if cs.filled:
        lowers, uppers = cs._get_lowers_and_uppers()
        out["bands"] = [[float(a), float(b)] for a, b in zip(lowers, uppers)]
        out["colors"] = cs.get_facecolor()
    else:
        out["colors"] = cs.get_edgecolor()
        out["linewidths"] = cs.get_linewidth()
        out["linestyles"] = [[float(off), None if d is None else [float(x) for x in d]]
                             for off, d in cs.get_linestyle()]
    out["warnings"] = caught
    out["paths"] = [canon_path(p) for p in cs.get_paths()]
    return out


def contour_op(**args):
    return _summary(*_call(_quad_cs, False, args))


def contourf_op(**args):
    return _summary(*_call(_quad_cs, True, args))


def tricontour_op(**args):
    return _summary(*_call(_tri_cs, False, args))


def tricontourf_op(**args):
    return _summary(*_call(_tri_cs, True, args))


_RAW = {
    "contour": (_quad_cs, False),
    "contourf": (_quad_cs, True),
    "tricontour": (_tri_cs, False),
    "tricontourf": (_tri_cs, True),
}


def raw_paths_op(func, **args):
    builder, filled = _RAW[func]
    _, cs, _ = _call(builder, filled, args)
    return [raw_path(p) for p in cs.get_paths()]


def clabel_op(figsize, dpi, rect, xlim, ylim, x, y, z, levels, clabel):
    fig, ax = _fresh_axes(figsize=figsize, dpi=dpi, rect=rect)
    cs = ax.contour(np.asarray(x, float), np.asarray(y, float), np.asarray(z, float), levels=levels)
    ax.set_xlim(*xlim)
    ax.set_ylim(*ylim)
    kw = dict(clabel)
    if isinstance(kw.get("fmt"), list):
        kw["fmt"] = {float(k): v for k, v in kw["fmt"]}
    if "manual" in kw:
        kw["manual"] = [tuple(p) for p in kw["manual"]]
    texts = cs.clabel(**kw)
    labels = []
    for t, xy, cv in zip(texts, cs.labelXYs, cs.labelCValues):
        labels.append({
            "level": float(cv),
            "text": t.get_text(),
            "display": [float(xy[0]), float(xy[1])],
            "rotation": float(t.get_rotation()),
        })
    widths = [float(cs._get_nth_label_width(i)) for i in range(len(cs.labelLevelList))]
    return {
        "labels": labels,
        "label_widths": widths,
        "paths": [canon_path(p) for p in cs.get_paths()],
    }


def _ok(result):
    assert not isinstance(result, Exception), repr(result)
    return result


# ---------------------------------------------------------------------------
# Deterministic input data (input values are rounded to 4 decimals where they
# come from transcendental functions, so args stay short; outputs never are)
# ---------------------------------------------------------------------------


def _lin(n, lo, hi):
    return np.linspace(lo, hi, n)


def _r4(a):
    return np.round(np.asarray(a, dtype=float), 4)


X5 = _lin(5, -1, 1)  # exact: -1, -0.5, 0, 0.5, 1
X9 = _lin(9, -1, 1)  # exact multiples of 0.25
BOWL5 = (X5[None, :] ** 2 + X5[:, None] ** 2)
BOWL9 = (X9[None, :] ** 2 + X9[:, None] ** 2)
SADDLE5 = X5[None, :] * X5[:, None]

# Non-square smooth field with a peak, a pit and a tilt.
PX = _r4(_lin(11, -2, 2))
PY = _r4(_lin(9, -2, 2))
_PXX, _PYY = np.meshgrid(PX, PY)
PEAKS = _r4(np.exp(-((_PXX - 0.6) ** 2 + (_PYY - 0.4) ** 2))
            - 0.8 * np.exp(-((_PXX + 0.7) ** 2 + (_PYY + 0.5) ** 2) / 0.5)
            + 0.1 * _PXX)

# Concentric ripples: filled bands are annuli (rings with holes).
RX = _lin(11, -2, 2)  # step 0.4 (not exact in binary; fine as input)
RX = _r4(RX)
_RXX, _RYY = np.meshgrid(RX, RX)
RIPPLE = _r4(np.cos(2.0 * np.hypot(_RXX, _RYY)))

# Non-uniform rectilinear grid.
NUX = [0.0, 0.1, 0.3, 0.7, 1.5, 3.1]
NUY = [-1.0, -0.2, 0.0, 0.5, 2.0]
NUZ = _r4([[math.sin(1.3 * x) + 0.5 * y * y - 0.3 * x * y for x in NUX] for y in NUY])

# Curvilinear (polar) 2D grid: rows = angle, columns = radius.
_R = _lin(6, 0.5, 2.0)
_T = _lin(7, 0.0, math.pi)
POLX = _r4(_R[None, :] * np.cos(_T[:, None]))
POLY = _r4(_R[None, :] * np.sin(_T[:, None]))
POLZ = _r4(POLX + POLY ** 2)

# Integer-valued field on a 4x5 grid.
INTZ = np.array([[(i * 3 + j * 5) % 7 for j in range(5)] for i in range(4)], dtype=float)

# Small tilted field for colour / norm / extend probes (geometry stays small).
SX = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]
SY = [0.0, 1.0, 2.0, 3.0, 4.0]
SMALL = _r4([[math.sin(0.9 * x) + 0.4 * y - 0.05 * x * y for x in SX] for y in SY])


def L(a):
    """Plain nested float lists for args (-0.0 normalised to 0.0, NaN kept)."""
    return (np.asarray(a, dtype=float) + 0.0).tolist()


def _with_nan(z, points):
    z = np.array(z, dtype=float)
    for i, j in points:
        z[i, j] = np.nan
    return L(z)


def _mask(shape, points):
    m = np.zeros(shape, dtype=bool)
    for i, j in points:
        m[i, j] = True
    return m.tolist()


# ---------------------------------------------------------------------------
# Quad contour cases: (id, args).  Each is probed with contour and contourf
# unless listed in FILL_ONLY.
# ---------------------------------------------------------------------------

BOWL5_ARGS = {"x": L(X5), "y": L(X5), "z": L(BOWL5)}
BOWL9_ARGS = {"x": L(X9), "y": L(X9), "z": L(BOWL9)}
PEAKS_ARGS = {"x": L(PX), "y": L(PY), "z": L(PEAKS)}
RIPPLE_ARGS = {"x": L(RX), "y": L(RX), "z": L(RIPPLE)}
SMALL_ARGS = {"x": SX, "y": SY, "z": L(SMALL)}

QUAD_CASES = [
    # --- automatic levels ---
    ("bowl5_auto", {**BOWL5_ARGS}),
    ("bowl9_auto", {**BOWL9_ARGS}),
    ("bowl9_n1", {**BOWL9_ARGS, "levels": 1}),
    ("bowl9_n3", {**BOWL9_ARGS, "levels": 3}),
    ("bowl5_n10", {**BOWL5_ARGS, "levels": 10}),
    ("bowl5_n15", {**BOWL5_ARGS, "levels": 15}),
    ("bowl9_n0", {**BOWL9_ARGS, "levels": 0}),
    ("peaks_auto", {**PEAKS_ARGS}),
    ("peaks_n4", {**PEAKS_ARGS, "levels": 4}),
    ("peaks_n12", {**PEAKS_ARGS, "levels": 12}),
    ("ripple_explicit", {**RIPPLE_ARGS, "levels": [-0.8, -0.3, 0.2, 0.7]}),
    # --- explicit levels ---
    ("bowl9_explicit", {**BOWL9_ARGS, "levels": [0.3, 0.75, 1.2, 1.9]}),
    ("bowl9_single", {**BOWL9_ARGS, "levels": [0.5]}),
    ("bowl9_outside", {**BOWL9_ARGS, "levels": [-1.0, 5.0]}),
    ("bowl9_partly_outside", {**BOWL9_ARGS, "levels": [-1.0, 0.5, 3.0]}),
    ("bowl9_at_min", {**BOWL9_ARGS, "levels": [0.0, 1.0, 2.0]}),
    ("bowl9_at_grid_values", {**BOWL9_ARGS, "levels": [0.0625, 0.25, 0.5, 1.0625]}),
    ("bowl9_empty_levels", {**BOWL9_ARGS, "levels": []}),
    ("bowl9_unsorted", {**BOWL9_ARGS, "levels": [1.0, 0.5]}),
    ("bowl9_duplicate", {**BOWL9_ARGS, "levels": [0.5, 0.5, 1.0]}),
    ("bowl9_negative_n", {**BOWL9_ARGS, "levels": -1}),
    # --- extend ---
    ("peaks_extend_both", {**PEAKS_ARGS, "extend": "both"}),
    ("small_auto_extend_min", {**SMALL_ARGS, "extend": "min"}),
    ("small_auto_extend_max", {**SMALL_ARGS, "extend": "max"}),
    ("small_auto_extend_neither", {**SMALL_ARGS, "extend": "neither"}),
    ("small_auto", {**SMALL_ARGS}),
    ("small_explicit_extend_both", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "extend": "both"}),
    ("small_explicit_extend_min", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "extend": "min"}),
    ("small_explicit_extend_max", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "extend": "max"}),
    ("small_auto_extend_both", {**SMALL_ARGS, "extend": "both"}),
    ("bowl9_n3_extend_both", {**BOWL9_ARGS, "levels": 3, "extend": "both"}),
    ("bowl5_extend_bogus", {**BOWL5_ARGS, "extend": "sideways"}),
    # --- vmin / vmax (colour mapping only) ---
    ("small_vmin_vmax", {**SMALL_ARGS, "vmin": -2.0, "vmax": 0.5}),
    ("small_vmax_only", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "vmax": 3.0}),
    ("small_vmin_vmax_extend_both", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "vmin": 0.25, "vmax": 0.75,
                                     "extend": "both"}),
    # --- saddles, incl. exact ambiguity (quad centre value == level) ---
    ("saddle5", {"x": L(X5), "y": L(X5), "z": L(SADDLE5), "levels": [-0.5, -0.1, 0.1, 0.5]}),
    ("saddle5_level0", {"x": L(X5), "y": L(X5), "z": L(SADDLE5), "levels": [-0.25, 0.0, 0.25]}),
    ("saddle2_diag_lo", {"z": [[1.0, 0.0], [0.0, 1.0]], "levels": [0.25, 0.5, 0.75]}),
    ("saddle2_diag_hi", {"z": [[0.0, 1.0], [1.0, 0.0]], "levels": [0.25, 0.5, 0.75]}),
    ("saddle2_exact_only", {"z": [[1.0, 0.0], [0.0, 1.0]], "levels": [0.5]}),
    ("saddle2_exact_band", {"z": [[1.0, 0.0], [0.0, 1.0]], "levels": [0.5, 1.0]}),
    ("checker3", {"z": [[1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 1.0]], "levels": [0.5, 1.5]}),
    # loops around an isolated minimum: closed (CLOSEPOLY) vs 1-ulp-open in mpl2014
    ("pit_closed_loop", {"z": [[1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0]], "levels": [0.5, 1.5]}),
    ("pit_ulp_open_loop", {"z": [[1.0, 1.0, 1.0, 1.0], [1.0, 0.3, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0]],
                           "levels": [0.5, 1.5]}),
    ("checker4_asym", {"z": [[2.0, 0.0, 2.0, 0.0], [0.0, 1.0, 0.0, 3.0], [2.0, 0.0, 2.0, 0.0]],
                       "levels": [0.4, 0.75, 1.0, 1.6]}),
    # --- NaN holes and masks, corner_mask ---
    ("peaks_nan_interior", {**PEAKS_ARGS, "z": _with_nan(PEAKS, [(5, 6), (3, 3), (3, 4)])}),
    ("peaks_nan_interior_nocorner", {**PEAKS_ARGS, "z": _with_nan(PEAKS, [(5, 6), (3, 3), (3, 4)]),
                                     "corner_mask": False}),
    ("peaks_nan_edge_corner", {**PEAKS_ARGS, "z": _with_nan(PEAKS, [(0, 0), (8, 5), (4, 10)])}),
    ("peaks_nan_edge_corner_nocorner", {**PEAKS_ARGS, "z": _with_nan(PEAKS, [(0, 0), (8, 5), (4, 10)]),
                                        "corner_mask": False}),
    ("bowl9_mask_block", {**BOWL9_ARGS, "mask": _mask((9, 9), [(3, 3), (3, 4), (4, 3), (4, 4), (6, 1)]),
                          "levels": [0.2, 0.6, 1.0, 1.5]}),
    ("bowl9_mask_block_nocorner", {**BOWL9_ARGS, "mask": _mask((9, 9), [(3, 3), (3, 4), (4, 3), (4, 4), (6, 1)]),
                                   "levels": [0.2, 0.6, 1.0, 1.5], "corner_mask": False}),
    ("bowl9_mask_rc_nocorner", {**BOWL9_ARGS, "mask": _mask((9, 9), [(6, 1)]),
                                "levels": [0.6, 1.0], "rc": {"contour.corner_mask": False}}),
    ("checker3_mask_corner", {"z": [[1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 1.0]],
                              "mask": _mask((3, 3), [(2, 2)]), "levels": [0.5, 2.0]}),
    ("checker3_mask_nocorner", {"z": [[1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 1.0]],
                                "mask": _mask((3, 3), [(2, 2)]), "levels": [0.5, 2.0], "corner_mask": False}),
    ("bowl5_corner_mask_legacy", {**BOWL5_ARGS, "mask": _mask((5, 5), [(2, 2)]), "levels": [0.5, 1.5],
                                  "corner_mask": "legacy"}),
    ("bowl5_all_nan_but_one_quad", {"z": _with_nan(BOWL5, [(i, j) for i in range(5) for j in range(5)
                                                           if not (i < 2 and j < 2)]),
                                    "levels": [0.3, 1.0, 1.9]}),
    ("all_nan", {"z": [[float("nan")] * 3] * 3}),
    # --- constant / flat fields ---
    ("const_one", {"z": [[1.0] * 4] * 4}),
    ("const_zero", {"z": [[0.0] * 4] * 4}),
    ("const_neg", {"z": [[-7.0] * 3] * 3}),
    ("const_one_level_at", {"z": [[1.0] * 3] * 3, "levels": [0.0, 1.0, 2.0]}),
    ("const_one_level_lower", {"z": [[1.0] * 3] * 3, "levels": [1.0, 2.0]}),
    ("const_one_level_upper", {"z": [[1.0] * 3] * 3, "levels": [0.0, 1.0]}),
    ("const_one_level_above", {"z": [[1.0] * 3] * 3, "levels": [1.5, 2.0]}),
    ("flat_plateau", {"z": [[0.0, 0.0, 0.0, 0.0], [0.0, 1.0, 1.0, 0.0], [0.0, 1.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0]],
                      "levels": [0.0, 0.5, 1.0]}),
    # --- grid geometry: origin / extent without X, Y ---
    ("int_origin_none", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5]}),
    ("int_origin_none_extent", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "extent": [10.0, 22.0, -1.0, 1.5]}),
    ("int_origin_lower", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "lower"}),
    ("int_origin_lower_extent", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "lower",
                                 "extent": [10.0, 22.0, -1.0, 1.5]}),
    ("int_origin_upper", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "upper"}),
    ("int_origin_upper_extent", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "upper",
                                 "extent": [10.0, 22.0, -1.0, 1.5]}),
    ("int_origin_image_rc_lower", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "image",
                                   "rc": {"image.origin": "lower"}}),
    ("int_origin_image_default", {"z": L(INTZ), "levels": [1.5, 3.5, 5.5], "origin": "image"}),
    ("int_origin_bogus", {"z": L(INTZ), "origin": "middle"}),
    ("int_extent_bad_len", {"z": L(INTZ), "extent": [0.0, 1.0, 2.0]}),
    ("int_auto_levels", {"z": L(INTZ)}),
    # --- coordinates ---
    ("nonuniform_1d", {"x": NUX, "y": NUY, "z": L(NUZ), "levels": 6}),
    ("nonuniform_2d", {"x": L(np.meshgrid(NUX, NUY)[0]), "y": L(np.meshgrid(NUX, NUY)[1]), "z": L(NUZ),
                       "levels": 6}),
    ("polar_2d", {"x": L(POLX), "y": L(POLY), "z": L(POLZ), "levels": [-1.0, 0.0, 1.0, 2.0, 3.0]}),
    ("bowl9_x_reversed", {"x": L(X9[::-1]), "y": L(X9), "z": L(BOWL9 + 0.5 * X9[None, :]),
                          "levels": [0.4, 1.0, 1.6]}),
    ("bowl9_y_reversed", {"x": L(X9), "y": L(X9[::-1]), "z": L(BOWL9 + 0.5 * X9[None, :]),
                          "levels": [0.4, 1.0, 1.6]}),
    # --- dtypes ---
    ("bool_checker", {"z": [[True, False, True, False], [False, True, True, False], [True, True, False, False]],
                      "z_dtype": "bool"}),
    ("bool_explicit_levels", {"z": [[True, False, True], [False, True, False]], "z_dtype": "bool",
                              "levels": [0.25, 0.75]}),
    # --- log scale ---
    ("log_locator", {"z": [[1.0, 10.0, 100.0], [10.0, 100.0, 1000.0], [100.0, 1000.0, 10000.0]],
                     "locator": {"name": "LogLocator"}}),
    ("log_locator_numticks3", {"z": [[1.0, 10.0, 100.0], [10.0, 100.0, 1000.0], [100.0, 1000.0, 10000.0]],
                               "locator": {"name": "LogLocator", "numticks": 3}}),
    ("log_norm", {"z": [[1.0, 10.0, 100.0], [10.0, 100.0, 1000.0], [100.0, 1000.0, 10000.0]],
                  "norm": "log"}),
    ("log_norm_n3", {"z": [[1.0, 10.0, 100.0], [10.0, 100.0, 1000.0], [100.0, 1000.0, 10000.0]],
                     "norm": "log", "levels": 3}),
    ("log_nonpositive", {"z": [[-1.0, 10.0, 100.0], [0.0, 30.0, 1000.0], [100.0, 300.0, 5000.0]],
                         "locator": {"name": "LogLocator"}}),
    ("log_explicit_levels", {"z": [[1.0, 10.0, 100.0], [10.0, 100.0, 1000.0], [100.0, 1000.0, 10000.0]],
                             "norm": "log", "levels": [3.0, 30.0, 300.0, 3000.0], "extend": "both"}),
    ("maxn_symmetric", {"z": L(INTZ), "locator": {"name": "MaxNLocator", "nbins": 4, "symmetric": True}}),
    ("maxn_steps", {**PEAKS_ARGS, "locator": {"name": "MaxNLocator", "nbins": 5, "steps": [1, 3]}}),
    # --- chunking ---
    ("peaks_nchunk3", {**PEAKS_ARGS, "levels": [-0.4, 0.0, 0.4], "nchunk": 3}),
    ("ripple_nchunk4", {**RIPPLE_ARGS, "levels": [-0.3, 0.2], "nchunk": 4}),
    # --- colours / line styles ---
    ("small_colors_k", {**SMALL_ARGS, "levels": [-0.5, -0.25, 0.0, 0.5, 1.0], "colors": "k"}),
    ("small_colors_k_rc", {**SMALL_ARGS, "levels": [-0.5, 0.0, 0.5], "colors": "k",
                           "rc": {"contour.negative_linestyle": "dashdot", "contour.linewidth": 2.5}}),
    ("small_colors_list", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "colors": ["r", "g"]}),
    ("small_colors_extend_both", {**SMALL_ARGS, "levels": [0.0, 0.5, 1.0], "extend": "both",
                                  "colors": ["b", "c", "y", "m"]}),
    # --- argument errors ---
    ("err_z_1d", {"z": [1.0, 2.0, 3.0]}),
    ("err_z_one_row", {"z": [[1.0, 2.0, 3.0]]}),
    ("err_x_len", {"x": [0.0, 1.0], "y": [0.0, 1.0, 2.0], "z": [[0.0, 1.0, 2.0]] * 3}),
    ("err_y_len", {"x": [0.0, 1.0, 2.0], "y": [0.0, 1.0], "z": [[0.0, 1.0, 2.0]] * 3}),
    ("err_xy_ndim", {"x": [0.0, 1.0, 2.0], "y": [[0.0] * 3] * 3, "z": [[0.0, 1.0, 2.0]] * 3}),
    ("err_x2d_shape", {"x": [[0.0, 1.0]] * 3, "y": [[0.0, 1.0]] * 3, "z": [[0.0, 1.0, 2.0]] * 3}),
]

FILL_ONLY = {"saddle2_exact_band", "small_colors_extend_both"}


@pytest.mark.parametrize("case_id, args", QUAD_CASES, ids=[c[0] for c in QUAD_CASES])
def test_quad(case_id, args):
    if case_id not in FILL_ONLY:
        probes.probe("Axes.contour", contour_op, **args)
    probes.probe("Axes.contourf", contourf_op, **args)


RAW_CASES = [
    "pit_closed_loop", "pit_ulp_open_loop", "bowl5_auto", "saddle5", "saddle2_diag_lo", "checker3_mask_corner", "peaks_n4",
    "peaks_nan_interior", "ripple_explicit", "int_origin_upper", "log_locator", "const_one",
    "bowl9_x_reversed",
]


@pytest.mark.parametrize("case_id", RAW_CASES)
def test_quad_raw(case_id):
    args = dict(QUAD_CASES)[case_id]
    for func in ("contour", "contourf"):
        _ok(probes.probe("ContourSet.get_paths", raw_paths_op, func=func, **args))


ALGORITHM_CASES = ["pit_closed_loop", "saddle5", "checker3_mask_corner", "peaks_nan_interior",
                   "ripple_explicit", "int_origin_upper"]


@pytest.mark.parametrize("case_id", ALGORITHM_CASES)
@pytest.mark.parametrize("algorithm", ["mpl2005", "serial", "threaded"])
def test_quad_algorithms(case_id, algorithm):
    """The paths of every contouring algorithm, chosen by keyword and by
    ``contour.algorithm`` (mpl2005 turns corner masking off unless asked)."""
    args = dict(QUAD_CASES)[case_id]
    for func in ("contour", "contourf"):
        probes.probe("ContourSet.get_paths", raw_paths_op, func=func, algorithm=algorithm, **args)
        probes.probe("ContourSet.get_paths", raw_paths_op, func=func,
                     rc={"contour.algorithm": algorithm}, **args)


# ---------------------------------------------------------------------------
# Hand-written expectations for quad contours
# ---------------------------------------------------------------------------


def _get(case_id, filled):
    args = dict(QUAD_CASES)[case_id]
    return (contourf_op if filled else contour_op)(**args)


def test_quad_known_values():
    # Exact saddle ambiguity: the quad's centre value (mean of the 4 corners)
    # equal to the level counts as "below", so the high corners get cut off.
    r = _get("saddle2_exact_only", False)
    assert r["paths"] == [[
        {"closed": False, "vertices": [[0.5, 0.0], [0.0, 0.5]]},
        {"closed": False, "vertices": [[0.5, 1.0], [1.0, 0.5]]},
    ]]
    # mpl2014 only flags a loop closed if its recomputed end point is
    # bit-identical to its start.
    r = _get("pit_closed_loop", False)
    assert r["paths"][0] == [{"closed": True, "vertices": [[0.5, 1.0], [1.0, 1.5], [1.5, 1.0], [1.0, 0.5]]}]
    r = _get("pit_ulp_open_loop", False)
    assert r["paths"][0] == [{"closed": False, "vertices": [
        [1.0, 0.7142857142857142], [0.7142857142857143, 1.0], [1.0, 1.2857142857142856],
        [1.2857142857142856, 1.0], [1.0, 0.7142857142857143]]}]
    # Constant field: MaxNLocator expands the singular range.
    r = _get("const_one", False)
    assert r["levels"].tolist() == [0.999999999999975, 1.0, 1.000000000000025]
    assert r["warnings"] == [] and r["paths"] == [[], [], []]
    r = _get("const_one", True)
    assert len(r["paths"][0]) == 1 and r["paths"][1] == []
    # Levels must be increasing; filled contours need >= 2 levels.
    assert isinstance(probes.probe("Axes.contour", contour_op, **dict(QUAD_CASES)["bowl9_unsorted"]), ValueError)
    assert isinstance(probes.probe("Axes.contourf", contourf_op, **{**BOWL9_ARGS, "levels": [0.5]}), ValueError)
    # Log scale with non-positive data warns and masks.
    r = _get("log_nonpositive", False)
    assert r["warnings"] == [["UserWarning", "Log scale: values of z <= 0 have been masked"]]
    assert r["zmin"] == 10.0 and r["logscale"]
    # Extend adds +-1e250 sentinel bands for filled contours.
    r = _get("small_explicit_extend_both", True)
    assert r["bands"][0][0] == -1e250 and r["bands"][-1][1] == 1e250 and len(r["paths"]) == 4
    assert r["layers"].tolist() == [-5e249, 0.25, 0.75, 5e249]
    # Lowest band includes zmin: lower bound lowered by 1.
    r = _get("bowl9_at_min", True)
    assert r["bands"][0] == [-1.0, 1.0]
    # origin='upper' flips rows; extent with origin=None spans grid points.
    r = _get("int_origin_none_extent", False)
    assert r["xlim"] == [10.0, 22.0] and r["ylim"] == [-1.0, 1.5]
    # origin='lower'/'upper' with extent: grid points at pixel centres.
    lo = _get("int_origin_lower_extent", False)
    up = _get("int_origin_upper_extent", False)
    assert lo["xlim"] == pytest.approx([11.2, 20.8]) and lo["ylim"] == pytest.approx([-0.6875, 1.1875])
    assert up["xlim"] == lo["xlim"] and up["ylim"] == lo["ylim"]
    # ... and 'upper' mirrors the rows (y -> 0.5 - y), reversing line direction.
    mirrored = sorted(sorted((x, 0.5 - y) for x, y in sub["vertices"]) for lev in up["paths"] for sub in lev)
    plain = sorted(sorted((x, y) for x, y in sub["vertices"]) for lev in lo["paths"] for sub in lev)
    assert len(mirrored) == len(plain)
    for m, q in zip(mirrored, plain):
        assert np.allclose(m, q, atol=1e-12)


def _signed_area(v):
    v = np.asarray(v, dtype=float)
    x, y = v[:, 0], v[:, 1]
    return 0.5 * float(np.sum(x * np.roll(y, -1) - np.roll(x, -1) * y))


ORIENTED_CASES = ["bowl9_auto", "peaks_n12", "ripple_explicit", "peaks_nan_interior", "bowl9_mask_block",
                  "nonuniform_1d", "saddle5", "checker4_asym", "peaks_extend_both", "peaks_nchunk3",
                  "small_auto_extend_both", "int_auto_levels"]


@pytest.mark.parametrize("case_id", ORIENTED_CASES)
def test_quad_orientation(case_id):
    """Lines: higher z on the left; fills: outer CCW, holes CW (index space == data space here)."""
    args = dict(QUAD_CASES)[case_id]
    _, ax = _fresh_axes()
    Z = np.asarray(args["z"], dtype=float)
    if "mask" in args:
        Z = np.ma.array(Z, mask=args["mask"])
    x = np.asarray(args["x"], dtype=float) if "x" in args else np.arange(Z.shape[1], dtype=float)
    y = np.asarray(args["y"], dtype=float) if "y" in args else np.arange(Z.shape[0], dtype=float)
    kw = _kwargs(args)
    Zf = np.ma.filled(np.ma.masked_invalid(Z), np.nan)

    def bilinear(px, py):
        j = int(np.clip(np.searchsorted(x, px) - 1, 0, len(x) - 2))
        i = int(np.clip(np.searchsorted(y, py) - 1, 0, len(y) - 2))
        tx = (px - x[j]) / (x[j + 1] - x[j])
        ty = (py - y[i]) / (y[i + 1] - y[i])
        return (Zf[i, j] * (1 - tx) * (1 - ty) + Zf[i, j + 1] * tx * (1 - ty)
                + Zf[i + 1, j] * (1 - tx) * ty + Zf[i + 1, j + 1] * tx * ty)

    cs = ax.contour(x, y, Z, **kw)
    checked = 0
    for path in cs.get_paths():
        for sub in path._iter_connected_components():
            v = sub.vertices
            for a, b in zip(v[:-1], v[1:]):
                d = b - a
                n = np.hypot(*d)
                if n < 1e-9:
                    continue
                m = (a + b) / 2
                left = np.array([-d[1], d[0]]) / n * 1e-7
                zl, zr = bilinear(*(m + left)), bilinear(*(m - left))
                if np.isnan(zl) or np.isnan(zr):
                    continue
                assert zl > zr
                checked += 1
    assert checked > 0

    cf = ax.contourf(x, y, Z, **kw)
    gen = cf._contour_generator
    for lo, hi in zip(*cf._get_lowers_and_uppers()):
        for vs, cds in zip(*gen.create_filled_contour(lo, hi)):
            starts = list(np.flatnonzero(cds == Path.MOVETO))
            for k, (s, e) in enumerate(zip(starts, starts[1:] + [len(cds)])):
                assert cds[e - 1] == Path.CLOSEPOLY and (vs[e - 1] == vs[s]).all()
                area = _signed_area(vs[s:e - 1])
                if abs(area) > 1e-12:
                    assert (area > 0) == (k == 0)  # first ring of a group is the outer one


def test_quad_orientation_follows_index_space():
    flipped = _get("bowl9_x_reversed", False)
    normal = contour_op(**{**dict(QUAD_CASES)["bowl9_x_reversed"], "x": L(X9),
                           "z": L(BOWL9 - 0.5 * X9[None, :])})
    for a, b in zip(flipped["paths"], normal["paths"]):
        for ra, rb in zip(a, b):
            if ra["closed"]:
                assert _signed_area(ra["vertices"]) == -_signed_area(rb["vertices"]) != 0


@pytest.mark.parametrize("case_id", ["peaks_auto", "peaks_n12", "bowl9_at_min", "nonuniform_1d", "small_auto_extend_both"])
def test_filled_bands_tile_domain(case_id):
    r = _get(case_id, True)
    args = dict(QUAD_CASES)[case_id]
    x, y = args["x"], args["y"]
    total = sum(_signed_area(ring["vertices"]) for band in r["paths"] for ring in band)
    assert total == pytest.approx((x[-1] - x[0]) * (y[-1] - y[0]), rel=1e-12)


def test_raw_closepoly_repeats_start():
    runs = [(func, dict(QUAD_CASES)[c]) for c in RAW_CASES for func in ("contour", "contourf")]
    runs += [(func, args) for _, args in TRI_CASES[:16] for func in ("tricontour", "tricontourf")]
    for func, args in runs:
        filled = func.endswith("f")
        for p in raw_paths_op(func, **args):
            if p["codes"] is None:
                assert len(p["vertices"]) == 0
                continue
            codes = p["codes"]
            verts = p["vertices"]
            starts = [k for k, c in enumerate(codes) if c == Path.MOVETO]
            assert starts[0] == 0
            for s, e in zip(starts, starts[1:] + [len(codes)]):
                assert all(c == Path.LINETO for c in codes[s + 1:e - 1])
                if func == "contour":  # mpl2014: closed iff >1 point and exact end == start
                    assert (codes[e - 1] == Path.CLOSEPOLY) == (e - s > 1 and (verts[e - 1] == verts[s]).all())
                if codes[e - 1] == Path.CLOSEPOLY:
                    assert (verts[e - 1] == verts[s]).all()
                else:
                    # Filled output only has unclosed subpaths as the lone-MOVETO
                    # quirk of TriContourGenerator (see module docstring).
                    assert not filled or (e - s == 1 and func == "tricontourf")


# ---------------------------------------------------------------------------
# Triangular contours (explicit triangles only)
# ---------------------------------------------------------------------------


def _grid_triangulation(nx, ny, x0, x1, y0, y1):
    xs = _lin(nx, x0, x1)
    ys = _lin(ny, y0, y1)
    pts_x, pts_y, tris = [], [], []
    for i in range(ny):
        for j in range(nx):
            pts_x.append(float(xs[j]))
            pts_y.append(float(ys[i]))
    for i in range(ny - 1):
        for j in range(nx - 1):
            a, b, c, d = i * nx + j, i * nx + j + 1, (i + 1) * nx + j, (i + 1) * nx + j + 1
            if (i + j) % 2 == 0:
                tris += [[a, b, d], [a, d, c]]
            else:
                tris += [[a, b, c], [b, d, c]]
    return pts_x, pts_y, tris


FAN = {"x": [0.0, 1.0, 0.0, 1.0, 0.5], "y": [0.0, 0.0, 1.0, 1.0, 0.5],
       "triangles": [[0, 1, 4], [1, 3, 4], [3, 2, 4], [2, 0, 4]], "z": [0.0, 0.0, 0.0, 0.0, 1.0]}
FAN_CW = {**FAN, "triangles": [[0, 4, 1], [1, 4, 3], [3, 4, 2], [2, 4, 0]]}
_GX, _GY, _GT = _grid_triangulation(6, 5, -1.0, 1.5, -1.0, 1.0)
GRID_TRI = {"x": _GX, "y": _GY, "triangles": _GT,
            "z": L(_r4([math.sin(2.0 * x) * math.cos(1.5 * y) + 0.3 * y for x, y in zip(_GX, _GY)]))}
# Hexagon around a centre with an inner ring: masked middle triangles -> hole.
_ang = [k * math.pi / 3 for k in range(6)]
HEX = {
    "x": L(_r4([0.0] + [0.5 * math.cos(a) for a in _ang] + [1.2 * math.cos(a) for a in _ang])),
    "y": L(_r4([0.0] + [0.5 * math.sin(a) for a in _ang] + [1.2 * math.sin(a) for a in _ang])),
    "triangles": ([[0, 1 + k, 1 + (k + 1) % 6] for k in range(6)]
                  + [[1 + k, 7 + k, 7 + (k + 1) % 6] for k in range(6)]
                  + [[1 + k, 7 + (k + 1) % 6, 1 + (k + 1) % 6] for k in range(6)]),
    "z": [0.0] + [1.0, 2.0, 1.5, 1.0, 2.5, 1.2] + [3.0, 2.0, 4.0, 3.5, 2.5, 3.0],
}

TRI_CASES = [
    ("fan_peak", {**FAN, "levels": [0.25, 0.5, 0.75]}),
    ("fan_peak_cw_input", {**FAN_CW, "levels": [0.25, 0.5, 0.75]}),
    ("fan_auto", {**FAN}),
    ("fan_level_at_vertices", {**FAN, "levels": [0.0, 1.0]}),
    ("fan_bands_hole", {**FAN, "z": [1.0, 1.0, 1.0, 1.0, 0.0], "levels": [0.25, 0.5, 2.0]}),
    ("grid_auto", {**GRID_TRI}),
    ("grid_n4", {**GRID_TRI, "levels": 4}),
    ("grid_explicit", {**GRID_TRI, "levels": [-0.8, -0.2, 0.3, 0.9]}),
    ("grid_extend_both", {**GRID_TRI, "levels": [-0.5, 0.0, 0.5], "extend": "both"}),
    ("grid_extend_min_auto", {**GRID_TRI, "extend": "min"}),
    ("grid_vmin_vmax", {**GRID_TRI, "levels": [-0.5, 0.0, 0.5], "vmin": -2.0, "vmax": 2.0}),
    ("grid_masked_triangles", {**GRID_TRI, "levels": [-0.5, 0.0, 0.5],
                               "tri_mask": [k in (7, 8, 9, 10, 17, 18) for k in range(len(_GT))]}),
    ("grid_checkerboard", {**GRID_TRI, "z": [float(((k % 6) + (k // 6)) % 2) for k in range(30)],
                           "levels": [0.25, 0.5, 0.75]}),
    ("hex_hole", {**HEX, "tri_mask": [k < 6 for k in range(18)], "levels": [1.5, 2.25, 3.25]}),
    ("hex_full", {**HEX, "levels": [0.0, 1.5, 2.25, 3.25, 4.0]}),
    ("hex_unused_nan_point", {**HEX, "x": HEX["x"] + [5.0], "y": HEX["y"] + [5.0],
                              "z": HEX["z"] + [float("nan")], "levels": [1.5, 2.25]}),
    ("hex_log_locator", {**HEX, "z": [v + 0.5 for v in HEX["z"]], "locator": {"name": "LogLocator"}}),
    ("const_tri", {**FAN, "z": [2.0] * 5}),
    ("err_nan_used_point", {**FAN, "z": [0.0, float("nan"), 0.0, 0.0, 1.0]}),
    ("err_z_len", {**FAN, "z": [0.0, 0.0, 0.0, 1.0]}),
    ("err_log_negative", {**FAN, "z": [-1.0, 0.0, 0.0, 0.0, 1.0], "norm": "log"}),
    ("err_unsorted_levels", {**FAN, "levels": [0.5, 0.25]}),
]


@pytest.mark.parametrize("case_id, args", TRI_CASES, ids=[c[0] for c in TRI_CASES])
def test_tri(case_id, args):
    r1 = probes.probe("Axes.tricontour", tricontour_op, **args)
    r2 = probes.probe("Axes.tricontourf", tricontourf_op, **args)
    if case_id.startswith("err_"):
        assert isinstance(r1, ValueError) and isinstance(r2, ValueError)
    else:
        _ok(r1)
        _ok(r2)


@pytest.mark.parametrize("case_id", ["fan_peak", "grid_explicit", "hex_hole"])
def test_tri_raw(case_id):
    args = dict(TRI_CASES)[case_id]
    for func in ("tricontour", "tricontourf"):
        _ok(probes.probe("ContourSet.get_paths", raw_paths_op, func=func, **args))


def test_tri_known_values():
    r = tricontour_op(**dict(TRI_CASES)["fan_peak"])
    assert r["paths"][1] == [{"closed": True, "vertices": [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]]}]
    # CW input triangles are reoriented: identical result.
    assert tricontour_op(**dict(TRI_CASES)["fan_peak_cw_input"])["paths"] == r["paths"]
    f = tricontourf_op(**dict(TRI_CASES)["fan_bands_hole"])
    # band (0.5, 2]: the whole square (outer, CCW) with a CW square hole.
    areas = sorted(_signed_area(ring["vertices"]) for ring in f["paths"][1])
    assert areas == [pytest.approx(-0.25), pytest.approx(1.0)]
    total = sum(_signed_area(ring["vertices"]) for band in tricontourf_op(**dict(TRI_CASES)["hex_full"])["paths"]
                for ring in band)
    hexagon = _signed_area(list(zip(HEX["x"][7:], HEX["y"][7:])))
    assert total == pytest.approx(hexagon, rel=1e-12)


# ---------------------------------------------------------------------------
# clabel
# ---------------------------------------------------------------------------

CLABEL_BASE = {"figsize": [4.0, 4.0], "dpi": 100, "rect": [0.1, 0.1, 0.8, 0.8], "xlim": [-1.0, 1.0],
               "ylim": [-1.0, 1.0], **BOWL9_ARGS, "levels": [0.5, 1.0, 1.5]}
CLABEL_PEAKS = {"figsize": [6.0, 3.0], "dpi": 80, "rect": [0.125, 0.11, 0.775, 0.77], "xlim": [-2.0, 2.0],
                "ylim": [-2.0, 2.0], **PEAKS_ARGS, "levels": [-0.4, -0.2, 0.0, 0.2, 0.4, 0.6]}

CLABEL_CASES = [
    ("bowl_default", {**CLABEL_BASE, "clabel": {}}),
    ("bowl_not_inline", {**CLABEL_BASE, "clabel": {"inline": False}}),
    ("bowl_spacing0", {**CLABEL_BASE, "clabel": {"inline_spacing": 0}}),
    ("bowl_spacing15", {**CLABEL_BASE, "clabel": {"inline_spacing": 15}}),
    ("bowl_fmt_str", {**CLABEL_BASE, "clabel": {"fmt": "%1.2f"}}),
    ("bowl_fmt_dict", {**CLABEL_BASE, "clabel": {"fmt": [[0.5, "half"], [1.0, "one"], [1.5, "x"]]}}),
    ("bowl_fontsize14", {**CLABEL_BASE, "clabel": {"fontsize": 14}}),
    ("bowl_fontsize_small", {**CLABEL_BASE, "clabel": {"fontsize": "small"}}),
    ("bowl_not_rightside_up", {**CLABEL_BASE, "clabel": {"rightside_up": False}}),
    ("bowl_level_subset", {**CLABEL_BASE, "clabel": {"levels": [1.0]}}),
    ("bowl_level_not_present", {**CLABEL_BASE, "clabel": {"levels": [0.7]}}),
    ("bowl_manual", {**CLABEL_BASE, "clabel": {"manual": [[0.3, 0.3], [-0.8, 0.1], [0.9, -0.9]]}}),
    ("bowl_manual_not_inline", {**CLABEL_BASE, "clabel": {"manual": [[0.3, 0.3]], "inline": False}}),
    ("bowl_wide_axes", {**CLABEL_BASE, "figsize": [8.0, 3.0], "clabel": {}}),
    ("peaks_default", {**CLABEL_PEAKS, "clabel": {}}),
    ("peaks_fontsize8", {**CLABEL_PEAKS, "clabel": {"fontsize": 8, "inline_spacing": 2}}),
    ("saddle_open_lines", {"figsize": [3.0, 3.0], "dpi": 100, "rect": [0.0, 0.0, 1.0, 1.0], "xlim": [-1.0, 1.0],
                           "ylim": [-1.0, 1.0], "x": L(X9), "y": L(X9),
                           "z": L(X9[None, :] * X9[:, None]), "levels": [-0.5, -0.1, 0.1, 0.5],
                           "clabel": {"fontsize": 7}}),
]


@pytest.mark.parametrize("case_id, args", CLABEL_CASES, ids=[c[0] for c in CLABEL_CASES])
def test_clabel(case_id, args):
    r = probes.probe("ContourSet.clabel", clabel_op, tol=1e-9, **args)
    if case_id == "bowl_level_not_present":
        assert isinstance(r, ValueError)
        return
    _ok(r)
    if case_id == "bowl_fmt_str":
        assert {lab["text"] for lab in r["labels"]} == {"0.50", "1.00", "1.50"}
    if case_id == "bowl_default":
        assert {lab["text"] for lab in r["labels"]} == {"0.5", "1.0", "1.5"}
        # Automatic placement puts every label on a contour vertex (in pixels:
        # axes spans x, y in [40, 360] for this figure).
        assert all(40.0 <= lab["display"][0] <= 360.0 and 40.0 <= lab["display"][1] <= 360.0
                   for lab in r["labels"])
    if case_id == "bowl_not_inline":
        assert r["paths"] == contour_op(**{**BOWL9_ARGS, "levels": [0.5, 1.0, 1.5]})["paths"]
    if case_id == "bowl_level_subset":
        assert {lab["level"] for lab in r["labels"]} == {1.0}
    upright = [lab["rotation"] <= 90 or lab["rotation"] >= 270 for lab in r["labels"]]
    if case_id == "bowl_not_rightside_up":
        assert not all(upright)
    else:
        assert all(upright)
