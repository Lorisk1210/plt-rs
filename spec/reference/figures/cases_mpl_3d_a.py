"""matplotlib's own image tests from the first part of
``mpl_toolkits/mplot3d/tests/test_axes3d.py`` (``test_invisible_axes`` to
``test_wireframe3dzerorstride``): every ``@mpl3d_image_comparison`` and
``@image_comparison`` test and the ``fig_test`` side of every
``@check_figures_equal`` test.

``mpl3d_image_comparison`` tests ask for ``mpl20`` and run in the reference
configuration (6.4x4.8 in at 100 dpi) with ``remove_text=True`` unless they
say otherwise; ``remove_text=True`` becomes ``remove_ticks_and_titles`` at
the end of the build. ``check_figures_equal`` tests run under the test
suite's default style, ``classic`` plus ``_classic_test_patch``, on classic's
8x6 in canvas saved at 100 dpi. The tests' own rcParams changes become the
case's ``rc`` (``axes3d.automargin`` is only read when limits are set, so
setting it for the whole build changes nothing before the test sets it).
Parametrized tests contribute one case per parameter set. Where a test goes
through pyplot, the case builds on the figure it receives; a test's
``plt.figaspect`` becomes the size it computes.

Deviations from the tests:

* ``test_tricontour`` draws its points from ``spec_support.rng`` seeded with
  the test's seed instead of ``np.random``.
* ``test_plot_scatter_masks`` passes masked arrays, as the test does; the
  Rust translation hides the masked points with NaN, which is what the
  test's reference side does.
"""

import itertools

import matplotlib as mpl
import matplotlib.style
import numpy as np
from matplotlib import colors as mcolors
from matplotlib.testing.decorators import remove_ticks_and_titles
from mpl_toolkits.mplot3d import axes3d

from spec_support import case, rng

GROUP = "mpl_3d_a"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: What the tests written before matplotlib 3.9 changed the 3-D margins set.
AUTOMARGIN = {"axes3d.automargin": True}


