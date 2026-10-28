"""matplotlib's own image tests from ``tests/test_figure.py``,
``tests/test_constrainedlayout.py``, ``tests/test_tightlayout.py`` and
``tests/test_bbox_tight.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test.

Each case is named after its test without the ``test_`` prefix (the tests
of ``TestSubplotMosaic`` get a ``subplot_mosaic_`` prefix instead). A test
drawing several figures becomes one case per figure (``_1``, ``_2``), a
parametrized test one case per parameter set that draws something different
(sets differing only in list-versus-tuple containers draw the same figure).
Tests without a style of their own draw under ``classic`` plus
``_classic_test_patch`` (matplotlib's test fixture applies it to
``check_figures_equal`` tests too) on classic's 8x6 in canvas saved at
100 dpi; ``mpl20`` needs no rc. ``savefig_kwarg`` options become the
matching ``savefig.*`` rc parameters, ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and the tests' own
rcParams changes become the case's ``rc``. pyplot calls act on the case's
figure. The layout tests run under matplotlib's ``text_placeholders``
fixture, which draws text as rectangles; the cases draw the real text.
numpy's random data is replaced by ``spec_support.rng`` streams with the
tests' seeds.

Not ported: ``test_bbox`` (saves with an explicit ``bbox_inches`` box) and
``test_bbox_inches_tight_layout_constrained`` (``pad_inches="layout"``),
whose ``savefig`` arguments have no rc parameter.
"""

import functools
import warnings

import matplotlib as mpl
import matplotlib.pyplot as plt
import matplotlib.style
import numpy as np
from matplotlib import gridspec, ticker
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib.lines import Line2D
from matplotlib.testing.decorators import remove_ticks_and_titles
from mpl_toolkits.axes_grid1.inset_locator import inset_axes

from spec_support import case, rng

