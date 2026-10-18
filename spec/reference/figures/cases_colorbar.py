"""Colorbars: placement (orientation, location, shrink/aspect/pad/fraction,
cax, multiple parent axes, constrained layout), extensions, ticks/format/
label, discrete and non-linear norms, edges and contour lines."""

import numpy as np
from matplotlib import cm, colors, ticker

from spec_support import case, rng


def _field(n=40):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(xx) * np.cos(yy * 0.8) * np.exp(-(xx**2 + yy**2) / 12.0)


def _xyz(n=40):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return xx, yy, np.sin(xx) * np.cos(yy * 0.8) * np.exp(-(xx**2 + yy**2) / 12.0)


@case("colorbar/image_default")
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax)


@case("colorbar/horizontal", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), cmap="plasma")
    fig.colorbar(im, ax=ax, orientation="horizontal")


@case("colorbar/location_top", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax, location="top")


@case("colorbar/location_left", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax, location="left")


@case("colorbar/shrink_aspect_pad", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax, shrink=0.6, aspect=8, pad=0.15)


@case("colorbar/fraction", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), aspect="auto")
    fig.colorbar(im, ax=ax, fraction=0.3)


@case("colorbar/extend_both_contourf", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contourf(*_xyz(), levels=np.linspace(-0.5, 0.5, 11), extend="both", cmap="RdBu_r")
    fig.colorbar(cs, ax=ax)


@case("colorbar/extend_min_extendfrac", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), vmin=-0.3)
    fig.colorbar(im, ax=ax, extend="min", extendfrac=0.25)


@case("colorbar/extend_max_extendrect", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), vmax=0.3)
    fig.colorbar(im, ax=ax, extend="max", extendrect=True)


@case("colorbar/extend_both_horizontal_auto", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), vmin=-0.3, vmax=0.3, cmap="coolwarm")
    fig.colorbar(im, ax=ax, orientation="horizontal", extend="both", extendfrac="auto")


@case("colorbar/ticks_format_label", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax, ticks=[-0.6, -0.2, 0.0, 0.2, 0.6], format="%.2f", label="amplitude")


@case("colorbar/formatter_minorticks", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(np.abs(_field()), cmap="magma")
    cb = fig.colorbar(im, ax=ax, format=ticker.PercentFormatter(xmax=1.0))
    cb.minorticks_on()
    cb.set_label("fraction", loc="top")


@case("colorbar/boundarynorm_uniform", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cmap = colors.ListedColormap(["#1b9e77", "#d95f02", "#7570b3", "#e7298a", "#66a61e"])
    norm = colors.BoundaryNorm([-1.0, -0.4, -0.1, 0.1, 0.4, 1.0], cmap.N)
    im = ax.imshow(_field(), cmap=cmap, norm=norm)
    fig.colorbar(im, ax=ax)


@case("colorbar/boundarynorm_proportional_extend", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    norm = colors.BoundaryNorm([-0.6, -0.4, -0.1, 0.1, 0.4, 0.6], 256, extend="both")
    im = ax.imshow(_field(), cmap="viridis", norm=norm)
    fig.colorbar(im, ax=ax, spacing="proportional")


@case("colorbar/lognorm", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 4.0, 40)
    im = ax.imshow(10.0 ** np.add.outer(x, x / 2), norm=colors.LogNorm())
    fig.colorbar(im, ax=ax)


@case("colorbar/twoslopenorm", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    norm = colors.TwoSlopeNorm(vmin=-0.5, vcenter=0.0, vmax=2.0)
    im = ax.imshow(_field() * 3, norm=norm, cmap="RdBu_r")
    fig.colorbar(im, ax=ax)


@case("colorbar/drawedges", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field(), cmap=colors.ListedColormap(["navy", "teal", "gold", "tomato"]))
    fig.colorbar(im, ax=ax, drawedges=True)


@case("colorbar/scalarmappable_only", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2], [2, 0, 1])
    sm = cm.ScalarMappable(norm=colors.Normalize(0.0, 10.0), cmap="cividis")
    fig.colorbar(sm, ax=ax, label="speed")


@case("colorbar/scatter", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    g = rng(11)
    x, y = g.uniform(60), g.uniform(60)
    sc = ax.scatter(x, y, c=x + y, cmap="plasma")
    fig.colorbar(sc, ax=ax)


@case("colorbar/contour_lines", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contour(*_xyz(), levels=8)
    fig.colorbar(cs, ax=ax)


@case("colorbar/contourf_add_lines", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    xx, yy, zz = _xyz()
    cf = ax.contourf(xx, yy, zz, levels=8, cmap="Greens")
    cl = ax.contour(xx, yy, zz, levels=cf.levels, colors="k", linewidths=0.8)
    cb = fig.colorbar(cf, ax=ax)
    cb.add_lines(cl)


@case("colorbar/multiple_axes", figsize=(5, 5))
def _(fig):
    ax1, ax2 = fig.subplots(2, 1)
    im = ax1.imshow(_field(), vmin=-1, vmax=1)
    ax2.imshow(-_field(), vmin=-1, vmax=1)
    fig.colorbar(im, ax=[ax1, ax2])


@case("colorbar/cax_explicit", figsize=(5, 3.5))
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.65, 0.8))
    cax = fig.add_axes((0.82, 0.1, 0.04, 0.8))
    im = ax.imshow(_field(), aspect="auto")
    fig.colorbar(im, cax=cax)


@case("colorbar/inset_cax_horizontal", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    cax = ax.inset_axes((0.1, 0.9, 0.8, 0.05))
    fig.colorbar(im, cax=cax, orientation="horizontal")


@case("colorbar/constrained_layout", figsize=(6, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        im = ax.imshow(_field(20) * (i + 1), cmap="viridis")
        fig.colorbar(im, ax=ax)


@case("colorbar/constrained_shared", figsize=(6, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        im = ax.imshow(_field(20) * (i + 1), vmin=-2, vmax=2)
    fig.colorbar(im, ax=axs, shrink=0.8, location="bottom")


@case("colorbar/follows_set_clim", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    fig.colorbar(im, ax=ax)
    im.set_clim(-0.2, 0.4)
    im.set_cmap("magma")


@case("colorbar/follows_norm_change", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(10.0 ** (2.0 * _field() + 1.0))
    fig.colorbar(im, ax=ax)
    im.set_norm(colors.LogNorm())
