"""Navigating 3-D axes with the mouse (``mpl_toolkits.mplot3d``).

Area ``axes3d``. Every op draws ``Figure(figsize=(4, 3), dpi=100)`` with
``add_subplot(projection="3d")`` after applying ``view`` (``[elev, azim,
roll]``) and ``limits`` (``[[x0, x1], [y0, y1], [z0, z1]]``) when given.

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
"""

from types import SimpleNamespace

import matplotlib as mpl
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("axes3d")


def _axes(view=None, limits=None):
    fig = Figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot(projection="3d")
    if view is not None:
        ax.view_init(*view)
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


def _zoom(start, end, limits=None):
    ax = _axes(limits=limits)
    _move(ax, 3, start, end)
    return _limits(ax)


def _pan(start, end, view=None, limits=None):
    ax = _axes(view, limits)
    _move(ax, 2, start, end)
    return _limits(ax)


def _zoom_box(box, direction, view=None, limits=None, key=None):
    ax = _axes(view, limits)
    ax._set_view_from_bbox(tuple(box), direction, key)
    return _limits(ax)


def _format_coord(pixel, view=None, limits=None, rotating=False):
    ax = _axes(view, limits)
    if rotating:
        ax.button_pressed = 1
    xv, yv = ax.transData.inverted().transform(pixel)
    return ax.format_coord(xv, yv)


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
