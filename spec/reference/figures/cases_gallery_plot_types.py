"""The gallery's "Plot types" overview, one case per example: the small
2 x 2 inch thumbnails of the basic plots (lines, scatter, bars, stems,
fills, stackplot, stairs), the plots of arrays and fields (imshow,
pcolormesh, contours, barbs, quiver, streamplot), the statistics plots
(histograms, box and violin plots, error bars, events, hexbin, hist2d,
pie, ECDF), the plots of unstructured triangular grids and the 3-D plots.

Every example styles itself with matplotlib's private gallery sheets,
``_mpl-gallery`` (with a grid) or ``_mpl-gallery-nogrid``, applied here
through the case's ``rc``. The examples draw random data from
``np.random``; here it comes from ``spec_support.rng`` seeded with the
example's seed, with the gamma draws of ``eventplot`` built from its
uniforms.
"""

import matplotlib as mpl
import matplotlib.style
import numpy as np
from mpl_toolkits.mplot3d import axes3d

from spec_support import case, rng


GALLERY = "gallery_plot_types"


def plot_type(name, grid=True):
    """Registers a case of this group: a 2 x 2 inch figure under the
    gallery sheet with (``grid``) or without a grid."""
    sheet = "_mpl-gallery" if grid else "_mpl-gallery-nogrid"
    return case(f"{GALLERY}/{name}", figsize=(2, 2), rc=dict(matplotlib.style.library[sheet]))


def unit_square(ax):
    """The limits and ticks most thumbnails share: 0 to 8 on both axes,
    ticked at every integer in between."""
    ax.set(xlim=(0, 8), xticks=np.arange(1, 8), ylim=(0, 8), yticks=np.arange(1, 8))


def bump(x, y):
    """The surface the array and grid examples sample."""
    return (1 - x / 2 + x**5 + y**3) * np.exp(-(x**2) - y**2)


def bump_grid(n):
    """``bump`` over an ``n`` x ``n`` grid of [-3, 3]^2."""
    x, y = np.meshgrid(np.linspace(-3, 3, n), np.linspace(-3, 3, n))
    return x, y, bump(x, y)


def correlated(g):
    """5000 correlated points plus noise (``hexbin`` and ``hist2d``)."""
    x = g.normal(5000)
    return x, 1.2 * x + g.normal(5000) / 3


def scattered():
    """256 random points of [-3, 3]^2 and ``bump`` there (the unstructured
    grid examples)."""
    g = rng(1)
    x = g.uniform(256, -3, 3)
    y = g.uniform(256, -3, 3)
    return x, y, bump(x, y)


def extrema_levels(z):
    """Seven contour levels from the minimum to the maximum of ``z``."""
    return np.linspace(z.min(), z.max(), 7)


def columns(g, rows, loc, scale):
    """``np.random.normal(loc, scale, (rows, len(loc)))``."""
    return g.normal(rows * len(loc)).reshape(rows, len(loc)) * scale + loc


def ax3d(fig):
    """The single 3-D axes of the 3-D examples."""
    return fig.subplots(subplot_kw={"projection": "3d"})


def no_tick_labels(ax):
    """Blanks the tick labels of a 3-D axes, as every 3-D example does."""
    ax.set(xticklabels=[], yticklabels=[], zticklabels=[])


# --- basic -----------------------------------------------------------------------


@plot_type("bar")
def _(fig):
    x = 0.5 + np.arange(8)
    y = [4.8, 5.5, 3.5, 4.6, 6.5, 6.6, 2.6, 3.0]
    ax = fig.subplots()
    ax.bar(x, y, width=1, edgecolor="white", linewidth=0.7)
    unit_square(ax)


@plot_type("fill_between")
def _(fig):
    g = rng(1)
    x = np.linspace(0, 8, 16)
    y1 = 3 + 4 * x / 8 + g.uniform(len(x), 0.0, 0.5)
    y2 = 1 + 2 * x / 8 + g.uniform(len(x), 0.0, 0.5)
    ax = fig.subplots()
    ax.fill_between(x, y1, y2, alpha=0.5, linewidth=0)
    ax.plot(x, (y1 + y2) / 2, linewidth=2)
    unit_square(ax)


