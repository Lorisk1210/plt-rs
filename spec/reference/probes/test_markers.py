"""Marker styles (matplotlib.markers), Line2D drawstyles/step conversion and
``markevery`` subsampling (matplotlib.lines / matplotlib.cbook).

Area name: ``markers`` (golden: ``spec/goldens/probes/markers.json``).

Out of scope: mathtext markers (``'$...$'``, rendered via TextPath) are not
probed here.

Shared encodings
================

Path      ``{"vertices": [[x, y], ...], "codes": [int, ...] | null}``
          (STOP=0, MOVETO=1, LINETO=2, CURVE3=3, CURVE4=4, CLOSEPOLY=79;
          ``codes`` is null iff ``Path.codes is None``).
Matrix    3x3 ``[[a, c, e], [b, d, f], [0, 0, 1]]`` (``get_matrix()``).
          Values are matplotlib's exact floats (no tolerance): e.g. a marker
          set up with ``Affine2D().scale(0.5).rotate_deg(180)`` carries
          +-6.123233995736766e-17 off-diagonal entries, as produced by
          Affine2D.rotate (a=cos, b=sin of ``math.radians(deg)``, applied as
          ``[[a,-b,0],[b,a,0],[0,0,1]] @ M``).
Errors    ``{"error": "<ExceptionTypeName>"}`` (written by the framework).

MARKER (input arg ``marker``) -- plain JSON, decoded as follows:
  * JSON string  -> Python ``str`` (``"o"``, ``"None"``, ``""``, ``" "``, ...).
  * JSON integer -> Python ``int`` (TICKLEFT=0, TICKRIGHT=1, TICKUP=2,
    TICKDOWN=3, CARETLEFT=4, CARETRIGHT=5, CARETUP=6, CARETDOWN=7,
    CARETLEFTBASE=8, CARETRIGHTBASE=9, CARETUPBASE=10, CARETDOWNBASE=11).
  * JSON null    -> Python ``None`` (rejected: ValueError).
  * flat list of numbers ``[numsides, style]`` / ``[numsides, style, angle]``
    -> Python *tuple* marker (style 0 = regular polygon, 1 = star with inner
    radius 0.5, 2 = asterisk = star with inner radius 0; ``angle`` in degrees,
    default 0). Any other flat list (other length, style not in {0,1,2}) is
    also passed as a tuple and is rejected by matplotlib.
  * list of ``[x, y]`` lists (or the empty list) -> vertex-list marker
    (Python list of lists).
  * object ``{"vertices": ..., "codes": ...}`` -> ``Path(vertices, codes)``.

TRANSFORM OP LIST (input args ``transform`` / ``"transformed"`` steps): list of
operations applied in order to a fresh ``Affine2D()`` (each op post-composes,
i.e. the first op is applied to points first):
  ``["rotate_deg", deg]``, ``["rotate", rad]``, ``["scale", sx]``,
  ``["scale", sx, sy]``, ``["translate", tx, ty]``, ``["skew_deg", xdeg, ydeg]``.
  ``[]`` is an identity Affine2D (NOT the same as ``null`` = no user transform).

MARKER DESCRIPTION (output of ``markers.MarkerStyle`` and
``markers.MarkerStyle.derived``): an object with keys
  ``path``           Path of ``get_path()`` (the face / whole marker, unit-ish
                     coordinates, before ``transform``).
  ``transform``      Matrix of ``get_transform()`` = native marker transform
                     followed by the user transform (if any).
  ``alt_path``       Path of ``get_alt_path()`` or null (null for non-half
                     fillstyles and for markers without half-fill support).
  ``alt_transform``  Matrix of ``get_alt_transform()``, or null when the
                     marker has no alternate transform. (matplotlib actually
                     raises AttributeError/TypeError from get_alt_transform()
                     in that case because its internal ``_alt_transform`` is
                     None; plt-rs should model it as "absent".) ``alt_path`` is
                     null iff ``alt_transform`` is null.
  ``user_transform`` Matrix of ``get_user_transform()`` or null.
  ``joinstyle``      ``get_joinstyle()`` ("miter" | "round" | "bevel").
  ``capstyle``       ``get_capstyle()`` ("butt" | "round" | "projecting").
  ``filled``         ``is_filled()``.
  ``fillstyle``      ``get_fillstyle()``.
  ``snap_threshold`` ``get_snap_threshold()``: number (may be Infinity) or null.
  ``marker``         ``get_marker()`` re-encoded with the MARKER encoding above
                     (tuple -> flat list, Path -> path object, vertex list as
                     given, str/int as given).

Ops
===

``markers.MarkerStyle.class_attributes``  args ``{}``.
    -> ``{"markers": [[key, name], ...] (dict order; keys str or int),
    "filled_markers": [...], "fillstyles": [...]}``.

``markers.MarkerStyle``  args ``{marker, fillstyle, [transform], [capstyle],
    [joinstyle], [rc]}`` (bracketed keys optional; absent == null).
    Evaluates ``MarkerStyle(marker, fillstyle=fillstyle, transform=T,
    capstyle=capstyle, joinstyle=joinstyle)`` where T is built from the
    TRANSFORM OP LIST (null -> None). ``fillstyle`` null means
    ``rcParams["markers.fillstyle"]`` ("full" in the reference config).
    ``rc`` is an optional dict of rcParams applied around construction (only
    ``"markers.fillstyle"`` is used). -> MARKER DESCRIPTION, or error
    (unknown marker / fillstyle / capstyle / joinstyle -> ValueError, tuple
    marker with numsides 0 -> ZeroDivisionError, empty Path -> ValueError;
    negative numsides give an empty polygon path, non-integer numsides n use
    ``arange(n + 1)`` angles).
    NOTE the user ``joinstyle`` only takes effect for markers whose setup
    explicitly honours it (polygons, triangles, square, diamonds, pentagon,
    star, hexagons, octagon, carets, 'P', 'X', tuple markers); circles, '.',
    '+', 'x', '|', '_', tri, tick and custom markers always report "round".
    The user ``capstyle`` always wins (default "butt").
    Custom (Path / vertex-list) markers: path unchanged, transform =
    ``scale(0.5 / max(abs(all vertices incl. CLOSEPOLY/codes-ignored ones)))``
    (no recentering; all-zero vertices give inf/NaN entries, NaN vertices give
    a NaN scale).

``markers.MarkerStyle.styles``  args ``{marker, capstyle, joinstyle}``
    (each style string or null; fillstyle "full").
    -> ``{"joinstyle": get_joinstyle(), "capstyle": get_capstyle()}``.

``markers.MarkerStyle.derived``  args ``{marker, fillstyle, transform, steps}``.
    Builds ``MarkerStyle(marker, fillstyle=..., transform=T)`` (as above) and
    then applies each step in order, each returning a new MarkerStyle:
      ``["rotated", {"deg": x}]`` / ``["rotated", {"rad": x}]`` (kwargs dict
      passed verbatim; ``{}`` or both keys -> ValueError),
      ``["scaled", sx]`` / ``["scaled", sx, sy]``,
      ``["transformed", TRANSFORM OP LIST]``.
    Semantics (matplotlib): rotated/scaled modify a copy of the user transform
    (starting from identity when absent) by post-composing rotate/scale;
    transformed sets the user transform to ``user + T`` (T applied after) or
    to T when absent. -> MARKER DESCRIPTION of the final marker, or error.
    NOTE: after a ``transformed`` step on a marker that already had a user
    transform, the user transform is a CompositeAffine2D and a subsequent
    ``rotated``/``scaled`` raises AttributeError.

``cbook.pts_to_prestep`` / ``cbook.pts_to_poststep`` / ``cbook.pts_to_midstep``
    args ``{x: [..], ys: [[..], ...]}`` -> ``f(x, *ys)`` as a 2-D list with
    ``1 + len(ys)`` rows ``[x_out, y1_out, ...]``. Row length is ``2N-1`` for
    pre/post (0 for N=0) and ``2N`` for mid. y arrays broadcast against the
    slots (length 1 broadcasts, other mismatches -> ValueError).
    ``ys == []`` -> ValueError for N >= 1 (empty x returns ``[[]]``).
    pre:  x_out[2i]=x[i], x_out[2i+1]=x[i];  y_out[2i]=y[i], y_out[2i+1]=y[i+1].
    post: x_out[2i]=x[i], x_out[2i+1]=x[i+1]; y_out[2i]=y[i], y_out[2i+1]=y[i].
    mid:  x_out = x0, m0, m0, m1, m1, ..., x[N-1] with m_i=(x[i]+x[i+1])/2;
          y_out[2i]=y_out[2i+1]=y[i].

``lines.drawstyle_names``  args ``{}`` -> ``{"drawStyleKeys": Line2D.drawStyleKeys,
    "drawStyles": Line2D.drawStyles (name -> legacy method name),
    "STEP_LOOKUP_MAP": list of cbook.STEP_LOOKUP_MAP keys}``.

``lines.drawstyle``  args ``{drawstyle}`` -> ``{"draw_method":
    Line2D.drawStyles[name], "step_function": STEP_LOOKUP_MAP[name].__name__}``
    where the "default" entry (an identity lambda) is reported as
    ``"identity"``. Unknown name -> KeyError.

``lines.Line2D.set_drawstyle``  args ``{drawstyle}`` (string or null) ->
    ``get_drawstyle()`` after ``set_drawstyle`` (null -> "default"); unknown
    name -> ValueError.

``lines.Line2D.get_path``  args ``{x, y, drawstyle}`` ->
    Path of ``Line2D(x, y, drawstyle=drawstyle).get_path()`` (x and y are
    broadcast together first; the step function of the drawstyle is applied to
    ``(x, y)``; codes are null). Mismatched lengths -> ValueError.

``axes._process_plot_format``  args ``{fmt}`` -> ``[linestyle, marker, color]``
    for a MATLAB-style format string under matplotlib's defaults:
    line styles as ``"solid"``/``"dashed"``/``"dashdot"``/``"dotted"``/
    ``"none"`` (or null when unset), markers as their character (``"None"``
    for no marker, null when unset) and colours as ``[r, g, b, a]`` resolved
    against the default property cycle (null when unset). Invalid -> error.

``lines.markevery``  args ``{figsize: [w_in, h_in], dpi, rect: [l, b, w, h]
    (figure fraction), xscale, yscale ("linear"|"log"), xlim: [lo, hi],
    ylim: [lo, hi], x: [...], y: [...], drawstyle, markevery: MARKEVERY}``.
    Builds ``Figure(figsize, dpi)``, ``ax = fig.add_axes(rect)``,
    ``line, = ax.plot(x, y, marker="o", drawstyle=drawstyle)``,
    ``line.set_markevery(...)``, then sets xscale, yscale, xlim, ylim (in that
    order) and draws the figure with Agg. Returns the list of DATA INDICES at
    which markers are drawn, in matplotlib's drawing order (duplicates kept,
    negative indices normalised to ``i + N``). Indices of non-finite points
    are included when the selection picks them (the renderer then skips
    them). markevery null -> all indices; empty data -> ``[]``. Invalid
    markevery -> error (raised by matplotlib at draw time).

    MARKEVERY encoding (tagged object; JSON cannot tell tuple from list):
      ``{"kind": "none"}``                   -> None (all points)
      ``{"kind": "bool", "value": b}``       -> bool (True == int 1, False ==
                                                int 0 -> ValueError)
      ``{"kind": "int", "value": n}``        -> int n == (0, n)
      ``{"kind": "start_step", "value": [s, n]}`` -> tuple (int s, int n):
                                                ``verts[s::n]`` (numpy slice
                                                semantics, negative OK)
      ``{"kind": "slice", "start": a|null, "stop": b|null, "step": c|null}``
                                             -> ``slice(a, b, c)``
      ``{"kind": "indices", "value": [i, ...]}`` -> list of ints (numpy fancy
                                                index; out of range ->
                                                ValueError)
      ``{"kind": "mask", "value": [b, ...]}`` -> list of bools (boolean mask;
                                                length != N -> ValueError)
      ``{"kind": "float", "value": f}``      -> float f == (0.0, f)
      ``{"kind": "float_pair", "value": [s, f]}`` -> tuple (float s, float f)
      ``{"kind": "tuple", "value": [...], "types": ["int"|"float", ...]}``
                                             -> generic tuple, element i cast
                                                to types[i] (mixed / wrong
                                                length tuples; e.g. (0, 0.1)
                                                is valid, (0.5, 2) and
                                                3-tuples -> ValueError)

    Distance-based kinds (float step: "float", "float_pair", "tuple" whose
    2nd element is a float) follow matplotlib's ``_mark_every_path``:
      * ``tp`` = line points after the NON-AFFINE part of transData (identity
        for linear axes, log10 for log axes with non-positive values clipped
        to -1000), always with the default drawstyle (markers ignore
        drawstyle). Keep only rows where both coords are finite (``fin``).
      * ``d`` = display coords (affine part of transData, pixels) of the kept
        rows; ``delta`` = cumulative sum of segment lengths, ``delta[0]=0``.
        (No finite rows -> IndexError.)
      * ``scale`` = pixel length of the axes diagonal; ``start = s*scale``,
        ``step = f*scale`` (``step <= 0`` -> ValueError).
      * candidates ``m = delta - mod(delta - start, step)`` (numpy remainder),
        union ``m + step``; keep ``start <= m < delta[-1]`` (strict: so the
        last point is only chosen if a candidate lies closer to it than to
        its predecessor; a single point never gets a marker).
      * for each candidate: ``r = searchsorted(delta, m, "left")``,
        ``l = max(r-1, 0)``, ``r = min(r, len-1)``; pick ``r`` if
        ``|delta[r]-m| < |m-delta[l]|`` else ``l``; then replace each pick by
        ``searchsorted(delta, delta[pick], "left")`` (first of equal
        distances) and take sorted unique values. Indices are into the kept
        rows; the probe maps them back to original data indices.
    Every distance-based probe in the golden is tie-free: its selection is
    unchanged when the display segment lengths or the axes diagonal are
    perturbed by 1e-10 (relative), so plt-rs does not need last-ulp agreement
    of its transforms to reproduce it. (Exactly evenly spaced data whose
    spacing divides the marker step is tie-prone and deliberately avoided.)
"""

