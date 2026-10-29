"""The 3-D API the API audit found missing in pltrs and added: units on
3-D axes, the tick and gridlines of their axes, the view margin of their
limit setters, and the setters of their collections.

Area ``api_audit_3d``. Every op works on a fresh ``Figure`` with one 3-D
axes, drawn with the Agg canvas where it says so. matplotlib converts no
plotting values on 3-D axes by itself, so values with units go through
``Axes3D._process_unit_info`` first, as pltrs' plotting methods convert
them.

Ops
---
``Axes3D.convert_zunits``  {kind} -> the numbers ``convert_zunits``
    gives for numbers or dates, and whether the z axis then has a
    converter.
``Axis3D.update_units``  {kind} -> what ``update_units`` returns for dates
    on the z axis (``"dates"``) or names on the x axis (``"names"``), the
    numbers ``convert_units`` then gives for later values (``2020-01-03``
    and ``2020-01-10``, or ``["c", "d"]``), and the tick labels of the
    axis after a draw with limits around them.
``Axes3D.plot``  {zdir} -> the x, y and z limits and the x and z tick
    labels after plotting names, numbers and dates placed in the plane
    normal to ``zdir`` and a draw.
``Axes3D.get_zgridlines``  {zlim} -> how many gridlines (one per major
    tick) and the projected gridlines the axes draw across the panes,
    after a draw with z limits ``zlim``.
``Axes3D.get_zticklines``  {zlim} -> how many major ticks
    (``len(get_zticklines()) / 2``) and the projected ``tick1line`` data
    of those in view, after a draw.
``Axes3D.set_lim3d``  {setter, lim, view_margin, scale, automargin} -> the
    z limits after ``set_zlim`` or ``set_zbound`` with ``view_margin`` on
    an axis with ``scale``, with ``axes3d.automargin`` set as given.
``Collection3D.set_sort_zpos``  {kind} -> the z-orders of two collections
    of one ``kind`` (``"poly"``, ``"line"``, ``"scatter"``) at z = 0 and 1
    after a draw, then after ``set_sort_zpos(-5)`` on the second.
``Line3DCollection.set_segments``  {segments} -> the projected segments
    after ``set_segments`` and a draw.
``Patch3DCollection``  {depthshade} -> ``get_depthshade()`` of a patch
    collection lifted with ``add_collection3d`` (``None`` keeps
    ``axes3d.depthshade``) and after ``set_depthshade(not ...)``, its 3-D
    offsets, and those after ``set_3d_properties([0.5, 0.25], "x")``.
``Patch3DCollection.get_offsets``  {} -> the projected offsets of that
    collection after a draw.
``Poly3DCollection.set_verts``  {closed, codes} -> the number of vertices
    and the codes of the projected paths and the face colours after
    replacing a triangle by a triangle and a square (``set_verts``, or
    ``set_verts_and_codes`` with lines only when ``codes``) and a draw.
``Poly3DCollection.set_3d_properties``  {} -> the z-orders of two polygon
    collections, the second given a sort position and red faces through
    the 2-D ``set_facecolor``, after ``set_3d_properties`` on it and a
    draw, and its face colour.
"""

import datetime

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.collections import PatchCollection, PolyCollection
from matplotlib.figure import Figure
from matplotlib.patches import Circle
from matplotlib.path import Path
from mpl_toolkits.mplot3d import art3d

from spec_support import ProbeSet

probes = ProbeSet("api_audit_3d")


def _ax3d():
    fig = Figure()
    return fig, fig.add_subplot(projection="3d")


def _days(*days):
    return [datetime.datetime(2020, 1, d) for d in days]


def _draw(fig):
    FigureCanvasAgg(fig).draw()


def _labels(axis):
    return [t.get_text() for t in axis.get_ticklabels()]


def _convert_zunits(kind):
    _, ax = _ax3d()
    values = [1.5, 2.5] if kind == "numbers" else _days(1, 3)
    return [ax.convert_zunits(values), ax.zaxis.get_converter() is not None]