GROUP = "mpl_figure"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def image_test(name, *, classic=False, figsize=None, remove_text=False, rc=None):
    """Register ``build`` as ``mpl_figure/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, rc=params)(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


#: ``savefig_kwarg={"bbox_inches": "tight"}``.
TIGHT_BBOX = {"savefig.bbox": "tight"}
#: ``savefig_kwarg={"facecolor": "teal"}``.
TEAL = {"savefig.facecolor": "teal"}


def _randn(stream, n=30):
    """``np.random.randn(n, n)``, drawn from ``stream``."""
    return stream.normal(n * n).reshape(n, n)


# --- test_figure.py ------------------------------------------------------


@image_test("align_labels")
def _(fig):
    fig.set_layout_engine("tight")
    gs = gridspec.GridSpec(3, 3)

    ax = fig.add_subplot(gs[0, :2])
    ax.plot(np.arange(0, 1e6, 1000))
    ax.set_ylabel("Ylabel0 0")
    ax = fig.add_subplot(gs[0, -1])
    ax.plot(np.arange(0, 1e4, 100))

    for i in range(3):
        ax = fig.add_subplot(gs[1, i])
        ax.set_ylabel(f"YLabel1 {i}")
        ax.set_xlabel(f"XLabel1 {i}")
        if i in [0, 2]:
            ax.xaxis.set_label_position("top")
            ax.xaxis.tick_top()
        if i == 0:
            for tick in ax.get_xticklabels():
                tick.set_rotation(90)
        if i == 2:
            ax.yaxis.set_label_position("right")
            ax.yaxis.tick_right()

    for i in range(3):
        ax = fig.add_subplot(gs[2, i])
        ax.set_xlabel(f"XLabel2 {i}")
        ax.set_ylabel(f"YLabel2 {i}")

        if i == 2:
            ax.plot(np.arange(0, 1e4, 10))
            ax.yaxis.set_label_position("right")
            ax.yaxis.tick_right()
            for tick in ax.get_xticklabels():
                tick.set_rotation(90)

    fig.align_labels()


def _align_titles(fig, layout):
    fig.set_layout_engine(layout)
    axs = fig.subplots(1, 2, width_ratios=[2, 1])

    ax = axs[0]
    ax.plot(np.arange(0, 1e6, 1000))
    ax.set_title("Title0 left", loc="left")
    ax.set_title("Title0 center", loc="center")
    ax.set_title("Title0 right", loc="right")

    ax = axs[1]
    ax.plot(np.arange(0, 1e4, 100))
    ax.set_title("Title1")
    ax.set_xlabel("Xlabel0")
    ax.xaxis.set_label_position("top")
    ax.xaxis.tick_top()
    for tick in ax.get_xticklabels():
        tick.set_rotation(90)

    fig.align_titles()


image_test("align_titles_1")(functools.partial(_align_titles, layout="tight"))
image_test("align_titles_2")(functools.partial(_align_titles, layout="constrained"))


@image_test("figure")
def _(fig):
    # The named figure "today"; the red line goes to another figure.
    ax = fig.add_subplot()
    ax.set_title("today")
    ax.plot(np.arange(5))


@image_test("figure_legend")
def _(fig):
    axs = fig.subplots(2)
    axs[0].plot([0, 1], [1, 0], label="x", color="g")
    axs[0].plot([0, 1], [0, 1], label="y", color="r")
    axs[0].plot([0, 1], [0.5, 0.5], label="y", color="k")

    axs[1].plot([0, 1], [1, 0], label="_y", color="r")
    axs[1].plot([0, 1], [0, 1], label="z", color="b")
    fig.legend()


@image_test("suptitle")
def _(fig):
    fig.subplots()
    fig.suptitle("hello", color="r")
    fig.suptitle("title", color="g", rotation=30)


@classic("alpha", figsize=(2, 1),
         rc={"savefig.facecolor": (0, 1, 0.4), "savefig.edgecolor": "none"})
def _(fig):
    fig.set_facecolor((0, 1, 0.4))
    fig.patch.set_alpha(0.4)
    fig.patches.append(mpatches.CirclePolygon(
        [20, 20], radius=15, alpha=0.6, facecolor="red"))


@classic("savefig_transparent", rc={"savefig.transparent": True})
def _(fig):
    gs1 = fig.add_gridspec(3, 3, left=0.05, wspace=0.05)
    f1 = fig.add_subfigure(gs1[:, :])
    f2 = f1.add_subfigure(gs1[0, 0])

    ax12 = f2.add_subplot(gs1[:, :])

    ax1 = f1.add_subplot(gs1[:-1, :])
    iax1 = ax1.inset_axes([.1, .2, .3, .4])
    iax2 = iax1.inset_axes([.1, .2, .3, .4])

    ax2 = fig.add_subplot(gs1[-1, :-1])
    ax3 = fig.add_subplot(gs1[-1, -1])

    for ax in [ax12, ax1, iax1, iax2, ax2, ax3]:
        ax.set(xticks=[], yticks=[])
        ax.spines[:].set_visible(False)


@classic("tightlayout_autolayout_deconflict")
def _(fig):
    with mpl.rc_context({"figure.autolayout": True}):
        fig.subplots(ncols=2)
        fig.tight_layout(w_pad=10)


@classic("add_artist")
def _(fig):
    fig.subplots()
    l1 = Line2D([.2, .7], [.7, .7], gid="l1")
    l2 = Line2D([.2, .7], [.8, .8], gid="l2")
    r1 = mpatches.Circle((20, 20), 100, transform=None, gid="C1")
    r2 = mpatches.Circle((.7, .5), .05, gid="C2")
    r3 = mpatches.Circle((4.5, .8), .55, transform=fig.dpi_scale_trans,
                         facecolor="crimson", gid="C3")
    for a in [l1, l2, r1, r2, r3]:
        fig.add_artist(a)
    l2.remove()


@image_test("tightbbox_box_aspect", remove_text=True, rc={**TIGHT_BBOX, **TEAL})
def _(fig):
    gs = fig.add_gridspec(1, 2)
    ax1 = fig.add_subplot(gs[0, 0])
    ax2 = fig.add_subplot(gs[0, 1], projection="3d")
    ax1.set_box_aspect(.5)
    ax2.set_box_aspect((2, 1, 1))


@classic("animated_with_canvas_change")
def _(fig):
    ax = fig.subplots()
    ax.plot(range(5), animated=True)


# --- test_figure.py: TestSubplotMosaic -----------------------------------


def _titled_mosaic(fig, layout, **kwargs):
    """``subplot_mosaic`` with every axes titled by its key."""
    for key, ax in fig.subplot_mosaic(layout, **kwargs).items():
        ax.set_title(key)


classic("subplot_mosaic_basic_str")(functools.partial(
    _titled_mosaic, layout=[["A", "A", "B"], ["C", "D", "B"]]))
classic("subplot_mosaic_basic_int")(functools.partial(
    _titled_mosaic, layout=[[1, 1, 2], [3, 4, 2]]))


# TestSubplotMosaic.test_all_nested
@classic("subplot_mosaic_all_nested")
def _(fig):
    x = [["A", "B"], ["C", "D"]]
    y = [["E", "F"], ["G", "H"]]
    fig.set_layout_engine("constrained")
    for ax in fig.subplot_mosaic([[x, y]]).values():
        ax.set_title(ax.get_label())


@classic("subplot_mosaic_nested")
def _(fig):
    fig.set_layout_engine("constrained")
    x = [["A", "B"], ["C", "D"]]
    _titled_mosaic(fig, [["F"], [x]])


# TestSubplotMosaic.test_nested_tuple
@classic("subplot_mosaic_nested_tuple")
def _(fig):
    xt = (("A", "B", "B"), ("C", "C", "D"))
    fig.subplot_mosaic([["F"], [xt]])


classic("subplot_mosaic_empty_str")(functools.partial(
    _titled_mosaic, layout=[["A", None], [None, "B"]], empty_sentinel=None))
classic("subplot_mosaic_empty_int")(functools.partial(
    _titled_mosaic, layout=[[1, 0], [0, 2]], empty_sentinel=0))


@classic("subplot_mosaic_subplot_kw")
def _(fig):
    fig.subplot_mosaic([[1, 2]], subplot_kw={})


@classic("subplot_mosaic_subplot_kw_polar")
def _(fig):
    fig.subplot_mosaic([[1, 2]], subplot_kw={"projection": "polar"})


@classic("subplot_mosaic_per_subplot_kw")
def _(fig):
    fig.subplot_mosaic(
        "AB;CD",
        subplot_kw={"facecolor": "red"},
        per_subplot_kw={
            "D": {"facecolor": "blue"},
            "BC": {"facecolor": "green"},
        },
    )


# TestSubplotMosaic.test_single_str_input
classic("subplot_mosaic_single_str_input_1")(
    lambda fig: fig.subplot_mosaic("AAA\nBBB"))
classic("subplot_mosaic_single_str_input_2")(
    lambda fig: fig.subplot_mosaic("ABC\nDEF"))


# TestSubplotMosaic.test_hashable_keys
@classic("subplot_mosaic_hashable_keys")
def _(fig):
    fig.subplot_mosaic([[object(), object()]])


# --- test_figure.py: subfigures ------------------------------------------


@image_test("subfigure", rc=TEAL)
def _(fig):
    stream = rng(19680801)
    fig.set_layout_engine("constrained")
    sub = fig.subfigures(1, 2)

    axs = sub[0].subplots(2, 2)
    for ax in axs.flat:
        pc = ax.pcolormesh(_randn(stream), vmin=-2, vmax=2)
    sub[0].colorbar(pc, ax=axs)
    sub[0].suptitle("Left Side")
    sub[0].set_facecolor("white")

    axs = sub[1].subplots(1, 3)
    for ax in axs.flat:
        pc = ax.pcolormesh(_randn(stream), vmin=-2, vmax=2)
    sub[1].colorbar(pc, ax=axs, location="bottom")
    sub[1].suptitle("Right Side")
    sub[1].set_facecolor("white")

    fig.suptitle("Figure suptitle", fontsize="xx-large")

    # The draw order of subfigures.
    leg = fig.legend(handles=[Line2D([0], [0], label=f"Line{i}") for i in range(5)],
                     loc="center")
    sub[0].set_zorder(leg.get_zorder() - 1)
    sub[1].set_zorder(leg.get_zorder() + 1)


@image_test("subfigure_ss", rc=TEAL)
def _(fig):
    stream = rng(19680801)
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(1, 2)

    sub = fig.add_subfigure(gs[0], facecolor="pink")

    axs = sub.subplots(2, 2)
    for ax in axs.flat:
        pc = ax.pcolormesh(_randn(stream), vmin=-2, vmax=2)
    sub.colorbar(pc, ax=axs)
    sub.suptitle("Left Side")

    ax = fig.add_subplot(gs[1])
    ax.plot(np.arange(20))
    ax.set_title("Axes")

    fig.suptitle("Figure suptitle", fontsize="xx-large")


@image_test("subfigure_double", figsize=(10, 8), rc=TEAL)
def _(fig):
    stream = rng(19680801)
    fig.set_layout_engine("constrained")

    fig.suptitle("fig")

    subfigs = fig.subfigures(1, 2, wspace=0.07)

    subfigs[0].set_facecolor("coral")
    subfigs[0].suptitle("subfigs[0]")

    subfigs[1].set_facecolor("coral")
    subfigs[1].suptitle("subfigs[1]")

    subfigsnest = subfigs[0].subfigures(2, 1, height_ratios=[1, 1.4])
    subfigsnest[0].suptitle("subfigsnest[0]")
    subfigsnest[0].set_facecolor("r")
    axsnest0 = subfigsnest[0].subplots(1, 2, sharey=True)
    for ax in axsnest0:
        fontsize = 12
        pc = ax.pcolormesh(_randn(stream), vmin=-2.5, vmax=2.5)
        ax.set_xlabel("x-label", fontsize=fontsize)
        ax.set_ylabel("y-label", fontsize=fontsize)
        ax.set_title("Title", fontsize=fontsize)
    subfigsnest[0].colorbar(pc, ax=axsnest0)

    subfigsnest[1].suptitle("subfigsnest[1]")
    subfigsnest[1].set_facecolor("g")
    axsnest1 = subfigsnest[1].subplots(3, 1, sharex=True)
    for nn, ax in enumerate(axsnest1):
        ax.set_ylabel(f"ylabel{nn}")
    subfigsnest[1].supxlabel("supxlabel")
    subfigsnest[1].supylabel("supylabel")

    subfigs[1].subplots(2, 2)


@image_test("subfigure_scatter_size", remove_text=True)
def _(fig):
    gs = fig.add_gridspec(1, 2)
    ax0 = fig.add_subplot(gs[1])
    ax0.scatter([1, 2, 3], [1, 2, 3], s=30, marker="s")
    ax0.scatter([3, 4, 5], [1, 2, 3], s=[20, 30, 40], marker="s")

    sfig = fig.add_subfigure(gs[0])
    axs = sfig.subplots(1, 2)
    for ax in [ax0, axs[0]]:
        ax.scatter([1, 2, 3], [1, 2, 3], s=30, marker="s", color="r")
        ax.scatter([3, 4, 5], [1, 2, 3], s=[20, 30, 40], marker="s", color="g")


@classic("rcparams", rc={"figure.labelweight": "bold", "figure.labelsize": 15,
                         "figure.titleweight": "light", "figure.titlesize": 20})
def _(fig):
    fig.supxlabel("xlabel")
    fig.supylabel("ylabel")
    fig.suptitle("Title")


# --- test_constrainedlayout.py -------------------------------------------


def _example_plot(ax, fontsize=12, nodec=False):
    """The layout tests' ``example_plot``."""
    ax.plot([1, 2])
    ax.locator_params(nbins=3)
    if not nodec:
        ax.set_xlabel("x-label", fontsize=fontsize)
        ax.set_ylabel("y-label", fontsize=fontsize)
        ax.set_title("Title", fontsize=fontsize)
    else:
        ax.set_xticklabels([])
        ax.set_yticklabels([])