from __future__ import annotations

from numbers import Integral, Real
from unittest import mock

import matplotlib
import matplotlib.lines as mlines
import numpy as np
import pytest
from matplotlib import cbook
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.lines import Line2D
from matplotlib.markers import MarkerStyle
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

from spec_support import ProbeSet

probes = ProbeSet("markers")

MARKER_KEYS = list(MarkerStyle.markers)
FILLSTYLES = list(MarkerStyle.fillstyles)
JOINSTYLES = ["miter", "round", "bevel"]
CAPSTYLES = ["butt", "round", "projecting"]


# --------------------------------------------------------------------------
# encoding helpers
# --------------------------------------------------------------------------

def _enc_path(path):
    if path is None:
        return None
    return {
        "vertices": np.asarray(path.vertices, dtype=float).tolist(),
        "codes": None if path.codes is None else [int(c) for c in path.codes],
    }


def _enc_matrix(transform):
    if transform is None:
        return None
    return np.asarray(transform.get_matrix(), dtype=float).tolist()


def _is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _dec_marker(m):
    if isinstance(m, dict):
        return Path(np.asarray(m["vertices"], dtype=float).reshape(-1, 2), m.get("codes"))
    if isinstance(m, list):
        if m and all(_is_number(v) for v in m):
            return tuple(m)
        return [list(v) if isinstance(v, list) else v for v in m]
    return m


