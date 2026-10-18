"""Figure-level features: the gallery's subfigures (nested, with ratios,
spacing, colours, colorbars and super labels), super labels under tight
layout, figure legends outside and anchored, the compressed layout,
nested grid specs under constrained layout, subfigure legends and
`savefig.bbox = "tight"` with padding."""

import numpy as np

from spec_support import case, rng

T = np.linspace(0, 10, 40)


def _field(seed):
    """A smooth 30 x 30 field in about [-2.5, 2.5] (the gallery uses noise)."""
    g = rng(seed)
    x = np.linspace(0, 3, 30)
    phase = g.uniform(1)[0]
    return 2.5 * np.sin(np.add.outer(x, 1.3 * x) + 6 * phase)


def _example_plot(ax, seed, fontsize=12, hide_labels=False):
    pc = ax.pcolormesh(_field(seed), vmin=-2.5, vmax=2.5)
    if not hide_labels:
        ax.set_xlabel("x-label", fontsize=fontsize)
        ax.set_ylabel("y-label", fontsize=fontsize)
        ax.set_title("Title", fontsize=fontsize)
    return pc


@case("figure/gallery_subfigures", figsize=(9, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    subfigs = fig.subfigures(1, 2, wspace=0.07)
    axs_left = subfigs[0].subplots(1, 2, sharey=True)
    subfigs[0].set_facecolor("0.75")
    for i, ax in enumerate(axs_left):
        pc = _example_plot(ax, i)
    subfigs[0].suptitle("Left plots", fontsize="x-large")
    subfigs[0].colorbar(pc, shrink=0.6, ax=axs_left, location="bottom")
    axs_right = subfigs[1].subplots(3, 1, sharex=True)
    for nn, ax in enumerate(axs_right):
        pc = _example_plot(ax, 10 + nn, hide_labels=True)
        if nn == 2:
            ax.set_xlabel("xlabel")
        if nn == 1:
            ax.set_ylabel("ylabel")
    subfigs[1].set_facecolor("0.85")
    subfigs[1].colorbar(pc, shrink=0.6, ax=axs_right)
    subfigs[1].suptitle("Right plots", fontsize="x-large")
    fig.suptitle("Figure suptitle", fontsize="xx-large")


@case("figure/subfigures_nested_ratios", figsize=(8, 6.5))
def _(fig):
    fig.set_layout_engine("constrained")
    fig.suptitle("fig")
    subfigs = fig.subfigures(1, 2, wspace=0.07, width_ratios=[1.5, 1.0])
    subfigs[0].set_facecolor("coral")
    subfigs[0].suptitle("subfigs[0]")
    subfigs[1].set_facecolor("coral")
    subfigs[1].suptitle("subfigs[1]")
    nested = subfigs[0].subfigures(2, 1, height_ratios=[1, 1.4])
    nested[0].suptitle("subfigsnest[0]")
    nested[0].set_facecolor("r")
    ax0 = nested[0].subplots(1, 1)
    pc = _example_plot(ax0, 20, hide_labels=True)
    nested[0].colorbar(pc, ax=ax0)
    nested[1].suptitle("subfigsnest[1]")
    nested[1].set_facecolor("g")
    axs = nested[1].subplots(3, 1, sharex=True)
    for nn, ax in enumerate(axs):
        ax.set_ylabel(f"ylabel{nn}")
    nested[1].supxlabel("supxlabel")
    nested[1].supylabel("supylabel")
    axs_right = subfigs[1].subplots(2, 2)
    for i, ax in enumerate(axs_right.flat):
        ax.plot(T, np.cos(T + i))


@case("figure/tight_suptitle_suplabels", figsize=(6, 4.5))
def _(fig):
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        ax.plot(T * 1000, np.sin(T + i))
        ax.set_title(f"panel {i}")
        ax.tick_params(axis="x", labelrotation=30)
    fig.suptitle("A tight suptitle", fontsize=16)
    fig.supxlabel("time [ms]")
    fig.supylabel("signal")
    fig.tight_layout()


@case("figure/legends_outside_constrained", figsize=(7, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        ax.plot(T, np.sin(T + i), label=f"sin {i}")
        ax.plot(T, np.cos(T + i), "--", label=f"cos {i}")
    handles, labels = axs[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="outside right upper", title="right upper")
    fig.legend(handles[:1], labels[:1], loc="outside lower left", title="lower left", ncols=1)
    fig.legend(handles, labels, loc="outside left center", frameon=False)


@case("figure/legend_bbox_figure_coords", figsize=(6, 4))
def _(fig):
    axs = fig.subplots(1, 2)
    lines = []
    for i, ax in enumerate(axs):
        (line,) = ax.plot(T, np.sin(T + i), label=f"series {i}")
        lines.append(line)
    fig.subplots_adjust(bottom=0.25)
    fig.legend(handles=lines, loc="lower center", bbox_to_anchor=(0.5, 0.02),
               bbox_transform=fig.transFigure, ncols=2, mode="expand", borderaxespad=0.5)
    fig.legend(handles=lines[:1], loc="upper right", bbox_to_anchor=(1, 1), fontsize="small",
               title="anchored", title_fontsize="small")


@case("figure/compressed_layout_images", figsize=(5, 5))
def _(fig):
    fig.set_layout_engine("compressed")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        im = ax.imshow(_field(30 + i), vmin=-2.5, vmax=2.5)
        ax.set_title(f"image {i}")
    fig.colorbar(im, ax=axs, shrink=0.6)
    fig.suptitle("compressed")


@case("figure/constrained_subgridspec_colorbar", figsize=(7, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(1, 2, width_ratios=[1, 1.5])
    left = gs[0].subgridspec(2, 1)
    right = gs[1].subgridspec(2, 2, wspace=0.05)
    ax_top = fig.add_subplot(left[0])
    ax_bottom = fig.add_subplot(left[1])
    ax_top.plot(T, np.sin(T))
    ax_bottom.plot(T, np.cos(T))
    ax_bottom.set_xlabel("time")
    images = [fig.add_subplot(right[r, c]) for r in range(2) for c in range(2)]
    for i, ax in enumerate(images):
        mesh = ax.pcolormesh(_field(40 + i), cmap="magma", vmin=-2.5, vmax=2.5)
    fig.colorbar(mesh, ax=images, label="field")
    fig.colorbar(mesh, ax=[images[2], images[3]], location="bottom", shrink=0.7)


@case("figure/subfigure_legends_suplabels", figsize=(7, 3.5))
def _(fig):
    left, right = fig.subfigures(1, 2, wspace=0.05)
    for sub, offset in [(left, 0), (right, 1)]:
        ax = sub.subplots()
        ax.plot(T, np.sin(T + offset), label="sin")
        ax.plot(T, np.cos(T + offset), label="cos")
        sub.supxlabel(f"supxlabel {offset}")
    left.legend(loc="upper left", fontsize="small")
    right.legend(loc="lower center", ncols=2, frameon=False)
    right.set_facecolor("0.95")
    left.suptitle("left", x=0.1, ha="left")


@case("figure/savefig_tight_pad_extra_text", figsize=(5, 3.5),
      rc={"savefig.bbox": "tight", "savefig.pad_inches": 0.3})
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    ax.set_title("cropped to its content")
    fig.text(1.02, 0.5, "beyond the right edge", rotation=90, va="center")
    fig.text(-0.3, 0.5, "excluded from the box", va="center").set_in_layout(False)
    fig.subplots_adjust(left=0.3, right=0.6, top=0.7, bottom=0.3)