def _example_pcolor(ax, fontsize=12):
    """The layout tests' ``example_pcolor``."""
    dx, dy = 0.6, 0.6
    y, x = np.mgrid[slice(-3, 3 + dy, dy),
                    slice(-3, 3 + dx, dx)]
    z = (1 - x / 2. + x ** 5 + y ** 3) * np.exp(-x ** 2 - y ** 2)
    pcm = ax.pcolormesh(x, y, z[:-1, :-1], cmap="RdBu_r", vmin=-1., vmax=1.,
                        rasterized=True)
    ax.set_xlabel("x-label", fontsize=fontsize)
    ax.set_ylabel("y-label", fontsize=fontsize)
    ax.set_title("Title", fontsize=fontsize)
    return pcm


@image_test("constrained_layout1")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.add_subplot()
    _example_plot(ax, fontsize=24)


@image_test("constrained_layout2")
def _(fig):
    fig.set_layout_engine("constrained")
    for ax in fig.subplots(2, 2).flat:
        _example_plot(ax, fontsize=24)


@image_test("constrained_layout3")
def _(fig):
    fig.set_layout_engine("constrained")
    for nn, ax in enumerate(fig.subplots(2, 2).flat):
        pcm = _example_pcolor(ax, fontsize=24)
        pad = 0.08 if nn == 3 else 0.02
        fig.colorbar(pcm, ax=ax, pad=pad)


