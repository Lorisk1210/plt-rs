"""Navigating 3-D axes with the mouse, and their aspect
(``mpl_toolkits.mplot3d``).

Area ``axes3d``. Every op draws ``Figure(figsize=(4, 3), dpi=100)`` with
``add_subplot(projection="3d")`` after applying ``view`` (``[elev, azim,
roll]``), ``scales`` (``{"x": "log", ...}``, matplotlib 3.11's non-linear
3-D scales) and ``limits`` (``[[x0, x1], [y0, y1], [z0, z1]]``) when
given.

Ops
---
``axes3d.rotate``       {view?, style, start, end, control?} -> [elev, azim, roll]
    ``_on_move`` with button 1 from ``start`` to ``end`` (projected
    coordinates, matplotlib's ``event.xdata``/``ydata``) under
    ``axes3d.mouserotationstyle = style``; ``control`` holds the control
    key (snapping to ``axes3d.snap_rotation``).
``axes3d.zoom``         {limits?, start, end} -> [[x0, x1], [y0, y1], [z0, z1]]
    ``_on_move`` with button 3 (dragging down zooms in).
``axes3d.pan``          {view?, limits?, start, end} -> limits
    ``_on_move`` with button 2 (``event.x``/``y`` from ``transData``).
``axes3d.zoom_box``     {view?, limits?, box, direction, key?} -> limits
    ``_set_view_from_bbox(box, direction, key)``, ``box`` = ``[x0, y0, x1,
    y1]`` in display pixels, as the toolbar's zoom releases it.
``axes3d.format_coord`` {view?, limits?, pixel, rotating?} -> str
    ``format_coord`` at the projected point under display ``pixel``;
    ``rotating`` presses button 1 first (the view angles are shown).
``axes3d.set_aspect``   {limits?, aspect, adjustable, then?} -> {limits, box_aspect}
    ``set_aspect(aspect, adjustable=adjustable)``, then ``set_aspect(then,
    adjustable=adjustable)`` when given; the limits and ``_box_aspect``.
``axes3d.zoom_data``    {view?, limits?, aspect?, scale} -> limits
    ``_zoom_data_limits(*scale)`` after ``set_aspect(aspect)`` (box).

``zoom``, ``pan``, ``zoom_box`` and ``format_coord`` also take ``scales``.
"""

from types import SimpleNamespace

import matplotlib as mpl
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("axes3d")


def _axes(view=None, limits=None, scales=None):
    fig = Figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot(projection="3d")
    if view is not None:
        ax.view_init(*view)
    for name, scale in (scales or {}).items():
        getattr(ax, f"set_{name}scale")(scale)
    if limits is not None:
        ax.set_xlim(*limits[0])
        ax.set_ylim(*limits[1])
        ax.set_zlim(*limits[2])
    fig.draw_without_rendering()
    return ax


def _limits(ax):
    return [list(ax.get_xlim()), list(ax.get_ylim()), list(ax.get_zlim())]


def _move(ax, button, start, end, key=None):
    ax.button_pressed = button
    ax._sx, ax._sy = start
    x, y = ax.transData.transform(end)
    ax._on_move(SimpleNamespace(xdata=end[0], ydata=end[1], x=x, y=y, inaxes=ax, key=key))


def _rotate(style, start, end, view=None, control=False):
    with mpl.rc_context({"axes3d.mouserotationstyle": style}):
        ax = _axes(view)
        _move(ax, 1, start, end, "control" if control else None)
        return [ax.elev, ax.azim, ax.roll]


def _zoom(start, end, limits=None, scales=None):
    ax = _axes(limits=limits, scales=scales)
    _move(ax, 3, start, end)
    return _limits(ax)


def _pan(start, end, view=None, limits=None, scales=None):
    ax = _axes(view, limits, scales)
    _move(ax, 2, start, end)
    return _limits(ax)


def _zoom_box(box, direction, view=None, limits=None, key=None, scales=None):
    ax = _axes(view, limits, scales)
    ax._set_view_from_bbox(tuple(box), direction, key)
    return _limits(ax)


def _format_coord(pixel, view=None, limits=None, rotating=False, scales=None):
    ax = _axes(view, limits, scales)
    if rotating:
        ax.button_pressed = 1
    xv, yv = ax.transData.inverted().transform(pixel)
    return ax.format_coord(xv, yv)


def _set_aspect(aspect, adjustable, limits=None, then=None):
    ax = _axes(limits=limits)
    ax.set_aspect(aspect, adjustable=adjustable)
    if then is not None:
        ax.set_aspect(then, adjustable=adjustable)
    return {"limits": _limits(ax), "box_aspect": list(ax._box_aspect)}


