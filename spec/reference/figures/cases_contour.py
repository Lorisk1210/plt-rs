"""Contour and filled contour: levels, colours, line styles, extend, hatches,
labels, masking/corner_mask, log locator; unstructured triangular variants
(tricontour, tricontourf, tripcolor, triplot)."""

import matplotlib as mpl
import numpy as np
from matplotlib import ticker, tri

from spec_support import case, rng


def _xyz(nx=60, ny=45):
    """Two Gaussian bumps of opposite sign (the classic contour demo field)."""
    x = np.linspace(-3.0, 3.0, nx)
    y = np.linspace(-2.0, 2.0, ny)
    xx, yy = np.meshgrid(x, y)
    z1 = np.exp(-(xx**2) - yy**2)
    z2 = np.exp(-((xx - 1) ** 2) - (yy - 1) ** 2)
    return xx, yy, (z1 - z2) * 2


def _triangulation(n=60, seed=21):
    g = rng(seed)
    x = np.concatenate([g.uniform(n, -3.0, 3.0), [-3.0, 3.0, 3.0, -3.0]])
    y = np.concatenate([g.uniform(n, -2.0, 2.0), [-2.0, -2.0, 2.0, 2.0]])
    return tri.Triangulation(x, y)


def _tri_z(t):
    return np.exp(-(t.x**2) - t.y**2) * 2 - np.exp(-((t.x - 1) ** 2) - (t.y - 1) ** 2) * 2


@case("contour/contour_default")
def _(fig):
    ax = fig.subplots()
    ax.contour(_xyz()[2])


@case("contour/contour_xy_levels_int", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=12)


@case("contour/single_color_negative_dashed", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=10, colors="k")


@case("contour/negative_linestyles_dotted", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=10, colors="tab:blue", negative_linestyles="dotted")


@case("contour/linestyles_linewidths_lists", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=[-1.5, -0.5, 0.0, 0.5, 1.5], colors=["r", "g", "b"],
               linestyles=["-", "--", ":"], linewidths=[0.5, 1.5, 3.0])


@case("contour/single_level_cmap", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=[0.0], cmap="plasma", linewidths=2)


@case("contour/contourf_default")
def _(fig):
    ax = fig.subplots()
    ax.contourf(*_xyz())


