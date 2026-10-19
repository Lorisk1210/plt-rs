"""matplotlib's gallery section "Subplots, axes and figures": grids of
subplots (``subplots``, ``GridSpec``, ``subplot2grid``, nested grids and
subfigures), constrained and tight layout, spacing, margins and box
aspects, shared, twinned, inverted and secondary axes, aligned labels and
titles, figure titles and sizes, zoom effects and insets, broken axes,
geographic projections and spans.

Each case ports one figure of an example (``<example>_<n>`` when the
example draws several; figures identical to an earlier one of the same
example are left out: ``subplots_demo``'s third and ``figure_size_units``'
fourth). Random data comes from ``spec_support.rng`` seeded with the
example's seed, a fresh generator per figure; sample data files are
replaced by synthetic data of the same shape (``_stocks``,
``_bivariate_normal``). Callbacks that only react to a draw are run
directly once the figure is built.

Covered elsewhere: ``multiple_yaxis_with_spines`` is
``spines/multiple_yaxis`` and ``geo_demo``'s Mollweide figure is
``geo/mollweide_grid_title``.
"""

import datetime
import itertools
import warnings

import matplotlib as mpl
import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.gridspec import GridSpec, GridSpecFromSubplotSpec
from matplotlib.patches import Circle, Polygon
from matplotlib.transforms import Bbox, TransformedBbox, blended_transform_factory
from mpl_toolkits.axes_grid1.inset_locator import BboxConnector, BboxConnectorPatch, BboxPatch

from spec_support import case, rng

SEED = 19680801

GROUP = "gallery_subplots"


def _label_axes(fig):
    """``annotate_axes`` / ``format_axes``: name every axes in its middle
    and hide the tick labels."""
    for i, ax in enumerate(fig.axes):
        ax.text(0.5, 0.5, "ax%d" % (i + 1), va="center", ha="center")
        ax.tick_params(labelbottom=False, labelleft=False)


def _example_plot(ax, sizes=(12, 12, 14)):
    """The layout examples' ``example_plot``: a line with x label, y label
    and title in the given font sizes."""
    xsize, ysize, title_size = sizes
    ax.plot([1, 2])
    ax.set_xlabel("x-label", fontsize=xsize)
    ax.set_ylabel("y-label", fontsize=ysize)
    ax.set_title("Title", fontsize=title_size)


# --- subplot / subplots ----------------------------------------------------

_X1 = np.linspace(0.0, 5.0)
_Y1 = np.cos(2 * np.pi * _X1) * np.exp(-_X1)
_X2 = np.linspace(0.0, 2.0)
_Y2 = np.cos(2 * np.pi * _X2)


@case(f"{GROUP}/subplot_1")
def _(fig):
    ax1, ax2 = fig.subplots(2, 1)
    fig.suptitle("A tale of 2 subplots")
    ax1.plot(_X1, _Y1, "o-")
    ax1.set_ylabel("Damped oscillation")
    ax2.plot(_X2, _Y2, ".-")
    ax2.set_xlabel("time (s)")
    ax2.set_ylabel("Undamped")


@case(f"{GROUP}/subplot_2")
def _(fig):
    ax = fig.add_subplot(2, 1, 1)
    ax.plot(_X1, _Y1, "o-")
    ax.set_title("A tale of 2 subplots")
    ax.set_ylabel("Damped oscillation")
    ax = fig.add_subplot(2, 1, 2)
    ax.plot(_X2, _Y2, ".-")
    ax.set_xlabel("time (s)")
    ax.set_ylabel("Undamped")


_X = np.linspace(0, 2 * np.pi, 400)
_Y = np.sin(_X**2)


@case(f"{GROUP}/subplots_demo_1")
def _(fig):
    ax = fig.subplots()
    ax.plot(_X, _Y)
    ax.set_title("A single plot")


@case(f"{GROUP}/subplots_demo_2")
def _(fig):
    axs = fig.subplots(2)
    fig.suptitle("Vertically stacked subplots")
    axs[0].plot(_X, _Y)
    axs[1].plot(_X, -_Y)


@case(f"{GROUP}/subplots_demo_4")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    fig.suptitle("Horizontally stacked subplots")
    ax1.plot(_X, _Y)
    ax2.plot(_X, -_Y)


@case(f"{GROUP}/subplots_demo_5")
def _(fig):
    axs = fig.subplots(2, 2)
    axs[0, 0].plot(_X, _Y)
    axs[0, 0].set_title("Axis [0, 0]")
    axs[0, 1].plot(_X, _Y, "tab:orange")
    axs[0, 1].set_title("Axis [0, 1]")
    axs[1, 0].plot(_X, -_Y, "tab:green")
    axs[1, 0].set_title("Axis [1, 0]")
    axs[1, 1].plot(_X, -_Y, "tab:red")
    axs[1, 1].set_title("Axis [1, 1]")
    for ax in axs.flat:
        ax.set(xlabel="x-label", ylabel="y-label")
    for ax in axs.flat:
        ax.label_outer()


def _four_squares(axs, shifts=(0, 0, 0, 0)):
    """``y``, ``y**2``, ``-y`` and ``-y**2`` in the four axes, in the
    default, orange, green and red, shifted right by ``shifts``."""
    for ax, shift, ys, fmt in zip(axs, shifts, (_Y, _Y**2, -_Y, -(_Y**2)),
                                  ("C0", "tab:orange", "tab:green", "tab:red")):
        ax.plot(_X + shift, ys, fmt)
        ax.label_outer()


@case(f"{GROUP}/subplots_demo_6")
def _(fig):
    axs = fig.subplots(2, 2)
    fig.suptitle("Sharing x per column, y per row")
    _four_squares(axs.flat)