def _zoom_data(scale, view=None, limits=None, aspect=None):
    ax = _axes(view, limits)
    if aspect is not None:
        ax.set_aspect(aspect)
    ax._zoom_data_limits(*scale)
    return _limits(ax)


VIEWS = [None, [10, 35, 0], [-25, 120, 30], [60, -150, -45]]
LIMITS = [None, [[0, 10], [-2, 3], [100, 400]], [[5, -5], [0, 1], [-1, 1]]]
MOVES = [([0.0, 0.0], [0.03, 0.0]), ([0.01, -0.02], [-0.02, 0.025]), ([-0.06, 0.05], [0.07, 0.06])]


def test_rotate():
    for style in ["azel", "trackball", "sphere", "arcball"]:
        for view in VIEWS:
            for start, end in MOVES:
                probes.probe("axes3d.rotate", _rotate, style=style, start=start, end=end, view=view)
        probes.probe("axes3d.rotate", _rotate, style=style, start=[0.0, 0.0], end=[0.02, 0.01],
                     view=[10, 35, 0], control=True)


def test_zoom_and_pan():
    for limits in LIMITS:
        for start, end in MOVES:
            probes.probe("axes3d.zoom", _zoom, start=start, end=end, limits=limits)
        for view in VIEWS:
            for start, end in MOVES[:2]:
                probes.probe("axes3d.pan", _pan, start=start, end=end, view=view, limits=limits)


def test_zoom_box():
    boxes = [[120, 90, 220, 160], [300, 250, 140, 100], [100, 40, 140, 280]]
    for limits in LIMITS:
        for box in boxes:
            for direction in ["in", "out"]:
                probes.probe("axes3d.zoom_box", _zoom_box, box=box, direction=direction,
                             limits=limits, view=VIEWS[2])
        probes.probe("axes3d.zoom_box", _zoom_box, box=boxes[0], direction="in", limits=limits,
                     key="x")
        probes.probe("axes3d.zoom_box", _zoom_box, box=boxes[0], direction="in", limits=limits,
                     key="y")


def test_format_coord():
    pixels = [[200, 150], [150, 200], [260, 90], [120, 120], [230, 220]]
    for view in VIEWS:
        for limits in LIMITS:
            for pixel in pixels:
                probes.probe("axes3d.format_coord", _format_coord, pixel=pixel, view=view,
                             limits=limits)
        probes.probe("axes3d.format_coord", _format_coord, pixel=pixels[0], view=view,
                     rotating=True)


ASPECTS = ["auto", "equal", "equalxy", "equalxz", "equalyz"]
SCALED = [
    ({"z": "log"}, [[0, 10], [-2, 3], [0.1, 1000]]),
    ({"x": "log", "y": "symlog"}, [[1, 100], [-50, 50], [0, 1]]),
    ({"z": "logit"}, [[0, 1], [0, 1], [0.01, 0.9]]),
]


def test_set_aspect():
    for limits in LIMITS:
        for aspect in ASPECTS:
            for adjustable in ["box", "datalim"]:
                probes.probe("axes3d.set_aspect", _set_aspect, aspect=aspect,
                             adjustable=adjustable, limits=limits)
        for adjustable in ["box", "datalim"]:
            probes.probe("axes3d.set_aspect", _set_aspect, aspect="equal", adjustable=adjustable,
                         limits=limits, then="auto")


def test_zoom_data():
    scales = [[0.5, 1.0, 1.0], [1.0, 2.0, 0.8], [1.5, 1.5, 1.5]]
    for view in VIEWS:
        for scale in scales:
            for aspect in [None, "equal", "equalxz"]:
                probes.probe("axes3d.zoom_data", _zoom_data, scale=scale, view=view,
                             limits=LIMITS[1], aspect=aspect)


def test_scaled_navigation():
    pixels = [[200, 150], [150, 200], [260, 90]]
    for scales, limits in SCALED:
        for start, end in MOVES:
            probes.probe("axes3d.zoom", _zoom, start=start, end=end, limits=limits,
                         scales=scales)
            probes.probe("axes3d.pan", _pan, start=start, end=end, view=VIEWS[1],
                         limits=limits, scales=scales)
        probes.probe("axes3d.zoom_box", _zoom_box, box=[120, 90, 220, 160], direction="in",
                     limits=limits, view=VIEWS[2], scales=scales)
        for pixel in pixels:
            probes.probe("axes3d.format_coord", _format_coord, pixel=pixel, limits=limits,
                         scales=scales)