@case("contour/contourf_levels_cmap", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contourf(*_xyz(), levels=[-2.0, -1.0, -0.25, 0.0, 0.25, 1.0, 2.0], cmap="RdBu_r")


@case("contour/contourf_extend_both", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    cmap = mpl.colormaps["coolwarm"].with_extremes(under="black", over="yellow")
    ax.contourf(*_xyz(), levels=np.linspace(-0.8, 0.8, 9), extend="both", cmap=cmap)


@case("contour/contourf_alpha_overlay_lines", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy, zz = _xyz()
    ax.contourf(xx, yy, zz, levels=8, alpha=0.6)
    ax.contour(xx, yy, zz, levels=8, colors="k", linewidths=0.6)


@case("contour/contourf_hatches", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contourf(*_xyz(), levels=[-2.0, -0.5, 0.0, 0.5, 2.0],
                hatches=["/", "\\", None, "x"], cmap="Pastel1")


@case("contour/contourf_hatches_only", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.contourf(*_xyz(), levels=[-2.0, -0.5, 0.5, 2.0], colors="none",
                hatches=["..", None, "//"], extend="neither")
    ax.contour(*_xyz(), levels=[-0.5, 0.5], colors="k")


@case("contour/clabel_default", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contour(*_xyz(), levels=8)
    ax.clabel(cs)


@case("contour/clabel_fmt_fontsize_not_inline", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contour(*_xyz(), levels=[-1.0, -0.5, 0.5, 1.0], colors="k")
    ax.clabel(cs, fmt="%1.2f m", fontsize=7, inline=False)


@case("contour/clabel_dict_colors", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contour(*_xyz(), levels=[-1.0, 0.0, 1.0])
    ax.clabel(cs, fmt={-1.0: "low", 0.0: "zero", 1.0: "high"}, colors="k", fontsize=11,
              inline_spacing=8)


@case("contour/masked_corner_mask_true_false", figsize=(6, 2.5))
def _(fig):
    axs = fig.subplots(1, 2)
    xx, yy, zz = _xyz(13, 10)
    zz = np.ma.array(zz, mask=(np.arange(130).reshape(10, 13) % 7 == 0))
    for ax, corner_mask in zip(axs, [True, False]):
        ax.contourf(xx, yy, zz, levels=8, corner_mask=corner_mask)
        ax.contour(xx, yy, zz, levels=8, colors="k", corner_mask=corner_mask)
        ax.set_title(f"corner_mask={corner_mask}")


@case("contour/nan_region", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy, zz = _xyz()
    zz[(xx > 0.5) & (yy < 0.0)] = np.nan
    ax.contourf(xx, yy, zz, levels=10, cmap="viridis")


@case("contour/log_locator", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-3.0, 3.0, 60)
    xx, yy = np.meshgrid(x, x)
    zz = np.exp(-(xx**2) - yy**2) * 100 + np.exp(-((xx - 1.5) ** 2) - (yy - 1) ** 2) * 1e4 + 1e-3
    ax.contourf(xx, yy, zz, locator=ticker.LogLocator(), cmap="PuBu_r")


@case("contour/origin_extent_image", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    zz = _xyz(30, 20)[2]
    ax.imshow(zz, origin="lower", extent=(0, 6, 0, 4), cmap="gray")
    ax.contour(zz, levels=6, origin="lower", extent=(0, 6, 0, 4), cmap="autumn")


@case("contour/tricontour", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation()
    ax.tricontour(t, _tri_z(t), levels=10)


@case("contour/tricontourf_levels", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation()
    ax.tricontourf(t, _tri_z(t), levels=[-2.0, -1.0, -0.3, 0.3, 1.0, 2.0], cmap="RdYlBu")


@case("contour/tripcolor_flat", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation()
    ax.tripcolor(t, _tri_z(t), edgecolors="w", linewidth=0.3)


@case("contour/tripcolor_gouraud", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation()
    ax.tripcolor(t, _tri_z(t), shading="gouraud", cmap="magma")


@case("contour/tripcolor_facecolors", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation(30, 22)
    ax.tripcolor(t, facecolors=np.arange(len(t.triangles), dtype=float), cmap="tab20")


@case("contour/triplot_masked", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = _triangulation(30, 23)
    t.set_mask(np.arange(len(t.triangles)) % 4 == 0)
    ax.triplot(t, "o-", color="tab:green", lw=1, ms=3)
    ax.set_aspect("equal")


@case("contour/recolor_set_cmap", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    xx, yy, zz = _xyz()
    cf = ax.contourf(xx, yy, zz, levels=8)
    cl = ax.contour(xx, yy, zz, levels=cf.levels)
    ax.clabel(cl)
    fig.colorbar(cf, ax=ax)
    cf.set_cmap("RdBu_r")
    cf.set_clim(-1.5, 1.5)
    cl.set_cmap("Greys")


@case("contour/tripcolor_gouraud_colorbar", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    t = _triangulation()
    tc = ax.tripcolor(t, _tri_z(t), shading="gouraud", cmap="magma")
    fig.colorbar(tc, ax=ax)
    tc.set_clim(-1.5, 1.5)


# --- gallery: contourf extensions, hatching legends, label formats


def _wave(nx=150, ny=120):
    x = np.linspace(-3, 5, nx)
    y = np.linspace(-3, 5, ny)
    xx, yy = np.meshgrid(x, y)
    return xx, yy, np.cos(xx) + np.sin(yy)


@case("contour/contourf_extend_variants_colorbars", figsize=(6.4, 5))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    cmap = mpl.colormaps["winter"].with_extremes(under="yellow", over="cyan")
    xx, yy, zz = _xyz(40, 30)
    levels = [-1.5, -1, -0.5, 0, 0.5, 1]
    for ax, extend in zip(axs.flat, ["neither", "both", "min", "max"]):
        cs = ax.contourf(xx, yy, zz, levels, cmap=cmap, extend=extend)
        fig.colorbar(cs, ax=ax, shrink=0.9)
        ax.set_title(f"extend = {extend}")


@case("contour/contourf_hatching_legend", figsize=(7, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xx, yy, zz = _wave()
    cs = ax1.contourf(xx, yy, zz, hatches=["-", "/", "\\", "//"], cmap="gray", extend="both", alpha=0.5)
    fig.colorbar(cs, ax=ax1)
    ax2.contour(xx, yy, zz, 6, colors="black", linestyles="-")
    cs = ax2.contourf(xx, yy, zz, 6, colors="none", hatches=[".", "/", "\\", None, "\\\\", "*"])
    artists, labels = cs.legend_elements(str_format="{:2.1f}".format)
    ax2.legend(artists, labels, handleheight=2, framealpha=1)


@case("contour/line_contour_legend", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    cs = ax.contour(*_xyz(), levels=5, cmap="viridis", linewidths=2)
    artists, labels = cs.legend_elements()
    ax.legend(artists, labels, loc="upper left", fontsize=8)


@case("contour/contourf_lognorm_levels_colorbar", figsize=(5, 3.5))
def _(fig):
    from matplotlib import colors

    ax = fig.subplots()
    x = np.linspace(-3.0, 3.0, 60)
    xx, yy = np.meshgrid(x, x)
    zz = np.exp(-(xx**2) - yy**2) * 1e3 + np.exp(-((xx - 1.5) ** 2) - (yy - 1) ** 2) * 50 + 1e-2
    cs = ax.contourf(xx, yy, zz, levels=np.logspace(-2, 3, 11), norm=colors.LogNorm(), cmap="PuBu_r")
    fig.colorbar(cs, ax=ax)


def _percent_fmt(x):
    s = f"{x:.1f}"
    if s.endswith("0"):
        s = f"{x:.0f}"
    return f"{s} %"


@case("contour/clabel_label_demo", figsize=(7, 3))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    delta = 0.025
    x = np.arange(-3.0, 3.0, delta)
    y = np.arange(-2.0, 2.0, delta)
    xx, yy = np.meshgrid(x, y)
    # The gallery's field, with the second bump stretched along x so that
    # the zero contour is not a straight line through grid points (where
    # every label position is equally good).
    z1 = np.exp(-(xx**2) - yy**2)
    z2 = np.exp(-((xx - 1) ** 2) / 1.5 - (yy - 1) ** 2)
    zz = (z1 - z2) * 2
    cs = ax1.contour(xx, yy, zz)
    ax1.clabel(cs, cs.levels, fmt=_percent_fmt, fontsize=10)
    cs = ax2.contour(xx, yy, zz)
    names = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh"]
    ax2.clabel(cs, cs.levels[::2], fmt=dict(zip(cs.levels, names)), fontsize=10)
    cs = ax3.contour(xx, yy, 100**zz, locator=ticker.LogLocator())
    fmt = ticker.LogFormatterMathtext()
    fmt.create_dummy_axis()
    ax3.clabel(cs, cs.levels, fmt=fmt)


@case("contour/clabel_manual_rightside_up", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xx, yy, zz = _xyz()
    cs = ax1.contour(xx, yy, zz, levels=[-1.5, -0.5, 0.5, 1.5], colors="k")
    ax1.clabel(cs, manual=[(-1.0, 0.5), (0.5, -0.5), (1.5, 1.2), (-2.0, -1.5)], inline_spacing=12)
    cs = ax2.contour(xx, yy, zz, levels=[-1.0, 0.0, 1.0], cmap="coolwarm")
    ax2.clabel(cs, rightside_up=False, fontsize=8, fmt="%.1f")


@case(
    "contour/negative_linestyle_and_width_rc",
    figsize=(4, 3),
    rc={"contour.negative_linestyle": "dashdot", "contour.linewidth": 2.0},
)
def _(fig):
    ax = fig.subplots()
    ax.contour(*_xyz(), levels=8, colors="k")


@case("contour/levels_outside_and_constant_field", figsize=(6, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xx, yy, zz = _xyz(30, 20)
    ax1.contourf(xx, yy, zz, levels=[-0.5, 0.0, 0.5, 5.0, 6.0], cmap="viridis")
    ax1.contour(xx, yy, zz, levels=[5.0, 6.0], colors="r")
    ax2.contourf(xx, yy, np.full_like(zz, 0.5))
    ax2.contour(xx, yy, np.full_like(zz, 0.5), colors="k")
