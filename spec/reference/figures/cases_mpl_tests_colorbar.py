"""matplotlib's non-image tests from ``tests/test_colorbar.py`` that only
draw: ``test_colorbarbase`` (a colorbar of a colormap alone, in the
current axes), ``test_colorbar_axes_kw`` (the placement keywords of a
horizontal colorbar), ``test_mappable_no_alpha`` (a colorbar of a bare
mappable whose colormap changes), ``test_mappable_2d_alpha`` (a mesh with
one alpha per cell), ``test_colorbar_no_warning_rcparams_grid_true`` (no
grid on the colorbar of a style with grids), ``test_colorbar_axes_parameters``
(five colorbars of one image taking space from one or both axes) and
``test_colorbar_wrong_figure`` (a colorbar asked of another figure, which
goes into the image's own constrained figure). The tests' assertions are
probes of the ``mpl_colorbar`` area; the cases draw what the tests make.

Adaptations: pyplot's current axes and image become the ones named; the
axes sequences of ``test_colorbar_axes_parameters`` (an array, a list, a
tuple and dict values) all come as lists; ``test_colorbar_wrong_figure``
draws the constrained figure, the tight one getting nothing; plt-rs's
colorbars keep a copy of a bare mappable, so ``test_mappable_no_alpha``'s
colorbar takes the new colormap with ``update_normal``.
"""

import warnings

import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import cm
from matplotlib.colorbar import Colorbar
from matplotlib.figure import Figure

from figures._mpl_tests import group_case

GROUP = "mpl_tests_colorbar"
mpl_test = group_case(GROUP)


@mpl_test("colorbarbase")
def _(fig):
    Colorbar(fig.gca(), cmap=plt.colormaps["bone"])


@mpl_test("colorbar_axes_kw")
def _(fig):
    im = fig.gca().imshow([[1, 2], [3, 4]])
    fig.colorbar(im, orientation="horizontal", fraction=0.2, pad=0.2, shrink=0.5, aspect=10,
                 anchor=(0., 0.), panchor=(0., 1.))


@mpl_test("mappable_no_alpha")
def _(fig):
    ax = fig.subplots()
    sm = cm.ScalarMappable(norm=mcolors.Normalize(), cmap="viridis")
    fig.colorbar(sm, ax=ax)
    sm.set_cmap("plasma")


@mpl_test("mappable_2d_alpha")
def _(fig):
    ax = fig.subplots()
    x = np.arange(1, 5).reshape(2, 2) / 4
    pc = ax.pcolormesh(x, alpha=x)
    fig.colorbar(pc, ax=ax)


@mpl_test("colorbar_no_warning_rcparams_grid_true", rc={"axes.grid": True})
def _(fig):
    ax = fig.subplots()
    ax.grid(False)
    im = ax.pcolormesh([0, 1], [0, 1], [[1]])
    fig.colorbar(im)


@mpl_test("colorbar_axes_parameters")
def _(fig):
    ax = fig.subplots(2)
    im = ax[0].imshow([[0, 1], [2, 3]])
    fig.colorbar(im, ax=list(ax))
    fig.colorbar(im, ax=ax[0])
    fig.colorbar(im, ax=list(ax))
    fig.colorbar(im, ax=list(ax))
    fig.colorbar(im, ax=list(ax))


@mpl_test("colorbar_wrong_figure")
def _(fig):
    fig.set_layout_engine("constrained")
    im = fig.add_subplot().imshow([[0, 1]])
    fig_tl = Figure(layout="tight")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig_tl.colorbar(im)
