"""Behaviour of quiver and barbs plots, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_quiver.py`` and the quiver and
barb tests of ``test_collections.py``.

Area ``quiver_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at
100 dpi) with an Agg canvas and ``ax = fig.add_subplot()``.

Ops
---
``quiver.limits``  {scene} -> {"datalim", "xlim", "ylim"}
    ``ax.dataLim`` (``[[x0, y0], [x1, y1]]``) and the view limits after:

    ``grid``  ``ax.quiver(arange(8), arange(10), U, U)`` with ``U =
        linspace(0, 10, 80).reshape(10, 8)``.
    ``translated``  ``ax.quiver(X, Y, sin(X), cos(Y),
        transform=Affine2D().translate(25, 32) + ax.transData)`` on the
        grid of ``linspace(-5, 10, 20)`` x ``linspace(-2, 4, 10)``: the
        positions count where the transform puts them.
    ``barbs_translated``  ``ax.barbs`` of the same.

    Mirrors ``test_quiver_limits`` and ``test_barb_limits``.
``quiver.arguments``  {plot, x, y, u, v, c?} -> true
    ``ax.quiver`` / ``ax.barbs`` (``plot``) of the given arrays: ``true``,
    or the error for sizes that do not match (``test_quiver_arg_sizes``,
    ``test_bad_masked_sizes``).
``quiver.draw``  {scene} -> "drawn"
    ``zero_head``: ``ax.quiver(cos(X), sin(Y), headlength=0,
    headaxislength=0)`` on a 10 x 10 index grid (``test_zero_headlength``);
    ``angles_xy``: ``ax.quiver(X, Y, 1, 1, angles=phi, scale_units="xy")``
    with ``phi`` from ``spec_support.rng(4)`` (``test_angles_and_scale``,
    ``test_no_warnings``).

Left out: the memory-leak and copy tests (Python reference counting and
array copies), ``test_quiver_number_of_args`` (positional argument
counts), positions given in coordinates that do not end in the data
ones (matplotlib maps them back through the view of the moment the
quiver is made; plt-rs leaves them out of the data limits, as for other
collections), ``test_quiver_setuvc_numbers`` (plt-rs plots take their vectors
once), and the quiver key tests (``test_quiverkey_angles*``: the number of
paths of the key arrow is not observable from outside plt-rs).
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.transforms import Affine2D

from spec_support import ProbeSet, rng

probes = ProbeSet("quiver_behaviour")


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _translated_field():
    x = np.linspace(-5, 10, 20)
    y = np.linspace(-2, 4, 10)
    y, x = np.meshgrid(y, x)
    return x, y, np.sin(x), np.cos(y)


def _limits(scene):
    fig, ax = _axes()
    if scene == "grid":
        u = np.linspace(0, 10, 80).reshape(10, 8)
        ax.quiver(np.arange(8), np.arange(10), u, u)
    else:
        trans = Affine2D().translate(25, 32) + ax.transData
        plot = ax.barbs if scene == "barbs_translated" else ax.quiver
        plot(*_translated_field(), transform=trans)
    return {"datalim": ax.dataLim.get_points().tolist(), "xlim": list(ax.get_xlim()),
            "ylim": list(ax.get_ylim())}


def _arguments(plot, x, y, u, v, c=None):
    fig, ax = _axes()
    args = [x, y, u, v] + ([] if c is None else [c])
    getattr(ax, plot)(*args)
    return True


def _draw(scene):
    fig, ax = _axes()
    if scene == "zero_head":
        X, Y = np.meshgrid(np.arange(10), np.arange(10))
        ax.quiver(np.cos(X), np.sin(Y), headlength=0, headaxislength=0)
    else:
        X, Y = np.meshgrid(np.arange(15), np.arange(10))
        phi = (np.array(rng(4).uniform(150)).reshape(10, 15) - .5) * 150
        ax.quiver(X, Y, np.ones_like(X), np.ones_like(X), angles=phi, scale_units="xy")
    fig.canvas.draw()
    return "drawn"


def test_limits():
    for scene in ["grid", "translated", "barbs_translated"]:
        probes.probe("quiver.limits", _limits, scene=scene)


X2, X3 = [1, 2], [1, 2, 3]


def test_arguments():
    for plot in ["quiver", "barbs"]:
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X2, y=X2, u=X2, v=X2)
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X2, y=X3, u=X2, v=X2)
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X2, y=X2, u=X3, v=X2)
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X2, y=X2, u=X2, v=X3)
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X2, y=X2, u=X2, v=X2,
                     c=X3)
        probes.probe("quiver.arguments", _arguments, plot=plot, x=X3, y=X3,
                     u=[15.0, 15.0, 15.0, 15.0], v=[15.0, 15.0, 15.0, 15.0])


def test_draws():
    for scene in ["zero_head", "angles_xy"]:
        probes.probe("quiver.draw", _draw, scene=scene)