@case(f"{GROUP}/subplots_demo_7")
def _(fig):
    ax1, ax2 = fig.subplots(2)
    fig.suptitle("Axes values are scaled individually by default")
    ax1.plot(_X, _Y)
    ax2.plot(_X + 1, -_Y)


@case(f"{GROUP}/subplots_demo_8")
def _(fig):
    ax1, ax2 = fig.subplots(2, sharex=True)
    fig.suptitle("Aligning x-axis using sharex")
    ax1.plot(_X, _Y)
    ax2.plot(_X + 1, -_Y)


def _three_shared(axs):
    """``y**2``, ``0.3 * y`` with circles and ``y`` with pluses."""
    axs[0].plot(_X, _Y**2)
    axs[1].plot(_X, 0.3 * _Y, "o")
    axs[2].plot(_X, _Y, "+")


@case(f"{GROUP}/subplots_demo_9")
def _(fig):
    axs = fig.subplots(3, sharex=True, sharey=True)
    fig.suptitle("Sharing both axes")
    _three_shared(axs)


@case(f"{GROUP}/subplots_demo_10")
def _(fig):
    axs = fig.subplots(3, sharex=True, sharey=True)
    fig.suptitle("Restored xtick labels on to Axes")
    _three_shared(axs)
    axs[0].tick_params(labelbottom=True)


@case(f"{GROUP}/subplots_demo_11")
def _(fig):
    gs = fig.add_gridspec(3, hspace=0)
    axs = gs.subplots(sharex=True, sharey=True)
    fig.suptitle("Sharing both axes")
    _three_shared(axs)
    for ax in axs:
        ax.label_outer()


@case(f"{GROUP}/subplots_demo_12")
def _(fig):
    gs = fig.add_gridspec(2, 2, hspace=0, wspace=0)
    axs = gs.subplots(sharex="col", sharey="row")
    fig.suptitle("Sharing x per column, y per row")
    _four_squares(axs.flat, shifts=(0, 0, 1, 2))


@case(f"{GROUP}/subplots_demo_13")
def _(fig):
    axs = fig.subplots(2, 2)
    axs[0, 0].plot(_X, _Y)
    axs[0, 0].set_title("main")
    axs[1, 0].plot(_X, _Y**2)
    axs[1, 0].set_title("shares x with main")
    axs[1, 0].sharex(axs[0, 0])
    axs[0, 1].plot(_X + 1, _Y + 1)
    axs[0, 1].set_title("unrelated")
    axs[1, 1].plot(_X + 2, _Y + 2)
    axs[1, 1].set_title("also unrelated")
    fig.tight_layout()


@case(f"{GROUP}/subplots_demo_14")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2, subplot_kw=dict(projection="polar"))
    ax1.plot(_X, _Y)
    ax2.plot(_X, _Y**2)


# --- grids -------------------------------------------------------------------


@case(f"{GROUP}/axes_demo")
def _(fig):
    dt = 0.001
    t = np.arange(0.0, 10.0, dt)
    r = np.exp(-t[:1000] / 0.05)
    x = rng(SEED).normal(len(t))
    s = np.convolve(x, r)[:len(x)] * dt
    main_ax = fig.subplots()
    main_ax.plot(t, s)
    main_ax.set_xlim(0, 1)
    main_ax.set_ylim(1.1 * np.min(s), 2 * np.max(s))
    main_ax.set_xlabel("time (s)")
    main_ax.set_ylabel("current (nA)")
    main_ax.set_title("Gaussian colored noise")
    right_inset_ax = fig.add_axes((.65, .6, .2, .2), facecolor="k")
    right_inset_ax.hist(s, 400, density=True)
    right_inset_ax.set(title="Probability", xticks=[], yticks=[])
    left_inset_ax = fig.add_axes((.2, .6, .2, .2), facecolor="k")
    left_inset_ax.plot(t[:len(r)], r)
    left_inset_ax.set(title="Impulse response", xlim=(0, .2), xticks=[], yticks=[])


@case(f"{GROUP}/gridspec_customization_1")
def _(fig):
    fig.suptitle("Controlling subplot sizes with width_ratios and height_ratios")
    gs = GridSpec(2, 2, width_ratios=[1, 2], height_ratios=[4, 1])
    for i in range(4):
        fig.add_subplot(gs[i])
    _label_axes(fig)


@case(f"{GROUP}/gridspec_customization_2")
def _(fig):
    fig.suptitle("Controlling spacing around and between subplots")
    gs1 = GridSpec(3, 3, left=0.05, right=0.48, wspace=0.05)
    fig.add_subplot(gs1[:-1, :])
    fig.add_subplot(gs1[-1, :-1])
    fig.add_subplot(gs1[-1, -1])
    gs2 = GridSpec(3, 3, left=0.55, right=0.98, hspace=0.05)
    fig.add_subplot(gs2[:, :-1])
    fig.add_subplot(gs2[:-1, -1])
    fig.add_subplot(gs2[-1, -1])
    _label_axes(fig)


@case(f"{GROUP}/gridspec_multicolumn")
def _(fig):
    fig.set_layout_engine("constrained")
    gs = GridSpec(3, 3, figure=fig)
    fig.add_subplot(gs[0, :])
    fig.add_subplot(gs[1, :-1])
    fig.add_subplot(gs[1:, -1])
    fig.add_subplot(gs[-1, 0])
    fig.add_subplot(gs[-1, -2])
    fig.suptitle("GridSpec")
    _label_axes(fig)


