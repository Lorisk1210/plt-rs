"""Behaviour of ``Line2D`` artists, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_lines.py``.

Area ``line_behaviour``. Unless noted, a scene draws on ``Figure()`` (6.4 x
4.8 in at 100 dpi) with an Agg canvas and ``ax = fig.add_subplot()``;
bounding boxes are ``[[x0, y0], [x1, y1]]`` in display pixels.

Ops
---
``line.contains``  {scene, points, coords} -> {gid: [ind | null, ...]}
    ``line.contains(event)`` for an event at each point of ``points``
    (display pixels, or data coordinates of ``ax`` when ``coords`` is
    ``"data"``), per line of the scene: the indices hit, or ``null`` for a
    miss.

    ``segment_hits``  ``Figure(figsize=(10, 15), dpi=72)`` holding a line
        from ``(553, 95)`` to ``(553, 947)`` in display coordinates
        (``transform=IdentityTransform()``) with ``pickradius=6.94``
        (``test_segment_hits``: at 72 dpi a point is a pixel).
    ``picking``  three ``ax.plot([0, 1], [0, 1], picker=True)`` lines: one
        as is (pick radius 5), one with ``pickradius=20`` and one given
        ``set_pickradius(20)`` afterwards (``test_picking``).
    ``markers_only``  ``ax.plot([0.2, 0.5, 0.8], [0.3, 0.6, 0.4], "o",
        linestyle="none")``: points within the pick radius of a vertex.
    ``steps``  ``ax.plot([0, 1, 2, 3], [0, 1, 0, 1], drawstyle="steps-mid")``
        with ``xlim=(-1, 4)``: segment indices halved, as matplotlib does.

``line.window_extent``  {scene} -> [bbox, ...]
    ``get_window_extent()`` of each line of the scene after a draw: the
    display bbox of the line's data points, padded by half the marker size
    when there is a marker; null when no point is finite.

    ``plain``  ``ax.plot([0, 1, 2], [0, 2, 1])`` and the same with ``"o"``,
        ``"s"`` at ``markersize=12`` and ``drawstyle="steps-post"``.
    ``nonfinite``  data with a NaN, an infinity and only NaNs.
    ``axlines``  ``ax.axline((0.2, 0.3), (0.6, 0.9))``, ``ax.axline((0.5,
        0.5), slope=-2)``, ``ax.axline((0.4, 0), slope=inf)`` and a
        horizontal one, over ``xlim=(0, 1), ylim=(0, 2)``: the part of the
        line inside the axes.
    ``figure_line``  ``fig.add_artist(Line2D([0.1, 0.9], [0.2, 0.8],
        marker="o"))`` in figure coordinates.

``line.axline_small_slope``  {} -> {"rises", "below_4e-12"}
    For ``ax.axline((0, 0), slope=1e-14)``: whether the drawn line rises
    from its first to its last point, by less than 4e-12 pixels
    (``test_axline_small_slope``; the height of its window extent).

``line.tightbbox``  {scene} -> bbox
    ``fig.get_tightbbox()`` in inches: ``unclipped`` has a line with
    ``clip_on=False`` reaching outside the axes, and markers;
    ``clipped`` the same line clipped (it does not count).

``line.draw``  {scene} -> ``"drawn"`` or ``"error"``
    Whether the figure draws:

    ``zero_width_dashed``  ``ax.plot([0, 1], [0, 1], ls="--", lw=0)``
        (``test_zero_linewidth_dashed_uses_solid_gc_dashes``).
    ``figure_markevery_float``  a figure-level line with ``marker="o",
        markevery=0.5`` (``test_markevery_figure_line_unsupported_relsize``:
        float markevery needs an axes).
    ``extreme_zoom``  ``ax.plot(x, sin(x), marker="o", markevery=0.05)``
        viewed over ``xlim=(5, 5.000001)`` (``test_markevery_extreme_zoom``).
    ``color_variants``  lines coloured ``"none"``, ``"r"``, ``".3"``, ``(1,
        0, 0, 1)`` and ``(1, 0, 0)`` (``test_line_colors``).
    ``nan_data``  ``ax.plot([nan] * 2000, range(2000))``
        (``test_is_sorted_and_has_non_nan``).

Left out: ``test_invalid_line_data``, ``test_valid_colors``,
``test_valid_linestyles``, ``test_valid_drawstyles`` (Python type and
string validation; plt-rs takes typed values and the parsers are covered
by the ``rcsetup`` and ``markers`` areas), ``test_linestyle_variants``
(tuple line styles are a Python spelling), ``test_set_drawstyle`` and the
markevery tests (covered by the ``markers`` area),
``test_axline_setters`` (plt-rs fixes an infinite line's second point or
slope when it is made, so the setters and their conflicts do not exist),
``test_marker_as_markerstyle`` and ``test_markerfacecolor_fillstyle``
(Python getters returning what was set), ``test_set_line_coll_dash``
(contour line styles, ``contour_behaviour``).
"""

