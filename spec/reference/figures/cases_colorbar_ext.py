"""Colorbars from matplotlib's test suite: extension lengths and shapes for
uniform and proportional spacing, edges with extensions, explicit
boundaries and values, log, power, symlog and centred norms, anchors and
tick locations."""

import matplotlib as mpl
import numpy as np
from matplotlib import colors
from matplotlib.colorbar import Colorbar

from spec_support import case


def _cmap_norms():
    """`_get_cmap_norms` of test_colorbar.py."""
    cmap = mpl.colormaps["RdBu"].resampled(5)
    clevs = [-5.0, -2.5, -0.5, 0.5, 1.5, 3.5]
    norms = {
        "neither": colors.BoundaryNorm(clevs, len(clevs) - 1),
        "min": colors.BoundaryNorm([-10] + clevs[1:], len(clevs) - 1),
        "max": colors.BoundaryNorm(clevs[:-1] + [10], len(clevs) - 1),
        "both": colors.BoundaryNorm([-10] + clevs[1:-1] + [10], len(clevs) - 1),
    }
    return cmap, norms


def _extension_length(fig, spacing):
    cmap, norms = _cmap_norms()
    fig.subplots_adjust(hspace=0.6)
    for i, extension_type in enumerate(("neither", "min", "max", "both")):
        norm = norms[extension_type]
        boundaries = values = norm.boundaries
        values = values[:-1]
        for j, extendfrac in enumerate((None, "auto", 0.1)):
            cax = fig.add_subplot(12, 1, i * 3 + j + 1)
            Colorbar(cax, cmap=cmap, norm=norm, boundaries=boundaries, values=values,
                     extend=extension_type, extendfrac=extendfrac, orientation="horizontal",
                     spacing=spacing)
            cax.tick_params(left=False, labelleft=False, bottom=False, labelbottom=False)


@case("colorbar/extension_length_uniform")
def _(fig):
    _extension_length(fig, "uniform")


@case("colorbar/extension_length_proportional")
def _(fig):
    _extension_length(fig, "proportional")


@case("colorbar/extension_shape_rect")
def _(fig):
    cmap, norms = _cmap_norms()
    fig.subplots_adjust(hspace=4, wspace=0.3)
    for col, spacing in enumerate(("uniform", "proportional")):
        for i, extension_type in enumerate(("neither", "min", "max", "both")):
            norm = norms[extension_type]
            boundaries = values = norm.boundaries
            values = values[:-1]
            cax = fig.add_subplot(4, 2, 2 * i + col + 1)
            Colorbar(cax, cmap=cmap, norm=norm, boundaries=boundaries, values=values,
                     extend=extension_type, extendrect=True, orientation="horizontal",
                     spacing=spacing)
            cax.tick_params(left=False, labelleft=False, bottom=False, labelbottom=False)


@case("colorbar/drawedges_with_extensions", figsize=(5, 4))
def _(fig):
    cmap = colors.ListedColormap(["tab:blue", "tab:orange", "tab:green", "tab:red",
                                  "tab:purple", "tab:brown"])
    fig.subplots_adjust(hspace=1.5)
    for i, (extend, frac) in enumerate([("neither", None), ("both", None), ("min", 0.3),
                                        ("max", "auto")]):
        cax = fig.add_subplot(5, 1, i + 1)
        lo = 1 if extend in ("min", "both") else 0
        hi = len(cmap.colors) - (1 if extend in ("max", "both") else 0)
        norm = colors.BoundaryNorm(np.arange(lo, hi + 1), cmap.N, extend=extend)
        Colorbar(cax, cmap=cmap, norm=norm, orientation="horizontal", drawedges=True,
                 extendfrac=frac)
        cax.set_title(f"extend={extend}", fontsize=8)
    cax = fig.add_subplot(5, 1, 5)
    norm = colors.BoundaryNorm([0, 1, 2, 4, 6], cmap.N, extend="both")
    Colorbar(cax, cmap=cmap, norm=norm, orientation="horizontal", drawedges=True,
             extendrect=True, spacing="proportional")


def _wave(n=40):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(xx) * np.cos(yy * 0.8) * np.exp(-(xx**2 + yy**2) / 12.0)


@case("colorbar/boundaries_values_irregular", figsize=(6, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    im = ax1.imshow(_wave(), cmap="viridis")
    fig.colorbar(im, ax=ax1, boundaries=[-0.8, -0.4, -0.1, 0.0, 0.2, 0.8],
                 values=[-0.6, -0.25, -0.05, 0.1, 0.5], ticks=[-0.8, -0.1, 0.2, 0.8],
                 format="%.2f")
    norm = colors.BoundaryNorm([-1.0, -0.3, -0.1, 0.0, 0.1, 0.3, 1.0], 256)
    im = ax2.imshow(_wave(), cmap="coolwarm", norm=norm)
    fig.colorbar(im, ax=ax2, orientation="horizontal", spacing="proportional",
                 ticks=[-1.0, -0.1, 0.1, 1.0], label="proportional")


@case("colorbar/lognorm_extend_minor_custom_ticks", figsize=(6, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = np.linspace(0.0, 4.0, 40)
    data = 10.0 ** np.add.outer(x, x / 2)
    im = ax1.imshow(data, norm=colors.LogNorm(vmin=10, vmax=1e4))
    cb = fig.colorbar(im, ax=ax1, extend="both")
    cb.minorticks_on()
    cb.ax.tick_params(which="minor", length=4, color="tab:red")
    im = ax2.imshow(data, norm=colors.LogNorm(), cmap="magma")
    cb = fig.colorbar(im, ax=ax2, orientation="horizontal")
    cb.set_ticks([1, 100, 1e4, 1e6], labels=["one", "hundred", "10k", "1M"])
    cb.minorticks_off()


@case("colorbar/power_symlog_centered_norms", figsize=(8, 3))
def _(fig):
    axs = fig.subplots(1, 3)
    data = _wave() * 3
    im = axs[0].imshow(np.abs(data), norm=colors.PowerNorm(gamma=0.5), cmap="cividis")
    fig.colorbar(im, ax=axs[0])
    im = axs[1].imshow(data * 10, norm=colors.SymLogNorm(linthresh=0.3, vmin=-30, vmax=30),
                       cmap="RdBu_r")
    fig.colorbar(im, ax=axs[1])
    im = axs[2].imshow(data + 0.5, norm=colors.CenteredNorm(vcenter=0.5, halfrange=2.0),
                       cmap="PiYG")
    fig.colorbar(im, ax=axs[2], extend="both")


@case("colorbar/anchor_panchor_ticklocation", figsize=(6, 4))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    im = ax1.imshow(_wave(), cmap="plasma")
    fig.colorbar(im, ax=ax1, shrink=0.5, anchor=(0.0, 1.0), panchor=(1.0, 0.0),
                 ticklocation="left", label="left ticks")
    im = ax2.imshow(_wave(), cmap="plasma")
    fig.colorbar(im, ax=ax2, location="top", shrink=0.7, panchor=False, aspect=10,
                 label="on top")