def _enc_marker(m):
    if isinstance(m, Path):
        return _enc_path(m)
    if isinstance(m, tuple):
        return list(m)
    if isinstance(m, np.ndarray):
        return m.tolist()
    return m


def _dec_transform(ops):
    if ops is None:
        return None
    t = Affine2D()
    for op in ops:
        name, params = op[0], op[1:]
        if name not in ("rotate_deg", "rotate", "scale", "translate", "skew_deg"):
            raise AssertionError(f"unknown transform op {name!r}")
        getattr(t, name)(*params)
    return t


def _describe(ms):
    alt_t = None if ms._alt_transform is None else ms.get_alt_transform()
    assert (ms.get_alt_path() is None) == (alt_t is None)
    return {
        "path": _enc_path(ms.get_path()),
        "transform": _enc_matrix(ms.get_transform()),
        "alt_path": _enc_path(ms.get_alt_path()),
        "alt_transform": _enc_matrix(alt_t),
        "user_transform": _enc_matrix(ms.get_user_transform()),
        "joinstyle": ms.get_joinstyle(),
        "capstyle": ms.get_capstyle(),
        "filled": bool(ms.is_filled()),
        "fillstyle": ms.get_fillstyle(),
        "snap_threshold": ms.get_snap_threshold(),
        "marker": _enc_marker(ms.get_marker()),
    }


def _make_marker(marker, fillstyle=None, transform=None, capstyle=None,
                 joinstyle=None, rc=None):
    with matplotlib.rc_context(rc or {}), np.errstate(all="ignore"):
        return MarkerStyle(_dec_marker(marker), fillstyle=fillstyle,
                           transform=_dec_transform(transform),
                           capstyle=capstyle, joinstyle=joinstyle)


# --------------------------------------------------------------------------
# adapters
# --------------------------------------------------------------------------

def _class_attributes():
    return {
        "markers": [[k, v] for k, v in MarkerStyle.markers.items()],
        "filled_markers": list(MarkerStyle.filled_markers),
        "fillstyles": list(MarkerStyle.fillstyles),
    }


def _marker_style(marker, fillstyle, transform=None, capstyle=None,
                  joinstyle=None, rc=None):
    return _describe(_make_marker(marker, fillstyle, transform, capstyle,
                                  joinstyle, rc))


def _marker_styles(marker, capstyle, joinstyle):
    ms = _make_marker(marker, "full", None, capstyle, joinstyle)
    return {"joinstyle": ms.get_joinstyle(), "capstyle": ms.get_capstyle()}


def _marker_derived(marker, fillstyle, transform, steps):
    ms = _make_marker(marker, fillstyle, transform)
    for step in steps:
        kind = step[0]
        if kind == "rotated":
            ms = ms.rotated(**step[1])
        elif kind == "scaled":
            ms = ms.scaled(*step[1:])
        elif kind == "transformed":
            ms = ms.transformed(_dec_transform(step[1]))
        else:
            raise AssertionError(f"unknown step {kind!r}")
    return _describe(ms)


def _step_fn(fn):
    def adapter(x, ys):
        return np.asarray(fn(x, *ys))
    return adapter


def _identity_name(fn):
    return "identity" if fn is cbook.STEP_LOOKUP_MAP["default"] else fn.__name__


def _drawstyle_names():
    return {
        "drawStyleKeys": list(Line2D.drawStyleKeys),
        "drawStyles": dict(Line2D.drawStyles),
        "STEP_LOOKUP_MAP": list(cbook.STEP_LOOKUP_MAP),
    }


def _drawstyle(drawstyle):
    return {
        "draw_method": Line2D.drawStyles[drawstyle],
        "step_function": _identity_name(cbook.STEP_LOOKUP_MAP[drawstyle]),
    }


def _set_drawstyle(drawstyle):
    line = Line2D([0.0, 1.0], [0.0, 1.0])
    line.set_drawstyle(drawstyle)
    return line.get_drawstyle()


def _line_path(x, y, drawstyle):
    return _enc_path(Line2D(x, y, drawstyle=drawstyle).get_path())


def _dec_markevery(spec):
    kind = spec["kind"]
    if kind == "none":
        return None
    if kind == "bool":
        return bool(spec["value"])
    if kind == "int":
        return int(spec["value"])
    if kind == "float":
        return float(spec["value"])
    if kind == "start_step":
        s, n = spec["value"]
        return (int(s), int(n))
    if kind == "float_pair":
        s, f = spec["value"]
        return (float(s), float(f))
    if kind == "tuple":
        cast = {"int": int, "float": float}
        return tuple(cast[t](v) for t, v in zip(spec["types"], spec["value"], strict=True))
    if kind == "slice":
        return slice(spec["start"], spec["stop"], spec["step"])
    if kind == "indices":
        return [int(i) for i in spec["value"]]
    if kind == "mask":
        return [bool(b) for b in spec["value"]]
    raise AssertionError(f"unknown markevery kind {kind!r}")


def _is_distance_based(me):
    if isinstance(me, Integral):
        return False
    if isinstance(me, Real):
        return True
    if isinstance(me, tuple) and len(me) == 2:
        return isinstance(me[1], Real) and not isinstance(me[1], Integral)
    return False