from types import SimpleNamespace

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.lines import Line2D
from matplotlib.transforms import IdentityTransform

from spec_support import ProbeSet

probes = ProbeSet("line_behaviour")


def _figure(**kw):
    fig = Figure(**kw)
    FigureCanvasAgg(fig)
    return fig


def _bbox(bbox):
    return bbox.get_points().tolist()


def _segment_hits():
    fig = _figure(figsize=(10, 15), dpi=72)
    line = Line2D([553.0, 553.0], [95.0, 947.0], pickradius=6.94,
                  transform=IdentityTransform())
    fig.add_artist(line)
    return fig, {"line": line}


def _picking():
    fig = _figure()
    ax = fig.add_subplot()
    default, = ax.plot([0, 1], [0, 1], picker=True)
    wide, = ax.plot([0, 1], [0, 1], picker=True, pickradius=20)
    later, = ax.plot([0, 1], [0, 1], picker=True)
    later.set_pickradius(20)
    return fig, {"default": default, "wide": wide, "later": later}


def _markers_only():
    fig = _figure()
    ax = fig.add_subplot()
    line, = ax.plot([0.2, 0.5, 0.8], [0.3, 0.6, 0.4], "o", linestyle="none")
    return fig, {"markers": line}


def _steps():
    fig = _figure()
    ax = fig.add_subplot()
    line, = ax.plot([0, 1, 2, 3], [0, 1, 0, 1], drawstyle="steps-mid")
    ax.set_xlim(-1, 4)
    return fig, {"steps": line}


CONTAINS_SCENES = {
    "segment_hits": _segment_hits,
    "picking": _picking,
    "markers_only": _markers_only,
    "steps": _steps,
}


def _contains(scene, points, coords):
    fig, lines = CONTAINS_SCENES[scene]()
    fig.canvas.draw()
    if coords == "data":
        points = fig.axes[0].transData.transform(points)
    hits = {}
    for gid, line in lines.items():
        hits[gid] = []
        for x, y in points:
            found, info = line.contains(SimpleNamespace(x=x, y=y))
            hits[gid].append([int(i) for i in info["ind"]] if found else None)
    return hits


def _plain(fig):
    ax = fig.add_subplot()
    x, y = [0, 1, 2], [0, 2, 1]
    return [ax.plot(x, y)[0], ax.plot(x, y, "o")[0], ax.plot(x, y, "s", markersize=12)[0],
            ax.plot(x, y, drawstyle="steps-post")[0]]


def _nonfinite(fig):
    ax = fig.add_subplot()
    return [ax.plot([0, 1, np.nan, 3], [1, 0, 2, 2])[0],
            ax.plot([0, 1, 2], [1, np.inf, 2], marker="o")[0],
            ax.plot([np.nan, np.nan], [1, 2])[0]]


def _axlines(fig):
    ax = fig.add_subplot()
    lines = [ax.axline((0.2, 0.3), (0.6, 0.9)), ax.axline((0.5, 0.5), slope=-2),
             ax.axline((0.4, 0), slope=np.inf), ax.axline((0, 1.5), slope=0)]
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 2)
    return lines