@image_test("constrained_layout4")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for ax in axs.flat:
        pcm = _example_pcolor(ax, fontsize=24)
    fig.colorbar(pcm, ax=axs, pad=0.01, shrink=0.6)


@image_test("constrained_layout5")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for ax in axs.flat:
        pcm = _example_pcolor(ax, fontsize=24)
    fig.colorbar(pcm, ax=axs, use_gridspec=False, pad=0.01, shrink=0.6,
                 location="bottom")


@image_test("constrained_layout6")
def _(fig):
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(1, 2)
    gsl = gs[0].subgridspec(2, 2)
    gsr = gs[1].subgridspec(1, 2)
    for spec in gsl:
        ax = fig.add_subplot(spec)
        _example_plot(ax, fontsize=12)
    ax.set_xlabel("x-label\nMultiLine")
    axsr = []
    for spec in gsr:
        ax = fig.add_subplot(spec)
        axsr += [ax]
        pcm = _example_pcolor(ax, fontsize=12)

    fig.colorbar(pcm, ax=axsr, pad=0.01, shrink=0.99, location="bottom",
                 ticks=ticker.MaxNLocator(nbins=5))


@image_test("constrained_layout8", figsize=(10, 5))
def _(fig):
    fig.set_layout_engine("constrained")
    gs = gridspec.GridSpec(3, 5, figure=fig)
    axs = []
    for j in [0, 1]:
        ilist = [1] if j == 0 else [0, 4]
        for i in ilist:
            ax = fig.add_subplot(gs[j, i])
            axs += [ax]
            _example_pcolor(ax, fontsize=9)
            if i > 0:
                ax.set_ylabel("")
            if j < 1:
                ax.set_xlabel("")
            ax.set_title("")
    ax = fig.add_subplot(gs[2, :])
    axs += [ax]
    pcm = _example_pcolor(ax, fontsize=9)

    fig.colorbar(pcm, ax=axs, pad=0.01, shrink=0.6)