@plot_type("plot")
def _(fig):
    x = np.linspace(0, 10, 100)
    y = 4 + 1 * np.sin(2 * x)
    x2 = np.linspace(0, 10, 25)
    y2 = 4 + 1 * np.sin(2 * x2)
    ax = fig.subplots()
    ax.plot(x2, y2 + 2.5, "x", markeredgewidth=2)
    ax.plot(x, y, linewidth=2.0)
    ax.plot(x2, y2 - 2.5, "o-", linewidth=2)
    unit_square(ax)


@plot_type("scatter_plot")
def _(fig):
    g = rng(3)
    x = 4 + g.normal(24, 0, 2)
    y = 4 + g.normal(len(x), 0, 2)
    sizes = g.uniform(len(x), 15, 80)
    colors = g.uniform(len(x), 15, 80)
    ax = fig.subplots()
    ax.scatter(x, y, s=sizes, c=colors, vmin=0, vmax=100)
    unit_square(ax)


@plot_type("stackplot")
def _(fig):
    x = np.arange(0, 10, 2)
    ay = [1, 1.25, 2, 2.75, 3]
    by = [1, 1, 1, 1, 1]
    cy = [2, 1, 2, 1, 2]
    ax = fig.subplots()
    ax.stackplot(x, np.vstack([ay, by, cy]))
    unit_square(ax)


@plot_type("stairs")
def _(fig):
    y = [4.8, 5.5, 3.5, 4.6, 6.5, 6.6, 2.6, 3.0]
    ax = fig.subplots()
    ax.stairs(y, linewidth=2.5)
    unit_square(ax)


@plot_type("stem")
def _(fig):
    x = 0.5 + np.arange(8)
    y = [4.8, 5.5, 3.5, 4.6, 6.5, 6.6, 2.6, 3.0]
    ax = fig.subplots()
    ax.stem(x, y)
    unit_square(ax)


# --- arrays ----------------------------------------------------------------------


@plot_type("barbs", grid=False)
def _(fig):
    X, Y = np.meshgrid([1, 2, 3, 4], [1, 2, 3, 4])
    angle = np.pi / 180 * np.array([[15., 30, 35, 45],
                                    [25., 40, 55, 60],
                                    [35., 50, 65, 75],
                                    [45., 60, 75, 90]])
    amplitude = np.array([[5, 10, 25, 50],
                          [10, 15, 30, 60],
                          [15, 26, 50, 70],
                          [20, 45, 80, 100]])
    U = amplitude * np.sin(angle)
    V = amplitude * np.cos(angle)
    ax = fig.subplots()
    ax.barbs(X, Y, U, V, barbcolor="C0", flagcolor="C0", length=7, linewidth=1.5)
    ax.set(xlim=(0, 4.5), ylim=(0, 4.5))


@plot_type("contour", grid=False)
def _(fig):
    X, Y, Z = bump_grid(256)
    ax = fig.subplots()
    ax.contour(X, Y, Z, levels=extrema_levels(Z))


@plot_type("contourf", grid=False)
def _(fig):
    X, Y, Z = bump_grid(256)
    ax = fig.subplots()
    ax.contourf(X, Y, Z, levels=extrema_levels(Z))


@plot_type("imshow", grid=False)
def _(fig):
    _, _, Z = bump_grid(16)
    ax = fig.subplots()
    ax.imshow(Z, origin="lower")


@plot_type("pcolormesh", grid=False)
def _(fig):
    x = [-3, -2, -1.6, -1.2, -.8, -.5, -.2, .1, .3, .5, .8, 1.1, 1.5, 1.9, 2.3, 3]
    X, Y = np.meshgrid(x, np.linspace(-3, 3, 128))
    ax = fig.subplots()
    ax.pcolormesh(X, Y, bump(X, Y), vmin=-0.5, vmax=1.0)


@plot_type("quiver", grid=False)
def _(fig):
    x = np.linspace(-4, 4, 6)
    y = np.linspace(-4, 4, 6)
    X, Y = np.meshgrid(x, y)
    U = X + Y
    V = Y - X
    ax = fig.subplots()
    ax.quiver(X, Y, U, V, color="C0", angles="xy", scale_units="xy", scale=5, width=.015)
    ax.set(xlim=(-5, 5), ylim=(-5, 5))


@plot_type("streamplot", grid=False)
def _(fig):
    X, Y, Z = bump_grid(256)
    V = np.diff(Z[1:, :], axis=1)
    U = -np.diff(Z[:, 1:], axis=0)
    ax = fig.subplots()
    ax.streamplot(X[1:, 1:], Y[1:, 1:], U, V)