def _figure_line(fig):
    return [fig.add_artist(Line2D([0.1, 0.9], [0.2, 0.8], marker="o"))]


EXTENT_SCENES = {
    "plain": _plain,
    "nonfinite": _nonfinite,
    "axlines": _axlines,
    "figure_line": _figure_line,
}


def _window_extent(scene):
    fig = _figure()
    lines = EXTENT_SCENES[scene](fig)
    fig.canvas.draw()
    return [_bbox(line.get_window_extent()) for line in lines]


def _axline_small_slope():
    fig = _figure()
    line = fig.add_subplot().axline((0, 0), slope=1e-14)
    fig.canvas.draw()
    p1 = line.get_transform().transform_point((0, 0))
    p2 = line.get_transform().transform_point((1, 1))
    dy = p2[1] - p1[1]
    return {"rises": bool(dy > 0), "below_4e-12": bool(dy < 4e-12)}


def _tightbbox(scene):
    fig = _figure()
    ax = fig.add_subplot()
    ax.plot([-0.5, 0.5, 1.5], [0.5, 1.4, 0.5], marker="o", markersize=20,
            clip_on=scene == "clipped")
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    return _bbox(fig.get_tightbbox(fig.canvas.get_renderer()))


def _zero_width_dashed(fig):
    fig.add_subplot().plot([0, 1], [0, 1], ls="--", lw=0)


def _figure_markevery_float(fig):
    fig.add_artist(Line2D([0, 1], [0, 1], marker="o", markevery=.5))


def _extreme_zoom(fig):
    ax = fig.add_subplot()
    x = np.linspace(0, 10, 200)
    ax.plot(x, np.sin(x), marker="o", markevery=0.05)
    ax.set_xlim(5.000000, 5.000001)


def _color_variants(fig):
    ax = fig.add_subplot()
    for color in ["none", "r", ".3", (1, 0, 0, 1), (1, 0, 0)]:
        ax.plot(range(10), color=color)


def _nan_data(fig):
    fig.add_subplot().plot([np.nan] * 2000, range(2000))


DRAW_SCENES = {
    "zero_width_dashed": _zero_width_dashed,
    "figure_markevery_float": _figure_markevery_float,
    "extreme_zoom": _extreme_zoom,
    "color_variants": _color_variants,
    "nan_data": _nan_data,
}


def _draw(scene):
    fig = _figure()
    DRAW_SCENES[scene](fig)
    try:
        fig.canvas.draw()
    except ValueError:
        return "error"
    return "drawn"


CONTAINS_POINTS = {
    "segment_hits": ("display", [[553, 902], [559.9, 500], [560, 500], [553, 89], [553, 88],
                                 [553, 953], [553, 954], [546.1, 95]]),
    "picking": ("display", [[320, 255], [320, 240], [320, 237], [300, 250], [338, 250],
                            [80, 53], [576, 422], [600, 445], [70, 53]]),
    "markers_only": ("data", [[0.2, 0.3], [0.21, 0.31], [0.22, 0.3], [0.5, 0.62],
                              [0.5, 0.63], [0.35, 0.45], [0.8, 0.41]]),
    "steps": ("data", [[0, 0], [0.4, 0], [0.5, 0.5], [0.52, 0.5], [1.5, 0.5], [1.5, 1],
                       [2.5, 0.02], [3, 1], [3.2, 1]]),
}


def test_contains():
    for scene, (coords, points) in CONTAINS_POINTS.items():
        probes.probe("line.contains", _contains, scene=scene, points=points, coords=coords)


def test_window_extents():
    for scene in EXTENT_SCENES:
        probes.probe("line.window_extent", _window_extent, scene=scene)
    probes.probe("line.axline_small_slope", _axline_small_slope)
    for scene in ["unclipped", "clipped"]:
        probes.probe("line.tightbbox", _tightbbox, scene=scene)


def test_draws():
    for scene in DRAW_SCENES:
        probes.probe("line.draw", _draw, scene=scene)