def _distance_indices(me, tpath, affine, ax, scale_jitter=0.0, delta_jitter=0.0):
    """Mirror of matplotlib's float branch, returning row indices of tpath.

    The result is cross-checked against matplotlib's own output vertices.
    The jitter parameters (relative perturbations of the axes diagonal and of
    the cumulative distances) are only used by the robustness check.
    """
    start, step = (0.0, me) if isinstance(me, Real) else me
    verts = tpath.vertices
    fin = np.isfinite(verts).all(axis=1)
    kept = np.nonzero(fin)[0]
    disp = affine.transform(verts[fin])
    delta = np.empty((len(disp), 2))
    delta[0, :] = 0
    delta[1:, :] = disp[1:, :] - disp[:-1, :]
    seglen = np.hypot(*delta.T)
    if delta_jitter:  # perturb segment lengths (zero-length segments stay zero)
        seglen = seglen * (1 + delta_jitter * (np.arange(len(seglen)) % 3 - 1))
    delta = seglen.cumsum()
    (x0, y0), (x1, y1) = ax.transAxes.transform([[0, 0], [1, 1]])
    scale = np.hypot(x1 - x0, y1 - y0) * (1 + scale_jitter)
    marker_start = start * scale
    marker_step = step * scale
    md = delta - np.remainder(delta - marker_start, marker_step)
    md = np.union1d(md, md + marker_step)
    md = md[(md >= marker_start) & (md < delta[-1])]
    right = np.searchsorted(delta, md, side="left")
    left = np.maximum(right - 1, 0)
    right = np.minimum(right, len(delta) - 1)
    inds = np.where(np.abs(delta[right] - md) < np.abs(md - delta[left]), right, left)
    inds = np.searchsorted(delta, delta[inds], side="left")
    inds = np.unique(inds)
    return kept[inds]


_LAST_CAPTURE = {}


def _markevery(figsize, dpi, rect, xscale, yscale, xlim, ylim, x, y,
               drawstyle, markevery):
    me = _dec_markevery(markevery)
    fig = Figure(figsize=figsize, dpi=dpi)
    FigureCanvasAgg(fig)
    ax = fig.add_axes(rect)
    xa = np.asarray(x, dtype=float)
    ya = np.asarray(y, dtype=float)
    line, = ax.plot(xa, ya, marker="o", drawstyle=drawstyle)
    line.set_markevery(me)
    ax.set_xscale(xscale)
    ax.set_yscale(yscale)
    ax.set_xlim(*xlim)
    ax.set_ylim(*ylim)

    calls = []
    original = mlines._mark_every_path

    def spy(markevery_, tpath, affine, axes):
        out = original(markevery_, tpath, affine, axes)
        calls.append((tpath, affine, axes, out))
        return out

    with mock.patch.object(mlines, "_mark_every_path", spy), np.errstate(all="ignore"):
        fig.canvas.draw()

    n = len(xa)
    if me is None or n == 0:
        assert not calls
        return list(range(n))
    assert len(calls) == 1
    tpath, affine, axes, out = calls[0]
    _LAST_CAPTURE.update(me=me, tpath=tpath, affine=affine, axes=axes)
    if _is_distance_based(me):
        idx = _distance_indices(me, tpath, affine, axes)
    else:
        index_path = Path(np.column_stack([np.arange(n), np.arange(n)]).astype(float))
        idx = original(me, index_path, affine, axes).vertices[:, 0].astype(int)
    assert np.array_equal(tpath.vertices[idx], out.vertices, equal_nan=True)
    return [int(i) for i in idx]


# --------------------------------------------------------------------------
# MarkerStyle: registry, every marker x every fillstyle
# --------------------------------------------------------------------------

def test_class_attributes():
    res = probes.probe("markers.MarkerStyle.class_attributes", _class_attributes)
    assert res["fillstyles"] == ["full", "left", "right", "bottom", "top", "none"]
    assert len(res["markers"]) == 41


FILLED_FOR_REAL = set(MarkerStyle.filled_markers) | {","}  # pixel is filled too
HALF_FILLABLE = set(MarkerStyle.filled_markers)  # every filled marker except ','


@pytest.mark.parametrize("key", MARKER_KEYS, ids=repr)
def test_every_marker_every_fillstyle(key):
    for fs in FILLSTYLES:
        res = probes.probe("markers.MarkerStyle", _marker_style, marker=key, fillstyle=fs)
        assert res["fillstyle"] == fs
        assert res["marker"] == key
        assert res["filled"] == (fs != "none" and key in FILLED_FOR_REAL)
        has_alt = fs in MarkerStyle._half_fillstyles and key in HALF_FILLABLE
        assert (res["alt_path"] is not None) == has_alt
        assert res["capstyle"] == "butt"
        if MarkerStyle.markers[key] == "nothing":
            assert res["path"]["vertices"] == []
            assert res["transform"] == [[1, 0, 0], [0, 1, 0], [0, 0, 1]]


def test_known_marker_geometry():
    circle = probes.probe("markers.MarkerStyle", _marker_style, marker="o", fillstyle="full")
    assert len(circle["path"]["vertices"]) == 26
    assert circle["transform"][0][0] == 0.5 and circle["snap_threshold"] == float("inf")
    point = probes.probe("markers.MarkerStyle", _marker_style, marker=".", fillstyle="full")
    assert point["transform"][0][0] == 0.25
    pixel = probes.probe("markers.MarkerStyle", _marker_style, marker=",", fillstyle="full")
    assert pixel["transform"][0][2] == -0.49999
    square = probes.probe("markers.MarkerStyle", _marker_style, marker="s", fillstyle="full")
    assert square["path"]["vertices"][:4] == [[0, 0], [1, 0], [1, 1], [0, 1]]
    assert square["transform"] == [[1, 0, -0.5], [0, 1, -0.5], [0, 0, 1]]
    plus = probes.probe("markers.MarkerStyle", _marker_style, marker="+", fillstyle="full")
    assert plus["path"]["codes"] == [1, 2, 1, 2] and not plus["filled"]


def test_default_fillstyle_and_rc():
    for key in ["o", "s", "^", "x", 4]:
        res = probes.probe("markers.MarkerStyle", _marker_style, marker=key, fillstyle=None)
        assert res["fillstyle"] == "full"
    for fs in ["left", "none"]:
        for key in ["o", "D"]:
            res = probes.probe("markers.MarkerStyle", _marker_style, marker=key,
                               fillstyle=None, rc={"markers.fillstyle": fs})
            assert res["fillstyle"] == fs
    # explicit fillstyle beats the rcParam
    res = probes.probe("markers.MarkerStyle", _marker_style, marker="o",
                       fillstyle="top", rc={"markers.fillstyle": "left"})
    assert res["fillstyle"] == "top"


