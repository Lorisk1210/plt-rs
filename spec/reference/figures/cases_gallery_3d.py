"""matplotlib's gallery section "3D plotting" (``mplot3d``): lines, scatter
plots, bars and histograms, surfaces (coloured, shaded, checkered, radial,
with offsets), wireframes, triangulated surfaces, line and filled contours
projected onto the panes, ``fill_between``, error bars, stems, quivers,
voxels, texts and text paths, polygons, images on planes, scales,
projections, view planes and limits clipping the data.

Each case ports one figure of an example (``<example>_<n>`` when the
example draws several). Random data comes from ``spec_support.rng`` seeded
with the example's seed (``imshow3d`` has none and takes the gallery's).
``custom_shaded_3d_surface`` replaces the ``jacksboro_fault_dem.npz``
sample data by a synthetic elevation model of the same shape and range
(``_terrain``), and ``pathpatch3d`` typesets its formula with mathtext
instead of LaTeX (without ``\\displaystyle``, which mathtext lacks). The
animations ``rotate_axes3d_sgskip`` and ``wire3d_animation`` are left
out.
"""

import math

import matplotlib as mpl
import numpy as np
from matplotlib import colors as mcolors
from matplotlib import tri
from matplotlib.colors import LightSource, Normalize
from matplotlib.patches import Circle, PathPatch
from matplotlib.text import TextPath
from matplotlib.ticker import LinearLocator
from matplotlib.transforms import Affine2D
from mpl_toolkits.mplot3d import art3d, axes3d
from mpl_toolkits.mplot3d.art3d import Poly3DCollection

from spec_support import case, rng

GALLERY = "gallery_3d"
SEED = 19680801


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{GALLERY}/{name}", **options)


def ax3d(fig, *args):
    """``fig.add_subplot(*args, projection='3d')``."""
    return fig.add_subplot(*args, projection="3d")


def sinc_surface():
    """``sin(sqrt(X**2 + Y**2))`` on ``np.arange(-5, 5, 0.25)`` squared, the
    surface of several examples."""
    X, Y = np.meshgrid(np.arange(-5, 5, 0.25), np.arange(-5, 5, 0.25))
    return X, Y, np.sin(np.sqrt(X**2 + Y**2))


def label_xyz(ax, x="X", y="Y", z="Z"):
    """``ax.set_xlabel(x)``, ``ax.set_ylabel(y)``, ``ax.set_zlabel(z)``."""
    ax.set_xlabel(x)
    ax.set_ylabel(y)
    ax.set_zlabel(z)


def ring_points(n_angles):
    """The rings of the triangulation examples: 8 radii from 0.25 to 0.95,
    every other ring turned by half a step, and ``cos(r) cos(3 angle)``."""
    radii = np.linspace(0.25, 0.95, 8)
    angles = np.linspace(0, 2 * np.pi, n_angles, endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], 8, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x = (radii * np.cos(angles)).flatten()
    y = (radii * np.sin(angles)).flatten()
    z = (np.cos(radii) * np.cos(3 * angles)).flatten()
    return x, y, z


def ring_triangulation(x, y, inside):
    """The Delaunay triangulation of ``ring_points`` without the triangles
    whose centroid ``inside(xmid, ymid)`` finds inside the inner ring."""
    triang = tri.Triangulation(x, y)
    triang.set_mask(inside(x[triang.triangles].mean(axis=1), y[triang.triangles].mean(axis=1)))
    return triang


def midpoints(x):
    """The voxel examples' ``midpoints``: the mean of each cell's corners."""
    sl = ()
    for _ in range(x.ndim):
        x = (x[sl + np.index_exp[:-1]] + x[sl + np.index_exp[1:]]) / 2.0
        sl += np.index_exp[:]
    return x


# --- lines, scatter plots and text ------------------------------------------


@gallery("2dcollections3d")
def _(fig):
    ax = ax3d(fig)
    x = np.linspace(0, 1, 100)
    y = np.sin(x * 2 * np.pi) / 2 + 0.5
    ax.plot(x, y, zs=0, zdir="z", label="curve in (x, y)")
    colors = ("r", "g", "b", "k")
    g = rng(SEED)
    x = g.uniform(20 * len(colors))
    y = g.uniform(20 * len(colors))
    c_list = [c for c in colors for _ in range(20)]
    ax.scatter(x, y, zs=0, zdir="y", c=c_list, label="points in (x, z)")
    ax.legend()
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.set_zlim(0, 1)
    label_xyz(ax)
    ax.view_init(elev=20.0, azim=-35, roll=0)


@gallery("lines3d")
def _(fig):
    ax = ax3d(fig)
    theta = np.linspace(-4 * np.pi, 4 * np.pi, 100)
    z = np.linspace(-2, 2, 100)
    r = z**2 + 1
    ax.plot(r * np.sin(theta), r * np.cos(theta), z, label="parametric curve")
    ax.legend()


