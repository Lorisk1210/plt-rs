"""matplotlib's non-image tests from ``tests/test_constrainedlayout.py``
that draw and then check where the axes went: their figures, drawn here
in the test suite's default style, record every axes' position (colorbar
axes included). One case per figure a test makes (``_1``, ``_2``, or the
colorbar location).

Adaptations: the tests run with text drawn as rectangles
(``text_placeholders``); these draw real text, so the positions are
matplotlib's for real text and the tests' literals do not apply;
``np.random`` data comes from ``spec_support.rng``; the figures of pyplot
calls are made on the case's figure; the deprecated
``set_constrained_layout`` of ``test_discouraged_api`` and the
``constrained_layout=`` arguments are ``set_layout_engine``.
"""

import numpy as np

from figures._mpl_tests import group_case
from spec_support import rng

GROUP = "mpl_tests_constrainedlayout"
mpl_test = group_case(GROUP)


def _example_plot(ax, fontsize=12):
    """The tests' ``example_plot``."""
    ax.plot([1, 2])
    ax.locator_params(nbins=3)
    ax.set_xlabel("x-label", fontsize=fontsize)
    ax.set_ylabel("y-label", fontsize=fontsize)
    ax.set_title("Title", fontsize=fontsize)


def _example_pcolor(ax, fontsize=12):
    """The tests' ``example_pcolor``."""
    dx, dy = 0.6, 0.6
    y, x = np.mgrid[slice(-3, 3 + dy, dy), slice(-3, 3 + dx, dx)]
    z = (1 - x / 2. + x ** 5 + y ** 3) * np.exp(-x ** 2 - y ** 2)
    pcm = ax.pcolormesh(x, y, z[:-1, :-1], cmap="RdBu_r", vmin=-1., vmax=1., rasterized=True)
    ax.set_xlabel("x-label", fontsize=fontsize)
    ax.set_ylabel("y-label", fontsize=fontsize)
    ax.set_title("Title", fontsize=fontsize)
    return pcm


@mpl_test("identical_subgridspec")
def _(fig):
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(2, 1)
    gsa = gs[0].subgridspec(1, 3)
    gsb = gs[1].subgridspec(1, 3)
    for i in range(3):
        fig.add_subplot(gsa[i])
        fig.add_subplot(gsb[i])


@mpl_test("constrained_layout18")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax2 = ax.twinx()
    _example_plot(ax)
    _example_plot(ax2, fontsize=24)


@mpl_test("constrained_layout19")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax2 = ax.twiny()
    _example_plot(ax)
    _example_plot(ax2, fontsize=24)
    ax2.set_title("")
    ax.set_title("")


@mpl_test("constrained_layout20")
def _(fig):
    gx = np.linspace(-5, 5, 4)
    img = np.hypot(gx, gx[:, None])
    ax = fig.add_axes((0, 0, 1, 1))
    mesh = ax.pcolormesh(gx, gx, img[:-1, :-1])
    fig.colorbar(mesh)


@mpl_test("hidden_axes")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    axs[0, 1].set_visible(False)


def _colorbar_align(fig, location):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for nn, ax in enumerate(axs.flat):
        ax.tick_params(direction="in")
        pc = _example_pcolor(ax)
        cb = fig.colorbar(pc, ax=ax, location=location, shrink=0.6, pad=0.04)
        cb.ax.tick_params(direction="in")
        if nn != 1:
            cb.ax.xaxis.set_ticks([])
            cb.ax.yaxis.set_ticks([])
            ax.set_xticklabels([])
            ax.set_yticklabels([])
    fig.get_layout_engine().set(w_pad=4 / 72, h_pad=4 / 72, hspace=0.1, wspace=0.1)


for _location in ["right", "left", "top", "bottom"]:
    mpl_test(f"colorbar_align_{_location}")(
        lambda fig, location=_location: _colorbar_align(fig, location))


@mpl_test("manually_set_position_1")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    axs[0].set_position([0.2, 0.2, 0.3, 0.3])


@mpl_test("manually_set_position_2")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    axs[0].set_position([0.2, 0.2, 0.3, 0.3])
    pc = axs[0].pcolormesh(rng(3).uniform(400).reshape(20, 20))
    fig.colorbar(pc, ax=axs[0])


@mpl_test("gridspec_addressing")
def _(fig):
    gs = fig.add_gridspec(3, 3)
    fig.add_subplot(gs[0:, 1:])


@mpl_test("discouraged_api")
def _(fig):
    fig.set_layout_engine("constrained", w_pad=0.02, h_pad=0.02)
    fig.subplots()


@mpl_test("kwargs")
def _(fig):
    fig.set_layout_engine("constrained", h_pad=0.02)
    fig.subplots()


@mpl_test("rect_1")
def _(fig):
    fig.set_layout_engine("constrained")
    fig.subplots()
    fig.get_layout_engine().set(rect=[0, 0, 0.5, 0.5])


@mpl_test("rect_2")
def _(fig):
    fig.set_layout_engine("constrained")
    fig.subplots()
    fig.get_layout_engine().set(rect=[0.2, 0.2, 0.3, 0.3])


def _compressed1(fig, nrows, ncols, seed):
    fig.set_layout_engine("compressed")
    axs = fig.subplots(nrows, ncols, sharex=True, sharey=True)
    for nn, ax in enumerate(axs.flat):
        pc = ax.imshow(rng(seed + nn).normal(400).reshape(20, 20))
    fig.colorbar(pc, ax=axs)


@mpl_test("compressed1_1")
def _(fig):
    _compressed1(fig, 3, 2, 10)


@mpl_test("compressed1_2", figsize=(5, 4))
def _(fig):
    _compressed1(fig, 2, 3, 20)


@mpl_test("submerged_subfig", figsize=(4, 5))
def _(fig):
    fig.set_layout_engine("constrained")
    figures = fig.subfigures(3, 1)
    for f in figures.flatten():
        gs = f.add_gridspec(2, 2)
        for i in range(2):
            f.add_subplot(gs[i, 0]).plot()
        f.add_subplot(gs[:, 1]).plot()