def _update_units(kind):
    fig, ax = _ax3d()
    if kind == "dates":
        updated = ax.zaxis.update_units(_days(1, 2))
        converted = ax.zaxis.convert_units(_days(3, 10))
        ax.set_zlim(converted[0], converted[1])
        axis = ax.zaxis
    else:
        updated = ax.xaxis.update_units(["a", "b", "c"])
        converted = ax.xaxis.convert_units(["c", "d"])
        ax.set_xlim(-0.5, 3.5)
        axis = ax.xaxis
    _draw(fig)
    return [updated, list(converted), _labels(axis)]


def _unjuggle(values, zdir):
    """The plotting arguments whose juggled values are ``values``."""
    a, b, c = values
    if zdir == "x":
        return b, c, a
    if zdir == "y":
        return a, c, b
    return a, b, c


def _plot(zdir):
    fig, ax = _ax3d()
    xs, ys, zs = ["a", "b", "c", "a"], [0, 1, 2, 3], _days(1, 2, 4, 8)
    on_axes = art3d.juggle_axes(xs, ys, zs, zdir)
    converted = ax._process_unit_info(list(zip("xyz", on_axes)))
    ax.plot(*_unjuggle(converted, zdir), zdir=zdir)
    _draw(fig)
    return [list(ax.get_xlim()), list(ax.get_ylim()), list(ax.get_zlim()),
            _labels(ax.xaxis), _labels(ax.zaxis)]


def _zgridlines(zlim):
    fig, ax = _ax3d()
    ax.set_zlim(*zlim)
    _draw(fig)
    segments = [np.asarray(s).tolist() for s in ax.zaxis.gridlines.get_segments()]
    return [len(ax.get_zgridlines()), segments]


def _zticklines(zlim):
    fig, ax = _ax3d()
    ax.set_zlim(*zlim)
    _draw(fig)
    ticks = ax.zaxis.get_major_ticks()
    low, high = sorted(ax.get_zlim())
    in_view = [low - 1e-10 <= loc <= high + 1e-10 for loc in ax.zaxis.get_majorticklocs()]
    marks = [[list(map(float, line)) for line in tick.tick1line.get_data()]
             for tick, shown in zip(ticks, in_view) if shown]
    return [len(ax.get_zticklines()) // 2, marks]


def _set_lim3d(setter, lim, view_margin, scale, automargin):
    import matplotlib

    with matplotlib.rc_context({"axes3d.automargin": automargin}):
        _, ax = _ax3d()
        ax.set_zscale(scale)
        getattr(ax, setter)(*lim, view_margin=view_margin)
        return list(ax.get_zlim())


def _triangle(z):
    return [[0, 0, z], [1, 0, z], [0, 1, z]]


def _collection(ax, kind, z):
    if kind == "poly":
        col = art3d.Poly3DCollection([_triangle(z)])
    elif kind == "line":
        col = art3d.Line3DCollection([_triangle(z)])
    else:
        return ax.scatter([0, 1], [0, 1], [z, z])
    ax.add_collection3d(col)
    return col


def _sort_zpos(kind):
    fig, ax = _ax3d()
    first, second = _collection(ax, kind, 0), _collection(ax, kind, 1)
    _draw(fig)
    before = [first.get_zorder(), second.get_zorder()]
    second.set_sort_zpos(-5)
    _draw(fig)
    return [before, [first.get_zorder(), second.get_zorder()]]


def _set_segments(segments):
    fig, ax = _ax3d()
    col = art3d.Line3DCollection([[[0, 0, 0], [1, 1, 1]]])
    ax.add_collection3d(col)
    col.set_segments(segments)
    _draw(fig)
    return [np.asarray(s).tolist() for s in col.get_segments()]


def _patches(ax):
    col = PatchCollection([Circle((0, 0), 0.01)], offsets=[(0.2, 0.3), (0.6, 0.7)])
    ax.add_collection3d(col, zs=[0.1, 0.9], zdir="y")
    return col


def _offsets3d(col):
    return [np.asarray(c, float).tolist() for c in col._offsets3d]


def _patch3d(depthshade):
    _, ax = _ax3d()
    col = _patches(ax)
    if depthshade is not None:
        col.set_depthshade(depthshade)
    shading = col.get_depthshade()
    col.set_depthshade(not shading)
    lifted = _offsets3d(col)
    col.set_3d_properties([0.5, 0.25], "x")
    return [shading, col.get_depthshade(), lifted, _offsets3d(col)]


def _patch_offsets():
    fig, ax = _ax3d()
    col = _patches(ax)
    _draw(fig)
    return np.asarray(col.get_offsets(), float).tolist()


def _paths(col):
    return [[len(p.vertices), None if p.codes is None else [int(c) for c in p.codes]]
            for p in col.get_paths()]


def _set_verts(closed, codes):
    fig, ax = _ax3d()
    col = art3d.Poly3DCollection([_triangle(0)], facecolors=["r", "b"])
    ax.add_collection3d(col)
    square = [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]]
    if codes:
        lines = [[Path.MOVETO] + [Path.LINETO] * (len(f) - 1) for f in (_triangle(2), square)]
        col.set_verts_and_codes([_triangle(2), square], lines)
    else:
        col.set_verts([_triangle(2), square], closed=closed)
    _draw(fig)
    return [_paths(col), np.asarray(col.get_facecolor()).tolist()]


