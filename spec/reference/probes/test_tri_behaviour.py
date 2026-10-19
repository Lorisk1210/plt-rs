"""Behaviour of plots on triangular grids, ported from the non-image tests
of matplotlib's ``lib/matplotlib/tests/test_triangulation.py`` that the
``tri`` and ``contour`` areas do not already cover.

Area ``tri_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at 100
dpi) with an Agg canvas and ``ax = fig.add_subplot()``.

Ops
---
``tri.tripcolor``  {points, c?, facecolors?, shading, clim?} ->
    {"kind", "clim", "faces"}
    ``ax.tripcolor(x, y, c, facecolors=..., shading=..., clim=...)`` on the
    Delaunay triangulation of ``points``: ``"diamond"`` (``x = [-1, 0, 1,
    0]``, ``y = [0, -1, 0, 1]``, two triangles) or ``"random"`` (10 points
    from ``spec_support.rng(5)``). ``kind`` is ``"flat"`` (a
    ``PolyCollection``) or ``"gouraud"``; ``clim`` the norm limits;
    ``faces`` the face colours of a flat plot after
    ``update_scalarmappable()`` (``null`` for Gouraud). Values that fit
    neither the points nor the triangles, face values of the wrong length
    and Gouraud shading of face values raise. Mirrors
    ``test_tripcolor_color`` and ``test_tripcolor_clim``.
``tri.delaunay_offset``  {offset} -> int
    The number of Delaunay triangles of the unit square and its centre
    shifted by ``offset`` in x and y (``test_qhull_large_offset``).
``tri.triplot_legend``  {} -> {"labels", "handles", "same_artist"}
    ``ax.triplot(x, y, [[0, 1, 2]], label="label")`` then
    ``ax.get_legend_handles_labels()``: one entry, the edge lines
    (``test_triplot_label``, ``test_triplot_return``).

Left out: the triangulation, finder, interpolator, refiner and analyzer
tests (``tri`` area), the tricontour path and error tests (``contour``
area), ``test_tripcolor_warnings`` (warnings), ``test_no_modify``,
``test_extract_triangulation_positional_mask``,
``test_trirefiner_fortran_contiguous_triangles``,
``test_internal_cpp_api`` and ``test_tricontourset_reuse`` (Python arrays,
argument parsing, private extension API and generator identity).
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.tri import Triangulation

from spec_support import ProbeSet, rng

probes = ProbeSet("tri_behaviour")


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _points(name):
    if name == "diamond":
        return [-1.0, 0.0, 1.0, 0.0], [0.0, -1.0, 0.0, 1.0]
    generator = rng(5)
    return generator.uniform(10).tolist(), generator.uniform(10).tolist()


def _tripcolor(points, shading, c=None, facecolors=None, clim=None):
    fig, ax = _axes()
    x, y = _points(points)
    triangulation = Triangulation(x, y)
    args = [] if c is None else [c]
    kw = {"shading": shading}
    if facecolors is not None:
        kw["facecolors"] = facecolors
    if clim is not None:
        kw["clim"] = clim
    artist = ax.tripcolor(triangulation, *args, **kw)
    flat = shading == "flat"
    if flat:
        artist.update_scalarmappable()
    return {"kind": shading, "clim": [float(artist.norm.vmin), float(artist.norm.vmax)],
            "faces": artist.get_facecolor().tolist() if flat else None}


def _delaunay_offset(offset):
    x = np.asarray([0, 1, 0, 1, 0.5]) + offset
    y = np.asarray([0, 0, 1, 1, 0.5]) + offset
    return len(Triangulation(x, y).triangles)


def _triplot_legend():
    fig, ax = _axes()
    lines, markers = ax.triplot([0, 2, 1], [0, 0, 1], [[0, 1, 2]], label="label")
    handles, labels = ax.get_legend_handles_labels()
    return {"labels": labels, "handles": len(handles), "same_artist": handles[0] is lines}


TRIPCOLORS = [
    {"c": [1, 2, 3]},
    {"facecolors": [1, 2, 3, 4]},
    {"facecolors": [1, 2], "shading": "gouraud"},
    {"c": [1, 2], "shading": "gouraud"},
    {"c": [1, 2, 3, 4]},
    {"c": [1, 2, 3, 4], "shading": "gouraud"},
    {"c": [1, 2]},
    {"facecolors": [1, 2]},
    {"c": [0.5, 0.25, 4.0, -1.0], "clim": [0.0, 1.0]},
]


def test_tripcolor():
    for case in TRIPCOLORS:
        probes.probe("tri.tripcolor", _tripcolor, points="diamond",
                     shading=case.get("shading", "flat"),
                     **{k: v for k, v in case.items() if k != "shading"})
    values = rng(6).uniform(10).tolist()
    for shading in ["flat", "gouraud"]:
        probes.probe("tri.tripcolor", _tripcolor, points="random", shading=shading, c=values,
                     clim=[0.25, 0.75])


def test_delaunay_offset():
    for offset in [0.0, 1e10]:
        probes.probe("tri.delaunay_offset", _delaunay_offset, offset=offset)


def test_triplot_legend():
    probes.probe("tri.triplot_legend", _triplot_legend)
