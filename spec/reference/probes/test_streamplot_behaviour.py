"""Behaviour of streamplots, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_streamplot.py``.

Area ``streamplot_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in
at 100 dpi) with an Agg canvas and ``ax = fig.add_subplot()``.

Ops
---
``streamplot.field``  {x, y, u, v, options} -> int
    ``ax.streamplot(x, y, u, v, **options)``: the number of streamlines
    (``len(lines.get_segments())``), or the error for grids
    that are not evenly spaced and increasing, meshes whose rows of ``x``
    or columns of ``y`` differ, and integration scales that are not
    positive. ``x`` and ``y`` are 1-D or 2-D; ``options`` holds
    ``integration_max_step_scale``, ``integration_max_error_scale``,
    ``start_points``, ``density`` and ``color`` (a 2-D array of values).
    Mirrors ``test_streamplot_grid``, ``test_streamplot_integration_params``
    and ``test_streamplot_inputs`` (``np.random`` data replaced by
    ``spec_support.rng``).
``streamplot.datalim``  {x, y, u, v, options} -> bbox
    ``ax.dataLim`` after the same call: matplotlib counts the arrows'
    outlines too (see the waivers).

Left out: ``test_streamplot_limits`` (it draws barbs; see
``quiver_behaviour``) and the image comparisons.
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet, rng

probes = ProbeSet("streamplot_behaviour")


def _streamplot(x, y, u, v, options):
    fig = Figure()
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    kw = dict(options)
    for key in ("color", "start_points"):
        if key in kw:
            kw[key] = np.asarray(kw[key], float)
    stream = ax.streamplot(np.asarray(x, float), np.asarray(y, float), np.asarray(u, float),
                           np.asarray(v, float), **kw)
    return ax, stream


def _field(x, y, u, v, options):
    return len(_streamplot(x, y, u, v, options)[1].lines.get_segments())


def _datalim(x, y, u, v, options):
    return _streamplot(x, y, u, v, options)[0].dataLim.get_points().tolist()


ONES2, ZEROS2 = np.ones((2, 2)).tolist(), np.zeros((2, 2)).tolist()
ONES3, ZEROS3 = np.ones((3, 3)).tolist(), np.zeros((3, 3)).tolist()
MESH_X = [[10, 20], [10, 20]]
MESH_Y = [[10, 10], [20, 20]]
NAN3 = np.full((3, 3), np.nan).tolist()
GRID = np.linspace(-2, 2, 9).tolist()
SWIRL_U = [[-y for x in GRID] for y in GRID]
SWIRL_V = [[x for x in GRID] for y in GRID]


def test_fields():
    cases = [
        ([[10, 20], [10, 30]], MESH_Y, ONES2, ZEROS2, {}),
        (MESH_X, [[10, 10], [20, 30]], ONES2, ZEROS2, {}),
        (MESH_X, MESH_Y, ONES2, ZEROS2, {}),
        ([0, 10, 20], [0, 10, 30], ONES3, ZEROS3, {}),
        ([0, 20, 40], [0, 20, 10], ONES3, ZEROS3, {}),
        ([0, 10], [0], ONES2, ZEROS2, {}),
        (MESH_X, MESH_Y, ONES2, ZEROS2, {"integration_max_step_scale": -0.5}),
        (MESH_X, MESH_Y, ONES2, ZEROS2, {"integration_max_error_scale": 0.0}),
        (MESH_X, MESH_Y, ONES2, ZEROS2, {"integration_max_step_scale": 0.5,
                                         "integration_max_error_scale": 2.0}),
        ([0, 1, 2], [0, 1, 2], NAN3, NAN3, {"color": rng(7).uniform(9).reshape(3, 3).tolist()}),
        ([0, 1, 2], [0, 1, 2], rng(8).uniform(9).reshape(3, 3).tolist(),
         rng(9).uniform(9).reshape(3, 3).tolist(), {}),
        (GRID, GRID, SWIRL_U, SWIRL_V, {"start_points": [[1.0, 0.0], [0.0, -0.5]]}),
        (GRID, GRID, SWIRL_U, SWIRL_V, {"density": 0.5}),
    ]
    for x, y, u, v, options in cases:
        probes.probe("streamplot.field", _field, x=x, y=y, u=u, v=v, options=options)
    for x, y, u, v, options in [cases[2], cases[8], cases[11], cases[12]]:
        probes.probe("streamplot.datalim", _datalim, x=x, y=y, u=u, v=v, options=options)
