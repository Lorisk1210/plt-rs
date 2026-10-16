"""Figure layout: subplot grids, GridSpec, mosaics, tight/constrained layout,
manual axes, subfigures, insets, label alignment and canvas size/dpi."""

import numpy as np
from matplotlib.gridspec import GridSpec, GridSpecFromSubplotSpec

from spec_support import case

T = np.linspace(0, 10, 40)
GRID = np.add.outer(np.sin(np.linspace(0, 3, 12)), np.cos(np.linspace(0, 4, 16)))


def _fill(ax, i=0):
    ax.plot(T, np.sin(T + i))


# --- subplot grids ---------------------------------------------------------


@case("layout/subplots_2x2")
def _(fig):
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        _fill(ax, i)


@case("layout/subplots_share_col_row")
def _(fig):
    axs = fig.subplots(2, 3, sharex="col", sharey="row")
    for i, ax in enumerate(axs.flat):
        ax.plot(T * (1 + i % 3), np.sin(T) * (1 + 3 * (i // 3)))



@case("layout/subplots_adjust")
def _(fig):
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        _fill(ax, i)
    fig.subplots_adjust(left=0.1, right=0.98, bottom=0.08, top=0.95, wspace=0.4, hspace=0.6)


@case("layout/add_subplot_codes")
def _(fig):
    _fill(fig.add_subplot(221))
    _fill(fig.add_subplot(2, 2, 2), 1)
    _fill(fig.add_subplot(212), 2)


# --- GridSpec and mosaics ----------------------------------------------------


@case("layout/gridspec_ratios")
def _(fig):
    gs = fig.add_gridspec(2, 3, width_ratios=[1, 2, 3], height_ratios=[2, 1])
    for i in range(6):
        _fill(fig.add_subplot(gs[i // 3, i % 3]), i)


@case("layout/gridspec_spans")
def _(fig):
    gs = GridSpec(3, 3, figure=fig)
    _fill(fig.add_subplot(gs[0, :]))
    _fill(fig.add_subplot(gs[1, :-1]), 1)
    _fill(fig.add_subplot(gs[1:, -1]), 2)
    _fill(fig.add_subplot(gs[-1, 0]), 3)
    _fill(fig.add_subplot(gs[-1, -2]), 4)


@case("layout/gridspec_spacing_and_extent")
def _(fig):
    gs = fig.add_gridspec(2, 2, left=0.2, right=0.9, bottom=0.15, top=0.85, wspace=0.05, hspace=0.05)
    axs = gs.subplots(sharex=True, sharey=True)
    for i, ax in enumerate(axs.flat):
        _fill(ax, i)


@case("layout/gridspec_from_subplotspec")
def _(fig):
    outer = GridSpec(1, 2, figure=fig, wspace=0.3)
    inner = GridSpecFromSubplotSpec(2, 2, subplot_spec=outer[1], wspace=0.1, hspace=0.1)
    _fill(fig.add_subplot(outer[0]))
    for i in range(4):
        _fill(fig.add_subplot(inner[i]), i + 1)


@case("layout/subplot_mosaic_string")
def _(fig):
    axd = fig.subplot_mosaic("AAB;C.B")
    for i, (name, ax) in enumerate(sorted(axd.items())):
        _fill(ax, i)
        ax.set_title(name)


@case("layout/subplot_mosaic_nested_ratios")
def _(fig):
    axd = fig.subplot_mosaic([["main", [["a", "b"], ["c", "d"]]], ["bottom", "bottom"]], width_ratios=[2, 1], height_ratios=[3, 1])
    for i, (name, ax) in enumerate(sorted(axd.items())):
        _fill(ax, i)
        ax.set_title(name, fontsize=8)


# --- tight layout ----------------------------------------------------------


@case("layout/tight_layout_long_labels")
def _(fig):
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        ax.plot(T, 1e4 * np.sin(T + i))
        ax.set_xlabel("a rather long x axis label")
        ax.set_ylabel("long y label\nwith two lines")
        ax.set_title(f"panel title {i}")
    fig.tight_layout()


@case("layout/tight_layout_pads")
def _(fig):
    axs = fig.subplots(1, 2)
    for i, ax in enumerate(axs):
        _fill(ax, i)
        ax.set_title("title")
        ax.set_xlabel("x")
    fig.suptitle("Suptitle")
    fig.tight_layout(pad=2.0, w_pad=4.0)


# --- constrained layout ----------------------------------------------------


@case("layout/constrained_basic")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        _fill(ax, i)
        ax.set_title(f"title {i}")
        ax.set_xlabel("x label")
        ax.set_ylabel("y label")


@case("layout/constrained_colorbars")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        mesh = ax.pcolormesh(GRID * (i + 1), cmap="viridis")
        fig.colorbar(mesh, ax=ax)


@case("layout/constrained_shared_colorbar")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for ax in axs.flat:
        mesh = ax.pcolormesh(GRID, vmin=-2, vmax=2, cmap="RdBu_r")
    fig.colorbar(mesh, ax=axs, shrink=0.6, location="bottom")


@case("layout/constrained_suptitle_suplabels")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 3, sharey=True)
    for i, ax in enumerate(axs):
        _fill(ax, i)
    fig.suptitle("Constrained suptitle", fontsize=16)
    fig.supxlabel("time")
    fig.supylabel("value")


@case("layout/constrained_legend_outside")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2)
    for i, ax in enumerate(axs):
        ax.plot(T, np.sin(T + i), label=f"sin {i}")
        ax.plot(T, np.cos(T + i), label=f"cos {i}")
    fig.legend(loc="outside upper center", ncols=4)
    axs[1].legend(loc="center left", bbox_to_anchor=(1, 0.5))


@case("layout/constrained_pads")
def _(fig):
    fig.set_layout_engine("constrained", w_pad=0.2, h_pad=0.2, wspace=0.1, hspace=0.1)
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        _fill(ax, i)


# --- manual axes, subfigures and insets -------------------------------------


@case("layout/add_axes_rects")
def _(fig):
    _fill(fig.add_axes((0.1, 0.1, 0.8, 0.8)))
    ax = fig.add_axes((0.55, 0.55, 0.3, 0.25), facecolor="0.9")
    _fill(ax, 2)
    ax.set_title("overlay")


@case("layout/subfigures_side_by_side")
def _(fig):
    left, right = fig.subfigures(1, 2, wspace=0.05, width_ratios=[2, 1])
    left.set_facecolor("0.92")
    left.suptitle("left subfigure")
    for i, ax in enumerate(left.subplots(2, 1)):
        _fill(ax, i)
    right.suptitle("right subfigure")
    _fill(right.subplots(), 3)


@case("layout/subfigures_nested_constrained")
def _(fig):
    fig.set_layout_engine("constrained")
    top, bottom = fig.subfigures(2, 1)
    top.suptitle("top")
    for i, ax in enumerate(top.subplots(1, 3)):
        _fill(ax, i)
    left, right = bottom.subfigures(1, 2)
    left.set_facecolor("lightyellow")
    left.supxlabel("left x")
    _fill(left.subplots())
    right.set_facecolor("lightcyan")
    mesh = right.subplots().pcolormesh(GRID)
    right.colorbar(mesh)


@case("layout/inset_axes")
def _(fig):
    ax = fig.subplots()
    _fill(ax)
    axins = ax.inset_axes([0.6, 0.6, 0.35, 0.3])
    axins.plot(T, np.cos(T), "C1")


@case("layout/indicate_inset_zoom")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(3 * T) * np.exp(-T / 5))
    axins = ax.inset_axes([0.55, 0.55, 0.4, 0.4], xlim=(1, 2.5), ylim=(-0.8, 0.6), xticklabels=[], yticklabels=[])
    axins.plot(T, np.sin(3 * T) * np.exp(-T / 5))
    ax.indicate_inset_zoom(axins, edgecolor="black")


@case("layout/align_labels")
def _(fig):
    fig.subplots_adjust(left=0.2, wspace=0.6)
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        ax.plot(T, np.sin(T) * 10 ** (2 * i))
        ax.set_ylabel(f"y label {i}")
        ax.set_xlabel(f"x label {i}")
    axs[0, 0].tick_params(axis="x", labelrotation=55)
    fig.align_labels()


# --- figure canvas ---------------------------------------------------------


@case("layout/figure_facecolor_edgecolor_linewidth", figsize=(4, 3))
def _(fig):
    fig.set_facecolor("lightsteelblue")
    fig.set_edgecolor("navy")
    fig.set_linewidth(6)
    _fill(fig.subplots())


@case("layout/dpi_72", dpi=72)
def _(fig):
    ax = fig.subplots()
    _fill(ax)
    ax.set_title("72 dpi")


@case("layout/dpi_150", figsize=(4, 3), dpi=150)
def _(fig):
    ax = fig.subplots()
    _fill(ax)
    ax.set_title("150 dpi")


@case("layout/figsize_wide_short", figsize=(8, 2))
def _(fig):
    _fill(fig.subplots())


@case("layout/figsize_tall_narrow", figsize=(2.5, 5))
def _(fig):
    axs = fig.subplots(3, 1)
    for i, ax in enumerate(axs):
        _fill(ax, i)


@case("layout/colorbar_steals_space")
def _(fig):
    axs = fig.subplots(1, 2)
    mesh = axs[0].pcolormesh(GRID)
    fig.colorbar(mesh, ax=axs[0])
    mesh = axs[1].pcolormesh(GRID, cmap="magma")
    fig.colorbar(mesh, ax=axs[1], orientation="horizontal", extend="both")