@case(f"{GROUP}/gridspec_nested")
def _(fig):
    gs0 = GridSpec(1, 2, figure=fig)
    gs00 = GridSpecFromSubplotSpec(3, 3, subplot_spec=gs0[0])
    fig.add_subplot(gs00[:-1, :])
    fig.add_subplot(gs00[-1, :-1])
    fig.add_subplot(gs00[-1, -1])
    gs01 = gs0[1].subgridspec(3, 3)
    fig.add_subplot(gs01[:, :-1])
    fig.add_subplot(gs01[:-1, -1])
    fig.add_subplot(gs01[-1, -1])
    fig.suptitle("GridSpec Inside GridSpec")
    _label_axes(fig)


@case(f"{GROUP}/gridspec_and_subplots")
def _(fig):
    axs = fig.subplots(ncols=3, nrows=3)
    gs = axs[1, 2].get_gridspec()
    for ax in axs[1:, -1]:
        ax.remove()
    axbig = fig.add_subplot(gs[1:, -1])
    axbig.annotate("Big Axes \nGridSpec[1:, -1]", (0.1, 0.5),
                   xycoords="axes fraction", va="center")
    fig.tight_layout()


@case(f"{GROUP}/subplot2grid")
def _(fig):
    plt.subplot2grid((3, 3), (0, 0), colspan=3, fig=fig)
    plt.subplot2grid((3, 3), (1, 0), colspan=2, fig=fig)
    plt.subplot2grid((3, 3), (1, 2), rowspan=2, fig=fig)
    plt.subplot2grid((3, 3), (2, 0), fig=fig)
    plt.subplot2grid((3, 3), (2, 1), fig=fig)
    _label_axes(fig)


# --- layout engines and spacing ---------------------------------------------


@case(f"{GROUP}/demo_constrained_layout_1")
def _(fig):
    for ax in fig.subplots(nrows=2, ncols=2).flat:
        _example_plot(ax)


@case(f"{GROUP}/demo_constrained_layout_2")
def _(fig):
    fig.set_layout_engine("constrained")
    for ax in fig.subplots(nrows=2, ncols=2).flat:
        _example_plot(ax)


@case(f"{GROUP}/demo_constrained_layout_3")
def _(fig):
    fig.set_layout_engine("constrained")
    gs0 = GridSpec(1, 2, figure=fig)
    for cell, rows in ((gs0[0], 3), (gs0[1], 2)):
        nested = GridSpecFromSubplotSpec(rows, 1, subplot_spec=cell)
        for n in range(rows):
            _example_plot(fig.add_subplot(nested[n]))


def _tight_plots(axes, calls_before):
    """``demo_tight_layout``'s ``example_plot`` on each axes, its font
    sizes cycling through 8, 16, 24 and 32 from where the example's
    ``calls_before`` earlier calls left the cycle."""
    sizes = itertools.cycle([8, 16, 24, 32])
    for _ in range(3 * calls_before % 4):
        next(sizes)
    for ax in axes:
        _example_plot(ax, (next(sizes), next(sizes), next(sizes)))


@case(f"{GROUP}/demo_tight_layout_1")
def _(fig):
    _tight_plots([fig.subplots()], 0)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_2")
def _(fig):
    _tight_plots(fig.subplots(nrows=2, ncols=2).flat, 1)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_3")
def _(fig):
    _tight_plots(fig.subplots(nrows=2, ncols=1), 5)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_4")
def _(fig):
    _tight_plots(fig.subplots(nrows=1, ncols=2), 7)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_5")
def _(fig):
    _tight_plots(fig.subplots(nrows=3, ncols=3).flat, 9)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_6")
def _(fig):
    axes = [fig.add_subplot(221), fig.add_subplot(223), fig.add_subplot(122)]
    _tight_plots(axes, 18)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_7")
def _(fig):
    axes = [plt.subplot2grid((3, 3), (0, 0), fig=fig),
            plt.subplot2grid((3, 3), (0, 1), colspan=2, fig=fig),
            plt.subplot2grid((3, 3), (1, 0), colspan=2, rowspan=2, fig=fig),
            plt.subplot2grid((3, 3), (1, 2), rowspan=2, fig=fig)]
    _tight_plots(axes, 21)
    fig.tight_layout()


@case(f"{GROUP}/demo_tight_layout_8")
def _(fig):
    gs1 = fig.add_gridspec(3, 1)
    _tight_plots([fig.add_subplot(gs1[i]) for i in range(3)], 25)
    gs1.tight_layout(fig, rect=[None, None, 0.45, None])
    gs2 = fig.add_gridspec(2, 1)
    _tight_plots([fig.add_subplot(gs2[i]) for i in range(2)], 28)
    with warnings.catch_warnings():
        # gs2 cannot handle gs1's subplots; the tops and bottoms are matched below.
        warnings.simplefilter("ignore", UserWarning)
        gs2.tight_layout(fig, rect=[0.45, None, None, None])
    top = min(gs1.top, gs2.top)
    bottom = max(gs1.bottom, gs2.bottom)
    gs1.update(top=top, bottom=bottom)
    gs2.update(top=top, bottom=bottom)


@case(f"{GROUP}/subplots_adjust")
def _(fig):
    g = rng(SEED)
    ax = fig.add_subplot(211)
    ax.imshow(g.uniform(100 * 100).reshape(100, 100))
    ax = fig.add_subplot(212)
    im = ax.imshow(g.uniform(100 * 100).reshape(100, 100))
    fig.subplots_adjust(bottom=0.1, right=0.8, top=0.9)
    cax = fig.add_axes((0.85, 0.1, 0.075, 0.8))
    fig.colorbar(im, cax=cax)


@case(f"{GROUP}/auto_subplots_adjust")
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10))
    ax.set_yticks([2, 5, 7], labels=["really, really, really", "long", "labels"])
    # The example's draw callback: make room for the widest tick label.
    bboxes = [label.get_window_extent().transformed(fig.transFigure.inverted())
              for label in ax.get_yticklabels()]
    bbox = Bbox.union(bboxes)
    if fig.subplotpars.left < bbox.width:
        fig.subplots_adjust(left=1.1 * bbox.width)