@image_test("constrained_layout9")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2, sharex=False, sharey=False)
    for ax in axs.flat:
        pcm = _example_pcolor(ax, fontsize=24)
        ax.set_xlabel("")
        ax.set_ylabel("")
    ax.set_aspect(2.)
    fig.colorbar(pcm, ax=axs, pad=0.01, shrink=0.6)
    fig.suptitle("Test Suptitle", fontsize=28)


@image_test("constrained_layout10")
def _(fig):
    fig.set_layout_engine("constrained")
    for ax in fig.subplots(2, 2).flat:
        ax.plot(np.arange(12), label="This is a label")
    ax.legend(loc="center left", bbox_to_anchor=(0.8, 0.5))


def _constrained_layout11(fig, width_ratios=None, height_ratios=None):
    fig.set_layout_engine("constrained")
    gs0 = gridspec.GridSpec(1, 2, figure=fig, width_ratios=width_ratios)
    gsl = gridspec.GridSpecFromSubplotSpec(1, 2, gs0[0])
    gsl0 = gridspec.GridSpecFromSubplotSpec(2, 2, gsl[1], height_ratios=height_ratios)
    ax = fig.add_subplot(gs0[1])
    _example_plot(ax, fontsize=9)
    axs = []
    for spec in gsl0:
        ax = fig.add_subplot(spec)
        axs += [ax]
        pcm = _example_pcolor(ax, fontsize=9)
    fig.colorbar(pcm, ax=axs, shrink=0.6, aspect=70.)
    ax = fig.add_subplot(gsl[0])
    _example_plot(ax, fontsize=9)


