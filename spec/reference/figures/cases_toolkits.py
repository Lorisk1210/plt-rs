"""``mpl_toolkits``: ``axes_grid1`` image grids, dividers with colorbars
and marginal axes, inset locators and RGB axes, and ``axisartist`` axis
lines."""

import numpy as np
from mpl_toolkits import axisartist
from mpl_toolkits.axes_grid1 import ImageGrid, make_axes_locatable
from mpl_toolkits.axes_grid1.axes_rgb import RGBAxes
from mpl_toolkits.axes_grid1.inset_locator import inset_axes, mark_inset, zoomed_inset_axes

from spec_support import case, rng


def _field(n=20, phase=0.0):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(xx + phase) * np.cos(yy)


# --- axes_grid1: image grids


@case("toolkits/imagegrid_cbar_edge", figsize=(7, 4))
def _(fig):
    right = ImageGrid(fig, 121, nrows_ncols=(2, 2), axes_pad=0.1, cbar_mode="edge",
                      cbar_location="right", cbar_pad=0.05)
    for i, ax in enumerate(right):
        im = ax.imshow(_field(phase=i), cmap=["viridis", "plasma"][i // 2], vmin=-1, vmax=1)
        if i % 2 == 1:
            right.cbar_axes[i // 2].colorbar(im)
    bottom = ImageGrid(fig, 122, nrows_ncols=(2, 2), axes_pad=0.1, cbar_mode="edge",
                       cbar_location="bottom", cbar_pad=0.25, cbar_size="10%")
    for i, ax in enumerate(bottom):
        im = ax.imshow(_field(phase=-i) * (1 + i % 2), cmap=["cividis", "magma"][i % 2])
        if i >= 2:
            bottom.cbar_axes[i % 2].colorbar(im)


@case("toolkits/imagegrid_direction_label_modes", figsize=(7, 4))
def _(fig):
    lines = ImageGrid(fig, 121, nrows_ncols=(2, 3), direction="column", label_mode="1",
                      share_all=True, aspect=False, axes_pad=(0.3, 0.2), n_axes=5)
    x = np.linspace(0.0, 2 * np.pi, 50)
    for i, ax in enumerate(lines):
        ax.plot(x, np.sin(x + i), color=f"C{i}")
        ax.set_title(str(i), fontsize=8)
    images = ImageGrid(fig, 122, nrows_ncols=(2, 2), label_mode="all", axes_pad=0.4)
    for i, ax in enumerate(images):
        ax.imshow(_field(8, phase=i), cmap="gray")


@case("toolkits/imagegrid_each_cbar_left", figsize=(7, 3))
def _(fig):
    grid = ImageGrid(fig, 111, nrows_ncols=(1, 3), axes_pad=0.7, cbar_mode="each",
                     cbar_location="left", cbar_size="10%", cbar_pad=0.25)
    for i, (ax, cax) in enumerate(zip(grid, grid.cbar_axes)):
        im = ax.imshow(_field(12, phase=i) * 10 ** i, cmap="RdBu_r")
        cax.colorbar(im, label=f"scale 1e{i}")


# --- axes_grid1: dividers


@case("toolkits/divider_colorbars_sides", figsize=(6, 5))
def _(fig):
    ax = fig.subplots()
    field = _field(30)
    im = ax.imshow(field, cmap="viridis")
    divider = make_axes_locatable(ax)
    fig.colorbar(im, cax=divider.append_axes("right", size="5%", pad=0.05))
    fig.colorbar(im, cax=divider.append_axes("bottom", size=0.15, pad=0.5),
                 orientation="horizontal")
    left = divider.append_axes("left", size="20%", pad=0.4, sharey=ax)
    left.plot(np.abs(field).mean(axis=1), np.arange(30))
    left.invert_xaxis()


@case("toolkits/divider_marginal_histograms", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    r = rng(41)
    x = r.normal(400)
    y = 0.6 * x + r.normal(400, scale=0.7)
    ax.scatter(x, y, s=6, alpha=0.6)
    ax.set_aspect(1.0)
    divider = make_axes_locatable(ax)
    top = divider.append_axes("top", 1.0, pad=0.1, sharex=ax)
    right = divider.append_axes("right", 1.0, pad=0.1, sharey=ax)
    top.xaxis.set_tick_params(labelbottom=False)
    right.yaxis.set_tick_params(labelleft=False)
    bins = np.linspace(-4.0, 4.0, 33)
    top.hist(x, bins=bins)
    right.hist(y, bins=bins, orientation="horizontal", color="tab:orange")


# --- axes_grid1: inset locators and RGB axes


@case("toolkits/inset_locator_inset_axes", figsize=(6, 4))
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.65, 0.8))
    im = ax.imshow(_field(40), extent=(-3, 3, -3, 3), cmap="coolwarm")
    upper = inset_axes(ax, width="30%", height="40%")
    upper.plot(np.linspace(0, 1, 20) ** 2)
    lower = inset_axes(ax, width=1.0, height=0.6, loc="lower left", borderpad=1.5)
    lower.hist(_field(40).ravel(), bins=20, color="0.4")
    lower.set_xticks([])
    lower.set_yticks([])
    padded = inset_axes(ax, width="25%", height="20%", loc="lower right", borderpad=(3, 0.5))
    padded.set_facecolor("lightyellow")
    cax = inset_axes(ax, width="5%", height="100%", loc="lower left",
                     bbox_to_anchor=(1.05, 0.0, 1, 1), bbox_transform=ax.transAxes, borderpad=0)
    fig.colorbar(im, cax=cax)


@case("toolkits/zoomed_inset_mark_inset", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_field(60), extent=(-3, 3, -3, 3), origin="lower", cmap="magma")
    axins = zoomed_inset_axes(ax, zoom=2.5, loc="upper left")
    axins.imshow(_field(60), extent=(-3, 3, -3, 3), origin="lower", cmap="magma")
    axins.set_xlim(0.5, 1.5)
    axins.set_ylim(-0.8, 0.2)
    axins.set_xticks([])
    axins.set_yticks([])
    mark_inset(ax, axins, loc1=2, loc2=4, fc="none", ec="0.8")
    line_ax = zoomed_inset_axes(ax, zoom=0.6, loc="lower right", borderpad=1)
    line_ax.plot([0, 1, 2], [1, 0, 1], color="w")
    line_ax.set_facecolor("0.2")


@case("toolkits/rgb_axes", figsize=(5, 4))
def _(fig):
    ax = RGBAxes(fig, [0.1, 0.1, 0.8, 0.8], pad=0.0)
    x = np.linspace(-1.0, 1.0, 24)
    xx, yy = np.meshgrid(x, x)
    r = np.clip(1 - xx**2 - yy**2, 0, 1)
    g = (xx + 1) / 2
    b = (yy + 1) / 2
    ax.imshow_rgb(r, g, b, origin="lower")


# --- axisartist


@case("toolkits/axisartist_axisline_styles", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.axislines.AxesZero)
    ax.axis["xzero"].set_axisline_style("->", size=1.5)
    ax.axis["yzero"].set_axisline_style("-|>", size=2.0)
    for direction in ["xzero", "yzero"]:
        ax.axis[direction].set_visible(True)
    for direction in ["left", "right", "bottom", "top"]:
        ax.axis[direction].set_visible(False)
    x = np.linspace(-1.0, 1.0, 50)
    ax.plot(x, x**3 - 0.5 * x)
    ax.set_xlim(-1.2, 1.2)
    ax.set_ylim(-0.8, 0.8)