@case(f"{GROUP}/ganged_plots")
def _(fig):
    t = np.arange(0.0, 2.0, 0.01)
    s1 = np.sin(2 * np.pi * t)
    s2 = np.exp(-t)
    s3 = s1 * s2
    axs = fig.subplots(3, 1, sharex=True)
    fig.subplots_adjust(hspace=0)
    axs[0].plot(t, s1)
    axs[0].set_yticks(np.arange(-0.9, 1.0, 0.4))
    axs[0].set_ylim(-1, 1)
    axs[1].plot(t, s2)
    axs[1].set_yticks(np.arange(0.1, 1.0, 0.2))
    axs[1].set_ylim(0, 1)
    axs[2].plot(t, s3)
    axs[2].set_yticks(np.arange(-0.9, 1.0, 0.4))
    axs[2].set_ylim(-1, 1)


# --- margins and aspects ------------------------------------------------------


def _damped(t):
    return np.exp(-t) * np.cos(2 * np.pi * t)


@case(f"{GROUP}/axes_margins_1")
def _(fig):
    t1 = np.arange(0.0, 3.0, 0.01)
    ax1 = fig.add_subplot(212)
    ax1.margins(0.05)
    ax1.plot(t1, _damped(t1))
    ax2 = fig.add_subplot(221)
    ax2.margins(2, 2)
    ax2.plot(t1, _damped(t1))
    ax2.set_title("Zoomed out")
    ax3 = fig.add_subplot(222)
    ax3.margins(x=0, y=-0.25)
    ax3.plot(t1, _damped(t1))
    ax3.set_title("Zoomed in")


@case(f"{GROUP}/axes_margins_2")
def _(fig):
    y, x = np.mgrid[:5, 1:6]
    poly_coords = [(0.25, 2.75), (3.25, 2.75), (2.25, 0.75), (0.25, 0.75)]
    ax1, ax2 = fig.subplots(ncols=2)
    ax2.use_sticky_edges = False
    for ax, status in zip((ax1, ax2), ("Is", "Is Not")):
        ax.pcolor(x, y, x + y, cmap="inferno", shading="auto")
        ax.add_patch(Polygon(poly_coords, color="forestgreen", alpha=0.5))
        ax.margins(x=0.1, y=0.05)
        ax.set_aspect("equal")
        ax.set_title(f"{status} Sticky")


@case(f"{GROUP}/axes_box_aspect_1")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(300, 400)
    ax.set_box_aspect(1)


@case(f"{GROUP}/axes_box_aspect_2")
def _(fig):
    ax, ax2 = fig.subplots(ncols=2, sharey=True)
    ax.plot([1, 5], [0, 10])
    ax2.plot([100, 500], [10, 15])
    ax.set_box_aspect(1)
    ax2.set_box_aspect(1)


@case(f"{GROUP}/axes_box_aspect_3")
def _(fig):
    ax = fig.subplots()
    ax2 = ax.twinx()
    ax.plot([0, 10])
    ax2.plot([12, 10])
    ax.set_box_aspect(1)


@case(f"{GROUP}/axes_box_aspect_4")
def _(fig):
    fig.set_layout_engine("constrained")
    ax, ax2 = fig.subplots(ncols=2)
    im = rng(SEED).uniform(16 * 27).reshape(16, 27)
    ax.imshow(im)
    ax2.plot([23, 45])
    ax2.set_box_aspect(im.shape[0] / im.shape[1])


@case(f"{GROUP}/axes_box_aspect_5")
def _(fig):
    axs = fig.subplots(2, 2, sharex="col", sharey="row",
                       gridspec_kw=dict(height_ratios=[1, 3], width_ratios=[3, 1]))
    axs[0, 1].set_visible(False)
    axs[0, 0].set_box_aspect(1 / 3)
    axs[1, 0].set_box_aspect(1)
    axs[1, 1].set_box_aspect(3 / 1)
    x, y = rng(SEED).normal(2 * 400).reshape(2, 400) * [[.5], [180]]
    axs[1, 0].scatter(x, y)
    axs[0, 0].hist(x)
    axs[1, 1].hist(y, orientation="horizontal")


@case(f"{GROUP}/axes_box_aspect_6")
def _(fig):
    ax = fig.subplots()
    ax.add_patch(Circle((5, 3), 1))
    ax.set_aspect("equal", adjustable="datalim")
    ax.set_box_aspect(0.5)
    ax.autoscale()