def lorenz(xyz, *, s=10, r=28, b=2.667):
    """The time derivatives of the Lorenz system at ``xyz``."""
    x, y, z = xyz
    return np.array([s * (y - x), r * x - y - x * z, x * y - b * z])


@gallery("lorenz_attractor")
def _(fig):
    dt = 0.01
    num_steps = 10000
    xyzs = np.empty((num_steps + 1, 3))
    xyzs[0] = (0.0, 1.0, 1.05)
    for i in range(num_steps):
        xyzs[i + 1] = xyzs[i] + lorenz(xyzs[i]) * dt
    ax = ax3d(fig)
    ax.plot(*xyzs.T, lw=0.5)
    label_xyz(ax, "X Axis", "Y Axis", "Z Axis")
    ax.set_title("Lorenz Attractor")


@gallery("scales3d")
def _(fig):
    x = np.linspace(0, 1, 400)
    y = 10 ** (2 * x)
    phase = 2 * np.pi * (10 ** (2 * x) - 1) / (2 * np.log(10))
    z = np.sin(phase) * x**2 * 10
    ax = ax3d(fig)
    ax.plot(x, y, z)
    label_xyz(ax, "Time (linear)", "Frequency, Hz (log)", "Amplitude (symlog)")
    ax.set_yscale("log")
    ax.set_zscale("symlog")


@gallery("scatter3d")
def _(fig):
    g = rng(SEED)
    ax = ax3d(fig)
    n = 100
    for m, zlow, zhigh in [("o", -50, -25), ("^", -30, -5)]:
        xs = g.uniform(n, 23, 32)
        ys = g.uniform(n, 0, 100)
        zs = g.uniform(n, zlow, zhigh)
        ax.scatter(xs, ys, zs, marker=m)
    label_xyz(ax, "X Label", "Y Label", "Z Label")


@gallery("text3d")
def _(fig):
    ax = ax3d(fig)
    zdirs = (None, "x", "y", "z", (1, 1, 0), (1, 1, 1))
    xs = (1, 4, 4, 9, 4, 1)
    ys = (2, 5, 8, 10, 1, 2)
    zs = (10, 3, 8, 9, 1, 8)
    for zdir, x, y, z in zip(zdirs, xs, ys, zs):
        ax.text(x, y, z, "(%d, %d, %d), dir=%s" % (x, y, z, zdir), zdir)
    ax.text(9, 0, 0, "red", color="red")
    ax.text2D(0.05, 0.95, "2D Text", transform=ax.transAxes)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.set_zlim(0, 10)
    label_xyz(ax, "X axis", "Y axis", "Z axis")


def text3d(ax, xyz, s, zdir="z", size=None, angle=0, **kwargs):
    """``pathpatch3d``'s ``text3d``: the outline of ``s`` laid in the plane
    normal to ``zdir`` through ``xyz``, turned by ``angle``."""
    x, y, z = xyz
    if zdir == "y":
        xy1, z1 = (x, z), y
    elif zdir == "x":
        xy1, z1 = (y, z), x
    else:
        xy1, z1 = (x, y), z
    text_path = TextPath((0, 0), s, size=size)
    trans = Affine2D().rotate(angle).translate(xy1[0], xy1[1])
    p1 = PathPatch(trans.transform_path(text_path), **kwargs)
    ax.add_patch(p1)
    art3d.pathpatch_2d_to_3d(p1, z=z1, zdir=zdir)


@gallery("pathpatch3d")
def _(fig):
    ax = ax3d(fig)
    p = Circle((5, 5), 3)
    ax.add_patch(p)
    art3d.pathpatch_2d_to_3d(p, z=0, zdir="x")
    text3d(ax, (4, -2, 0), "X-axis", zdir="z", size=0.5, ec="none", fc="k")
    text3d(ax, (12, 4, 0), "Y-axis", zdir="z", size=0.5, angle=np.pi / 2, ec="none", fc="k")
    text3d(ax, (12, 10, 4), "Z-axis", zdir="y", size=0.5, angle=np.pi / 2, ec="none", fc="k")
    text3d(ax, (1, 5, 0),
           r"$G_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4} T_{\mu\nu}  $",
           zdir="z", size=1, ec="none", fc="k")
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.set_zlim(0, 10)


# --- bars and histograms -----------------------------------------------------