# --- statistics ------------------------------------------------------------------


@plot_type("boxplot_plot")
def _(fig):
    D = columns(rng(10), 100, (3, 5, 4), (1.25, 1.00, 1.25))
    ax = fig.subplots()
    ax.boxplot(D, positions=[2, 4, 6], widths=1.5, patch_artist=True,
               showmeans=False, showfliers=False,
               medianprops={"color": "white", "linewidth": 0.5},
               boxprops={"facecolor": "C0", "edgecolor": "white", "linewidth": 0.5},
               whiskerprops={"color": "C0", "linewidth": 1.5},
               capprops={"color": "C0", "linewidth": 1.5})
    unit_square(ax)


@plot_type("ecdf")
def _(fig):
    x = 4 + rng(1).normal(200, 0, 1.5)
    ax = fig.subplots()
    ax.ecdf(x)


@plot_type("errorbar_plot")
def _(fig):
    x = [2, 4, 6]
    y = [3.6, 5, 4.2]
    yerr = [0.9, 1.2, 0.5]
    ax = fig.subplots()
    ax.errorbar(x, y, yerr, fmt="o", linewidth=2, capsize=6)
    unit_square(ax)


@plot_type("eventplot")
def _(fig):
    x = [2, 4, 6]
    D = rng(1).gamma(3 * 50, 4).reshape(3, 50)
    ax = fig.subplots()
    ax.eventplot(D, orientation="vertical", lineoffsets=x, linewidth=0.75)
    unit_square(ax)


@plot_type("hexbin", grid=False)
def _(fig):
    x, y = correlated(rng(1))
    ax = fig.subplots()
    ax.hexbin(x, y, gridsize=20)
    ax.set(xlim=(-2, 2), ylim=(-3, 3))


@plot_type("hist2d", grid=False)
def _(fig):
    x, y = correlated(rng(1))
    ax = fig.subplots()
    ax.hist2d(x, y, bins=(np.arange(-3, 3, 0.1), np.arange(-3, 3, 0.1)))
    ax.set(xlim=(-2, 2), ylim=(-3, 3))


@plot_type("hist_plot")
def _(fig):
    x = 4 + rng(1).normal(200, 0, 1.5)
    ax = fig.subplots()
    ax.hist(x, bins=8, linewidth=0.5, edgecolor="white")
    ax.set(xlim=(0, 8), xticks=np.arange(1, 8), ylim=(0, 56), yticks=np.linspace(0, 56, 9))


@plot_type("pie", grid=False)
def _(fig):
    x = [1, 2, 3, 4]
    colors = mpl.colormaps["Blues"](np.linspace(0.2, 0.7, len(x)))
    ax = fig.subplots()
    ax.pie(x, colors=colors, radius=3, center=(4, 4),
           wedgeprops={"linewidth": 1, "edgecolor": "white"}, frame=True)
    unit_square(ax)


@plot_type("violin")
def _(fig):
    D = columns(rng(10), 200, (3, 5, 4), (0.75, 1.00, 0.75))
    ax = fig.subplots()
    vp = ax.violinplot(D, [2, 4, 6], widths=2,
                       showmeans=False, showmedians=False, showextrema=False)
    for body in vp["bodies"]:
        body.set_alpha(0.9)
    unit_square(ax)


# --- unstructured grids ----------------------------------------------------------


@plot_type("tricontour", grid=False)
def _(fig):
    x, y, z = scattered()
    ax = fig.subplots()
    ax.plot(x, y, "o", markersize=2, color="lightgrey")
    ax.tricontour(x, y, z, levels=extrema_levels(z))
    ax.set(xlim=(-3, 3), ylim=(-3, 3))


@plot_type("tricontourf", grid=False)
def _(fig):
    x, y, z = scattered()
    ax = fig.subplots()
    ax.plot(x, y, "o", markersize=2, color="grey")
    ax.tricontourf(x, y, z, levels=extrema_levels(z))
    ax.set(xlim=(-3, 3), ylim=(-3, 3))


@plot_type("tripcolor", grid=False)
def _(fig):
    x, y, z = scattered()
    ax = fig.subplots()
    ax.plot(x, y, "o", markersize=2, color="grey")
    ax.tripcolor(x, y, z)
    ax.set(xlim=(-3, 3), ylim=(-3, 3))


@plot_type("triplot", grid=False)
def _(fig):
    x, y, _ = scattered()
    ax = fig.subplots()
    ax.triplot(x, y)
    ax.set(xlim=(-3, 3), ylim=(-3, 3))


