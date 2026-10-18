"""mplot3d API: equal aspects (box and datalim), custom ticks and tick
parameters, bounds and margins, errorbar, fill_between (quad and polygon),
add_collection3d, tricontour/tricontourf, extended contours, non-linear
scales (log z, symlog x), shared axes/views, and data hidden outside the
view limits (`axlim_clip`)."""

import numpy as np
from matplotlib.collections import LineCollection, PolyCollection
from matplotlib.ticker import AutoMinorLocator, MultipleLocator
from matplotlib.tri import Triangulation

from spec_support import case, rng


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


# axlim_clip: limits cut through the data, never exactly at a data value.
# matplotlib 3.11.2 raises when it has to hide part of a bar (Patch3D), a
# contour set (Collection3D), an errorbar cap (a Line3D of lists) or a
# polygon lifted by add_collection3d, so those are left out or kept whole.


@case("mplot3d/axlim_plot_text", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = _ellipse_helix()
    ax.plot(x, y, z, axlim_clip=True)
    ax.plot(x[::6], y[::6], z[::6], "o", markersize=4, axlim_clip=True)
    ax.text(0.0, 0.0, 1.0, "inside", zdir="x", axlim_clip=True)
    ax.text(1.8, 0.0, 2.5, "hidden", axlim_clip=True)
    ax.text(-1.8, 0.0, 2.8, "unclipped", color="tab:red")
    ax.set(xlim=(-1.5, 1.5), ylim=(-1.0, 1.0), zlim=(-0.2, 2.2))


def _axlim_scatter(fig, axlim_clip):
    ax = _ax3d(fig)
    g = rng(44)
    x, y, z = g.uniform(120, -1.0, 1.0), g.uniform(120, -1.0, 1.0), g.uniform(120, -1.0, 1.0)
    ax.scatter(x, y, z, s=10 + 60 * g.uniform(120), c=z, cmap="viridis", axlim_clip=axlim_clip)
    ax.scatter(0.5 * x[:30], 0.5 * y[:30], z[:30] + 0.6, marker="^", s=30, color="tab:red",
               depthshade=False, axlim_clip=axlim_clip)
    ax.set(xlim=(-0.6, 0.65), ylim=(-0.8, 0.75), zlim=(-0.5, 1.05))


@case("mplot3d/axlim_scatter", figsize=(4, 4))
def _(fig):
    _axlim_scatter(fig, True)


@case("mplot3d/axlim_scatter_unclipped", figsize=(4, 4))
def _(fig):
    _axlim_scatter(fig, False)


def _axlim_surface(fig, axlim_clip):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(), cmap="viridis", edgecolor="k", linewidth=0.3,
                    axlim_clip=axlim_clip)
    ax.set(xlim=(-2.05, 2.55), ylim=(-2.55, 2.05), zlim=(-0.1, 0.3))


@case("mplot3d/axlim_surface", figsize=(4, 4))
def _(fig):
    _axlim_surface(fig, True)


@case("mplot3d/axlim_surface_unclipped", figsize=(4, 4))
def _(fig):
    _axlim_surface(fig, False)


@case("mplot3d/axlim_wireframe", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_wireframe(*_surface(), rstride=2, cstride=2, color="0.3", linewidth=0.6,
                      axlim_clip=True)
    ax.set(xlim=(-2.05, 2.55), zlim=(-0.1, 0.3))


@case("mplot3d/axlim_trisurf", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = _polar_points()
    ax.plot_trisurf(x, y, z, cmap="Spectral", edgecolor="grey", linewidth=0.2, axlim_clip=True)
    ax.set(xlim=(-0.8, 1.05), zlim=(-1.0, 1.2))


@case("mplot3d/axlim_fill_between", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 30)
    ax.fill_between(np.cos(t), np.sin(t), t / 2, np.cos(t), np.sin(t), 0.2 * np.sin(3 * t),
                    alpha=0.6, axlim_clip=True)
    x = np.linspace(-1.0, 1.0, 25)
    ax.fill_between(x, -1.1, np.cos(3 * x) + 1.5, x, -1.1, 0.0, where=np.abs(x - 0.45) > 0.1,
                    mode="polygon", facecolors="tab:orange", edgecolor="k", linewidth=0.5,
                    axlim_clip=True)
    ax.set(xlim=(-0.8, 0.9), ylim=(-1.2, 1.2), zlim=(-0.1, 2.2))


@case("mplot3d/axlim_bar3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y = np.meshgrid(np.arange(4), np.arange(3))
    x, y = x.ravel(), y.ravel()
    heights = 1 + (x * 3 + y * 5) % 7
    ax.bar3d(x, y, np.zeros_like(x), 0.6, 0.6, heights, color="tab:cyan", edgecolor="k",
             linewidth=0.5, axlim_clip=True)
    ax.set(xlim=(0.3, 3.8), ylim=(-0.2, 2.8), zlim=(-0.5, 4.5))


@case("mplot3d/axlim_quiver", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    g = np.linspace(-1.0, 1.0, 4)
    xx, yy, zz = np.meshgrid(g, g, g)
    ax.quiver(xx, yy, zz, -yy, xx, 0.3 * zz, length=0.4, normalize=True, axlim_clip=True)
    ax.set(xlim=(-0.8, 1.2), ylim=(-1.2, 0.9), zlim=(-1.2, 1.2))


@case("mplot3d/axlim_errorbar", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 9)
    x, y, z = np.cos(t), np.sin(t), t / 2
    limit = np.arange(9)
    ax.errorbar(x, y, z, zerr=0.4, xerr=0.15, fmt="o-", capsize=3, errorevery=(1, 2),
                zuplims=limit == 1, zlolims=limit == 7, ecolor="tab:red", axlim_clip=True)
    ax.set(xlim=(-0.95, 1.2), ylim=(-1.2, 1.2), zlim=(0.2, 2.9))


@case("mplot3d/axlim_stem", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 16)
    ax.stem(np.cos(t), np.sin(t), t, axlim_clip=True)
    ax.set(xlim=(-0.8, 1.2), ylim=(-1.2, 1.2), zlim=(-0.5, 4.5))


@case("mplot3d/axlim_voxels", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = np.indices((4, 4, 4))
    filled = (x - 1.5) ** 2 + (y - 1.5) ** 2 + (z - 1.5) ** 2 < 4.0
    colors = np.where(z < 2, "tab:blue", "tab:orange")
    ax.voxels(filled, facecolors=colors, edgecolor="k", linewidth=0.3, axlim_clip=True)
    ax.set(xlim=(0.5, 4.2), ylim=(-0.2, 3.4), zlim=(0.3, 3.6))


@case("mplot3d/axlim_add_collection3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x = np.linspace(0.0, 3.0, 10)
    polys = [[(0.0, 0.0), *zip(x, np.abs(np.sin(x + k)) + 0.2), (3.0, 0.0)] for k in range(3)]
    ax.add_collection3d(PolyCollection(polys, facecolors=["r", "g", "b"], alpha=0.6),
                        zs=[0.0, 1.0, 5.0], zdir="y", axlim_clip=True)
    lines = [list(zip(x, np.cos(x + k))) for k in range(2)]
    ax.add_collection3d(LineCollection(lines, colors="k", linewidths=1.5), zs=[0.5, 1.5],
                        zdir="y", axlim_clip=True)
    ax.set(xlim=(-0.2, 3.2), ylim=(-0.5, 2.0), zlim=(-0.5, 1.4))