@gallery("3d_bars", figsize=(8, 3))
def _(fig):
    ax1 = ax3d(fig, 121)
    ax2 = ax3d(fig, 122)
    _xx, _yy = np.meshgrid(np.arange(4), np.arange(5))
    x, y = _xx.ravel(), _yy.ravel()
    top = x + y
    bottom = np.zeros_like(top)
    ax1.bar3d(x, y, bottom, 1, 1, top, shade=True)
    ax1.set_title("Shaded")
    ax2.bar3d(x, y, bottom, 1, 1, top, shade=False)
    ax2.set_title("Not Shaded")


@gallery("bars3d")
def _(fig):
    g = rng(SEED)
    ax = ax3d(fig)
    yticks = [3, 2, 1, 0]
    for c, k in zip(["r", "g", "b", "y"], yticks):
        xs = np.arange(20)
        ys = g.uniform(20)
        cs = [c] * len(xs)
        cs[0] = "c"
        ax.bar(xs, ys, zs=k, zdir="y", color=cs, alpha=0.8)
    label_xyz(ax)
    ax.set_yticks(yticks)


@gallery("hist3d")
def _(fig):
    ax = ax3d(fig)
    x, y = rng(SEED).uniform(200, 0, 4).reshape(2, 100)
    hist, xedges, yedges = np.histogram2d(x, y, bins=4, range=[[0, 4], [0, 4]])
    xpos, ypos = np.meshgrid(xedges[:-1] + 0.25, yedges[:-1] + 0.25, indexing="ij")
    ax.bar3d(xpos.ravel(), ypos.ravel(), 0, 0.5, 0.5, hist.ravel(), zsort="average")


# --- contours ------------------------------------------------------------------


@gallery("contour3d")
def _(fig):
    ax3d(fig).contour(*axes3d.get_test_data(0.05), cmap="coolwarm")


@gallery("contour3d_2")
def _(fig):
    ax3d(fig).contour(*axes3d.get_test_data(0.05), extend3d=True, cmap="coolwarm")


def projected_contours(fig, method):
    """``contour3d_3`` and ``contourf3d_2``: a translucent surface and its
    contours (``method`` names ``contour`` or ``contourf``) projected onto
    three panes."""
    ax = ax3d(fig)
    X, Y, Z = axes3d.get_test_data(0.05)
    ax.plot_surface(X, Y, Z, edgecolor="royalblue", lw=0.5, rstride=8, cstride=8, alpha=0.3)
    draw = getattr(ax, method)
    draw(X, Y, Z, zdir="z", offset=-100, cmap="coolwarm")
    draw(X, Y, Z, zdir="x", offset=-40, cmap="coolwarm")
    draw(X, Y, Z, zdir="y", offset=40, cmap="coolwarm")
    ax.set(xlim=(-40, 40), ylim=(-40, 40), zlim=(-100, 100), xlabel="X", ylabel="Y", zlabel="Z")


@gallery("contour3d_3")
def _(fig):
    projected_contours(fig, "contour")


@gallery("contourf3d")
def _(fig):
    ax3d(fig).contourf(*axes3d.get_test_data(0.05), cmap="coolwarm")


@gallery("contourf3d_2")
def _(fig):
    projected_contours(fig, "contourf")


@gallery("box3d", figsize=(5, 4))
def _(fig):
    Nx, Ny, Nz = 100, 300, 500
    X, Y, Z = np.meshgrid(np.arange(Nx), np.arange(Ny), -np.arange(Nz))
    data = ((X + 100) ** 2 + (Y - 20) ** 2 + 2 * Z) / 1000 + 1
    kw = {"vmin": data.min(), "vmax": data.max(),
          "levels": np.linspace(data.min(), data.max(), 10)}
    ax = ax3d(fig, 111)
    ax.contourf(X[:, :, 0], Y[:, :, 0], data[:, :, 0], zdir="z", offset=0, **kw)
    ax.contourf(X[0, :, :], data[0, :, :], Z[0, :, :], zdir="y", offset=0, **kw)
    C = ax.contourf(data[:, -1, :], Y[:, -1, :], Z[:, -1, :], zdir="x", offset=X.max(), **kw)
    xmin, xmax = X.min(), X.max()
    ymin, ymax = Y.min(), Y.max()
    zmin, zmax = Z.min(), Z.max()
    ax.set(xlim=(xmin, xmax), ylim=(ymin, ymax), zlim=(zmin, zmax))
    edges_kw = dict(color="0.4", linewidth=1, zorder=1e3)
    ax.plot([xmax, xmax], [ymin, ymax], 0, **edges_kw)
    ax.plot([xmin, xmax], [ymin, ymin], 0, **edges_kw)
    ax.plot([xmax, xmax], [ymin, ymin], [zmin, zmax], **edges_kw)
    ax.set(xlabel="X [km]", ylabel="Y [km]", zlabel="Z [m]", zticks=[0, -150, -300, -450])
    ax.view_init(40, -30, 0)
    ax.set_box_aspect(None, zoom=0.9)
    fig.colorbar(C, ax=ax, fraction=0.02, pad=0.1, label="Name [units]")