image_test("constrained_layout11", figsize=(13, 3))(_constrained_layout11)
image_test("constrained_layout11rat", figsize=(10, 3))(functools.partial(
    _constrained_layout11, width_ratios=[6, 1], height_ratios=[2, 1]))


@image_test("constrained_layout12", figsize=(6, 8))
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = gridspec.GridSpec(6, 2, figure=fig)

    ax1 = fig.add_subplot(gs0[:3, 1])
    ax2 = fig.add_subplot(gs0[3:, 1])

    _example_plot(ax1, fontsize=18)
    _example_plot(ax2, fontsize=18)

    ax = fig.add_subplot(gs0[0:2, 0])
    _example_plot(ax, nodec=True)
    ax = fig.add_subplot(gs0[2:4, 0])
    _example_plot(ax, nodec=True)
    ax = fig.add_subplot(gs0[4:, 0])
    _example_plot(ax, nodec=True)
    ax.set_xlabel("x-label")


def _padded_colorbars(fig, **padding):
    fig.set_layout_engine("constrained")
    for ax in fig.subplots(2, 2).flat:
        pcm = _example_pcolor(ax, fontsize=12)
        fig.colorbar(pcm, ax=ax, shrink=0.6, aspect=20., pad=0.02)
    fig.get_layout_engine().set(**padding)


image_test("constrained_layout13")(functools.partial(
    _padded_colorbars, w_pad=24. / 72., h_pad=24. / 72.))
image_test("constrained_layout14")(functools.partial(
    _padded_colorbars, w_pad=3. / 72., h_pad=3. / 72., hspace=0.2, wspace=0.2))


@image_test("constrained_layout15", rc={"figure.constrained_layout.use": True})
def _(fig):
    for ax in fig.subplots(2, 2).flat:
        _example_plot(ax, fontsize=12)


@image_test("constrained_layout16")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    _example_plot(ax, fontsize=12)
    fig.add_axes((0.2, 0.2, 0.4, 0.4))


@image_test("constrained_layout17")
def _(fig):
    fig.set_layout_engine("constrained")
    gs = gridspec.GridSpec(3, 3, figure=fig)

    for spec in [gs[0, 0], gs[0, 1:], gs[1:, 0:2], gs[1:, -1]]:
        _example_plot(fig.add_subplot(spec))


@image_test("colorbar_location", remove_text=True)
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(4, 5)
    for ax in axs.flat:
        pcm = _example_pcolor(ax)
        ax.set_xlabel("")
        ax.set_ylabel("")
    fig.colorbar(pcm, ax=axs[:, 1], shrink=0.4)
    fig.colorbar(pcm, ax=axs[-1, :2], shrink=0.5, location="bottom")
    fig.colorbar(pcm, ax=axs[0, 2:], shrink=0.5, location="bottom", pad=0.05)
    fig.colorbar(pcm, ax=axs[-2, 3:], shrink=0.5, location="top")
    fig.colorbar(pcm, ax=axs[0, 0], shrink=0.5, location="left")
    fig.colorbar(pcm, ax=axs[1:3, 2], shrink=0.5, location="right")