def classic(name, **options):
    """A ``check_figures_equal`` case: the test suite's default style on
    classic's 8x6 in canvas."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), rc=CLASSIC, **options)


def mpl3d(name, remove_text=True, **options):
    """An ``mpl3d_image_comparison(style='mpl20')`` case, without titles and
    tick labels unless ``remove_text`` is false."""
    def register(build):
        def wrapped(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)
        case(f"{GROUP}/{name}", **options)(wrapped)
        return build
    return register


def ax3d(fig, *args):
    """``fig.add_subplot(*args, projection='3d')``."""
    return fig.add_subplot(*args, projection="3d")


def plot_cuboid(ax, scale):
    """The test module's ``plot_cuboid``: the edges of a cuboid with side
    lengths ``scale``."""
    r = [0, 1]
    pts = itertools.combinations(np.array(list(itertools.product(r, r, r))), 2)
    for start, end in pts:
        if np.sum(np.abs(start - end)) == r[1] - r[0]:
            ax.plot3D(*zip(start * np.array(scale), end * np.array(scale)))


def bar_grid(height_offset):
    """The 4x5 grid of bars of the ``bar3d`` shading tests: their x and y
    corners and their heights ``x + y + height_offset``."""
    x2d, y2d = np.meshgrid(np.arange(4), np.arange(5))
    x2d, y2d = x2d.ravel(), y2d.ravel()
    return x2d, y2d, x2d + y2d + height_offset


def disc_points():
    """The points of the ``trisurf3d`` tests: rings of a disc and the saddle
    ``sin(-x y)`` over them."""
    n_angles = 36
    n_radii = 8
    radii = np.linspace(0.125, 1.0, n_radii)
    angles = np.linspace(0, 2 * np.pi, n_angles, endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x = np.append(0, (radii * np.cos(angles)).flatten())
    y = np.append(0, (radii * np.sin(angles)).flatten())
    return x, y, np.sin(-x * y)


def sinc_surface():
    """``sin(hypot(x, y))`` on ``arange(-5, 5, 0.25)`` squared."""
    x, y = np.meshgrid(np.arange(-5, 5, 0.25), np.arange(-5, 5, 0.25))
    return x, y, np.sin(np.hypot(x, y))


def two_markers(fig, colors, azim, axis_off=False):
    """The marker draw-order tests: two large markers seen edge-on."""
    ax = ax3d(fig)
    if axis_off:
        ax.set_axis_off()
    ax.scatter([-1, 1], [1, -1], [0, 0], s=3500, c=colors)
    ax.view_init(elev=0, azim=azim, roll=0)


# --- axes, ticks and labels ------------------------------------------------


@classic("invisible_axes")
def _(fig):
    ax = fig.subplots(subplot_kw=dict(projection="3d"))
    ax.set_visible(False)


@mpl3d("grid_off")
def _(fig):
    ax3d(fig).grid(False)


@mpl3d("invisible_ticks_axis")
def _(fig):
    ax = ax3d(fig)
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_zticks([])
    for axis in [ax.xaxis, ax.yaxis, ax.zaxis]:
        axis.line.set_visible(False)


@mpl3d("axis_positions", remove_text=False)
def _(fig):
    positions = ["upper", "lower", "both", "none"]
    axs = fig.subplots(2, 2, subplot_kw={"projection": "3d"})
    for ax, pos in zip(axs.flatten(), positions):
        for axis in ax.xaxis, ax.yaxis, ax.zaxis:
            axis.set_label_position(pos)
            axis.set_ticks_position(pos)
        ax.set(xlabel="x", ylabel="y", zlabel="z", title=f"{pos}")


@mpl3d("axis_positions_inverted", remove_text=False, figsize=(10, 6))
def _(fig):
    combinations = list(itertools.product([False, True], repeat=3))
    axs = fig.subplots(2, 4, subplot_kw={"projection": "3d"})
    for ax, (invert_x, invert_y, invert_z) in zip(axs.flatten(), combinations):
        ax.plot([0, 1, 1], [0, 0, 1], [0, 1, 1])
        if invert_x:
            ax.invert_xaxis()
        if invert_y:
            ax.invert_yaxis()
        if invert_z:
            ax.invert_zaxis()
        title = (f'{"-" if invert_x else ""}x, '
                 f'{"-" if invert_y else ""}y, '
                 f'{"-" if invert_z else ""}z')
        ax.set(xlabel="x", ylabel="y", zlabel="z", title=title)


@mpl3d("aspects", remove_text=False)
def _(fig):
    aspects = ("auto", "equal", "equalxy", "equalyz", "equalxz", "equal")
    axs = fig.subplots(2, 3, subplot_kw={"projection": "3d"})
    for ax in axs.flatten()[0:-1]:
        plot_cuboid(ax, scale=[1, 1, 5])
    plot_cuboid(axs[1][2], scale=[1, 1, 1])
    for i, ax in enumerate(axs.flatten()):
        ax.set_title(aspects[i])
        ax.set_box_aspect((3, 4, 5))
        ax.set_aspect(aspects[i], adjustable="datalim")
    axs[1][2].set_title("equal (cube)")


@mpl3d("aspects_adjust_box", remove_text=False, figsize=(11, 3))
def _(fig):
    aspects = ("auto", "equal", "equalxy", "equalyz", "equalxz")
    axs = fig.subplots(1, len(aspects), subplot_kw={"projection": "3d"})
    for i, ax in enumerate(axs):
        plot_cuboid(ax, scale=[4, 3, 5])
        ax.set_title(aspects[i])
        ax.set_aspect(aspects[i], adjustable="box")


@mpl3d("axes3d_primary_views")
def _(fig):
    views = [(90, -90, 0), (0, -90, 0), (0, 0, 0),
             (-90, 90, 0), (0, 90, 0), (0, 180, 0)]
    fig.set_layout_engine("tight")
    axs = fig.subplots(2, 3, subplot_kw={"projection": "3d"})
    for i, ax in enumerate(axs.flat):
        ax.set_xlabel("x")
        ax.set_ylabel("y")
        ax.set_zlabel("z")
        ax.set_proj_type("ortho")
        ax.view_init(elev=views[i][0], azim=views[i][1], roll=views[i][2])


# --- bars --------------------------------------------------------------------


@mpl3d("bar3d")
def _(fig):
    ax = ax3d(fig)
    for c, z in zip(["r", "g", "b", "y"], [30, 20, 10, 0]):
        xs = np.arange(20)
        ys = np.arange(20)
        cs = [c] * len(xs)
        cs[0] = "c"
        ax.bar(xs, ys, zs=z, zdir="y", align="edge", color=cs, alpha=0.8)


@mpl3d("bar3d_shaded", figsize=(16, 4))
def _(fig):
    x2d, y2d, z = bar_grid(1)
    views = [(30, -60, 0), (30, 30, 30), (-30, 30, -90), (300, -30, 0)]
    axs = fig.subplots(1, len(views), subplot_kw=dict(projection="3d"))
    for ax, (elev, azim, roll) in zip(axs, views):
        ax.bar3d(x2d, y2d, x2d * 0, 1, 1, z, shade=True)
        ax.view_init(elev=elev, azim=azim, roll=roll)


@mpl3d("bar3d_notshaded")
def _(fig):
    x2d, y2d, z = bar_grid(0)
    ax3d(fig).bar3d(x2d, y2d, x2d * 0, 1, 1, z, shade=False)


# --- contours ----------------------------------------------------------------


@mpl3d("contour3d", rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    X, Y, Z = axes3d.get_test_data(0.05)
    ax.contour(X, Y, Z, zdir="z", offset=-100, cmap="coolwarm")
    ax.contour(X, Y, Z, zdir="x", offset=-40, cmap="coolwarm")
    ax.contour(X, Y, Z, zdir="y", offset=40, cmap="coolwarm")
    ax.axis(xmin=-40, xmax=40, ymin=-40, ymax=40, zmin=-100, zmax=100)


@mpl3d("contour3d_extend3d", rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    X, Y, Z = axes3d.get_test_data(0.05)
    ax.contour(X, Y, Z, zdir="z", offset=-100, cmap="coolwarm", extend3d=True)
    ax.set_xlim(-30, 30)
    ax.set_ylim(-20, 40)
    ax.set_zlim(-80, 80)


@mpl3d("contourf3d", rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    X, Y, Z = axes3d.get_test_data(0.05)
    ax.contourf(X, Y, Z, zdir="z", offset=-100, cmap="coolwarm")
    ax.contourf(X, Y, Z, zdir="x", offset=-40, cmap="coolwarm")
    ax.contourf(X, Y, Z, zdir="y", offset=40, cmap="coolwarm")
    ax.set_xlim(-40, 40)
    ax.set_ylim(-40, 40)
    ax.set_zlim(-100, 100)


@mpl3d("contourf3d_fill", rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    X, Y = np.meshgrid(np.arange(-2, 2, 0.25), np.arange(-2, 2, 0.25))
    Z = X.clip(0, 0)
    Z[::5, ::5] = 0.1
    ax.contourf(X, Y, Z, offset=0, levels=[-0.1, 0], cmap="coolwarm")
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)
    ax.set_zlim(-1, 1)


def contourf3d_extend(extend, levels):
    """``test_contourf3d_extend``: filled contours of a paraboloid with
    ``extend`` and a norm narrower than the levels."""
    @classic(f"contourf3d_extend_{extend}")
    def _(fig):
        X, Y = np.meshgrid(np.arange(-2, 2, 0.25), np.arange(-2, 2, 0.25))
        Z = X**2 + Y**2
        ax = ax3d(fig)
        ax.contourf(X, Y, Z, levels, extend=extend, vmin=1, vmax=7)
        ax.set_xlim(-2, 2)
        ax.set_ylim(-2, 2)
        ax.set_zlim(-10, 10)


contourf3d_extend("both", [2, 4, 6])
contourf3d_extend("min", [2, 4, 6, 8])
contourf3d_extend("max", [0, 2, 4, 6])


@mpl3d("tricontour", rc=AUTOMARGIN)
def _(fig):
    g = rng(19680801)
    x = g.uniform(1000) - 0.5
    y = g.uniform(1000) - 0.5
    z = -(x**2 + y**2)
    ax3d(fig, 1, 2, 1).tricontour(x, y, z)
    ax3d(fig, 1, 2, 2).tricontourf(x, y, z)


# --- lines -------------------------------------------------------------------


@mpl3d("lines3d")
def _(fig):
    theta = np.linspace(-4 * np.pi, 4 * np.pi, 100)
    z = np.linspace(-2, 2, 100)
    r = z ** 2 + 1
    ax3d(fig).plot(r * np.sin(theta), r * np.cos(theta), z)


@classic("plot_scalar")
def _(fig):
    ax3d(fig).plot([1], [1], "o")


@mpl3d("mixedsubplot", figsize=(4, 8), rc=AUTOMARGIN)
def _(fig):
    def f(t):
        return np.cos(2 * np.pi * t) * np.exp(-t)

    t1 = np.arange(0.0, 5.0, 0.1)
    t2 = np.arange(0.0, 5.0, 0.02)
    ax = fig.add_subplot(2, 1, 1)
    ax.plot(t1, f(t1), "bo", t2, f(t2), "k--", markerfacecolor="green")
    ax.grid(True)
    ax = ax3d(fig, 2, 1, 2)
    X, Y, Z = sinc_surface()
    ax.plot_surface(X, Y, Z, rcount=40, ccount=40, linewidth=0, antialiased=False)
    ax.set_zlim3d(-1, 1)


@classic("tight_layout_text")
def _(fig):
    ax3d(fig).text(.5, .5, .5, s="some string")
    fig.tight_layout()


@mpl3d("plot_3d_from_2d")
def _(fig):
    ax = ax3d(fig)
    xs = np.arange(0, 5)
    ys = np.arange(5, 10)
    ax.plot(xs, ys, zs=0, zdir="x")
    ax.plot(xs, ys, zs=0, zdir="y")


# --- scatter -----------------------------------------------------------------


@mpl3d("scatter3d")
def _(fig):
    ax = ax3d(fig)
    ax.scatter(np.arange(10), np.arange(10), np.arange(10), c="r", marker="o")
    x = y = z = np.arange(10, 20)
    ax.scatter(x, y, z, c="b", marker="^")
    z[-1] = 0
    ax.scatter([], [], [], c="r", marker="X")


@mpl3d("scatter3d_color")
def _(fig):
    ax = ax3d(fig)
    ax.scatter(np.arange(10), np.arange(10), np.arange(10),
               facecolor="r", edgecolor="none", marker="o")
    ax.scatter(np.arange(10), np.arange(10), np.arange(10),
               facecolor="none", edgecolor="r", marker="o")
    ax.scatter(np.arange(10, 20), np.arange(10, 20), np.arange(10, 20),
               color="b", marker="s")


@mpl3d("scatter3d_linewidth")
def _(fig):
    ax3d(fig).scatter(np.arange(10), np.arange(10), np.arange(10),
                      marker="o", linewidth=np.arange(10))


@classic("scatter3d_cmap_alpha")
def _(fig):
    x, y, z = np.arange(5), np.zeros(5), np.arange(5)
    ax3d(fig).scatter(x, y, z, c=np.array([0, 1, np.nan, 3, 4]))


@classic("scatter3d_linewidth_modification")
def _(fig):
    c = ax3d(fig).scatter(np.arange(10), np.arange(10), np.arange(10), marker="o")
    c.set_linewidths(np.arange(10))


@classic("scatter3d_modification")
def _(fig):
    c = ax3d(fig).scatter(np.arange(10), np.arange(10), np.arange(10),
                          marker="o", depthshade=True)
    c.set_facecolor("C1")
    c.set_edgecolor("C2")
    c.set_alpha([0.3, 0.7] * 5)
    c.set_depthshade(False)
    c.set_sizes(np.full(10, 75))
    c.set_linewidths(3)


@classic("scatter3d_sorting")
def _(fig):
    y, x = np.mgrid[:10, :10]
    z = np.arange(x.size).reshape(x.shape)

    sizes = np.full(z.shape, 25)
    sizes[0::2, 0::2] = 100
    sizes[1::2, 1::2] = 100

    facecolors = np.full(z.shape, "C0")
    facecolors[:5, :5] = "C1"
    facecolors[6:, :4] = "C2"
    facecolors[6:, 6:] = "C3"

    edgecolors = np.full(z.shape, "C4")
    edgecolors[1:5, 1:5] = "C5"
    edgecolors[5:9, 1:5] = "C6"
    edgecolors[5:9, 5:9] = "C7"

    linewidths = np.full(z.shape, 2)
    linewidths[0::2, 0::2] = 5
    linewidths[1::2, 1::2] = 5

    x, y, z, sizes, facecolors, edgecolors, linewidths = (
        a.flatten() for a in [x, y, z, sizes, facecolors, edgecolors, linewidths]
    )
    ax3d(fig).scatter(x, y, z, s=sizes, fc=facecolors, ec=edgecolors,
                      lw=linewidths, alpha=1, depthshade=False)


@classic("marker_draw_order_data_reversed_m50")
def _(fig):
    two_markers(fig, ["b", "y"], -50)


@classic("marker_draw_order_data_reversed_130")
def _(fig):
    two_markers(fig, ["b", "y"], 130)


@classic("marker_draw_order_view_rotated")
def _(fig):
    two_markers(fig, ["b", "y"], 130, axis_off=True)


@classic("plot_scatter_masks")
def _(fig):
    x = np.linspace(0, 10, 100)
    y = np.linspace(0, 10, 100)
    z = np.sin(x) * np.cos(y)
    z_masked = np.ma.array(z, mask=z > 0)
    ax = ax3d(fig)
    ax.scatter(x, y, z_masked)
    ax.plot(x, y, z_masked)


# --- fill_between --------------------------------------------------------------


@mpl3d("fill_between_quad")
def _(fig):
    theta = np.linspace(0, 2 * np.pi, 50)
    x1 = np.cos(theta)
    y1 = np.sin(theta)
    z1 = 0.1 * np.sin(6 * theta)
    x2 = 0.6 * np.cos(theta)
    y2 = 0.6 * np.sin(theta)
    z2 = 2
    where = (theta < np.pi / 2) | (theta > 3 * np.pi / 2)
    ax3d(fig).fill_between(x1, y1, z1, x2, y2, z2, where=where, mode="auto",
                           alpha=0.5, edgecolor="k")


@mpl3d("fill_between_polygon")
def _(fig):
    theta = np.linspace(0, 2 * np.pi, 50)
    x1 = x2 = theta
    y1 = y2 = 0
    z1 = np.cos(theta)
    z2 = z1 + 1
    where = (theta < np.pi / 2) | (theta > 3 * np.pi / 2)
    ax3d(fig).fill_between(x1, y1, z1, x2, y2, z2, where=where, mode="auto",
                           edgecolor="k")


# --- surfaces ------------------------------------------------------------------


@mpl3d("surface3d", rc={"pcolormesh.snap": False, **AUTOMARGIN})
def _(fig):
    ax = ax3d(fig)
    X, Y, Z = sinc_surface()
    surf = ax.plot_surface(X, Y, Z, rcount=40, ccount=40, cmap="coolwarm",
                           lw=0, antialiased=False)
    ax.set_zlim(-1.01, 1.01)
    fig.colorbar(surf, shrink=0.5, aspect=5)


@mpl3d("surface3d_label_offset_tick_position", remove_text=False)
def _(fig):
    ax = ax3d(fig)
    x, y = np.mgrid[0:6 * np.pi:0.25, 0:4 * np.pi:0.25]
    z = np.sqrt(np.abs(np.cos(x) + np.cos(y)))
    ax.plot_surface(x * 1e5, y * 1e6, z * 1e8, cmap="autumn", cstride=2, rstride=2)
    ax.set_xlabel("X label")
    ax.set_ylabel("Y label")
    ax.set_zlabel("Z label")


@mpl3d("surface3d_shaded", rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    X, Y = np.meshgrid(np.arange(-5, 5, 0.25), np.arange(-5, 5, 0.25))
    Z = np.sin(np.sqrt(X ** 2 + Y ** 2))
    ax.plot_surface(X, Y, Z, rstride=5, cstride=5,
                    color=[0.25, 1, 0.25], lw=1, antialiased=False)
    ax.set_zlim(-1.01, 1.01)


@mpl3d("surface3d_masked")
def _(fig):
    ax = ax3d(fig)
    x, y = np.meshgrid(np.arange(1, 12), np.arange(1, 9))
    matrix = np.array([
        [-1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [-1, 1, 2, 3, 4, 4, 4, 3, 2, 1, 1],
        [-1, -1., 4, 5, 6, 8, 6, 5, 4, 3, -1.],
        [-1, -1., 7, 8, 11, 12, 11, 8, 7, -1., -1.],
        [-1, -1., 8, 9, 10, 16, 10, 9, 10, 7, -1.],
        [-1, -1., -1., 12, 16, 20, 16, 12, 11, -1., -1.],
        [-1, -1., -1., -1., 22, 24, 22, 20, 18, -1., -1.],
        [-1, -1., -1., -1., -1., 28, 26, 25, -1., -1., -1.],
    ])
    z = np.ma.masked_less(matrix, 0)
    norm = mcolors.Normalize(vmax=z.max(), vmin=z.min())
    colors = mpl.colormaps["plasma"](norm(z))
    ax.plot_surface(x, y, z, facecolors=colors)
    ax.view_init(30, -80, 0)


@classic("plot_surface_None_arg")
def _(fig):
    x, y = np.meshgrid(np.arange(5), np.arange(5))
    ax3d(fig).plot_surface(x, y, x + y, facecolors=None)


@mpl3d("surface3d_masked_strides")
def _(fig):
    ax = ax3d(fig)
    x, y = np.mgrid[-6:6.1:1, -6:6.1:1]
    z = np.ma.masked_less(x * y, 2)
    ax.plot_surface(x, y, z, rstride=4, cstride=4)
    ax.view_init(60, -45, 0)


@mpl3d("trisurf3d")
def _(fig):
    ax3d(fig).plot_trisurf(*disc_points(), cmap="jet", linewidth=0.2)


@mpl3d("trisurf3d_shaded")
def _(fig):
    ax3d(fig).plot_trisurf(*disc_points(), color=[1, 0.5, 0], linewidth=0.2)


@mpl3d("wireframe3d")
def _(fig):
    ax3d(fig).plot_wireframe(*axes3d.get_test_data(0.05), rcount=13, ccount=13)


@mpl3d("wireframe3dasymmetric")
def _(fig):
    X, Y, Z = axes3d.get_test_data(0.05)
    ax3d(fig).plot_wireframe(X[:-1], Y[:-1], Z[:-1], rcount=3, ccount=13)


@mpl3d("wireframe3dzerocstride")
def _(fig):
    ax3d(fig).plot_wireframe(*axes3d.get_test_data(0.05), rcount=13, ccount=0)


@mpl3d("wireframe3dzerorstride")
def _(fig):
    ax3d(fig).plot_wireframe(*axes3d.get_test_data(0.05), rstride=0, cstride=10)


# --- text --------------------------------------------------------------------


TEXT_DIRS = (None, "x", "y", "z", (1, 1, 0), (1, 1, 1))
TEXT_XS = (2, 6, 4, 9, 7, 2)
TEXT_YS = (6, 4, 8, 7, 2, 2)
TEXT_ZS = (4, 2, 5, 6, 1, 7)


@mpl3d("text3d", remove_text=False, rc=AUTOMARGIN)
def _(fig):
    ax = ax3d(fig)
    for zdir, x, y, z in zip(TEXT_DIRS, TEXT_XS, TEXT_YS, TEXT_ZS):
        ax.text(x, y, z, "(%d, %d, %d), dir=%s" % (x, y, z, zdir), zdir)
    ax.text(1, 1, 1, "red", color="red")
    ax.text2D(0.05, 0.95, "2D Text", transform=ax.transAxes)
    ax.set_xlim3d(0, 10)
    ax.set_ylim3d(0, 10)
    ax.set_zlim3d(0, 10)
    ax.set_xlabel("X axis")
    ax.set_ylabel("Y axis")
    ax.set_zlabel("Z axis")


@classic("text3d_modification")
def _(fig):
    ax = ax3d(fig)
    ax.set_xlim3d(0, 10)
    ax.set_ylim3d(0, 10)
    ax.set_zlim3d(0, 10)
    for zdir, x, y, z in zip(TEXT_DIRS, TEXT_XS, TEXT_YS, TEXT_ZS):
        t = ax.text(0, 0, 0, f"({x}, {y}, {z}), dir={zdir}")
        t.set_position_3d((x, y, z), zdir=zdir)