def ring_contours(fig, method):
    """``tricontour3d`` and ``tricontourf3d``: contours of ``ring_points``
    over their masked triangulation, seen from 45 degrees up."""
    x, y, z = ring_points(48)
    ax = ax3d(fig)
    triang = ring_triangulation(x, y, lambda xmid, ymid: np.hypot(xmid, ymid) < 0.25)
    getattr(ax, method)(triang, z, cmap="CMRmap")
    ax.view_init(elev=45.0)


@gallery("tricontour3d")
def _(fig):
    ring_contours(fig, "tricontour")


@gallery("tricontourf3d")
def _(fig):
    ring_contours(fig, "tricontourf")


# --- surfaces and wireframes ---------------------------------------------------


@gallery("surface3d")
def _(fig):
    ax = ax3d(fig)
    surf = ax.plot_surface(*sinc_surface(), cmap="coolwarm", linewidth=0, antialiased=False)
    ax.set_zlim(-1.01, 1.01)
    ax.zaxis.set_major_locator(LinearLocator(10))
    ax.zaxis.set_major_formatter("{x:.02f}")
    fig.colorbar(surf, shrink=0.5, aspect=5)


@gallery("surface3d_2")
def _(fig):
    ax = ax3d(fig)
    u = np.linspace(0, 2 * np.pi, 100)
    v = np.linspace(0, np.pi, 100)
    x = 10 * np.outer(np.cos(u), np.sin(v))
    y = 10 * np.outer(np.sin(u), np.sin(v))
    z = 10 * np.outer(np.ones(np.size(u)), np.cos(v))
    ax.plot_surface(x, y, z)
    ax.set_aspect("equal")


@gallery("surface3d_3")
def _(fig):
    ax = ax3d(fig)
    X, Y, Z = sinc_surface()
    colortuple = ("y", "b")
    colors = np.empty(X.shape, dtype=str)
    for y in range(X.shape[0]):
        for x in range(X.shape[1]):
            colors[y, x] = colortuple[(x + y) % len(colortuple)]
    ax.plot_surface(X, Y, Z, facecolors=colors, linewidth=0)
    ax.set_zlim(-1, 1)
    ax.zaxis.set_major_locator(LinearLocator(6))


@gallery("surface3d_radial")
def _(fig):
    ax = ax3d(fig)
    R, P = np.meshgrid(np.linspace(0, 1.25, 50), np.linspace(0, 2 * np.pi, 50))
    Z = (R**2 - 1) ** 2
    ax.plot_surface(R * np.cos(P), R * np.sin(P), Z, cmap="YlGnBu_r")
    ax.set_zlim(0, 1)
    label_xyz(ax, r"$\phi_\mathrm{real}$", r"$\phi_\mathrm{im}$", r"$V(\phi)$")


@gallery("offset")
def _(fig):
    ax = ax3d(fig)
    X, Y = np.mgrid[0:6 * np.pi:0.25, 0:4 * np.pi:0.25]
    Z = np.sqrt(np.abs(np.cos(X) + np.cos(Y)))
    ax.plot_surface(X + 1e5, Y + 1e5, Z, cmap="autumn", cstride=2, rstride=2)
    label_xyz(ax, "X label", "Y label", "Z label")
    ax.set_zlim(0, 2)


def _terrain():
    """A stand-in for the part of ``jacksboro_fault_dem.npz`` the example
    shows: the same 45 x 45 grid of longitudes and latitudes, and whole
    elevations from about 370 to 730 m (a ridge cut by a valley)."""
    x = np.linspace(-84.41375, -84.07791666666667, 403)[5:50]
    y = np.linspace(36.73291666666667, 36.44625, 344)[5:50]
    x, y = np.meshgrid(x, y)
    u, v = np.meshgrid(np.linspace(0, 1, 45), np.linspace(0, 1, 45))
    ridge = 120 * np.sin(5 * u + 2 * v) * np.cos(3 * v)
    valley = 100 * np.exp(-(((u - v - 0.1) / 0.15) ** 2))
    return x, y, np.round(560 + ridge - valley)


@gallery("custom_shaded_3d_surface")
def _(fig):
    x, y, z = _terrain()
    ax = ax3d(fig)
    ls = LightSource(270, 45)
    rgb = ls.shade(z, cmap=mpl.colormaps["gist_earth"], vert_exag=0.1, blend_mode="soft")
    ax.plot_surface(x, y, z, rstride=1, cstride=1, facecolors=rgb, linewidth=0,
                    antialiased=False, shade=False)


