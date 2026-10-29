"""Behaviour of collections, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_collections.py``.

Area ``collection_behaviour``. Every scene draws on ``Figure()`` (6.4 x 4.8
in at 100 dpi) with an Agg canvas and ``ax = fig.add_subplot()`` unless
noted.

Ops
---
``collection.limits``  {scene} -> {"datalim", "xlim", "ylim"}
    ``ax.dataLim`` (``[[x0, y0], [x1, y1]]``) and the view limits once the
    scene is built:

    ``scatter``  ``ax.scatter([0, 1], [0, 1])``; ``empty_scatter`` adds
        ``ax.scatter([], [])`` (``test_add_collection``: no change).
    ``blended``  ``ax.plot([2, 3, 4], [0.4, 0.6, 0.5])`` then a
        ``LineCollection`` of vertical lines at ``x = 1 ... 5`` from 0 to 1
        in ``blended_transform_factory(ax.transData, ax.transAxes)``: only
        x counts (``test_blended_collection_autolim``).
    ``singleton``  ``ax.scatter(0, 0)`` (``test_singleton_autolim``).
    ``zeros_data`` / ``zeros_axes``  ``ax.scatter(0, 0,
        transform=ax.transData | ax.transAxes)`` then ``ax.scatter(3, 3)``
        (``test_autolim_with_zeros``).
    ``paths_no_offsets``  ``ax.add_collection(PolyCollection([[(0, 0),
        (1, 0), (1, 2)], [(-1, -1), (0, -3), (0.5, -1)]]))`` (paths in data
        coordinates without offsets; ``test_no_offsets_datalim``).

``collection.colors``  {scene} -> [{"faces": [rgba, ...], "edges": [rgba,
    ...]}, ...]
    ``get_facecolor()`` and ``get_edgecolor()`` after
    ``update_scalarmappable()``, per collection of the scene (``"none"``
    gives no colours):

    ``alpha_mapped``  ``ax.scatter(x, x, c=x, alpha=x / 5)`` and
        ``ax.scatter(x, x, c=x)`` given ``set_alpha(x / 5)`` afterwards,
        ``x = arange(5)`` (``test_scatter_alpha_array``).
    ``alpha_colors``  ``ax.scatter(x, x, color=["r", "g", "b", "c", "m"],
        alpha=x / 5)`` and ``ax.scatter(x, x, color="r", alpha=x / 5)``.
    ``alpha_colors_set``  the same with ``set_alpha(x / 5)`` afterwards.
    ``line_collection``  ``LineCollection(None, linewidth=2.2,
        edgecolor="r", zorder=3, facecolors=[0, 1, 0, 1])`` and
        ``LineCollection(None, facecolor=None)``, outside any axes
        (``test_LineCollection_args``).
    ``mesh_clim``  ``ax.pcolormesh([[1]], cmap="plasma", norm=Normalize(0,
        1))``, then the same mesh after ``set_clim(1, 2)``
        (``test_quadmesh_vmin_vmax``).
    ``mesh_alpha``  ``ax.pcolormesh(x, y, z, alpha=a)`` and a mesh given
        ``set_alpha(a)``, ``z = arange(9).reshape(3, 3)``, ``a = z / 8``
        (``test_quadmesh_alpha_array``; flattened alpha).
    ``pcolor_edges``  ``pcolor(z, edgecolors="red", facecolors="none")``
        and ``pcolor(z)`` given ``set_facecolor("none")``,
        ``set_edgecolor("red")``, then ``set_alpha(0.5)``, with ``z =
        arange(12).reshape(3, 4)`` (``test_color_logic``).

``collection.contains``  {scene, points} -> [ind | null, ...]
    ``coll.contains(event)`` for an event at each point of ``points``
    (data coordinates):

    ``scatter``  ``ax.scatter([0], [0], [1000])`` (``test_picking``).
    ``mesh``  ``ax.pcolormesh(X)`` with ``X = x[:, None] * x[None, :]``,
        ``x = arange(4)`` (``test_quadmesh_contains``).
    ``mesh_concave``  ``ax.pcolormesh([[0, -1], [1, 0]], [[0, 1], [1, -1]],
        [[0]])``, a V-shaped cell (``test_quadmesh_contains_concave``).

``collection.legend_elements``  {sizes, values, options} -> {"labels",
    "handles"}
    ``sc = ax.scatter(x, y, c=values, s=sizes, cmap="jet", marker="o",
    linewidths=0)`` with ``x``, ``y`` from ``spec_support.rng(1)`` (10
    points each), then ``sc.legend_elements(**options)``: the labels and the
    number of handles (the handles' own properties are not observable from
    outside pltrs, whose legend handles are opaque). ``options`` holds
    ``prop``, ``num`` (a number, a list or ``"auto"``), ``fmt`` (a format
    string), ``func`` (``"double"`` for ``2 * x`` or ``"inverse5"`` for ``5 /
    x``), ``alpha`` and ``color``.
    Mirrors ``test_pathcollection_legend_elements`` and
    ``test_legend_inverse_size_label_relationship`` (``np.random`` data
    replaced by the spec generator).

``collection.cursor``  {pixel} -> str
    The status bar message for ``ax.pcolormesh([1, 2, 1, 2], [1, 2, 1, 2],
    ones((3, 3)))`` (every quad covers the same square) at a display pixel
    (``test_quadmesh_cursor_data_multiple_points``).

``collection.hatchcolors``  {kw, steps, rcparams} -> [rgba, ...]
    ``PathCollection([Path.unit_rectangle()], hatch="//", **kw)`` followed
    by the setters in ``steps`` (``set_edgecolor``, ``set_hatchcolor``,
    with lists of colours), all under ``rc_context(rcparams)``:
    ``to_rgba_array(get_hatchcolor())``. ``kw`` holds ``edgecolor`` and
    ``hatchcolor`` (lists, or ``"edge"``). Mirrors
    ``test_collection_hatchcolor_inherit_logic`` and
    ``test_collection_hatchcolor_fallback_logic``.

Left out: the tests the ``mpl_collections`` area ports or lists (getters
returning what was set, broadcast widths and styles, data limits, mesh
arrays and coordinates, argument validation, the ``pcolor`` variants and the
look of legend handles), ``test_quiver_limits`` and ``test_barb_limits`` (in
``quiver_behaviour``) and the Python-only ones.
"""