def test_invalid_markers_and_options():
    for bad in ["0", "q", "oo", "$", 12, -1, None, [], [5, 4], [5, 3], [5, 3, 10],
                [5, 0, 30, 1], [[1, 2, 3]], [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
                {"vertices": [], "codes": None}]:
        res = probes.probe("markers.MarkerStyle", _marker_style, marker=bad, fillstyle="full")
        assert isinstance(res, ValueError), bad
    res = probes.probe("markers.MarkerStyle", _marker_style, marker=[0, 0], fillstyle="full")
    assert isinstance(res, ZeroDivisionError)
    for kw in [{"fillstyle": "half"}, {"fillstyle": "Full"},
               {"fillstyle": "full", "capstyle": "square"},
               {"fillstyle": "full", "joinstyle": "mitre"}]:
        res = probes.probe("markers.MarkerStyle", _marker_style, marker="o", **kw)
        assert isinstance(res, ValueError), kw


# --------------------------------------------------------------------------
# tuple markers (numsides, style[, angle])
# --------------------------------------------------------------------------

@pytest.mark.parametrize("style", [0, 1, 2])
def test_tuple_markers(style):
    for numsides in [3, 4, 5, 6, 8, 17]:
        for tail in [[], [0], [30], [-45.5], [90]]:
            res = probes.probe("markers.MarkerStyle", _marker_style,
                               marker=[numsides, style, *tail], fillstyle="full")
            n_closed = numsides * (1 if style == 0 else 2) + 1
            assert len(res["path"]["vertices"]) == n_closed
            assert res["path"]["codes"][-1] == Path.CLOSEPOLY
            assert res["filled"] == (style != 2)
            assert res["joinstyle"] == ("miter" if style == 0 else "bevel")
            assert res["alt_path"] is None


def test_tuple_marker_edge_cases():
    for marker in [[1, 0], [2, 0], [4.5, 0], [-3, 0], [2, 1], [1, 2], [3, 0, 360.0],
                   [5, 1, 1e-9]]:
        probes.probe("markers.MarkerStyle", _marker_style, marker=marker, fillstyle="full")
    for marker in [[5, 0, 0], [6, 1, 15], [4, 2, 0]]:
        for fs in FILLSTYLES:
            res = probes.probe("markers.MarkerStyle", _marker_style, marker=marker, fillstyle=fs)
            assert res["alt_path"] is None  # tuple markers ignore half fills
            assert res["filled"] == (fs != "none" and marker[1] != 2)


# --------------------------------------------------------------------------
# custom Path markers and vertex-list markers
# --------------------------------------------------------------------------

PATH_MARKERS = {
    "square_closepoly": {"vertices": [[-2, -2], [2, -2], [2, 2], [-2, 2], [0, 0]],
                         "codes": [1, 2, 2, 2, 79]},
    "offcenter_triangle": {"vertices": [[1, 1], [3, 1], [2, 4]], "codes": None},
    "curves": {"vertices": [[0, 0], [1, 2], [2, 0], [3, -1], [4, 0], [5, 1.5]],
               "codes": [1, 3, 3, 4, 4, 4]},
    "large_closepoly_vertex": {"vertices": [[0, 0], [1, 0], [0, 1], [10, 10]],
                               "codes": [1, 2, 2, 79]},
    "tiny": {"vertices": [[0, 0], [0.001, 0.002], [-0.003, 0]], "codes": None},
    "two_subpaths": {"vertices": [[-1, 0], [1, 0], [0, -0.5], [0, 0.5]],
                     "codes": [1, 2, 1, 2]},
    "negative_extent": {"vertices": [[-7, 0.5], [0.25, 3], [1, -1]], "codes": [1, 2, 2]},
}


@pytest.mark.parametrize("name", list(PATH_MARKERS))
def test_path_markers(name):
    path = PATH_MARKERS[name]
    res = probes.probe("markers.MarkerStyle", _marker_style, marker=path, fillstyle="full")
    rescale = max(abs(v) for xy in path["vertices"] for v in xy)
    assert res["transform"] == [[0.5 / rescale, 0, 0], [0, 0.5 / rescale, 0], [0, 0, 1]]
    assert res["path"]["vertices"] == path["vertices"]
    assert res["marker"] == path
    assert res["joinstyle"] == "round" and res["filled"]


def test_path_marker_fillstyles_and_degenerate():
    for name in ["square_closepoly", "offcenter_triangle"]:
        for fs in FILLSTYLES:
            res = probes.probe("markers.MarkerStyle", _marker_style,
                               marker=PATH_MARKERS[name], fillstyle=fs)
            assert res["alt_path"] is None and res["filled"] == (fs != "none")
    zero = probes.probe("markers.MarkerStyle", _marker_style,
                        marker={"vertices": [[0, 0], [0, 0]], "codes": None}, fillstyle="full")
    assert zero["transform"][0][0] == float("inf")
    probes.probe("markers.MarkerStyle", _marker_style,
                 marker={"vertices": [[0, 0], [float("nan"), 1], [1, 0]], "codes": None},
                 fillstyle="full")


VERTEX_MARKERS = [
    [[-1, -1], [1, -1], [0, 2]],
    [[0, 0], [3, 0], [3, 1], [0, 1]],
    [[0, 0], [1, 1]],
    [[0.5, -0.25]],
    [[-0.1, 0.2], [0.3, -0.4], [0.05, 0.05], [-0.2, -0.2], [0.1, 0.4]],
]


def test_vertex_list_markers():
    for verts in VERTEX_MARKERS:
        res = probes.probe("markers.MarkerStyle", _marker_style, marker=verts, fillstyle="full")
        rescale = max(abs(v) for xy in verts for v in xy)
        assert res["transform"][0][0] == 0.5 / rescale
        assert res["path"]["codes"] is None and res["marker"] == verts
    for fs in FILLSTYLES:
        probes.probe("markers.MarkerStyle", _marker_style, marker=VERTEX_MARKERS[0], fillstyle=fs)


# --------------------------------------------------------------------------
# user transform, capstyle, joinstyle
# --------------------------------------------------------------------------

TRANSFORMS = [
    [["rotate_deg", 30]],
    [["scale", 2, 0.5], ["translate", 0.25, -1]],
    [["skew_deg", 10, 0]],
    [["rotate", 0.5], ["scale", 3]],
    [],
]


def test_user_transform():
    cases = [("o", "full"), ("^", "left"), ("s", "bottom"), ([5, 1, 10], "full"),
             (PATH_MARKERS["offcenter_triangle"], "full"), ("+", "full"), ("x", "left"),
             ("o", "top"), (6, "full")]
    for marker, fs in cases:
        for ops in TRANSFORMS:
            res = probes.probe("markers.MarkerStyle", _marker_style, marker=marker,
                               fillstyle=fs, transform=ops)
            assert res["user_transform"] is not None
            plain = _marker_style(marker, fs)
            user = np.array(res["user_transform"])
            np.testing.assert_allclose(res["transform"],
                                       user @ np.array(plain["transform"]), atol=1e-12)
    res = probes.probe("markers.MarkerStyle", _marker_style, marker="o", fillstyle="full",
                       transform=None)
    assert res["user_transform"] is None


def test_explicit_cap_and_join_full_description():
    for marker in ["o", "s", "^", "*", "+", [5, 2, 0], [4, 0, 45], 7]:
        for cap, join in [("round", "bevel"), ("projecting", "miter"), ("butt", "round")]:
            res = probes.probe("markers.MarkerStyle", _marker_style, marker=marker,
                               fillstyle="full", capstyle=cap, joinstyle=join)
            assert res["capstyle"] == cap


# Markers whose set-up honours a user joinstyle (others stay "round").
JOIN_HONOURED = {"v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X",
                 4, 5, 6, 7, 8, 9, 10, 11}


@pytest.mark.parametrize(
    "marker",
    MARKER_KEYS + [[5, 0], [5, 1], [5, 2], PATH_MARKERS["offcenter_triangle"]],
    ids=repr,
)
def test_cap_join_sweep(marker):
    for join in JOINSTYLES:
        res = probes.probe("markers.MarkerStyle.styles", _marker_styles, marker=marker,
                           capstyle=None, joinstyle=join)
        honoured = isinstance(marker, list) or (
            not isinstance(marker, dict) and marker in JOIN_HONOURED)
        if honoured:
            assert res["joinstyle"] == join
        else:
            assert res["joinstyle"] == "round"
        assert res["capstyle"] == "butt"
    for cap in CAPSTYLES:
        res = probes.probe("markers.MarkerStyle.styles", _marker_styles, marker=marker,
                           capstyle=cap, joinstyle=None)
        assert res["capstyle"] == cap


# --------------------------------------------------------------------------
# derived markers: rotated / scaled / transformed
# --------------------------------------------------------------------------

DERIVED = [
    ("o", "full", None, [["rotated", {"deg": 45}]]),
    ("s", "full", None, [["rotated", {"rad": 0.3}]]),
    ("^", "left", None, [["rotated", {"deg": 90}], ["scaled", 2]]),
    ("D", "full", None, [["scaled", 2]]),
    ("D", "top", None, [["scaled", 2, 0.5]]),
    ("P", "right", None, [["transformed", [["translate", 1, 0]]]]),
    ("*", "full", [["rotate_deg", 10]], [["rotated", {"deg": 20}]]),
    ("*", "full", [["rotate_deg", 10]], [["scaled", 1.5, 3]]),
    ("h", "bottom", [["scale", 2]], [["transformed", [["rotate_deg", 30]]]]),
    ("x", "full", None, [["scaled", 0]]),
    ([6, 1, 0], "full", None, [["rotated", {"deg": 15}], ["rotated", {"deg": 15}]]),
    ("8", "full", None, [["transformed", [["skew_deg", 5, 5]]], ["rotated", {"deg": 10}]]),
    ("v", "full", None, [["scaled", -1, 1], ["transformed", [["translate", 0, 0.5]]]]),
    ("o", "full", None, []),
    # errors
    ("o", "full", None, [["rotated", {}]]),
    ("o", "full", None, [["rotated", {"deg": 10, "rad": 0.1}]]),
    ("s", "full", [["rotate_deg", 10]], [["transformed", [["scale", 2]]], ["rotated", {"deg": 5}]]),
    ("s", "full", [["rotate_deg", 10]], [["transformed", [["scale", 2]]], ["scaled", 2]]),
]


def test_derived_markers():
    for marker, fs, ops, steps in DERIVED:
        res = probes.probe("markers.MarkerStyle.derived", _marker_derived, marker=marker,
                           fillstyle=fs, transform=ops, steps=steps)
        if steps and steps[-1] == ["rotated", {}] or steps == [["rotated", {"deg": 10, "rad": 0.1}]]:
            assert isinstance(res, ValueError)
        elif len(steps) == 2 and steps[0][0] == "transformed" and ops is not None:
            assert isinstance(res, AttributeError)
        else:
            assert isinstance(res, dict)
            assert res["fillstyle"] == fs and res["marker"] == marker
    rot = _marker_derived("o", "full", None, [["rotated", {"deg": 45}]])
    np.testing.assert_allclose(
        rot["user_transform"], Affine2D().rotate_deg(45).get_matrix(), atol=0)


# --------------------------------------------------------------------------
# drawstyles and step conversion
# --------------------------------------------------------------------------

STEP_CASES = [
    ([0, 1, 2, 3], [[0, 1, 4, 9]]),
    ([0, 1, 2, 3], [[0, 1, 4, 9], [5, 4, 3, 2]]),
    ([0, 1, 2, 3], [[0, 1, 4, 9], [5, 4, 3, 2], [-1, -1, -2, -2]]),
    ([0.5, 1.5, 4.0, 4.25, 10.0], [[1.0, -2.0, 0.5, 3.0, 3.0]]),
    ([3, 2, 1], [[1, 2, 3]]),
    ([1, 2, float("nan"), 4], [[1, float("nan"), 3, 4]]),
    ([0, 1, float("inf")], [[1, 2, 3]]),
    ([5], [[7]]),
    ([5], [[7], [8]]),
    ([], [[]]),
    ([], [[], []]),
    ([], []),
    ([1, 2, 3], []),
    ([1, 2, 3], [[4]]),
    ([1, 2, 3], [[1, 2]]),
    ([1, 2], [[1, 2, 3]]),
]


@pytest.mark.parametrize("name", ["pts_to_prestep", "pts_to_poststep", "pts_to_midstep"])
def test_step_functions(name):
    fn = getattr(cbook, name)
    for x, ys in STEP_CASES:
        res = probes.probe(f"cbook.{name}", _step_fn(fn), x=x, ys=ys)
        if isinstance(res, Exception):
            continue
        n = len(x)
        assert res.shape[0] == 1 + len(ys)
        assert res.shape[1] == (2 * n if name == "pts_to_midstep" else max(2 * n - 1, 0))
    res = probes.probe(f"cbook.{name}", _step_fn(fn), x=[0, 1, 2], ys=[[10, 20, 30]])
    expected = {
        "pts_to_prestep": [[0, 0, 1, 1, 2], [10, 20, 20, 30, 30]],
        "pts_to_poststep": [[0, 1, 1, 2, 2], [10, 10, 20, 20, 30]],
        "pts_to_midstep": [[0, 0.5, 0.5, 1.5, 1.5, 2], [10, 10, 20, 20, 30, 30]],
    }[name]
    assert res.tolist() == expected


def test_drawstyle_mapping():
    names = probes.probe("lines.drawstyle_names", _drawstyle_names)
    assert names["drawStyleKeys"] == ["default", "steps-mid", "steps-pre", "steps-post", "steps"]
    for ds in ["default", "steps", "steps-pre", "steps-post", "steps-mid", "step", "steps-", ""]:
        res = probes.probe("lines.drawstyle", _drawstyle, drawstyle=ds)
        if ds in names["drawStyleKeys"]:
            assert res["step_function"] == {
                "default": "identity", "steps": "pts_to_prestep", "steps-pre": "pts_to_prestep",
                "steps-post": "pts_to_poststep", "steps-mid": "pts_to_midstep"}[ds]
        else:
            assert isinstance(res, KeyError)
    for ds in ["default", "steps", "steps-pre", "steps-post", "steps-mid", None,
               "step", "Steps-pre", "mid"]:
        res = probes.probe("lines.Line2D.set_drawstyle", _set_drawstyle, drawstyle=ds)
        if ds is None:
            assert res == "default"
        elif ds in names["drawStyleKeys"]:
            assert res == ds
        else:
            assert isinstance(res, ValueError)


LINE_PATH_CASES = [
    ([0, 1, 2, 3], [0, 1, 4, 9]),
    ([0, 1, 2], [1, float("nan"), 3]),
    ([0.0, 0.5, 3.0], [2.0, -1.0, 2.5]),
    ([0], [1, 2, 3]),
    ([4], [5]),
    ([], []),
    ([0, 1], [1, 2, 3]),
]


def test_line_get_path():
    for ds in ["default", "steps", "steps-pre", "steps-post", "steps-mid"]:
        for x, y in LINE_PATH_CASES:
            res = probes.probe("lines.Line2D.get_path", _line_path, x=x, y=y, drawstyle=ds)
            if len(x) not in (1, len(y)):
                assert isinstance(res, ValueError)
            else:
                assert res["codes"] is None


# --------------------------------------------------------------------------
# markevery
# --------------------------------------------------------------------------

def _assert_robust_selection(result):
    """Float markevery selections must not hinge on exact ties.

    Re-runs the selection with ~1e-10 relative perturbations of the display
    segment lengths and of the axes diagonal; a probe whose result changes would
    depend on last-ulp arithmetic and is therefore kept out of the spec.
    """
    cap = _LAST_CAPTURE
    for kw in ({"scale_jitter": 1e-10}, {"scale_jitter": -1e-10},
               {"delta_jitter": 1e-10}, {"delta_jitter": -1e-10}):
        jittered = _distance_indices(cap["me"], cap["tpath"], cap["affine"], cap["axes"], **kw)
        assert jittered.tolist() == result, (kw, result, jittered.tolist())


GEOM_A = {"figsize": [4.0, 3.0], "dpi": 100, "rect": [0.125, 0.1, 0.75, 0.8]}
GEOM_B = {"figsize": [6.4, 4.8], "dpi": 72, "rect": [0.1, 0.2, 0.5, 0.6]}

_x_uniform = np.linspace(0, 10, 21).tolist()
_x_uneven = [0.0, 0.1, 0.2, 0.5, 1.0, 2.0, 3.5, 5.0, 5.2, 5.4, 7.0, 9.5, 10.0]
_x13 = np.linspace(0, 10, 13).tolist()

DATASETS = {
    "uniform_line": dict(x=_x_uniform, y=[0.6 * v + 2.0 for v in _x_uniform],
                         xlim=[0, 10], ylim=[0, 10], xscale="linear", yscale="linear"),
    "uneven_sine": dict(x=_x_uneven, y=(np.sin(np.array(_x_uneven)) * 4 + 5).tolist(),
                        xlim=[0, 10], ylim=[0, 10], xscale="linear", yscale="linear"),
    "nan_gaps": dict(x=_x13, y=[1, 2, 3, float("nan"), 5, 6, 7, float("nan"), float("nan"),
                                8, 7, 6, 5],
                     xlim=[0, 10], ylim=[0, 10], xscale="linear", yscale="linear"),
    "repeated_points": dict(x=[0, 1, 1, 1, 2, 3, 2, 1, 1, 4, 5],
                            y=[0, 0.7, 0.7, 0.7, 2.1, 2.6, 2.1, 0.7, 0.7, 3.3, 4.6],
                            xlim=[0, 5], ylim=[0, 5], xscale="linear", yscale="linear"),
    "logx": dict(x=np.logspace(-1, 2, 16).tolist(), y=(np.linspace(0, 1, 16) ** 2).tolist(),
                 xlim=[0.1, 100], ylim=[0, 1], xscale="log", yscale="linear"),
    "loglog": dict(x=np.logspace(0, 3, 12).tolist(),
                   y=(10 ** (3 * (1 - np.linspace(0, 1, 12)) ** 1.5)).tolist(),
                   xlim=[1, 1000], ylim=[1, 1000], xscale="log", yscale="log"),
    "log_nonpositive": dict(x=[1, -2, 3, 4, 5, 0, 7], y=[1, 2, 3, 4, 5, 6, 7],
                            xlim=[0.5, 10], ylim=[0, 8], xscale="log", yscale="linear"),
    "reversed_xlim": dict(x=_x_uneven, y=(np.cos(np.array(_x_uneven)) * 3 + 5).tolist(),
                          xlim=[10, 0], ylim=[0, 10], xscale="linear", yscale="linear"),
    "zoomed": dict(x=_x_uniform, y=[v * v / 10 for v in _x_uniform], xlim=[2, 4], ylim=[0, 2],
                   xscale="linear", yscale="linear"),
    "with_inf": dict(x=[0, 1, 2, 3, 4, 5], y=[0, 1.3, float("inf"), 2.2, 4.1, 3.6],
                     xlim=[0, 5], ylim=[0, 5], xscale="linear", yscale="linear"),
    "two_points": dict(x=[1.0, 4.0], y=[1.0, 3.0], xlim=[0, 5], ylim=[0, 5],
                       xscale="linear", yscale="linear"),
    "single_point": dict(x=[2.0], y=[3.0], xlim=[0, 5], ylim=[0, 5],
                         xscale="linear", yscale="linear"),
    "all_nan": dict(x=[float("nan")] * 3, y=[0, 1, 2], xlim=[0, 5], ylim=[0, 5],
                    xscale="linear", yscale="linear"),
    "empty": dict(x=[], y=[], xlim=[0, 5], ylim=[0, 5], xscale="linear", yscale="linear"),
}

INDEX_MARKEVERY = [
    {"kind": "none"},
    {"kind": "bool", "value": True},
    {"kind": "bool", "value": False},
    {"kind": "int", "value": 1},
    {"kind": "int", "value": 2},
    {"kind": "int", "value": 3},
    {"kind": "int", "value": 5},
    {"kind": "int", "value": 100},
    {"kind": "int", "value": 0},
    {"kind": "int", "value": -1},
    {"kind": "int", "value": -2},
    {"kind": "start_step", "value": [0, 3]},
    {"kind": "start_step", "value": [2, 3]},
    {"kind": "start_step", "value": [1, -1]},
    {"kind": "start_step", "value": [-3, 1]},
    {"kind": "start_step", "value": [20, 2]},
    {"kind": "start_step", "value": [1, 0]},
    {"kind": "slice", "start": None, "stop": None, "step": None},
    {"kind": "slice", "start": 1, "stop": None, "step": 3},
    {"kind": "slice", "start": None, "stop": 5, "step": None},
    {"kind": "slice", "start": -4, "stop": None, "step": None},
    {"kind": "slice", "start": None, "stop": None, "step": -3},
    {"kind": "slice", "start": 2, "stop": 8, "step": 2},
    {"kind": "slice", "start": 0, "stop": 0, "step": 1},
    {"kind": "indices", "value": [0, 2, 5]},
    {"kind": "indices", "value": [5, 2, 0]},
    {"kind": "indices", "value": [1, 1, 1]},
    {"kind": "indices", "value": [-1, 0]},
    {"kind": "indices", "value": []},
    {"kind": "indices", "value": [0, 100]},
    {"kind": "mask", "value": [True, False, False, True, True, False, False, False, True, True,
                               False, False, True]},
    {"kind": "mask", "value": [False] * 13},
    {"kind": "mask", "value": [True, False]},
    {"kind": "tuple", "value": [1, 2, 3], "types": ["int", "int", "int"]},
    {"kind": "tuple", "value": [0.5, 2], "types": ["float", "int"]},
    {"kind": "tuple", "value": [1], "types": ["int"]},
]


def test_markevery_index_based():
    data = DATASETS["nan_gaps"]
    n = len(data["x"])
    for me in INDEX_MARKEVERY:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **data,
                           drawstyle="default", markevery=me)
        kind = me["kind"]
        if kind == "none" or me == {"kind": "bool", "value": True} or me == {"kind": "int", "value": 1}:
            assert res == list(range(n))
        elif kind == "int" and me["value"] > 0:
            assert res == list(range(0, n, me["value"]))
        elif me in ({"kind": "bool", "value": False}, {"kind": "int", "value": 0},
                    {"kind": "start_step", "value": [1, 0]}) or kind == "tuple":
            assert isinstance(res, ValueError)
        elif kind == "indices" and 100 in me["value"]:
            assert isinstance(res, ValueError)
        elif kind == "mask" and len(me["value"]) != n:
            assert isinstance(res, ValueError)
    # markers ignore the drawstyle
    for ds in ["steps-pre", "steps-mid"]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **data, drawstyle=ds,
                           markevery={"kind": "int", "value": 3})
        assert res == [0, 3, 6, 9, 12]