@image_test("colorbars_no_overlapV", figsize=(2, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1, sharex=True, sharey=True)
    for ax in axs:
        ax.yaxis.set_major_formatter(ticker.NullFormatter())
        ax.tick_params(axis="both", direction="in")
        im = ax.imshow([[1, 2], [3, 4]])
        fig.colorbar(im, ax=ax, orientation="vertical")
    fig.suptitle("foo")


@image_test("colorbars_no_overlapH", figsize=(4, 2))
def _(fig):
    fig.set_layout_engine("constrained")
    fig.suptitle("foo")
    axs = fig.subplots(1, 2, sharex=True, sharey=True)
    for ax in axs:
        ax.yaxis.set_major_formatter(ticker.NullFormatter())
        ax.tick_params(axis="both", direction="in")
        im = ax.imshow([[1, 2], [3, 4]])
        fig.colorbar(im, ax=ax, orientation="horizontal")


@image_test("bboxtight", remove_text=True, rc=TIGHT_BBOX)
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.set_aspect(1.)


def _compressed_colorbars(fig, nrows, ncols, orientation):
    fig.set_layout_engine("compressed")
    arr = np.arange(100).reshape((10, 10))
    for ax in fig.subplots(nrows, ncols).flat:
        im = ax.imshow(arr)
        fig.colorbar(im, ax=ax, orientation=orientation)


@image_test("compressed_suptitle_colorbar", figsize=(4, 2))
def _(fig):
    _compressed_colorbars(fig, 1, 2, "vertical")
    fig.suptitle("Title")


@image_test("compressed_supylabel_colorbar", figsize=(3, 4))
def _(fig):
    _compressed_colorbars(fig, 2, 1, "horizontal")
    fig.supylabel("Title")


@image_test("submerged_with_colorbar")
def _(fig):
    fig.set_layout_engine("constrained")
    ax_dict = fig.subplot_mosaic("AABBCC;DDDEEE")
    cf = ax_dict["A"].contourf([[0, 1], [2, 3]])
    fig.colorbar(cf)


# --- test_tightlayout.py -------------------------------------------------


@image_test("tight_layout1")
def _(fig):
    ax = fig.subplots()
    _example_plot(ax, fontsize=24)
    fig.tight_layout()


@image_test("tight_layout2")
def _(fig):
    for ax in fig.subplots(nrows=2, ncols=2).flat:
        _example_plot(ax)
    fig.tight_layout()


@image_test("tight_layout3")
def _(fig):
    for spec in [221, 223, 122]:
        _example_plot(fig.add_subplot(spec))
    fig.tight_layout()


@image_test("tight_layout4")
def _(fig):
    for loc, rowspan, colspan in [((0, 0), 1, 1), ((0, 1), 1, 2),
                                  ((1, 0), 2, 2), ((1, 2), 2, 1)]:
        ax = plt.subplot2grid((3, 3), loc, rowspan=rowspan, colspan=colspan, fig=fig)
        _example_plot(ax)
    fig.tight_layout()


@image_test("tight_layout5")
def _(fig):
    ax = fig.add_subplot()
    arr = np.arange(100).reshape((10, 10))
    ax.imshow(arr, interpolation="none")
    fig.tight_layout()


@image_test("tight_layout6")
def _(fig):
    # tight_layout warns that it cannot do this fully automatically; the
    # layout is edited by hand.
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        gs1 = mpl.gridspec.GridSpec(2, 1)
        ax1 = fig.add_subplot(gs1[0])
        ax2 = fig.add_subplot(gs1[1])

        _example_plot(ax1)
        _example_plot(ax2)

        gs1.tight_layout(fig, rect=[0, 0, 0.5, 1])

        gs2 = mpl.gridspec.GridSpec(3, 1)

        for ss in gs2:
            ax = fig.add_subplot(ss)
            _example_plot(ax)
            ax.set_title("")
            ax.set_xlabel("")

        ax.set_xlabel("x-label", fontsize=12)

        gs2.tight_layout(fig, rect=[0.5, 0, 1, 1], h_pad=0.45)

        top = min(gs1.top, gs2.top)
        bottom = max(gs1.bottom, gs2.bottom)

        gs1.tight_layout(fig, rect=[None, 0 + (bottom-gs1.bottom),
                                    0.5, 1 - (gs1.top-top)])
        gs2.tight_layout(fig, rect=[0.5, 0 + (bottom-gs2.bottom),
                                    None, 1 - (gs2.top-top)],
                         h_pad=0.45)


@image_test("tight_layout7")
def _(fig):
    fontsize = 24
    ax = fig.subplots()
    ax.plot([1, 2])
    ax.locator_params(nbins=3)
    ax.set_xlabel("x-label", fontsize=fontsize)
    ax.set_ylabel("y-label", fontsize=fontsize)
    ax.set_title("Left Title", loc="left", fontsize=fontsize)
    ax.set_title("Right Title", loc="right", fontsize=fontsize)
    fig.tight_layout()


@image_test("tight_layout8")
def _(fig):
    fig.set_layout_engine(layout="tight", pad=0.1)
    ax = fig.add_subplot()
    _example_plot(ax, fontsize=24)


@image_test("tight_layout9")
def _(fig):
    axarr = fig.subplots(2, 2)
    axarr[1][1].set_visible(False)
    fig.tight_layout()


# --- test_bbox_tight.py --------------------------------------------------


@image_test("bbox_inches_tight", remove_text=True, rc=TIGHT_BBOX)
def _(fig):
    data = [[66386, 174296, 75131, 577908, 32015],
            [58230, 381139, 78045, 99308, 160454],
            [89135, 80552, 152558, 497981, 603535],
            [78415, 81858, 150656, 193263, 69638],
            [139361, 331509, 343164, 781380, 52269]]

    col_labels = ("Freeze", "Wind", "Flood", "Quake", "Hail")
    row_labels = [f"{x} year" for x in (100, 50, 20, 10, 5)]

    rows = len(data)
    ind = np.arange(len(col_labels)) + 0.3  # the x locations for the groups
    cell_text = []
    width = 0.4  # the width of the bars
    yoff = np.zeros(len(col_labels))
    # the bottom values for stacked bar chart
    ax = fig.subplots(1, 1)
    for row in range(rows):
        ax.bar(ind, data[row], width, bottom=yoff, align="edge")
        yoff = yoff + data[row]
        cell_text.append([f"{x / 1000:1.1f}" for x in yoff])
    ax.set_xticks([])
    ax.set_xlim(0, 5)
    ax.legend(["1", "2", "3", "4", "5"], loc=(1.2, 0.2))
    fig.legend(["a", "b", "c", "d", "e"], bbox_to_anchor=(0, 0.2), loc="lower left")
    # Add a table at the bottom of the axes
    cell_text.reverse()
    ax.table(cellText=cell_text, rowLabels=row_labels, colLabels=col_labels,
             loc="bottom")


@image_test("bbox_inches_tight_suptile_legend", rc=TIGHT_BBOX)
def _(fig):
    ax = fig.add_subplot()
    ax.plot(np.arange(10), label="a straight line")
    ax.legend(bbox_to_anchor=(0.9, 1), loc="upper left")
    ax.set_title("Axis title")
    fig.suptitle("Figure title")

    # put an extra long y tick on to see that the bbox is accounted for
    def y_formatter(y, pos):
        if int(y) == 4:
            return "The number 4"
        else:
            return str(y)
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(y_formatter))

    ax.set_xlabel("X axis")