def _set_3d_properties():
    fig, ax = _ax3d()
    first, second = _collection(ax, "poly", 0), _collection(ax, "poly", 1)
    second.set_sort_zpos(-5)
    PolyCollection.set_facecolor(second, "red")
    second.set_3d_properties()
    _draw(fig)
    return [[first.get_zorder(), second.get_zorder()], np.asarray(second.get_facecolor()).tolist()]


def test_units():
    for kind in ["numbers", "dates"]:
        probes.probe("Axes3D.convert_zunits", _convert_zunits, kind=kind)
    for kind in ["dates", "names"]:
        probes.probe("Axis3D.update_units", _update_units, kind=kind)
    for zdir in ["z", "x", "y"]:
        probes.probe("Axes3D.plot", _plot, zdir=zdir)


def test_tick_and_grid_lines():
    for zlim in [[0, 1], [0.05, 0.95]]:
        probes.probe("Axes3D.get_zgridlines", _zgridlines, zlim=zlim)
        probes.probe("Axes3D.get_zticklines", _zticklines, zlim=zlim)


def test_view_margin():
    for setter in ["set_zlim", "set_zbound"]:
        for view_margin in [None, 0.0, 0.1, -0.2]:
            for automargin in [False, True]:
                probes.probe("Axes3D.set_lim3d", _set_lim3d, setter=setter, lim=[4, 1],
                             view_margin=view_margin, scale="linear", automargin=automargin)
    probes.probe("Axes3D.set_lim3d", _set_lim3d, setter="set_zlim", lim=[1, 100],
                 view_margin=0.25, scale="log", automargin=False)


def test_collections():
    for kind in ["poly", "line", "scatter"]:
        probes.probe("Collection3D.set_sort_zpos", _sort_zpos, kind=kind)
    probes.probe("Line3DCollection.set_segments", _set_segments,
                 segments=[[[0, 0, 0], [0.5, 1, 0], [1, 0, 1]], [[1, 1, 1], [0, 0, 1]]])
    for depthshade in [None, False]:
        probes.probe("Patch3DCollection", _patch3d, depthshade=depthshade)
    probes.probe("Patch3DCollection.get_offsets", _patch_offsets)
    for closed, codes in [(True, False), (False, False), (False, True)]:
        probes.probe("Poly3DCollection.set_verts", _set_verts, closed=closed, codes=codes)
    probes.probe("Poly3DCollection.set_3d_properties", _set_3d_properties)
