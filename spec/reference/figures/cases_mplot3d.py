"""mplot3d: lines, scatter (depth shading), surfaces, wireframes, trisurf,
3D contours, bar3d, quiver, voxels, stem, text, view and projection
control, box aspect, labels/limits, inverted axes, panes, computed zorder."""

import matplotlib as mpl
import numpy as np

from spec_support import case, rng


def _surface(n=30):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    r = np.hypot(xx, yy)
    return xx, yy, np.sin(r) / (r + 1.0)


def _helix(n=200):
    t = np.linspace(0.0, 4 * np.pi, n)
    return np.cos(t), np.sin(t), t / (4 * np.pi)


def _ax3d(fig, **kwargs):
    return fig.add_subplot(projection="3d", **kwargs)


@case("mplot3d/empty_default")
def _(fig):
    _ax3d(fig)


@case("mplot3d/plot_line", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix(), label="helix")
    ax.plot([-1, 1], [-1, 1], [0, 1], "r--o")
    ax.legend()


@case("mplot3d/scatter_depthshade", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    g = rng(41)
    ax.scatter(g.uniform(80), g.uniform(80), g.uniform(80), s=40)
    ax.scatter(g.uniform(30), g.uniform(30), g.uniform(30), marker="^", s=60)


@case("mplot3d/scatter_no_depthshade_cmap", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    g = rng(42)
    x, y, z = g.uniform(100), g.uniform(100), g.uniform(100)
    ax.scatter(x, y, z, c=z, cmap="plasma", depthshade=False, s=30, edgecolors="k")


@case("mplot3d/plot_surface_cmap")
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(40), cmap="viridis")


@case("mplot3d/plot_surface_strides_edges", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(40), rstride=4, cstride=4, color="tab:orange", edgecolor="k",
                    linewidth=0.4)


@case("mplot3d/plot_surface_no_shade_alpha", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(25), color="tab:blue", shade=False, alpha=0.5)


@case("mplot3d/plot_surface_facecolors", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    xx, yy, zz = _surface(16)
    checker = (np.indices(zz.shape).sum(axis=0) % 2).astype(float)
    ax.plot_surface(xx, yy, zz, facecolors=mpl.colormaps["coolwarm"](checker), shade=False)


@case("mplot3d/plot_wireframe", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_wireframe(*_surface(40), rstride=4, cstride=6, color="k", linewidth=0.6)


@case("mplot3d/plot_trisurf", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    g = rng(43)
    x, y = g.uniform(150, -3.0, 3.0), g.uniform(150, -3.0, 3.0)
    ax.plot_trisurf(x, y, np.cos(x) * np.sin(y), cmap="Spectral", linewidth=0.2, edgecolor="grey")


@case("mplot3d/contour_projections", figsize=(5, 5))
def _(fig):
    ax = _ax3d(fig)
    xx, yy, zz = _surface(40)
    ax.plot_surface(xx, yy, zz, color="0.8", alpha=0.4)
    ax.contour(xx, yy, zz, zdir="z", offset=-0.6, cmap="coolwarm")
    ax.contour(xx, yy, zz, zdir="x", offset=-4, cmap="coolwarm")
    ax.contour(xx, yy, zz, zdir="y", offset=4, cmap="coolwarm")
    ax.set(xlim=(-4, 3), ylim=(-3, 4), zlim=(-0.6, 0.8))


@case("mplot3d/contour3d_lines", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.contour(*_surface(40), levels=12, cmap="viridis")


@case("mplot3d/contourf3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.contourf(*_surface(40), levels=8, cmap="magma")


@case("mplot3d/bar3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y = np.meshgrid(np.arange(4), np.arange(3))
    x, y = x.ravel(), y.ravel()
    heights = 1 + (x * 3 + y * 5) % 7
    ax.bar3d(x, y, np.zeros_like(x), 0.6, 0.6, heights, shade=True, color="tab:cyan")


@case("mplot3d/bar3d_no_shade_colors", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.bar3d([0, 1, 2], [0, 1, 2], [0, 0, 0], 0.5, 0.5, [3, 1, 2], shade=False,
             color=["red", "green", "blue"], edgecolor="k")


@case("mplot3d/bar_zdir", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x = np.arange(8)
    for i, c in enumerate(["r", "g", "b"]):
        ax.bar(x, (x * (i + 2)) % 7 + 1, zs=i * 10, zdir="y", color=c, alpha=0.8)


@case("mplot3d/quiver3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    g = np.linspace(-1.0, 1.0, 4)
    xx, yy, zz = np.meshgrid(g, g, g)
    ax.quiver(xx, yy, zz, -yy, xx, 0.3 * zz, length=0.3, normalize=True)


@case("mplot3d/voxels", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x, y, z = np.indices((6, 6, 6))
    filled = ((x - 2.5) ** 2 + (y - 2.5) ** 2 + (z - 2.5) ** 2) < 7.0
    colors = np.where(z < 3, "tab:blue", "tab:orange")
    ax.voxels(filled, facecolors=colors, edgecolor="k", linewidth=0.3)


@case("mplot3d/stem", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 16)
    ax.stem(np.cos(t), np.sin(t), t)


@case("mplot3d/text_and_text2d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.text(0, 0, 0, "origin", color="red")
    ax.text(1, 0, 0, "along x", zdir="x")
    ax.text(0, 1, 0, "along y", zdir="y")
    ax.text(0, 0, 1, "along z", zdir="z")
    ax.text2D(0.05, 0.95, "2D text", transform=ax.transAxes)
    ax.set(xlim=(0, 1), ylim=(0, 1), zlim=(0, 1))


@case("mplot3d/view_init_roll", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_wireframe(*_surface(20), rstride=2, cstride=2)
    ax.view_init(elev=20, azim=30, roll=15)


@case("mplot3d/view_top", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix())
    ax.view_init(elev=90, azim=-90)


@case("mplot3d/view_negative_elev", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(25), cmap="viridis")
    ax.view_init(elev=-25, azim=120)


@case("mplot3d/box_aspect", figsize=(5, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix())
    ax.set_box_aspect((3, 1, 0.5))


@case("mplot3d/proj_ortho", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig, proj_type="ortho")
    ax.plot_wireframe(*_surface(20), rstride=2, cstride=2)


@case("mplot3d/focal_length", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_wireframe(*_surface(20), rstride=2, cstride=2)
    ax.set_proj_type("persp", focal_length=0.2)


@case("mplot3d/labels_limits", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix())
    ax.set_xlabel("X axis")
    ax.set_ylabel("Y axis")
    ax.set_zlabel("Z axis")
    ax.set_title("limits")
    ax.set_xlim(-2, 2)
    ax.set_zlim(0, 0.5)


@case("mplot3d/inverted_axes", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix())
    ax.invert_zaxis()
    ax.invert_xaxis()


@case("mplot3d/pane_colors_no_grid", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot(*_helix())
    ax.xaxis.set_pane_color((1.0, 0.0, 0.0, 0.2))
    ax.yaxis.set_pane_color((0.0, 1.0, 0.0, 0.2))
    ax.zaxis.set_pane_color((0.0, 0.0, 1.0, 0.2))
    ax.grid(False)


@case("mplot3d/axis_off", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    ax.plot_surface(*_surface(25), cmap="viridis")
    ax.set_axis_off()


@case("mplot3d/computed_zorder_false", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig, computed_zorder=False)
    xx, yy, zz = _surface(25)
    ax.plot_surface(xx, yy, zz, color="0.7", zorder=1)
    ax.scatter([0, 1, -1], [0, -1, 1], [-0.5, -0.5, -0.5], c="r", s=80, zorder=2)