# --- 3-D -------------------------------------------------------------------------


@plot_type("bar3d_simple")
def _(fig):
    x = [1, 1, 2, 2]
    y = [1, 2, 1, 2]
    z = [0, 0, 0, 0]
    dx = np.ones_like(x) * 0.5
    dy = np.ones_like(x) * 0.5
    dz = [2, 3, 1, 4]
    ax = ax3d(fig)
    ax.bar3d(x, y, z, dx, dy, dz)
    no_tick_labels(ax)


@plot_type("fill_between3d_simple")
def _(fig):
    n = 50
    theta = np.linspace(0, 2 * np.pi, n)
    x1 = np.cos(theta)
    y1 = np.sin(theta)
    z1 = np.linspace(0, 1, n)
    x2 = np.cos(theta + np.pi)
    y2 = np.sin(theta + np.pi)
    z2 = z1
    ax = ax3d(fig)
    ax.fill_between(x1, y1, z1, x2, y2, z2, alpha=0.5)
    ax.plot(x1, y1, z1, linewidth=2, color="C0")
    ax.plot(x2, y2, z2, linewidth=2, color="C0")
    no_tick_labels(ax)


@plot_type("plot3d_simple")
def _(fig):
    n = 100
    xs = np.linspace(0, 1, n)
    ys = np.sin(xs * 6 * np.pi)
    zs = np.cos(xs * 6 * np.pi)
    ax = ax3d(fig)
    ax.plot(xs, ys, zs)
    no_tick_labels(ax)


@plot_type("quiver3d_simple")
def _(fig):
    n = 4
    x = np.linspace(-1, 1, n)
    y = np.linspace(-1, 1, n)
    z = np.linspace(-1, 1, n)
    X, Y, Z = np.meshgrid(x, y, z)
    U = (X + Y) / 5
    V = (Y - X) / 5
    W = Z * 0
    ax = ax3d(fig)
    ax.quiver(X, Y, Z, U, V, W)
    no_tick_labels(ax)


@plot_type("scatter3d_simple")
def _(fig):
    g = rng(19680801)
    n = 100
    xs = g.uniform(n, 23, 32)
    ys = g.uniform(n, 0, 100)
    zs = g.uniform(n, -50, -25)
    ax = ax3d(fig)
    ax.scatter(xs, ys, zs)
    no_tick_labels(ax)


@plot_type("stem3d")
def _(fig):
    n = 20
    x = np.sin(np.linspace(0, 2 * np.pi, n))
    y = np.cos(np.linspace(0, 2 * np.pi, n))
    z = np.linspace(0, 1, n)
    ax = ax3d(fig)
    ax.stem(x, y, z)
    no_tick_labels(ax)


@plot_type("surface3d_simple")
def _(fig):
    X, Y = np.meshgrid(np.arange(-5, 5, 0.25), np.arange(-5, 5, 0.25))
    Z = np.sin(np.sqrt(X**2 + Y**2))
    ax = ax3d(fig)
    ax.plot_surface(X, Y, Z, vmin=Z.min() * 2, cmap="Blues")
    no_tick_labels(ax)


@plot_type("trisurf3d_simple")
def _(fig):
    radii = np.linspace(0.125, 1.0, 8)
    angles = np.linspace(0, 2 * np.pi, 36, endpoint=False)[..., np.newaxis]
    x = np.append(0, (radii * np.cos(angles)).flatten())
    y = np.append(0, (radii * np.sin(angles)).flatten())
    z = np.sin(-x * y)
    ax = ax3d(fig)
    ax.plot_trisurf(x, y, z, vmin=z.min() * 2, cmap="Blues")
    no_tick_labels(ax)


@plot_type("voxels_simple")
def _(fig):
    x, y, z = np.indices((8, 8, 8))
    cube1 = (x < 3) & (y < 3) & (z < 3)
    cube2 = (x >= 5) & (y >= 5) & (z >= 5)
    ax = ax3d(fig)
    ax.voxels(cube1 | cube2, edgecolor="k")
    no_tick_labels(ax)


@plot_type("wire3d_simple")
def _(fig):
    X, Y, Z = axes3d.get_test_data(0.05)
    ax = ax3d(fig)
    ax.plot_wireframe(X, Y, Z, rstride=10, cstride=10)
    no_tick_labels(ax)
