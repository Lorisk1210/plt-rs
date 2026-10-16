"""Legends: placement, frame styling, entry geometry, figure legends and
legend handles for every common artist type."""

import numpy as np

from spec_support import case, rng

T = np.linspace(0, 10, 40)

LOCATIONS = [
    "upper right",
    "upper left",
    "lower left",
    "lower right",
    "right",
    "center left",
    "center right",
    "lower center",
    "upper center",
    "center",
]


def _three_lines(ax):
    ax.plot(T, np.sin(T), label="sine")
    ax.plot(T, np.cos(T), label="cosine")
    ax.plot(T, np.sin(T) * np.cos(T), label="product")


# --- placement -------------------------------------------------------------


@case("legend/default_best")
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend()


@case("legend/best_avoids_crowded_data", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 1, 30)
    ax.plot(x, 1 - x * 0.2, "o", label="top band")
    ax.plot(x, x * 0.1, "s", label="bottom band")
    ax.plot(np.full(30, 0.02), x, "^", label="left edge")
    ax.legend(loc="best")


@case("legend/all_location_strings", figsize=(10, 4.5))
def _(fig):
    axs = fig.subplots(2, 5)
    for ax, loc in zip(axs.flat, LOCATIONS):
        ax.plot([0, 1], [0, 1], label=loc)
        ax.legend(loc=loc, fontsize=7)
        ax.set_xticks([])
        ax.set_yticks([])


@case("legend/numeric_location_codes", figsize=(6, 4))
def _(fig):
    axs = fig.subplots(2, 2)
    for code, ax in zip([1, 3, 6, 10], axs.flat):
        ax.plot([0, 1], [1, 0], label=f"loc={code}")
        ax.legend(loc=code)


@case("legend/bbox_to_anchor_outside_right")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(right=0.7)
    _three_lines(ax)
    ax.legend(loc="upper left", bbox_to_anchor=(1.02, 1), borderaxespad=0)


@case("legend/bbox_to_anchor_below_mode_expand")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.25)
    _three_lines(ax)
    ax.legend(loc="upper left", bbox_to_anchor=(0, -0.22, 1, 0.1), mode="expand", ncols=3)


@case("legend/figure_legend")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.plot(T, np.sin(T), label="left sine")
    ax2.plot(T, np.cos(T), "C1", label="right cosine")
    fig.legend(loc="upper center", ncols=2)


@case("legend/figure_legend_explicit_handles", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    (a,) = ax.plot(T, np.sin(T))
    (b,) = ax.plot(T, np.cos(T))
    fig.legend([a, b], ["first", "second"], loc="lower right")


# --- layout of entries -----------------------------------------------------


@case("legend/ncols", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    for i in range(6):
        ax.plot(T, np.sin(T + i), label=f"phase {i}")
    ax.legend(ncols=3, loc="lower center")


@case("legend/title_and_fontsizes", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(title="Functions", title_fontsize=13, fontsize="small")


@case("legend/alignment_left_title", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(title="A rather long legend title", alignment="left")


@case("legend/handlelength_spacing_pads", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(handlelength=4, handleheight=1.5, handletextpad=1.5, labelspacing=1.2, borderpad=1.0, borderaxespad=2, columnspacing=3)


@case("legend/markerscale_numpoints", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T), "o-", markersize=4, label="dots")
    ax.plot(T, np.cos(T), "s--", markersize=4, label="squares")
    ax.legend(markerscale=2.5, numpoints=3)


@case("legend/scatterpoints_and_yoffsets", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = rng(300).uniform(30), rng(301).uniform(30)
    ax.scatter(x, y, s=np.linspace(10, 200, 30), label="sized")
    ax.scatter(y, x, marker="^", label="triangles")
    ax.legend(scatterpoints=3, scatteryoffsets=[0.2, 0.5, 0.8])


@case("legend/reverse_order", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(reverse=True)


@case("legend/nolegend_and_underscore_labels", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T), label="shown")
    ax.plot(T, np.cos(T), label="_nolegend_")
    ax.plot(T, -np.sin(T), label="_hidden")
    ax.plot(T, -np.cos(T))
    ax.plot(T, 0.5 * np.sin(T), label="also shown")
    ax.legend()


@case("legend/labelcolor_linecolor", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(labelcolor="linecolor")



# --- frame styling ---------------------------------------------------------


@case("legend/frameon_false", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(frameon=False)


@case("legend/square_frame_with_shadow", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _three_lines(ax)
    ax.legend(fancybox=False, shadow=True)


@case("legend/framealpha_facecolor_edgecolor", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.fill_between(T, -1, 1, color="tab:green")
    ax.plot(T, np.sin(T), "k", label="sine")
    ax.legend(framealpha=0.3, facecolor="yellow", edgecolor="red", loc="center")


# --- handles for artist types ----------------------------------------------


@case("legend/handles_lines_and_markers", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T), "--", linewidth=3, label="dashed thick")
    ax.plot(T, np.cos(T), "o", label="markers only")
    ax.plot(T, -np.sin(T), "s-", markevery=5, label="line + markers")
    ax.plot(T, -np.cos(T), "D", fillstyle="left", markersize=8, label="half filled")
    ax.legend()


@case("legend/handles_patches_and_bars", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar([0, 1, 2], [3, 2, 4], label="bars")
    ax.bar([3, 4], [1, 2], hatch="//", edgecolor="k", color="C1", label="hatched")
    ax.axvspan(4.5, 5.5, color="0.8", label="span")
    ax.legend()


@case("legend/handles_fill_between_and_stackplot", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.fill_between(T, np.sin(T) - 0.3, np.sin(T) + 0.3, alpha=0.4, label="band")
    ax1.plot(T, np.sin(T), label="center")
    ax1.legend()
    ax2.stackplot(T, np.abs(np.sin(T)), np.abs(np.cos(T)), labels=["first", "second"])
    ax2.legend(loc="upper left")


@case("legend/handles_errorbar_containers", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(6)
    ax.errorbar(x, x, yerr=0.5, fmt="o-", capsize=3, label="y errors")
    ax.errorbar(x, x + 3, xerr=0.3, yerr=0.4, fmt="s", label="x and y errors")
    ax.errorbar(x, x + 6, yerr=0.4, fmt="none", label="bars only")
    ax.legend()


@case("legend/handles_scatter_collections", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = rng(302).uniform(20), rng(303).uniform(20)
    ax.scatter(x, y, c=y, cmap="viridis", label="colormapped")
    ax.scatter(y, x, marker="x", color="k", label="crosses")
    ax.scatter(x, x, s=150, facecolors="none", edgecolors="r", label="hollow")
    ax.legend()


@case("legend/handles_stem_and_step", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(10)
    ax.stem(x, np.sin(x / 2), label="stem")
    ax.step(x, np.cos(x / 2) + 1, label="step")
    ax.stairs(np.arange(1, 10) / 5 - 1.5, label="stairs")
    ax.legend()


@case("legend/handles_hist_types", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    data = rng(304).normal(300)
    ax.hist(data, bins=15, label="bar")
    ax.hist(data + 3, bins=15, histtype="step", label="step")
    ax.hist(data - 3, bins=15, histtype="stepfilled", alpha=0.5, label="stepfilled")
    ax.legend()