FLOAT_MARKEVERY = [
    {"kind": "float", "value": 0.05},
    {"kind": "float", "value": 0.1},
    {"kind": "float", "value": 0.25},
    {"kind": "float", "value": 1.0},
    {"kind": "float", "value": 2.0},
    {"kind": "float_pair", "value": [0.0, 0.1]},
    {"kind": "float_pair", "value": [0.05, 0.1]},
    {"kind": "float_pair", "value": [0.5, 0.15]},
    {"kind": "float_pair", "value": [3.0, 0.1]},
    {"kind": "float_pair", "value": [-0.1, 0.2]},
    {"kind": "tuple", "value": [0, 0.1], "types": ["int", "float"]},
]


DISTANCE_DATASETS = [k for k in DATASETS
                     if k not in ("all_nan", "empty", "single_point", "two_points")]


@pytest.mark.parametrize("dataset", DISTANCE_DATASETS)
def test_markevery_distance_based(dataset):
    data = DATASETS[dataset]
    for geom in (GEOM_A, GEOM_B):
        for me in FLOAT_MARKEVERY:
            res = probes.probe("lines.markevery", _markevery, **geom, **data,
                               drawstyle="default", markevery=me)
            assert isinstance(res, list)
            _assert_robust_selection(res)
            assert res == sorted(set(res))
            if me["value"] == [3.0, 0.1] and dataset == "uniform_line":
                assert res == []  # start offset beyond the line length
    # int-start + float-step equals float-pair
    a = _markevery(**GEOM_A, **data, drawstyle="default",
                   markevery={"kind": "tuple", "value": [0, 0.1], "types": ["int", "float"]})
    b = _markevery(**GEOM_A, **data, drawstyle="default",
                   markevery={"kind": "float", "value": 0.1})
    assert a == b