import matplotlib.colors as mcolors
import numpy as np
from matplotlib.backend_bases import MouseEvent, NavigationToolbar2
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.collections import LineCollection, PathCollection, PolyCollection
from matplotlib.path import Path
from matplotlib.figure import Figure
from matplotlib.transforms import blended_transform_factory

from spec_support import ProbeSet, rng

probes = ProbeSet("collection_behaviour")


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _scatter(ax):
    ax.scatter([0, 1], [0, 1])


def _empty_scatter(ax):
    ax.scatter([0, 1], [0, 1])
    ax.scatter([], [])


def _blended(ax):
    ax.plot([2, 3, 4], [0.4, 0.6, 0.5])
    lines = [np.array([[x, 0], [x, 1]]) for x in [1, 2, 3, 4, 5]]
    trans = blended_transform_factory(ax.transData, ax.transAxes)
    ax.add_collection(LineCollection(lines, transform=trans))


def _paths_no_offsets(ax):
    ax.add_collection(PolyCollection([[(0, 0), (1, 0), (1, 2)],
                                      [(-1, -1), (0, -3), (0.5, -1)]]))


LIMIT_SCENES = {
    "scatter": _scatter,
    "empty_scatter": _empty_scatter,
    "blended": _blended,
    "singleton": lambda ax: ax.scatter(0, 0),
    "zeros_data": lambda ax: (ax.scatter(0, 0, transform=ax.transData), ax.scatter(3, 3)),
    "zeros_axes": lambda ax: (ax.scatter(0, 0, transform=ax.transAxes), ax.scatter(3, 3)),
    "paths_no_offsets": _paths_no_offsets,
}


def _limits(scene):
    fig, ax = _axes()
    LIMIT_SCENES[scene](ax)
    return {"datalim": ax.dataLim.get_points().tolist(), "xlim": list(ax.get_xlim()),
            "ylim": list(ax.get_ylim())}


X5 = np.arange(5)


def _alpha_mapped(ax):
    first = ax.scatter(X5, X5, c=X5, alpha=X5 / 5)
    second = ax.scatter(X5, X5, c=X5)
    second.set_alpha(X5 / 5)
    return [first, second]


