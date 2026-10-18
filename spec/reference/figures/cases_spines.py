"""Spines: the gallery's spine examples (bounds, dropped and centred spines,
a third y axis on an offset spine) plus spines on log axes, spines moved
with their ticks and spine styling."""

import numpy as np

from spec_support import case


@case("spines/gallery_bounds")
def _(fig):
    x = np.linspace(0, 2 * np.pi, 100)
    y = 2 * np.sin(x)
    ax0, ax1, ax2 = fig.subplots(nrows=3)
    ax0.plot(x, y)
    ax0.set_title("normal spines")
    ax1.plot(x, y)
    ax1.set_title("bottom-left spines")
    ax1.spines.right.set_visible(False)
    ax1.spines.top.set_visible(False)
    ax2.plot(x, y)
    ax2.set_title("spines with bounds limited to data range")
    ax2.spines.bottom.set_bounds(x.min(), x.max())
    ax2.spines.left.set_bounds(y.min(), y.max())
    ax2.spines.right.set_visible(False)
    ax2.spines.top.set_visible(False)
    fig.tight_layout()


@case("spines/dropped")
def _(fig):
    x = np.linspace(0, 2 * np.pi, 100)
    axs = fig.subplots(2, 2)
    axs[0, 0].plot(x, np.sin(x))
    axs[0, 1].plot(x, np.cos(x))
    axs[1, 0].plot(x, -np.cos(x))
    axs[1, 1].plot(x, -np.sin(x))
    for ax, visible in [
        (axs[0, 0], ["left"]),
        (axs[0, 1], []),
        (axs[1, 0], ["left", "bottom"]),
        (axs[1, 1], ["bottom"]),
    ]:
        ax.label_outer(remove_inner_ticks=True)
        ax.grid(color="0.9")
        for loc, spine in ax.spines.items():
            if loc in visible:
                spine.set_position(("outward", 10))
            else:
                spine.set_visible(False)


@case("spines/centered_with_arrows", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.spines[["left", "bottom"]].set_position(("data", 0))
    ax.spines[["top", "right"]].set_visible(False)
    ax.plot(1, 0, ">k", transform=ax.get_yaxis_transform(), clip_on=False)
    ax.plot(0, 1, "^k", transform=ax.get_xaxis_transform(), clip_on=False)
    x = np.linspace(-0.5, 1.0, 100)
    ax.plot(x, np.sin(x * np.pi))


@case("spines/multiple_yaxis")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(right=0.75)
    twin1 = ax.twinx()
    twin2 = ax.twinx()
    twin2.spines.right.set_position(("axes", 1.2))
    (p1,) = ax.plot([0, 1, 2], [0, 1, 2], "C0", label="Density")
    (p2,) = twin1.plot([0, 1, 2], [0, 3, 2], "C1", label="Temperature")
    (p3,) = twin2.plot([0, 1, 2], [50, 30, 15], "C2", label="Velocity")
    ax.set(xlim=(0, 2), ylim=(0, 2), xlabel="Distance", ylabel="Density")
    twin1.set(ylim=(0, 4), ylabel="Temperature")
    twin2.set(ylim=(1, 65), ylabel="Velocity")
    ax.yaxis.label.set_color(p1.get_color())
    twin1.yaxis.label.set_color(p2.get_color())
    twin2.yaxis.label.set_color(p3.get_color())
    ax.tick_params(axis="y", colors=p1.get_color())
    twin1.tick_params(axis="y", colors=p2.get_color())
    twin2.tick_params(axis="y", colors=p3.get_color())
    ax.legend(handles=[p1, p2, p3])


@case("spines/log_axes_outward_bounds", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.logspace(0, 3, 40)
    ax.loglog(x, 1e4 / x, "o-", markersize=3)
    ax.spines[["left", "bottom"]].set_position(("outward", 8))
    ax.spines.left.set_bounds(10, 1e4)
    ax.spines.bottom.set_bounds(1, 1e3)
    ax.spines[["top", "right"]].set_visible(False)


@case("spines/moved_with_ticks_and_labels", figsize=(6, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    t = np.linspace(-2, 3, 60)
    ax1.plot(t, t**2 - 2)
    ax1.spines.top.set_position(("data", 0))
    ax1.spines.bottom.set_visible(False)
    ax1.xaxis.set_ticks_position("top")
    ax1.spines.left.set_position(("axes", 0.3))
    ax1.set_xlabel("x label")
    ax1.set_ylabel("y label")
    ax2.plot(t, np.cos(t))
    ax2.spines.bottom.set_position(("axes", -0.08))
    ax2.spines.left.set_position(("outward", -10))
    ax2.spines.right.set_position("center")
    ax2.yaxis.set_ticks_position("both")
    fig.tight_layout()


@case("spines/styles_and_colors", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1])
    ax.spines.left.set(linewidth=4, color="tab:red", capstyle="round")
    ax.spines.bottom.set(linewidth=3, color="tab:blue", linestyle="--", alpha=0.5)
    ax.spines.top.set(linewidth=2, edgecolor="tab:green", linestyle=":")
    ax.spines.right.set(linewidth=6, color="0.6", zorder=0)
    ax.set_facecolor("ivory")