@gallery("mixed_subplots", figsize=(4, 8))
def _(fig):
    def f(t):
        return np.cos(2 * np.pi * t) * np.exp(-t)

    fig.suptitle("A tale of 2 subplots")
    ax = fig.add_subplot(2, 1, 1)
    t1 = np.arange(0.0, 5.0, 0.1)
    t2 = np.arange(0.0, 5.0, 0.02)
    ax.plot(t1, f(t1), "bo", t2, f(t2), "k--", markerfacecolor="green")
    ax.grid(True)
    ax.set_ylabel("Damped oscillation")
    ax = ax3d(fig, 2, 1, 2)
    ax.plot_surface(*sinc_surface(), rstride=1, cstride=1, linewidth=0, antialiased=False)
    ax.set_zlim(-1, 1)


@gallery("subplot3d", figsize=(9.6, 4.8))
def _(fig):
    ax = ax3d(fig, 1, 2, 1)
    surf = ax.plot_surface(*sinc_surface(), rstride=1, cstride=1, cmap="coolwarm", linewidth=0,
                           antialiased=False)
    ax.set_zlim(-1.01, 1.01)
    fig.colorbar(surf, shrink=0.5, aspect=10)
    ax = ax3d(fig, 1, 2, 2)
    ax.plot_wireframe(*axes3d.get_test_data(0.05), rstride=10, cstride=10)


@gallery("wire3d")
def _(fig):
    ax3d(fig).plot_wireframe(*axes3d.get_test_data(0.05), rstride=10, cstride=10)


@gallery("wire3d_zero_stride", figsize=(8, 12))
def _(fig):
    ax1, ax2 = fig.subplots(2, 1, subplot_kw={"projection": "3d"})
    X, Y, Z = axes3d.get_test_data(0.05)
    ax1.plot_wireframe(X, Y, Z, rstride=10, cstride=0)
    ax1.set_title("Column (x) stride set to 0")
    ax2.plot_wireframe(X, Y, Z, rstride=0, cstride=10)
    ax2.set_title("Row (y) stride set to 0")
    fig.tight_layout()


@gallery("axlim_clip")
def _(fig):
    ax = fig.subplots(subplot_kw={"projection": "3d"})
    X, Y = np.meshgrid(np.arange(-5, 5, 0.5), np.arange(-5, 5, 0.5))
    Z = np.sin(np.sqrt(X**2 + Y**2))
    ax.plot_wireframe(X, Y, Z, color="C0")
    ax.plot_wireframe(X, Y, Z, color="C1", axlim_clip=True)
    ax.set(xlim=(0, 10), ylim=(-5, 5), zlim=(-1, 0.5))
    ax.legend(["axlim_clip=False (default)", "axlim_clip=True"])


@gallery("projections")
def _(fig):
    axs = fig.subplots(1, 3, subplot_kw={"projection": "3d"})
    X, Y, Z = axes3d.get_test_data(0.05)
    for ax in axs:
        ax.plot_wireframe(X, Y, Z, rstride=10, cstride=10)
    axs[0].set_proj_type("ortho")
    axs[0].set_title("'ortho'\nfocal_length = ∞", fontsize=10)
    axs[1].set_proj_type("persp")
    axs[1].set_title("'persp'\nfocal_length = 1 (default)", fontsize=10)
    axs[2].set_proj_type("persp", focal_length=0.2)
    axs[2].set_title("'persp'\nfocal_length = 0.2", fontsize=10)


VIEWS = [("XY", (90, -90, 0)), ("XZ", (0, -90, 0)), ("YZ", (0, 0, 0)),
         ("-XY", (-90, 90, 0)), ("-XZ", (0, 90, 0)), ("-YZ", (0, 180, 0))]


def annotate_axes(ax, text, fontsize=18):
    """``view_planes_3d``'s label in the middle of the box."""
    ax.text(x=0.5, y=0.5, z=0.5, s=text, va="center", ha="center", fontsize=fontsize,
            color="black")


@gallery("view_planes_3d", figsize=(12, 8.5))
def _(fig):
    layout = [["XY", ".", "L", "."],
              ["XZ", "YZ", "-XZ", "-YZ"],
              [".", ".", "-XY", "."]]
    axd = fig.subplot_mosaic(layout, subplot_kw={"projection": "3d"})
    for plane, angles in VIEWS:
        axd[plane].set_xlabel("x")
        axd[plane].set_ylabel("y")
        axd[plane].set_zlabel("z")
        axd[plane].set_proj_type("ortho")
        axd[plane].view_init(elev=angles[0], azim=angles[1], roll=angles[2])
        axd[plane].set_box_aspect(None, zoom=1.25)
        annotate_axes(axd[plane], f"{plane}\n{angles}", fontsize=14)
    for plane in ("XY", "-XY"):
        axd[plane].set_zticklabels([])
        axd[plane].set_zlabel("")
    for plane in ("XZ", "-XZ"):
        axd[plane].set_yticklabels([])
        axd[plane].set_ylabel("")
    for plane in ("YZ", "-YZ"):
        axd[plane].set_xticklabels([])
        axd[plane].set_xlabel("")
    annotate_axes(axd["L"], "mplot3d primary view planes\nax.view_init(elev, azim, roll)",
                  fontsize=18)
    axd["L"].set_axis_off()