def _alpha_colors(ax):
    return [ax.scatter(X5, X5, color=["r", "g", "b", "c", "m"], alpha=X5 / 5),
            ax.scatter(X5, X5, color="r", alpha=X5 / 5)]


def _alpha_colors_set(ax):
    first = ax.scatter(X5, X5, color=["r", "g", "b", "c", "m"])
    first.set_alpha(X5 / 5)
    second = ax.scatter(X5, X5, color="r")
    second.set_alpha(X5 / 5)
    return [first, second]


def _line_collection(ax):
    return [LineCollection(None, linewidth=2.2, edgecolor="r", zorder=3,
                           facecolors=[0, 1, 0, 1]),
            LineCollection(None, facecolor=None)]


def _mesh_clim(ax):
    norm = mcolors.Normalize(vmin=0, vmax=1)
    first = ax.pcolormesh([[1]], cmap="plasma", norm=norm)
    first.update_scalarmappable()
    colors = {"faces": first.get_facecolor().tolist(), "edges": first.get_edgecolor().tolist()}
    first.set_clim(1, 2)
    return [colors, first]


Z9 = np.arange(9).reshape((3, 3))


def _mesh_alpha(ax):
    x = y = np.arange(4)
    first = ax.pcolormesh(x, y, Z9, alpha=Z9 / Z9.max())
    second = ax.pcolormesh(x, y, Z9)
    second.set_alpha(Z9 / Z9.max())
    return [first, second]


def _pcolor_edges(ax):
    z = np.arange(12).reshape(3, 4)
    first = ax.pcolor(z, edgecolors="red", facecolors="none")
    second = ax.pcolor(z)
    second.set_facecolor("none")
    second.set_edgecolor("red")
    second.update_scalarmappable()
    colors = {"faces": second.get_facecolor().tolist(), "edges": second.get_edgecolor().tolist()}
    third = ax.pcolor(z)
    third.set_facecolor("none")
    third.set_edgecolor("red")
    third.set_alpha(0.5)
    return [first, colors, third]


COLOR_SCENES = {
    "alpha_mapped": _alpha_mapped,
    "alpha_colors": _alpha_colors,
    "alpha_colors_set": _alpha_colors_set,
    "line_collection": _line_collection,
    "mesh_clim": _mesh_clim,
    "mesh_alpha": _mesh_alpha,
    "pcolor_edges": _pcolor_edges,
}


def _colors_of(collection):
    if isinstance(collection, dict):
        return collection
    collection.update_scalarmappable()
    return {"faces": collection.get_facecolor().tolist(),
            "edges": collection.get_edgecolor().tolist()}


def _colors(scene):
    fig, ax = _axes()
    return [_colors_of(c) for c in COLOR_SCENES[scene](ax)]


def _mesh(ax):
    x = np.arange(4)
    return ax.pcolormesh(x[:, None] * x[None, :])


CONTAINS_SCENES = {
    "scatter": lambda ax: ax.scatter([0], [0], [1000]),
    "mesh": _mesh,
    "mesh_concave": lambda ax: ax.pcolormesh([[0, -1], [1, 0]], [[0, 1], [1, -1]], [[0]]),
}


def _contains(scene, points):
    fig, ax = _axes()
    collection = CONTAINS_SCENES[scene](ax)
    fig.draw_without_rendering()
    hits = []
    for x, y in ax.transData.transform(points):
        event = MouseEvent("button_press_event", fig.canvas, x, y)
        event.x, event.y = x, y  # keep the exact position (events round to pixels)
        found, info = collection.contains(event)
        hits.append([int(i) for i in info["ind"]] if found else None)
    return hits


FUNCS = {"double": lambda x: 2 * x, "inverse5": lambda x: 5 / x}


def _legend_elements(sizes, values, options):
    generator = rng(1)
    x, y = generator.uniform(10), generator.uniform(10)
    fig, ax = _axes()
    sc = ax.scatter(x, y, c=values, s=sizes, cmap="jet", marker="o", linewidths=0)
    kw = dict(options)
    if "func" in kw:
        kw["func"] = FUNCS[kw["func"]]
    handles, labels = sc.legend_elements(**kw)
    return {"labels": labels, "handles": len(handles)}