def test_markevery_distance_edge_cases():
    uniform = DATASETS["uniform_line"]
    for me in [{"kind": "float", "value": 0.0}, {"kind": "float", "value": -0.1},
               {"kind": "float_pair", "value": [0.1, 0.0]}]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **uniform,
                           drawstyle="default", markevery=me)
        assert isinstance(res, ValueError)
    for ds in ["steps-post", "steps-mid"]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **uniform, drawstyle=ds,
                           markevery={"kind": "float", "value": 0.1})
        _assert_robust_selection(res)
        assert res == _markevery(**GEOM_A, **uniform, drawstyle="default",
                                 markevery={"kind": "float", "value": 0.1})
    # Two points, display length ~0.53 axes diagonals (GEOM_A): candidates at
    # or beyond the end are dropped, otherwise the nearer vertex wins.
    two = DATASETS["two_points"]
    expected = {0.2: [0, 1], 0.45: [0, 1], 0.6: [0], 1.0: [0], (0.3, 1.0): [1],
                (0.6, 1.0): [], (0.1, 0.15): [0, 1], (0.2, 1.0): [0]}
    for key, want in expected.items():
        me = ({"kind": "float_pair", "value": list(key)} if isinstance(key, tuple)
              else {"kind": "float", "value": key})
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **two,
                           drawstyle="default", markevery=me)
        _assert_robust_selection(res)
        assert res == want, (key, res)
    single = DATASETS["single_point"]
    for me in [{"kind": "float", "value": 0.1}, {"kind": "int", "value": 2}, {"kind": "none"},
               {"kind": "float_pair", "value": [0.0, 0.5]}]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **single,
                           drawstyle="default", markevery=me)
    assert _markevery(**GEOM_A, **single, drawstyle="default",
                      markevery={"kind": "float", "value": 0.1}) == []
    all_nan = DATASETS["all_nan"]
    for me in [{"kind": "float", "value": 0.1}, {"kind": "int", "value": 2}, {"kind": "none"}]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **all_nan,
                           drawstyle="default", markevery=me)
    assert isinstance(_markevery_safe(all_nan, {"kind": "float", "value": 0.1}), IndexError)
    empty = DATASETS["empty"]
    for me in [{"kind": "float", "value": 0.1}, {"kind": "int", "value": 2}, {"kind": "none"},
               {"kind": "indices", "value": [0, 5]}]:
        res = probes.probe("lines.markevery", _markevery, **GEOM_A, **empty,
                           drawstyle="default", markevery=me)
        assert res == []