# --- triangulated surfaces -------------------------------------------------------


@gallery("trisurf3d")
def _(fig):
    radii = np.linspace(0.125, 1.0, 8)
    angles = np.linspace(0, 2 * np.pi, 36, endpoint=False)[..., np.newaxis]
    x = np.append(0, (radii * np.cos(angles)).flatten())
    y = np.append(0, (radii * np.sin(angles)).flatten())
    ax3d(fig).plot_trisurf(x, y, np.sin(-x * y), linewidth=0.2, antialiased=True)


@gallery("trisurf3d_2", figsize=(9.6, 4.8))
def _(fig):
    u, v = np.meshgrid(np.linspace(0, 2.0 * np.pi, endpoint=True, num=50),
                       np.linspace(-0.5, 0.5, endpoint=True, num=10))
    u, v = u.flatten(), v.flatten()
    x = (1 + 0.5 * v * np.cos(u / 2.0)) * np.cos(u)
    y = (1 + 0.5 * v * np.cos(u / 2.0)) * np.sin(u)
    z = 0.5 * v * np.sin(u / 2.0)
    mobius = tri.Triangulation(u, v)
    ax = ax3d(fig, 1, 2, 1)
    ax.plot_trisurf(x, y, z, triangles=mobius.triangles, cmap="Spectral")
    ax.set_zlim(-1, 1)
    x, y, z = ring_points(36)
    ax = ax3d(fig, 1, 2, 2)
    triang = ring_triangulation(x, y, lambda xmid, ymid: xmid**2 + ymid**2 < 0.25**2)
    ax.plot_trisurf(triang, z, cmap="CMRmap")


# --- fill_between, error bars, stems, quivers and polygons ------------------------


@gallery("fillbetween3d")
def _(fig):
    theta = np.linspace(0, 2 * np.pi, 50)
    x1, y1, z1 = np.cos(theta), np.sin(theta), 0.1 * np.sin(6 * theta)
    x2, y2 = 0.6 * np.cos(theta), 0.6 * np.sin(theta)
    ax3d(fig).fill_between(x1, y1, z1, x2, y2, 2, alpha=0.5, edgecolor="k")


@gallery("fillunder3d")
def _(fig):
    gamma = np.vectorize(math.gamma)
    x = np.linspace(0.0, 10.0, 31)
    lambdas = range(1, 9)
    ax = ax3d(fig)
    facecolors = mpl.colormaps["viridis_r"](np.linspace(0, 1, len(lambdas)))
    for i, lam in enumerate(lambdas):
        ax.fill_between(x, lam, lam**x * np.exp(-lam) / gamma(x + 1), x, lam, 0,
                        facecolors=facecolors[i], alpha=0.7)
    ax.set(xlim=(0, 10), ylim=(1, 9), zlim=(0, 0.35), xlabel="x", ylabel=r"$\lambda$",
           zlabel="probability")


