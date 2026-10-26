"""The public API the API audit found missing in plt-rs and added
(``target/agents/api-audit/coverage.md`` in the audit's worktree).

Area ``api_audit``. Every op works on a fresh ``Figure``, drawn with the
Agg canvas where it says so.

Ops
---
``Axes.has_data``  {artist} -> ``has_data()`` after adding one artist:
    ``"none"``, ``"text"``, ``"line"``, ``"patch"``, ``"scatter"``,
    ``"image"``, ``"table"``, or a line removed again (``"removed"``).
``Figure.artists``  {} -> the lengths of ``[artists, images, texts,
    legends]`` of a figure holding an axes, a text, a circle and a line
    added with ``add_artist``, a ``figimage`` and a figure legend.
``Axes.callbacks``  {scenario} -> how often ``xlim_changed`` and
    ``ylim_changed`` callbacks ran, and the limits they saw:
    ``"set_xlim"``, ``"set_xbound"``, ``"invert"``, ``"shared"`` (two axes
    sharing x, limits set on the first), ``"read"`` (autoscaling when the
    y limits are read after plotting, read twice), ``"draw"`` (autoscaling
    while drawing), ``"clear"`` (callbacks of a cleared axes),
    ``"disconnect"``, ``"chain"`` (a callback of the first of two axes
    setting the y limits of the second, whose callbacks record them).
``Axes.set_subplotspec``  {to} -> the position bounds of an axes made in
    cell ``[0, 0]`` of a 2 × 2 grid and moved to cell ``to``, and of its
    twin.
``YAxis.set_offset_position``  {side} -> where the y offset text is drawn
    relative to the axes box: ``[left edge, right edge]`` minus the axes'
    left edge, in pixels.
``PolyCollection.set_verts``  {verts} -> vertices and codes of the paths
    after ``set_verts`` replaced a triangle.
``Collection.set_paths``  {verts} -> vertices of the paths of a path
    collection after ``set_paths``.
``FigureImage.get_extent``  {xo, yo, rows, cols} -> the extent of a
    ``figimage``, and its shape after ``set_data`` with one more row and
    column.
``NonUniformImage.set_data``  {x, y} -> the extent after ``set_data``
    (centres rounded to single precision).
``ScalarMappable.colorbar``  {} -> whether an image's ``colorbar`` is
    None before a colorbar is made, the first colorbar after, and the
    second after another one.
``Colorbar.solids``  {hatched} -> whether the colorbar of an image (or
    of hatched filled contours) has ``solids``, and how many
    ``solids_patches``.
``Divider``  {} -> ``[len(get_horizontal()), len(get_vertical()),
    get_aspect()]`` of a divider after ``append_size("left", ...)`` and
    ``append_size("top", ...)``, and ``get_aspect()`` of the divider of an
    axes with and without an equal aspect.
``Line3D.set_data_3d``  {x, y, z} -> ``get_data_3d()`` after
    ``set_data_3d`` and the x limits before and after.
``Text3D.get_position_3d``  {x, y, z} -> the position of ``text(x, y, z)``.
``Axis3D.get_rotate_label``  {label, rotate} -> ``get_rotate_label`` of
    the z label ``label`` after ``set_rotate_label(rotate)`` (``None``
    keeps the default, which rotates labels longer than four characters).
``Axes3D.set_title``  {loc} -> the left, centre and right titles after
    ``set_title("t", loc=loc)``.
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.collections import PathCollection, PolyCollection
from matplotlib.figure import Figure
from matplotlib.lines import Line2D
from matplotlib.patches import Circle
from matplotlib.path import Path
from mpl_toolkits.axes_grid1 import Divider, Size, make_axes_locatable

from spec_support import ProbeSet

probes = ProbeSet("api_audit")


def _has_data(artist):
    ax = Figure().add_subplot()
    if artist == "text":
        ax.text(0.5, 0.5, "note")
    elif artist == "line":
        ax.plot([0, 1], [0, 1])
    elif artist == "patch":
        ax.add_patch(Circle((0.5, 0.5), 0.1))
    elif artist == "scatter":
        ax.scatter([0, 1], [0, 1])
    elif artist == "image":
        ax.imshow([[0, 1], [1, 0]])
    elif artist == "table":
        ax.table(cellText=[["1", "2"]])
    elif artist == "removed":
        ax.plot([0, 1], [0, 1])[0].remove()
    return ax.has_data()


def _figure_artists():
    fig = Figure()
    ax = fig.add_subplot()
    (line,) = ax.plot([0, 1], [0, 1], label="data")
    fig.text(0.5, 0.5, "note")
    fig.add_artist(Circle((0.5, 0.5), 0.1))
    fig.add_artist(Line2D([0, 1], [0, 1]))
    fig.figimage(np.ones((2, 2)))
    fig.legend([line], ["data"])
    return [len(fig.artists), len(fig.images), len(fig.texts), len(fig.legends)]


class _Recorder:
    """Counts the limit callbacks of an axes and the limits they saw."""

    def __init__(self, ax):
        self.calls = {"x": [], "y": []}
        self.cids = [
            ax.callbacks.connect("xlim_changed", lambda a: self.calls["x"].append(list(a.get_xlim()))),
            ax.callbacks.connect("ylim_changed", lambda a: self.calls["y"].append(list(a.get_ylim()))),
        ]

    def counts(self):
        return [len(self.calls["x"]), len(self.calls["y"])]


def _callbacks(scenario):
    fig = Figure()
    if scenario == "shared":
        first, second = fig.subplots(1, 2, sharex=True)
        records = [_Recorder(first), _Recorder(second)]
        first.set_xlim(0, 3)
        return [r.counts() for r in records] + [records[1].calls["x"]]
    if scenario == "chain":
        first, second = fig.subplots(1, 2)
        record = _Recorder(second)
        first.callbacks.connect("xlim_changed", lambda a: second.set_ylim(*a.get_xlim()))
        first.set_xlim(1, 2)
        return [record.counts(), record.calls["y"]]
    ax = fig.add_subplot()
    record = _Recorder(ax)
    if scenario == "set_xlim":
        ax.set_xlim(0, 2)
    elif scenario == "set_xbound":
        ax.set_xbound(1, 3)
    elif scenario == "invert":
        ax.invert_yaxis()
    elif scenario == "read":
        ax.plot([0, 1], [0, 4])
        before = record.counts()
        ax.get_ylim()
        ax.get_ylim()
        return [before, record.counts(), record.calls["y"]]
    elif scenario == "draw":
        ax.plot([0, 1], [0, 4])
        FigureCanvasAgg(fig).draw()
        return [record.counts(), record.calls["x"], record.calls["y"]]
    elif scenario == "clear":
        ax.clear()
        ax.set_xlim(0, 2)
    elif scenario == "disconnect":
        ax.callbacks.disconnect(record.cids[0])
        ax.set_xlim(0, 2)
        ax.set_ylim(0, 2)
    return [record.counts(), record.calls["x"], record.calls["y"]]


def _set_subplotspec(to):
    fig = Figure()
    grid = fig.add_gridspec(2, 2)
    ax = fig.add_subplot(grid[0, 0])
    twin = ax.twinx()
    ax.set_subplotspec(grid[to[0], to[1]])
    return [list(ax.get_position().bounds), list(twin.get_position().bounds)]


def _offset_position(side):
    fig = Figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot()
    ax.plot([0, 1], [1e6, 1e6 + 1])
    ax.yaxis.set_offset_position(side)
    FigureCanvasAgg(fig).draw()
    box = ax.yaxis.offsetText.get_window_extent()
    left = ax.get_window_extent().x0
    return [box.x0 - left, box.x1 - left]


def _set_verts(verts):
    ax = Figure().add_subplot()
    polys = PolyCollection([[(0, 0), (1, 0), (0, 1)]])
    ax.add_collection(polys)
    polys.set_verts(verts)
    return [[p.vertices.tolist(), p.codes.tolist()] for p in polys.get_paths()]


def _set_paths(verts):
    ax = Figure().add_subplot()
    paths = PathCollection([Path([(0, 0), (1, 1)])])
    ax.add_collection(paths)
    paths.set_paths([Path(v) for v in verts])
    return [p.vertices.tolist() for p in paths.get_paths()]


def _figure_image(xo, yo, rows, cols):
    fig = Figure()
    image = fig.figimage(np.ones((rows, cols)), xo=xo, yo=yo)
    extent = list(image.get_extent())
    image.set_data(np.ones((rows + 1, cols + 1)))
    return [extent, list(image.get_array().shape)]


def _nonuniform_set_data(x, y):
    from matplotlib.image import NonUniformImage
    ax = Figure().add_subplot()
    image = NonUniformImage(ax)
    image.set_data([0, 1], [0, 1], np.ones((2, 2)))
    ax.add_image(image)
    image.set_data(x, y, np.ones((len(y), len(x))))
    return [float(v) for v in image.get_extent()]


def _mappable_colorbar():
    fig = Figure()
    ax = fig.add_subplot()
    image = ax.imshow([[0, 1], [1, 0]])
    before = image.colorbar is None
    first = fig.colorbar(image)
    after_first = image.colorbar is first
    second = fig.colorbar(image)
    return [before, after_first, image.colorbar is second]


def _colorbar_solids(hatched):
    fig = Figure()
    ax = fig.add_subplot()
    if hatched:
        z = [[0, 1, 2], [1, 2, 3], [2, 3, 4]]
        colorbar = fig.colorbar(ax.contourf(z, levels=[0, 2, 4], hatches=["/", "x"]))
    else:
        colorbar = fig.colorbar(ax.imshow([[0, 1], [1, 0]]))
    return [colorbar.solids is not None, len(colorbar.solids_patches)]


def _divider():
    fig = Figure()
    divider = Divider(fig, (0.1, 0.1, 0.8, 0.8), [Size.Fixed(1)], [Size.Fixed(1)], aspect=False)
    divider.append_size("left", Size.Fixed(0.5))
    divider.append_size("top", Size.Fixed(0.25))
    plain = make_axes_locatable(fig.add_subplot(1, 2, 1))
    equal_ax = fig.add_subplot(1, 2, 2)
    equal_ax.set_aspect("equal")
    equal = make_axes_locatable(equal_ax)
    return [len(divider.get_horizontal()), len(divider.get_vertical()), divider.get_aspect(),
            plain.get_aspect(), equal.get_aspect()]


def _line_data_3d(x, y, z):
    fig = Figure()
    ax = fig.add_subplot(projection="3d")
    (line,) = ax.plot([0, 1], [0, 1], [0, 1])
    FigureCanvasAgg(fig).draw()
    before = list(ax.get_xlim())
    line.set_data_3d(x, y, z)
    data = [np.asarray(c, float).tolist() for c in line.get_data_3d()]
    return [data, before, list(ax.get_xlim())]


def _text_position_3d(x, y, z):
    ax = Figure().add_subplot(projection="3d")
    return [float(v) for v in ax.text(x, y, z, "here").get_position_3d()]


def _rotate_label(label, rotate):
    ax = Figure().add_subplot(projection="3d")
    ax.set_zlabel(label)
    if rotate is not None:
        ax.zaxis.set_rotate_label(rotate)
    return ax.zaxis.get_rotate_label(ax.zaxis.label.get_text())


def _title_3d(loc):
    ax = Figure().add_subplot(projection="3d")
    ax.set_title("t", loc=loc)
    return [ax.get_title(loc="left"), ax.get_title(), ax.get_title(loc="right")]


def test_has_data():
    for artist in ["none", "text", "line", "patch", "scatter", "image", "table", "removed"]:
        probes.probe("Axes.has_data", _has_data, artist=artist)


def test_figure_artists():
    probes.probe("Figure.artists", _figure_artists)


def test_limit_callbacks():
    for scenario in ["set_xlim", "set_xbound", "invert", "shared", "read", "draw", "clear",
                     "disconnect", "chain"]:
        probes.probe("Axes.callbacks", _callbacks, scenario=scenario)


def test_set_subplotspec():
    for to in [[1, 1], [0, 1]]:
        probes.probe("Axes.set_subplotspec", _set_subplotspec, to=to)


def test_offset_position():
    for side in ["left", "right"]:
        probes.probe("YAxis.set_offset_position", _offset_position, side=side)


def test_collection_paths():
    probes.probe("PolyCollection.set_verts", _set_verts,
                 verts=[[[0, 0], [2, 0], [2, 2]], [[1, 1], [3, 1], [3, 3], [1, 3]]])
    probes.probe("Collection.set_paths", _set_paths, verts=[[[1, 1], [2, 2], [3, 1]]])


def test_figure_image():
    probes.probe("FigureImage.get_extent", _figure_image, xo=10, yo=20, rows=2, cols=3)


def test_nonuniform_set_data():
    probes.probe("NonUniformImage.set_data", _nonuniform_set_data, x=[0.1, 1, 4], y=[0, 2.3])


def test_mappable_colorbar():
    probes.probe("ScalarMappable.colorbar", _mappable_colorbar)


def test_colorbar_solids():
    for hatched in [False, True]:
        probes.probe("Colorbar.solids", _colorbar_solids, hatched=hatched)


def test_divider():
    probes.probe("Divider", _divider)


def test_axes3d_updates():
    probes.probe("Line3D.set_data_3d", _line_data_3d, x=[1, 2, 3], y=[0, 0, 0], z=[3, 2, 1])
    probes.probe("Text3D.get_position_3d", _text_position_3d, x=1, y=2, z=3)
    for label in ["z", "label"]:
        for rotate in [None, True, False]:
            probes.probe("Axis3D.get_rotate_label", _rotate_label, label=label, rotate=rotate)
    for loc in ["left", "center", "right"]:
        probes.probe("Axes3D.set_title", _title_3d, loc=loc)