def _markevery_safe(data, me):
    try:
        return _markevery(**GEOM_A, **data, drawstyle="default", markevery=me)
    except Exception as exc:  # noqa: BLE001
        return exc


# --- format strings (``_process_plot_format``)

FMT_TOKENS = ["-", "--", "-.", ":", " ", "o", ".", ",", "v", "^", "<", ">", "1", "2", "3", "4",
              "8", "s", "p", "P", "*", "h", "H", "+", "x", "X", "D", "d", "|", "_", "b", "g", "r",
              "c", "m", "y", "k", "w", "C0", "C7", "C10"]
FMT_EXTRA = ["", "0", "1", "1.0", "0.5", "red", "tab:blue", "#ff0000", "#f00", "C", "Cx", "rr",
             "--:", "oo", "o-o", "z", "ro--", "--ro", "C1o", "o C1", "k^:", "r o", "-.-", "None",
             "none", "CC", "C1C2", "r2", "2r", "xkcd:sky blue", "C1-"]


def _linestyle_canon(ls):
    if ls is None:
        return None
    return {"-": "solid", "--": "dashed", "-.": "dashdot", ":": "dotted"}.get(ls, "none")


def _plot_format(fmt):
    from matplotlib import colors as mcolors
    from matplotlib.axes._base import _process_plot_format
    linestyle, marker, color = _process_plot_format(fmt)
    marker = None if marker is None else ("None" if marker in ("None", "none", " ", "")
                                          else marker)
    color = None if color is None else [float(c) for c in mcolors.to_rgba(color)]
    return [_linestyle_canon(linestyle), marker, color]


def test_plot_formats():
    pairs = [a + b for a in FMT_TOKENS for b in FMT_TOKENS[::3]]
    for fmt in FMT_TOKENS + FMT_EXTRA + pairs:
        probes.probe("axes._process_plot_format", _plot_format, fmt=fmt)