@gallery("errorbar3d")
def _(fig):
    ax = ax3d(fig)
    t = np.arange(0, 2 * np.pi + 0.1, 0.01)
    x, y, z = np.sin(t), np.cos(3 * t), np.sin(5 * t)
    estep = 15
    i = np.arange(t.size)
    zuplims = (i % estep == 0) & (i // estep % 3 == 0)
    zlolims = (i % estep == 0) & (i // estep % 3 == 2)
    ax.errorbar(x, y, z, 0.2, zuplims=zuplims, zlolims=zlolims, errorevery=estep)
    label_xyz(ax, "X label", "Y label", "Z label")


def stem_curve():
    """``stem3d_demo``'s data: a unit circle rising with its angle."""
    theta = np.linspace(0, 2 * np.pi)
    return np.cos(theta - np.pi / 2), np.sin(theta - np.pi / 2), theta


@gallery("stem3d_demo_1")
def _(fig):
    fig.subplots(subplot_kw=dict(projection="3d")).stem(*stem_curve())


@gallery("stem3d_demo_2")
def _(fig):
    ax = fig.subplots(subplot_kw=dict(projection="3d"))
    markerline, stemlines, baseline = ax.stem(*stem_curve(), linefmt="grey", markerfmt="D",
                                              bottom=np.pi)
    markerline.set_markerfacecolor("none")


@gallery("stem3d_demo_3")
def _(fig):
    ax = fig.subplots(subplot_kw=dict(projection="3d"))
    ax.stem(*stem_curve(), bottom=-1, orientation="x")
    ax.set(xlabel="x", ylabel="y", zlabel="z")


@gallery("quiver3d")
def _(fig):
    ax = ax3d(fig)
    x, y, z = np.meshgrid(np.arange(-0.8, 1, 0.2), np.arange(-0.8, 1, 0.2),
                          np.arange(-0.8, 1, 0.8))
    u = np.sin(np.pi * x) * np.cos(np.pi * y) * np.cos(np.pi * z)
    v = -np.cos(np.pi * x) * np.sin(np.pi * y) * np.cos(np.pi * z)
    w = np.sqrt(2.0 / 3.0) * np.cos(np.pi * x) * np.cos(np.pi * y) * np.sin(np.pi * z)
    ax.quiver(x, y, z, u, v, w, length=0.1, normalize=True)


@gallery("polys3d")
def _(fig):
    angles = np.linspace(0, 2 * np.pi, 6, endpoint=False)
    x = np.append(np.cos(angles), np.cos(angles[0]))
    y = np.append(np.sin(angles), np.sin(angles[0]))
    verts = np.array([list(zip(x * z, y * z, np.full_like(x, z))) for z in [-3, -2, -1]])
    ax = ax3d(fig)
    ax.add_collection3d(Poly3DCollection(verts, alpha=0.7))
    ax.set_aspect("equalxy")


# --- images on planes -----------------------------------------------------------


def imshow3d(ax, array, value_direction="z", pos=0, norm=None, cmap=None):
    """``imshow3d``: ``array`` as a coloured plane normal to
    ``value_direction`` at ``pos``, one face per value."""
    if norm is None:
        norm = Normalize()
    colors = mpl.colormaps[cmap or mpl.rcParams["image.cmap"]](norm(array))
    if value_direction == "x":
        nz, ny = array.shape
        zi, yi = np.mgrid[0:nz + 1, 0:ny + 1]
        xi = np.full_like(yi, pos)
    elif value_direction == "y":
        nx, nz = array.shape
        xi, zi = np.mgrid[0:nx + 1, 0:nz + 1]
        yi = np.full_like(zi, pos)
    else:
        ny, nx = array.shape
        yi, xi = np.mgrid[0:ny + 1, 0:nx + 1]
        zi = np.full_like(xi, pos)
    ax.plot_surface(xi, yi, zi, rstride=1, cstride=1, facecolors=colors, shade=False)


@gallery("imshow3d")
def _(fig):
    ax = ax3d(fig)
    ax.set(xlabel="x", ylabel="y", zlabel="z")
    nx, ny, nz = 8, 10, 5
    g = rng(SEED)
    data_xy = np.arange(ny * nx).reshape(ny, nx) + 15 * g.uniform(ny * nx).reshape(ny, nx)
    data_yz = np.arange(nz * ny).reshape(nz, ny) + 10 * g.uniform(nz * ny).reshape(nz, ny)
    data_zx = np.arange(nx * nz).reshape(nx, nz) + 8 * g.uniform(nx * nz).reshape(nx, nz)
    imshow3d(ax, data_xy)
    imshow3d(ax, data_yz, value_direction="x", cmap="magma")
    imshow3d(ax, data_zx, value_direction="y", pos=ny, cmap="plasma")


def plot_quadrants(ax, array, fixed_coord, cmap):
    """``intersecting_planes``: the plane of ``array`` through its centre
    normal to ``fixed_coord``, drawn as four quadrants so that the planes
    cutting it sort correctly."""
    nx, ny, nz = array.shape
    index = {
        "x": (nx // 2, slice(None), slice(None)),
        "y": (slice(None), ny // 2, slice(None)),
        "z": (slice(None), slice(None), nz // 2),
    }[fixed_coord]
    plane_data = array[index]
    n0, n1 = plane_data.shape
    quadrants = [plane_data[:n0 // 2, :n1 // 2], plane_data[:n0 // 2, n1 // 2:],
                 plane_data[n0 // 2:, :n1 // 2], plane_data[n0 // 2:, n1 // 2:]]
    min_val, max_val = array.min(), array.max()
    cmap = mpl.colormaps[cmap]
    for i, quadrant in enumerate(quadrants):
        facecolors = cmap((quadrant - min_val) / (max_val - min_val))
        if fixed_coord == "x":
            Y, Z = np.mgrid[0:ny // 2, 0:nz // 2]
            X = nx // 2 * np.ones_like(Y)
            ax.plot_surface(X, Y + (i // 2) * ny // 2, Z + (i % 2) * nz // 2, rstride=1,
                            cstride=1, facecolors=facecolors, shade=False)
        elif fixed_coord == "y":
            X, Z = np.mgrid[0:nx // 2, 0:nz // 2]
            Y = ny // 2 * np.ones_like(X)
            ax.plot_surface(X + (i // 2) * nx // 2, Y, Z + (i % 2) * nz // 2, rstride=1,
                            cstride=1, facecolors=facecolors, shade=False)
        else:
            X, Y = np.mgrid[0:nx // 2, 0:ny // 2]
            Z = nz // 2 * np.ones_like(X)
            ax.plot_surface(X + (i // 2) * nx // 2, Y + (i % 2) * ny // 2, Z, rstride=1,
                            cstride=1, facecolors=facecolors, shade=False)


@gallery("intersecting_planes")
def _(fig):
    nx, ny, nz = 70, 100, 50
    r_square = (np.mgrid[-1:1:1j * nx, -1:1:1j * ny, -1:1:1j * nz] ** 2).sum(0)
    ax = ax3d(fig)
    ax.set_box_aspect(r_square.shape)
    for fixed_coord in "xyz":
        plot_quadrants(ax, r_square, fixed_coord, cmap="viridis_r")


# --- voxels -----------------------------------------------------------------------


@gallery("voxels")
def _(fig):
    x, y, z = np.indices((8, 8, 8))
    cube1 = (x < 3) & (y < 3) & (z < 3)
    cube2 = (x >= 5) & (y >= 5) & (z >= 5)
    link = abs(x - y) + abs(y - z) + abs(z - x) <= 2
    voxelarray = cube1 | cube2 | link
    colors = np.empty(voxelarray.shape, dtype=object)
    colors[link] = "red"
    colors[cube1] = "blue"
    colors[cube2] = "green"
    ax3d(fig).voxels(voxelarray, facecolors=colors, edgecolor="k")


def explode(data):
    """``voxels_numpy_logo``'s ``explode``: ``data`` spread out with an
    empty voxel between neighbours."""
    size = np.array(data.shape) * 2
    data_e = np.zeros(size - 1, dtype=data.dtype)
    data_e[::2, ::2, ::2] = data
    return data_e


@gallery("voxels_numpy_logo")
def _(fig):
    n_voxels = np.zeros((4, 3, 4), dtype=bool)
    n_voxels[0, 0, :] = True
    n_voxels[-1, 0, :] = True
    n_voxels[1, 0, 2] = True
    n_voxels[2, 0, 1] = True
    facecolors = np.where(n_voxels, "#FFD65DC0", "#7A88CCC0")
    edgecolors = np.where(n_voxels, "#BFAB6E", "#7D84A6")
    filled_2 = explode(np.ones(n_voxels.shape))
    fcolors_2 = explode(facecolors)
    ecolors_2 = explode(edgecolors)
    x, y, z = np.indices(np.array(filled_2.shape) + 1).astype(float) // 2
    x[0::2, :, :] += 0.05
    y[:, 0::2, :] += 0.05
    z[:, :, 0::2] += 0.05
    x[1::2, :, :] += 0.95
    y[:, 1::2, :] += 0.95
    z[:, :, 1::2] += 0.95
    ax = ax3d(fig)
    ax.voxels(x, y, z, filled_2, facecolors=fcolors_2, edgecolors=ecolors_2)
    ax.set_aspect("equal")


@gallery("voxels_rgb")
def _(fig):
    r, g, b = np.indices((17, 17, 17)) / 16.0
    rc, gc, bc = midpoints(r), midpoints(g), midpoints(b)
    sphere = (rc - 0.5) ** 2 + (gc - 0.5) ** 2 + (bc - 0.5) ** 2 < 0.5**2
    colors = np.stack([rc, gc, bc], axis=-1)
    ax = ax3d(fig)
    ax.voxels(r, g, b, sphere, facecolors=colors, edgecolors=np.clip(2 * colors - 0.5, 0, 1),
              linewidth=0.5)
    ax.set(xlabel="r", ylabel="g", zlabel="b")
    ax.set_aspect("equal")


@gallery("voxels_torus")
def _(fig):
    r, theta, z = np.mgrid[0:1:11j, 0:np.pi * 2:25j, -0.5:0.5:11j]
    x = r * np.cos(theta)
    y = r * np.sin(theta)
    rc, thetac, zc = midpoints(r), midpoints(theta), midpoints(z)
    sphere = (rc - 0.7) ** 2 + (zc + 0.2 * np.cos(thetac * 2)) ** 2 < 0.2**2
    hsv = np.stack([thetac / (np.pi * 2), rc, zc + 0.5], axis=-1)
    colors = mcolors.hsv_to_rgb(hsv)
    ax3d(fig).voxels(x, y, z, sphere, facecolors=colors,
                     edgecolors=np.clip(2 * colors - 0.5, 0, 1), linewidth=0.5)
