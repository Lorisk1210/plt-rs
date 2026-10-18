"""mplot3d API: equal aspects (box and datalim), custom ticks and tick
parameters, bounds and margins, errorbar, fill_between (quad and polygon),
add_collection3d, tricontour/tricontourf, extended contours, non-linear
scales (log z, symlog x) and shared axes/views."""

import numpy as np
from matplotlib.collections import LineCollection, PolyCollection
from matplotlib.ticker import AutoMinorLocator, MultipleLocator
from matplotlib.tri import Triangulation

from spec_support import case


def _ax3d(fig, *args, **kwargs):
    return fig.add_subplot(*args, projection="3d", **kwargs)


def _ellipse_helix(n=120):
    t = np.linspace(0.0, 4 * np.pi, n)
    return 2.0 * np.cos(t), 0.5 * np.sin(t), 3.0 * t / (4 * np.pi)


def _surface(n=20):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    r = np.hypot(xx, yy)
    return xx, yy, np.sin(r) / (r + 1.0)


def _polar_points():
    r = np.repeat(np.linspace(0.25, 1.0, 5), 12)
    a = np.tile(np.linspace(0.0, 2 * np.pi, 12, endpoint=False), 5) + r
    x, y = r * np.cos(a), r * np.sin(a)
    return x, y, np.cos(3 * x) * np.sin(2 * y) + x


@case("mplot3d/api_aspect_equal_box", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_ellipse_helix())
    ax.set_aspect("equal")


@case("mplot3d/api_aspect_equal_datalim", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_ellipse_helix())
    ax.set_aspect("equal", adjustable="datalim")


@case("mplot3d/api_aspect_equalxz_box", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_ellipse_helix(), color="tab:green")
    ax.set_aspect("equalxz")


@case("mplot3d/api_custom_ticks", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_wireframe(*_surface(), rstride=2, cstride=2, color="0.4", linewidth=0.5)
    ax.set_xticks([-3, 0, 2])
    ax.set_xticklabels(["low", "zero", "two"])
    ax.yaxis.set_major_locator(MultipleLocator(1.5))
    ax.yaxis.set_minor_locator(AutoMinorLocator(3))
    ax.zaxis.set_major_locator(MultipleLocator(0.25))
    ax.tick_params(axis="x", colors="tab:red", labelsize=7)
    ax.zaxis.set_tick_params(pad=6, labelcolor="tab:blue", labelrotation=20)


@case("mplot3d/api_tick_params_hidden", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(), cmap="viridis")
    ax.tick_params(axis="both", labelleft=False, labelsize=6, which="both")
    ax.yaxis.minorticks_on()


@case("mplot3d/api_bounds_margins", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_ellipse_helix(), color="tab:purple")
    ax.margins(0.2, tight=True)
    ax.set_zbound(-1, None)
    ax.set_ybound(1, -1)
    ax.set_zinverted(True)
    ax.set_xmargin(0.0)


@case("mplot3d/api_errorbar", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 9)
    x, y, z = np.cos(t), np.sin(t), t / 2
    ax.errorbar(x, y, z, zerr=0.4, xerr=0.15, yerr=[0.1, 0.2] * 4 + [0.1], fmt="o",
                capsize=3, zlolims=[True, False, False] * 3, errorevery=(1, 2),
                ecolor="tab:red")


@case("mplot3d/api_fill_between_quad", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 30)
    ax.fill_between(np.cos(t), np.sin(t), t / 2, np.cos(t), np.sin(t), 0.2 * np.sin(3 * t),
                    alpha=0.6)


@case("mplot3d/api_fill_between_polygon", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x = np.linspace(0.0, 4.0, 25)
    where = (x < 1.5) | (x > 2.2)
    ax.fill_between(x, 1.0, np.sin(2 * x) + 1.5, x, 1.0, 0.0, where=where,
                    facecolors="tab:orange", edgecolor="k", linewidth=0.5)
    ax.fill_between(x, 0.0, np.cos(x), x, 0.0, 0.0, mode="polygon", facecolors="tab:blue")


@case("mplot3d/api_add_collection3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x = np.linspace(0.0, 3.0, 10)
    polys = [[(0.0, 0.0), *zip(x, np.abs(np.sin(x + k)) + 0.2), (3.0, 0.0)] for k in range(3)]
    ax.add_collection3d(PolyCollection(polys, facecolors=["r", "g", "b"], alpha=0.6),
                        zs=[0.0, 1.0, 2.0], zdir="y")
    lines = [list(zip(x, np.cos(x + k))) for k in range(2)]
    ax.add_collection3d(LineCollection(lines, colors="k", linewidths=1.5), zs=[3.5, 4.0],
                        zdir="x")


@case("mplot3d/api_tricontour", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = _polar_points()
    ax.tricontour(x, y, z, levels=8, cmap="viridis")
    ax.tricontour(Triangulation(x, y), z, levels=6, zdir="x", offset=-1.2, colors="k")


@case("mplot3d/api_tricontourf", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = _polar_points()
    ax.tricontourf(x, y, z, levels=7, cmap="coolwarm")
    ax.tricontourf(x, y, z, levels=5, zdir="z", offset=-2.0, cmap="Greys", alpha=0.5)


@case("mplot3d/api_contour_extend3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.contour(*_surface(30), levels=6, extend3d=True, stride=4, cmap="plasma")


@case("mplot3d/api_log_z", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.set_zscale("log")
    xx, yy, _ = _surface(15)
    ax.plot_surface(xx, yy, 10.0 ** (1.5 + np.cos(xx) + 0.5 * yy), cmap="magma")


@case("mplot3d/api_symlog_x", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(-1.0, 1.0, 60)
    ax.plot(100 * t ** 3, np.cos(3 * t), t, "o-", markersize=3)
    ax.set_xscale("symlog")


@case("mplot3d/api_shared_views", figsize=(8, 4))
def _(fig):
    ax1 = _ax3d(fig, 1, 2, 1)
    ax1.plot(*_ellipse_helix())
    ax2 = _ax3d(fig, 1, 2, 2, shareview=ax1, sharez=ax1)
    ax2.plot_wireframe(*_surface(), rstride=3, cstride=3)
    ax1.view_init(elev=15, azim=40, roll=10, share=True)
    ax2.set_zlim(-1, 2)