def _cursor(pixel):
    fig, ax = _axes()
    ax.pcolormesh([1, 2, 1, 2], [1, 2, 1, 2], np.ones((3, 3)))
    fig.canvas.draw()
    event = MouseEvent("motion_notify_event", fig.canvas, *pixel)
    return NavigationToolbar2._mouse_event_to_message(event)


def test_limits():
    for scene in LIMIT_SCENES:
        probes.probe("collection.limits", _limits, scene=scene)


def test_colors():
    for scene in COLOR_SCENES:
        probes.probe("collection.colors", _colors, scene=scene)


CONTAINS_POINTS = {
    "scatter": [[0, 0], [0.0045, 0], [0.0047, 0.0], [0.01, 0], [0, 0.005], [0, 0.006]],
    "mesh": [[0.5, 0.5], [1.5, 1.5], [2.5, 0.5], [0.5, 2.5], [2.9, 2.9], [3.1, 1]],
    "mesh_concave": [[-0.5, 0.25], [0, 0.25], [0.5, 0.25], [0, -0.25], [0, 0.9], [0.9, -0.9]],
}


def test_contains():
    for scene, points in CONTAINS_POINTS.items():
        probes.probe("collection.contains", _contains, scene=scene, points=points)


SIZES = [12, 45, 290, 103, 45, 210, 12, 77, 290, 160]
VALUES = [0, 3, 1, 4, 2, 0, 1, 3, 4, 2]
LEGENDS = [
    {"fmt": "{x:g}"},
    {"num": 9},
    {"prop": "sizes", "alpha": 0.5, "color": "red"},
    {"prop": "sizes", "num": 4, "fmt": "{x:.2f}", "func": "double"},
    {"num": [-1, 0, 55.4, 260], "prop": "sizes", "fmt": "{x:g}"},
    {"prop": "sizes", "num": [0.2, 0.4, 0.6, 0.8], "func": "inverse5"},
    {},
    {"num": "auto", "prop": "sizes"},
]


def test_legend_elements():
    for options in LEGENDS:
        probes.probe("collection.legend_elements", _legend_elements, sizes=SIZES, values=VALUES,
                     options=options)
    inverse = [5 / (1 - v) for v in rng(2).uniform(50)]
    probes.probe("collection.legend_elements", _legend_elements, sizes=inverse[:10],
                 values=VALUES, options={"prop": "sizes", "num": [0.2, 0.4, 0.6, 0.8],
                                         "func": "inverse5"})


def _hatchcolors(kw, steps, rcparams):
    import matplotlib as mpl

    with mpl.rc_context(rcparams):
        col = PathCollection([Path.unit_rectangle()], hatch="//", **kw)
        for name, value in steps:
            getattr(col, name)(value)
        return mcolors.to_rgba_array(col.get_hatchcolor()).tolist()


EDGES = ["purple", "red", "green", "yellow"]
HATCHES = ["orange", "cyan", "blue", "magenta"]
HATCH_CASES = [
    ({"edgecolor": EDGES, "hatchcolor": HATCHES}, [], {"hatch.color": "edge"}),
    ({}, [["set_edgecolor", EDGES]], {"hatch.color": "edge"}),
    ({}, [["set_edgecolor", EDGES], ["set_hatchcolor", HATCHES]], {"hatch.color": "edge"}),
    ({}, [["set_hatchcolor", HATCHES]], {"hatch.color": "edge"}),
    ({}, [["set_hatchcolor", HATCHES], ["set_edgecolor", EDGES]], {"hatch.color": "edge"}),
    ({"hatchcolor": HATCHES}, [], {"hatch.color": "green"}),
    ({"edgecolor": EDGES, "hatchcolor": HATCHES}, [], {"hatch.color": "green"}),
    ({}, [], {"hatch.color": "green"}),
    ({}, [["set_edgecolor", EDGES]], {"hatch.color": "green"}),
    ({"edgecolor": EDGES}, [], {"hatch.color": "edge"}),
    ({"edgecolor": EDGES, "hatchcolor": "edge"}, [], {}),
    ({}, [], {}),
]


def test_hatchcolors():
    for kw, steps, rcparams in HATCH_CASES:
        probes.probe("collection.hatchcolors", _hatchcolors, kw=kw, steps=steps,
                     rcparams=rcparams)


def test_cursor():
    for pixel in [[328, 237], [200, 100], [500, 400], [100, 300]]:
        probes.probe("collection.cursor", _cursor, pixel=pixel)