# test_bbox_inches_tight_suptitle_non_default (its baseline image is named
# "suptile")
@image_test("bbox_inches_tight_suptile_non_default", rc=TIGHT_BBOX)
def _(fig):
    fig.subplots()
    fig.suptitle("Booo", x=0.5, y=1.1)


@classic("bbox_inches_tight_clipping", remove_text=True, rc=TIGHT_BBOX)
def _(fig):
    ax = fig.add_subplot()
    ax.scatter(np.arange(10), np.arange(10))
    ax.set_xlim(0, 5)
    ax.set_ylim(0, 5)

    # make a massive rectangle and clip it with a path
    patch = mpatches.Rectangle([-50, -50], 100, 100,
                               transform=ax.transData,
                               facecolor="blue", alpha=0.5)

    path = mpath.Path.unit_regular_star(5).deepcopy()
    path.vertices *= 0.25
    patch.set_clip_path(path, transform=ax.transAxes)
    fig.artists.append(patch)


@image_test("bbox_inches_tight_raster", remove_text=True, rc=TIGHT_BBOX)
def _(fig):
    ax = fig.subplots()
    ax.plot([1.0, 2.0], rasterized=True)


@image_test("bbox_inches_fixed_aspect", remove_text=True,
            rc={**TIGHT_BBOX, "figure.constrained_layout.use": True})
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1])
    ax.set_xlim(0, 1)
    ax.set_aspect("equal")


@image_test("bbox_inches_inset_rasterized", remove_text=True, rc=TIGHT_BBOX)
def _(fig):
    ax = fig.subplots()

    arr = np.arange(100).reshape(10, 10)
    im = ax.imshow(arr)
    inset = inset_axes(
        ax, width="10%", height="30%", loc="upper left",
        bbox_to_anchor=(0.045, 0., 1, 1), bbox_transform=ax.transAxes)

    fig.colorbar(im, cax=inset, orientation="horizontal")