@case(f"{GROUP}/axes_box_aspect_7")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 3, subplot_kw=dict(box_aspect=1), sharex=True, sharey=True)
    for i, ax in enumerate(axs.flat):
        ax.scatter(i % 3, -((i // 3) - 0.5) * 200, c=[mpl.colormaps["hsv"](i / 6)], s=300)


@case(f"{GROUP}/axis_equal_demo")
def _(fig):
    an = np.linspace(0, 2 * np.pi, 100)
    axs = fig.subplots(2, 2)
    for ax in axs.flat:
        ax.plot(3 * np.cos(an), 3 * np.sin(an))
    axs[0, 0].set_title("not equal, looks like ellipse", fontsize=10)
    axs[0, 1].axis("equal")
    axs[0, 1].set_title("equal, looks like circle", fontsize=10)
    axs[1, 0].axis("equal")
    axs[1, 0].set(xlim=(-3, 3), ylim=(-3, 3))
    axs[1, 0].set_title("still a circle, even after changing limits", fontsize=10)
    axs[1, 1].set_aspect("equal", "box")
    axs[1, 1].set_title("still a circle, auto-adjusted data limits", fontsize=10)
    fig.tight_layout()


# --- figures ------------------------------------------------------------------

_T = np.arange(0.0, 2.0, 0.01)
_S1 = np.sin(2 * np.pi * _T)
_S2 = np.sin(4 * np.pi * _T)


@case(f"{GROUP}/multiple_figs_demo_1")
def _(fig):
    top = fig.add_subplot(211)
    top.plot(_T, _S1)
    fig.add_subplot(212).plot(_T, 2 * _S1)
    top.plot(_T, _S2, "s")
    top.set_xticklabels([])


@case(f"{GROUP}/multiple_figs_demo_2")
def _(fig):
    fig.add_subplot().plot(_T, _S2)


def _size_label(fig, text):
    """``figure_size_units``: ``text`` in the middle of an empty axes."""
    fig.subplots().text(0.5, 0.5, text, ha="center", va="center", fontsize=28, color="C1")


_CM = 1 / 2.54  # centimeters in inches
_PX = 1 / 100  # pixels in inches at the default dpi


@case(f"{GROUP}/figure_size_units_1", figsize=(6, 2))
def _(fig):
    _size_label(fig, "6 inches x 2 inches")


@case(f"{GROUP}/figure_size_units_2", figsize=(15 * _CM, 5 * _CM))
def _(fig):
    _size_label(fig, "15cm x 5cm")


@case(f"{GROUP}/figure_size_units_3", figsize=(600 * _PX, 200 * _PX))
def _(fig):
    _size_label(fig, "600px x 200px")


def _random_mesh(ax, g, fontsize=12, hide_labels=False):
    """``subfigures``' ``example_plot``: a 30 x 30 mesh of normal noise."""
    pc = ax.pcolormesh(g.normal(30 * 30).reshape(30, 30), vmin=-2.5, vmax=2.5)
    if not hide_labels:
        ax.set_xlabel("x-label", fontsize=fontsize)
        ax.set_ylabel("y-label", fontsize=fontsize)
        ax.set_title("Title", fontsize=fontsize)
    return pc


def _left_plots(subfig, g):
    """Two labelled meshes sharing y, a colorbar below them, on grey."""
    axs = subfig.subplots(1, 2, sharey=True)
    subfig.set_facecolor("0.75")
    for ax in axs:
        pc = _random_mesh(ax, g)
    subfig.suptitle("Left plots", fontsize="x-large")
    subfig.colorbar(pc, shrink=0.6, ax=axs, location="bottom")


def _right_plots(subfig, g):
    """Three unlabelled meshes sharing x, a colorbar beside them."""
    axs = subfig.subplots(3, 1, sharex=True)
    for nn, ax in enumerate(axs):
        pc = _random_mesh(ax, g, hide_labels=True)
        if nn == 2:
            ax.set_xlabel("xlabel")
        if nn == 1:
            ax.set_ylabel("ylabel")
    subfig.set_facecolor("0.85")
    subfig.colorbar(pc, shrink=0.6, ax=axs)
    subfig.suptitle("Right plots", fontsize="x-large")


def _left_and_right(fig, width_ratios=None):
    fig.set_layout_engine("constrained")
    g = rng(19680808)
    subfigs = fig.subfigures(1, 2, wspace=0.07, width_ratios=width_ratios)
    _left_plots(subfigs[0], g)
    _right_plots(subfigs[1], g)
    fig.suptitle("Figure suptitle", fontsize="xx-large")


@case(f"{GROUP}/subfigures_1", figsize=(10, 4))
def _(fig):
    _left_and_right(fig)


@case(f"{GROUP}/subfigures_2", figsize=(10, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 3)
    gridspec = axs[0, 0].get_subplotspec().get_gridspec()
    for a in axs[:, 0]:
        a.remove()
    for a in axs[:, 1:].flat:
        a.plot(np.arange(10))
    _left_plots(fig.add_subfigure(gridspec[:, 0]), rng(19680808))
    fig.suptitle("Figure suptitle", fontsize="xx-large")


@case(f"{GROUP}/subfigures_3", figsize=(10, 4))
def _(fig):
    _left_and_right(fig, width_ratios=[2, 1])


@case(f"{GROUP}/subfigures_4", figsize=(10, 8))
def _(fig):
    fig.set_layout_engine("constrained")
    g = rng(19680808)
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
        pc = _random_mesh(ax, g, hide_labels=True)
    subfigsnest[0].colorbar(pc, ax=axsnest0)
    subfigsnest[1].suptitle("subfigsnest[1]")
    subfigsnest[1].set_facecolor("g")
    subfigsnest[1].subplots(3, 1, sharex=True)
    subfigs[1].subplots(2, 2)


@case(f"{GROUP}/custom_figure_class")
def _(fig):
    # What ``WatermarkFigure(watermark='draft')`` adds when it is created.
    bbox = dict(boxstyle="square", lw=3, ec="gray", fc=(0.9, 0.9, .9, .5), alpha=0.5)
    fig.text(0.5, 0.5, "draft", ha="center", va="center", rotation=30,
             fontsize=40, color="gray", alpha=0.5, bbox=bbox)
    x = np.linspace(-3, 3, 201)
    fig.add_subplot().plot(x, np.tanh(x) + 0.1 * np.cos(5 * x))


@case(f"{GROUP}/figure_title_1")
def _(fig):
    x = np.linspace(0.0, 5.0, 501)
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    ax1.plot(x, np.cos(6 * x) * np.exp(-x))
    ax1.set_title("damped")
    ax1.set_xlabel("time (s)")
    ax1.set_ylabel("amplitude")
    ax2.plot(x, np.cos(6 * x))
    ax2.set_xlabel("time (s)")
    ax2.set_title("undamped")
    fig.suptitle("Different types of oscillations", fontsize=16)


#: ``Stocks.csv``'s columns: name, first and last month (from January
#: 1990) without a price.
_STOCKS = [("IBM", None), ("AAPL", None), ("MSFT", None), ("XRX", None),
           ("AMZN", (0, 88)), ("DELL", (285, 347)), ("GOOGL", (0, 175)), ("ADBE", None)]


def _stocks():
    """Monthly prices from January 1990 to June 2022 like ``Stocks.csv``'s:
    geometric random walks, missing before a listing or while delisted."""
    dates = np.arange("1990-01", "2022-07", dtype="datetime64[M]").astype("datetime64[D]")
    g = rng(SEED)
    columns = []
    for name, missing in _STOCKS:
        price = np.exp(np.cumsum(g.normal(len(dates), 0.01, 0.08)))
        if missing is not None:
            price[missing[0]:missing[1]] = np.nan
        columns.append((name, price))
    return dates, columns


@case(f"{GROUP}/figure_title_2", figsize=(9, 5))
def _(fig):
    dates, columns = _stocks()
    fig.set_layout_engine("constrained")
    axs = fig.subplots(4, 2, sharex=True, sharey=True)
    for ax, (name, y) in zip(axs.flat, columns):
        ax.plot(dates, y / np.nanmax(y), lw=2.5)
        ax.set_title(name, fontsize="small", loc="left")
    fig.supxlabel("Year")
    fig.supylabel("Stock price relative to max")


# --- shared, twinned and secondary axes -----------------------------------------


@case(f"{GROUP}/shared_axis_demo")
def _(fig):
    t1 = np.linspace(0, 8, 201)
    y1 = np.sin(2 * np.pi * t1)
    t2 = np.linspace(2, 10, 201)
    y2 = 20 * np.cos(2 * np.pi * t2)**2 * np.exp(-0.3 * t2)
    ax1, ax2 = fig.subplots(2, sharex=True)
    ax1.plot(t1, y1)
    ax1.set_ylabel("Signal 1")
    ax2.plot(t2, y2)
    ax2.set_ylabel("Signal 2")
    ax2.set_xlabel("Time (s)")


@case(f"{GROUP}/invert_axes", figsize=(6.4, 4))
def _(fig):
    x = np.arange(0.01, 4.0, 0.01)
    y = np.exp(-x)
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(1, 2)
    fig.suptitle("Inverted axis with ...")
    ax1.plot(x, y)
    ax1.set_xlim(4, 0)
    ax1.set_title("fixed limits: set_xlim(4, 0)")
    ax1.set_xlabel("decreasing x ⟶")
    ax1.grid(True)
    ax2.plot(x, y)
    ax2.xaxis.set_inverted(True)
    ax2.set_title("autoscaling: set_inverted(True)")
    ax2.set_xlabel("decreasing x ⟶")
    ax2.grid(True)


@case(f"{GROUP}/two_scales")
def _(fig):
    t = np.arange(0.01, 10.0, 0.01)
    ax1 = fig.subplots()
    color = "tab:red"
    ax1.set_xlabel("time (s)")
    ax1.set_ylabel("exp", color=color)
    ax1.plot(t, np.exp(t), color=color)
    ax1.tick_params(axis="y", labelcolor=color)
    ax2 = ax1.twinx()
    color = "tab:blue"
    ax2.set_ylabel("sin", color=color)
    ax2.plot(t, np.sin(2 * np.pi * t), color=color)
    ax2.tick_params(axis="y", labelcolor=color)
    fig.tight_layout()


def _fahrenheit2celsius(temp):
    return (5. / 9.) * (temp - 32)


@case(f"{GROUP}/fahrenheit_celsius_scales")
def _(fig):
    ax_f = fig.subplots()
    ax_c = ax_f.twinx()

    def convert_ax_c_to_celsius(ax_f):
        y1, y2 = ax_f.get_ylim()
        ax_c.set_ylim(_fahrenheit2celsius(y1), _fahrenheit2celsius(y2))

    ax_f.callbacks.connect("ylim_changed", convert_ax_c_to_celsius)
    ax_f.plot(np.linspace(-40, 120, 100))
    ax_f.set_xlim(0, 100)
    ax_f.set_title("Two scales: Fahrenheit and Celsius")
    ax_f.set_ylabel("Fahrenheit")
    ax_c.set_ylabel("Celsius")


@case(f"{GROUP}/twin_axes_zorder")
def _(fig):
    x = np.linspace(0, 10, 400)
    y_main = np.sin(x)
    y_twin = 0.4 * np.cos(x) + 0.6
    ax = fig.subplots()
    ax2 = ax.twinx(delta_zorder=-1)
    ax2.fill_between(x, 0, y_twin, color="C1", alpha=0.35, label="twin fill")
    ax2.plot(x, y_twin, color="C1", lw=6, alpha=0.8)
    ax.scatter(x[::8], y_main[::8], s=35, color="C0", edgecolor="k", linewidth=0.5,
               zorder=3, label="main scatter")
    ax.plot(x, y_main, color="C0", lw=4)
    ax.set_xlabel("x")
    ax.set_ylabel("main y")
    ax2.set_ylabel("twin y")
    ax.set_title("Twin Axes drawn behind the main Axes using delta_zorder")
    fig.tight_layout()


@case(f"{GROUP}/secondary_axis_1")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = np.arange(0, 360, 1)
    ax.plot(x, np.sin(2 * x * np.pi / 180))
    ax.set_xlabel("angle [degrees]")
    ax.set_ylabel("signal")
    ax.set_title("Sine wave")
    secax = ax.secondary_xaxis("top", functions=(lambda x: x * np.pi / 180,
                                                 lambda x: x * 180 / np.pi))
    secax.set_xlabel("angle [rad]")


@case(f"{GROUP}/secondary_axis_2")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = np.arange(0, 10)
    ax.plot(x, rng(SEED).normal(len(x)))
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_title("Random data")
    secax = ax.secondary_xaxis(0, transform=ax.transData)
    secax.set_xlabel("Axis at Y = 0")


def _one_over(x):
    """Vectorized 1/x, treating x==0 manually"""
    x = np.array(x, float)
    near_zero = np.isclose(x, 0)
    x[near_zero] = np.inf
    x[~near_zero] = 1 / x[~near_zero]
    return x


@case(f"{GROUP}/secondary_axis_3")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x = np.arange(0.02, 1, 0.02)
    ax.loglog(x, rng(SEED).normal(len(x)) ** 2)
    ax.set_xlabel("f [Hz]")
    ax.set_ylabel("PSD")
    ax.set_title("Random spectrum")
    secax = ax.secondary_xaxis("top", functions=(_one_over, _one_over))
    secax.set_xlabel("period [s]")


@case(f"{GROUP}/secondary_axis_4")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    x1_vals = np.arange(2, 11, 0.4)
    x2_vals = x1_vals ** 2
    ydata = 50.0 + 20 * rng(SEED).normal(len(x1_vals))
    ax.plot(x1_vals, ydata, label="Plotted data")
    ax.plot(x1_vals, x2_vals, label=r"$x_2 = x_1^2$")
    ax.set_xlabel(r"$x_1$")
    ax.legend()
    x1n = np.linspace(0, 20, 201)
    x2n = x1n**2
    ax.axvline(np.sqrt(40), color="grey", ls="--")
    ax.axvline(10, color="grey", ls="--")
    secax = ax.secondary_xaxis("top", functions=(lambda x: np.interp(x, x1n, x2n),
                                                 lambda x: np.interp(x, x2n, x1n)))
    secax.set_xticks([10, 20, 40, 60, 80, 100])
    secax.set_xlabel(r"$x_2$")


@case(f"{GROUP}/secondary_axis_5")
def _(fig):
    dates = [datetime.datetime(2018, 1, 1) + datetime.timedelta(hours=k * 6)
             for k in range(240)]
    temperature = rng(SEED).normal(len(dates)) * 4 + 6.7
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.plot(dates, temperature)
    ax.set_ylabel(r"$T\ [^oC]$")
    ax.xaxis.set_tick_params(rotation=70)
    new_year = mdates.date2num(datetime.datetime(2018, 1, 1))
    secax_x = ax.secondary_xaxis("top", functions=(lambda x: x - new_year,
                                                   lambda x: x + new_year))
    secax_x.set_xlabel("yday [2018]")
    secax_y = ax.secondary_yaxis("right", functions=(lambda x: x * 1.8 + 32,
                                                     lambda x: (x - 32) / 1.8))
    secax_y.set_ylabel(r"$T\ [^oF]$")
    mean = np.mean(temperature)
    secax_y2 = ax.secondary_yaxis(1.2, functions=(lambda x: x - mean, lambda x: x + mean))
    secax_y2.set_ylabel(r"$T - \overline{T}\ [^oC]$")


# --- titles and labels ----------------------------------------------------------


@case(f"{GROUP}/align_labels_demo", figsize=(8.9, 5.5))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 3, gridspec_kw={"wspace": 0.1})
    for ax in axs.flat:
        scale = 2000 if ax.get_subplotspec().is_first_row() else 1
        ax.plot(scale * (1 - np.exp(-np.linspace(0, 5, 100))))
        if ax.get_subplotspec().is_last_row():
            ax.set_xlabel("xlabel", bbox=dict(facecolor="yellow", pad=5, alpha=0.2))
        ax.set_ylabel("ylabel", bbox=dict(facecolor="yellow", pad=5, alpha=0.2))
        ax.set_ylim(0, scale)
    axs[0, 0].xaxis.tick_top()
    axs[1, 2].tick_params(axis="x", rotation=55)
    axs[0, 0].set_title("ylabels not aligned")
    fig.align_titles()
    fig.align_xlabels()
    fig.align_ylabels(axs[:, 1])
    axs[0, 1].set_title("fig.align_ylabels()")
    for ax in axs[:, 2]:
        ax.yaxis.set_label_coords(-0.3, 0.5)
    axs[0, 2].set_title("ylabels manually aligned")


@case(f"{GROUP}/axis_labels_demo")
def _(fig):
    ax = fig.subplots()
    sc = ax.scatter([1, 2], [1, 2], c=[1, 2])
    ax.set_ylabel("YLabel", loc="top")
    ax.set_xlabel("XLabel", loc="left")
    cbar = fig.colorbar(sc)
    cbar.set_label("ZLabel", loc="top")


@case(f"{GROUP}/axes_props")
def _(fig):
    t = np.arange(0.0, 2.0, 0.01)
    ax = fig.subplots()
    ax.plot(t, np.sin(2 * np.pi * t))
    ax.grid(True, linestyle="-.")
    ax.tick_params(labelcolor="r", labelsize="medium", width=3)


# --- zooms, insets and the rest -------------------------------------------------


def _connect_bbox(bbox1, bbox2, loc1a, loc2a, loc1b, loc2b, prop_lines, prop_patches):
    c1 = BboxConnector(bbox1, bbox2, loc1=loc1a, loc2=loc2a, clip_on=False, **prop_lines)
    c2 = BboxConnector(bbox1, bbox2, loc1=loc1b, loc2=loc2b, clip_on=False, **prop_lines)
    bbox_patch1 = BboxPatch(bbox1, **prop_patches)
    bbox_patch2 = BboxPatch(bbox2, **prop_patches)
    p = BboxConnectorPatch(bbox1, bbox2, loc1a=loc1a, loc2a=loc2a, loc1b=loc1b, loc2b=loc2b,
                           clip_on=False, **prop_patches)
    return c1, c2, bbox_patch1, bbox_patch2, p


def _zoom_effect(ax1, ax2, mybbox1, mybbox2):
    """``zoom_effect01/02``: shade ``mybbox1`` on ``ax1`` and ``mybbox2``
    on ``ax2`` and join them with lines and a band."""
    prop_patches = {"ec": "none", "alpha": 0.2}
    c1, c2, bbox_patch1, bbox_patch2, p = _connect_bbox(
        mybbox1, mybbox2, loc1a=3, loc2a=2, loc1b=4, loc2b=1,
        prop_lines={}, prop_patches=prop_patches)
    ax1.add_patch(bbox_patch1)
    ax2.add_patch(bbox_patch2)
    ax2.add_patch(c1)
    ax2.add_patch(c2)
    ax2.add_patch(p)


@case(f"{GROUP}/axes_zoom_effect")
def _(fig):
    axs = fig.subplot_mosaic([["zoom1", "zoom2"], ["main", "main"]])
    axs["main"].set(xlim=(0, 5))
    bbox = Bbox.from_extents(0.2, 0, 0.8, 1)
    _zoom_effect(axs["zoom1"], axs["main"],
                 TransformedBbox(bbox, axs["zoom1"].get_xaxis_transform()),
                 TransformedBbox(bbox, axs["main"].get_xaxis_transform()))
    axs["zoom2"].set(xlim=(2, 3))
    ax1, ax2 = axs["zoom2"], axs["main"]
    tt = ax1.transScale + (ax1.transLimits + ax2.transAxes)
    trans = blended_transform_factory(ax2.transData, tt)
    _zoom_effect(ax1, ax2, ax1.bbox, TransformedBbox(ax1.viewLim, trans))


def _bivariate_normal():
    """The 15 x 15 grid of ``axes_grid/bivariate_normal.npy``: the
    difference of two Gaussians over [-3, 4] x [-4, 3], times ten."""
    x = np.arange(-3.0, 4.001, 0.5)
    y = np.arange(-4.0, 3.001, 0.5)
    X, Y = np.meshgrid(x, y)
    z1 = np.exp(-(X**2 + Y**2) / 2) / (2 * np.pi)
    z2 = np.exp(-((X - 1)**2 / 1.5**2 + (Y - 1)**2 / 0.5**2) / 2) / (2 * np.pi * 1.5 * 0.5)
    return (z1 - z2) * 10


@case(f"{GROUP}/zoom_inset_axes")
def _(fig):
    ax = fig.subplots()
    Z = _bivariate_normal()
    Z2 = np.zeros((150, 150))
    ny, nx = Z.shape
    Z2[30:30 + ny, 30:30 + nx] = Z
    extent = (-3, 4, -4, 3)
    ax.imshow(Z2, extent=extent, origin="lower")
    x1, x2, y1, y2 = -1.5, -0.9, -2.5, -1.9
    axins = ax.inset_axes([0.5, 0.5, 0.47, 0.47], xlim=(x1, x2), ylim=(y1, y2),
                          xticklabels=[], yticklabels=[])
    axins.imshow(Z2, extent=extent, origin="lower")
    ax.indicate_inset_zoom(axins, edgecolor="black")


@case(f"{GROUP}/broken_axis")
def _(fig):
    pts = rng(SEED).uniform(30) * .2
    pts[[3, 14]] += .8
    ax1, ax2 = fig.subplots(2, 1, sharex=True)
    fig.subplots_adjust(hspace=0.05)
    ax1.plot(pts)
    ax2.plot(pts)
    ax1.set_ylim(.78, 1.)
    ax2.set_ylim(0, .22)
    ax1.spines.bottom.set_visible(False)
    ax2.spines.top.set_visible(False)
    ax1.xaxis.tick_top()
    ax1.tick_params(labeltop=False)
    ax2.xaxis.tick_bottom()
    d = .5
    kwargs = dict(marker=[(-1, -d), (1, d)], markersize=12,
                  linestyle="none", color="k", mec="k", mew=1, clip_on=False)
    ax1.plot([0, 1], [0, 0], transform=ax1.transAxes, **kwargs)
    ax2.plot([0, 1], [1, 1], transform=ax2.transAxes, **kwargs)


def _geo(fig, projection, title):
    """``geo_demo``: an empty map with its grid."""
    ax = fig.add_subplot(projection=projection)
    ax.set_title(title)
    ax.grid(True)


@case(f"{GROUP}/geo_demo_1")
def _(fig):
    _geo(fig, "aitoff", "Aitoff")


@case(f"{GROUP}/geo_demo_2")
def _(fig):
    _geo(fig, "hammer", "Hammer")


@case(f"{GROUP}/geo_demo_3")
def _(fig):
    _geo(fig, "lambert", "Lambert")


@case(f"{GROUP}/axhspan_demo", figsize=(7, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    s = 2.9 * np.convolve(rng(SEED).normal(500), np.ones(30) / 30, mode="valid")
    ax1.plot(s)
    ax1.axhspan(-1, 1, alpha=0.1)
    ax1.set(ylim=(-1.5, 1.5), title="axhspan")
    mu = 8
    sigma = 2
    x = np.linspace(0, 16, 401)
    y = np.exp(-((x - mu)**2) / (2 * sigma**2))
    ax2.axvspan(mu - 2 * sigma, mu - sigma, color="0.95")
    ax2.axvspan(mu - sigma, mu + sigma, color="0.9")
    ax2.axvspan(mu + sigma, mu + 2 * sigma, color="0.95")
    ax2.axvline(mu, color="darkgrey", linestyle="--")
    ax2.plot(x, y)
    ax2.set(title="axvspan")
