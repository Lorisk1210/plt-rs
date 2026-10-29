"""matplotlib's own image tests from ``mplot3d/tests/test_axes3d.py`` (from
``test_quiver3d`` to the end of the module) and
``mplot3d/tests/test_legend3d.py``: every ``@image_comparison`` (and
``mpl3d_image_comparison``) test and the ``fig_test`` side of every
``@check_figures_equal`` test.

Each case keeps the test's style. ``mpl3d_image_comparison`` and most of the
tests ask for ``mpl20``/``default``, which is the reference configuration;
``check_figures_equal`` tests run in the test suite's default ``classic``
plus ``_classic_test_patch`` (8x6 in figures saved at classic's 100 dpi).
``remove_text=True`` (the default of ``mpl3d_image_comparison``) becomes
``remove_ticks_and_titles`` at the end of the build, and the tests' own
rcParams changes become the case's ``rc``; ``axes3d.automargin``, which some
tests switch on once the axes exist, is on for the whole case: matplotlib
reads it only when limits are set explicitly and when ticks are drawn.

Deviations from the tests:

* ``np.random`` draws come from ``spec_support.rng``, seeded with the
  test's seed.
* ``test_inverted`` also contributes its ``fig_ref`` side (``_ref``), which
  inverts the axis before plotting.

Not ported: ``test_patch_collection_modification``. Its ``Patch3DCollection``
of circles without offsets gets the projected origin as offsets in display
units, and keeps only the first of its face colours: ``set_3d_properties``
leaves ``_z_markers_idx`` at ``slice(-1)``, which drops the last colour of
every colour array longer than one. pltrs has no ``Patch3DCollection``.
"""

import itertools

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import patches as mpatch
from matplotlib.collections import LineCollection
from matplotlib.path import Path
from matplotlib.testing.decorators import remove_ticks_and_titles
from mpl_toolkits.mplot3d import art3d, proj3d

from spec_support import case, rng

GROUP = "mpl_3d_b"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: What the tests set with "Remove when image is regenerated".
AUTOMARGIN = {"axes3d.automargin": True}


def classic(name, **options):
    """A ``check_figures_equal`` case: the test suite's default style on an
    8x6 in figure."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), rc=CLASSIC, **options)


def mpl20(name, **options):
    """A case in the ``mpl20``/``default`` style (the reference configuration)."""
    return case(f"{GROUP}/{name}", **options)


def without_text(build):
    """``remove_text=True``: no titles, no tick labels."""
    def wrapped(fig):
        build(fig)
        remove_ticks_and_titles(fig)
    return wrapped


def _ax3d(fig, *args, **kwargs):
    return fig.add_subplot(*args, projection="3d", **kwargs)


# --- quiver ------------------------------------------------------------------


@mpl20("quiver3d")
@without_text
def _(fig):
    ax = _ax3d(fig)
    pivots = ["tip", "middle", "tail"]
    colors = ["tab:blue", "tab:orange", "tab:green"]
    for i, (pivot, color) in enumerate(zip(pivots, colors)):
        x, y, z = np.meshgrid([-0.5, 0.5], [-0.5, 0.5], [-0.5, 0.5])
        u = -x
        v = -y
        w = -z
        z += 2 * i
        ax.quiver(x, y, z, u, v, w, length=1, pivot=pivot, color=color)
        ax.scatter(x, y, z, color=color)
    ax.set_xlim(-3, 3)
    ax.set_ylim(-3, 3)
    ax.set_zlim(-1, 5)


@classic("quiver3d_empty")
def _(fig):
    x = y = z = u = v = w = []
    ax = _ax3d(fig)
    ax.quiver(x, y, z, u, v, w, length=0.1, pivot="tip", normalize=True)


@mpl20("quiver3d_masked")
@without_text
def _(fig):
    ax = _ax3d(fig)
    x, y, z = np.mgrid[-1:0.8:10j, -1:0.8:10j, -1:0.6:3j]
    u = np.sin(np.pi * x) * np.cos(np.pi * y) * np.cos(np.pi * z)
    v = -np.cos(np.pi * x) * np.sin(np.pi * y) * np.cos(np.pi * z)
    w = (2 / 3)**0.5 * np.cos(np.pi * x) * np.cos(np.pi * y) * np.sin(np.pi * z)
    u = np.ma.masked_where((-0.4 < x) & (x < 0.1), u, copy=False)
    v = np.ma.masked_where((0.1 < y) & (y < 0.7), v, copy=False)
    ax.quiver(x, y, z, u, v, w, length=0.1, pivot="tip", normalize=True)


@mpl20("quiver3d_colorcoded")
@without_text
def _(fig):
    ax = _ax3d(fig)
    x = y = dx = dz = np.zeros(10)
    z = dy = np.arange(10.)
    color = matplotlib.colormaps["Reds"](dy / dy.max())
    ax.quiver(x, y, z, dx, dy, dz, colors=color)
    ax.set_ylim(0, 10)


@classic("quiver3D_smoke")
def _(fig):
    x, y, z = np.meshgrid(np.arange(-0.8, 1, 0.2), np.arange(-0.8, 1, 0.2),
                          np.arange(-0.8, 1, 0.8))
    u = v = w = np.ones_like(x)
    _ax3d(fig).quiver(x, y, z, u, v, w, length=1.0, pivot="middle")


# --- collections -------------------------------------------------------------


def _two_triangles():
    """The two triangles of the ``poly3dcollection`` tests."""
    return (np.array([[0, 0, 1], [0, 1, 1], [0, 0, 0]], float),
            np.array([[0, 1, 1], [1, 1, 1], [1, 1, 0]], float))


@mpl20("poly3dcollection_closed")
@without_text
def _(fig):
    ax = _ax3d(fig)
    poly1, poly2 = _two_triangles()
    c1 = art3d.Poly3DCollection([poly1], linewidths=3, edgecolor="k",
                                facecolor=(0.5, 0.5, 1, 0.5), closed=True)
    c2 = art3d.Poly3DCollection([poly2], linewidths=3, edgecolor="k",
                                facecolor=(1, 0.5, 0.5, 0.5), closed=False)
    ax.add_collection3d(c1, autolim=False)
    ax.add_collection3d(c2, autolim=False)


@mpl20("poly3dcollection_alpha")
@without_text
def _(fig):
    ax = _ax3d(fig)
    poly1, poly2 = _two_triangles()
    c1 = art3d.Poly3DCollection([poly1], linewidths=3, edgecolor="k",
                                facecolor=(0.5, 0.5, 1), closed=True)
    c1.set_alpha(0.5)
    c2 = art3d.Poly3DCollection([poly2], linewidths=3, closed=False)
    c2.set_facecolor((1, 0.5, 0.5))
    c2.set_edgecolor("k")
    c2.set_alpha(0.5)
    ax.add_collection3d(c1, autolim=False)
    ax.add_collection3d(c2, autolim=False)


def _segments(points):
    """Consecutive pairs of ``points`` as line segments."""
    points = points.reshape(-1, 1, points.shape[-1])
    return np.concatenate([points[:-1], points[1:]], axis=1)


def _twilight_lines(segments, values):
    """The 2-D line collection of the ``add_collection3d_zs`` tests."""
    lc = LineCollection(segments, cmap="twilight", norm=matplotlib.colors.Normalize(0, 2 * np.pi))
    lc.set_array(values)
    return lc


@mpl20("add_collection3d_zs_array", rc=AUTOMARGIN)
@without_text
def _(fig):
    theta = np.linspace(-4 * np.pi, 4 * np.pi, 100)
    z = np.linspace(-2, 2, 100)
    r = z**2 + 1
    x = r * np.sin(theta)
    y = r * np.cos(theta)
    segments = _segments(np.column_stack([x, y, z]))
    ax = _ax3d(fig)
    lc = _twilight_lines(segments[:, :, :2], np.mod(theta, 2 * np.pi))
    ax.add_collection3d(lc, zs=segments[:, :, 2])
    ax.set_xlim(-5, 5)
    ax.set_ylim(-4, 6)
    ax.set_zlim(-2, 2)


@mpl20("add_collection3d_zs_scalar", rc=AUTOMARGIN)
@without_text
def _(fig):
    theta = np.linspace(0, 2 * np.pi, 100)
    z = 1
    r = z**2 + 1
    x = r * np.sin(theta)
    y = r * np.cos(theta)
    segments = _segments(np.column_stack([x, y]))
    ax = _ax3d(fig)
    ax.add_collection3d(_twilight_lines(segments, theta), zs=z)
    ax.set_xlim(-5, 5)
    ax.set_ylim(-4, 6)
    ax.set_zlim(0, 2)


# --- axes --------------------------------------------------------------------


@mpl20("axes3d_labelpad")
def _(fig):
    from mpl_toolkits.mplot3d import Axes3D
    ax = fig.add_axes(Axes3D(fig))
    ax.set_xlabel("X LABEL", labelpad=10)
    ax.set_ylabel("Y LABEL")
    ax.set_zlabel("Z LABEL", labelpad=20)
    ax.yaxis.labelpad = 20
    ax.zaxis.labelpad = -40
    for i, tick in enumerate(ax.yaxis.get_major_ticks()):
        tick.set_pad(tick.get_pad() + 5 - i * 5)


@mpl20("axes3d_cla")
def _(fig):
    ax = fig.add_subplot(1, 1, 1, projection="3d")
    ax.set_axis_off()
    ax.cla()


@mpl20("axes3d_rotated")
def _(fig):
    ax = fig.add_subplot(1, 1, 1, projection="3d")
    ax.view_init(90, 45, 0)


def _cube_corners(scale):
    """The corners of the projected cube of the ``proj_axes_cube`` tests."""
    xs = np.array([0, 1, 1, 0, 0, 0, 1, 1, 0, 0]) * scale
    ys = np.array([0, 0, 1, 1, 0, 0, 0, 1, 1, 0]) * scale
    zs = np.array([0, 0, 0, 0, 0, 1, 1, 1, 1, 1]) * scale
    return xs, ys, zs


def _view_matrix(eye, target):
    """``_view_transformation_uvw`` looking from ``eye`` at ``target``."""
    eye = np.array(eye)
    u, v, w = proj3d._view_axes(eye, np.array(target), np.array([0, 0, 1]), 0)
    return proj3d._view_transformation_uvw(u, v, w, eye)


def _proj_draw_axes(fig, M, s):
    """``_test_proj_draw_axes``: the projected unit axes, labelled."""
    txs, tys, tzs = proj3d.proj_transform([0, s, 0, 0], [0, 0, s, 0], [0, 0, 0, s], M)
    o, ax, ay, az = zip(txs, tys)
    lines = [(o, ax), (o, ay), (o, az)]
    ax = fig.subplots()
    ax.add_collection(LineCollection(lines), autolim="_datalim_only")
    for x, y, t in zip(txs, tys, ["o", "x", "y", "z"]):
        ax.text(x, y, t)
    return ax


def _label_corners(ax, txs, tys):
    ax.plot(txs, tys, c="r")
    for x, y, t in zip(txs, tys, "0 1 2 3 0 4 5 6 7 4".split()):
        ax.text(x, y, t)


@mpl20("proj_axes_cube", rc=AUTOMARGIN)
@without_text
def _(fig):
    M = np.dot(proj3d._persp_transformation(100, -100, 1),
               _view_matrix([1000, -1000, 2000], [100, 100, 100]))
    txs, tys, tzs = proj3d.proj_transform(*_cube_corners(300.0), M)
    ax = _proj_draw_axes(fig, M, s=400)
    ax.scatter(txs, tys, c=tzs)
    _label_corners(ax, txs, tys)
    ax.set_xlim(-0.2, 0.2)
    ax.set_ylim(-0.2, 0.2)


@mpl20("proj_axes_cube_ortho", rc=AUTOMARGIN)
@without_text
def _(fig):
    M = np.dot(proj3d._ortho_transformation(-1, 1), _view_matrix([200, 100, 100], [0, 0, 0]))
    txs, tys, tzs = proj3d.proj_transform(*_cube_corners(100), M)
    ax = _proj_draw_axes(fig, M, s=150)
    ax.scatter(txs, tys, s=300 - tzs)
    _label_corners(ax, txs, tys)
    ax.set_xlim(-200, 200)
    ax.set_ylim(-200, 200)


@classic("culling")
def _(fig):
    ax = _ax3d(fig)
    xs = np.linspace(0, -100, 101)
    ax.plot(xs, np.ones(101), np.zeros(101), "k")
    ax.set(xlim=(-5, 5), ylim=(-5, 5), zlim=(-5, 5))
    ax.view_init(5, 180, 0)


@mpl20("axes3d_focal_length")
def _(fig):
    axs = fig.subplots(1, 2, subplot_kw={"projection": "3d"})
    axs[0].set_proj_type("persp", focal_length=np.inf)
    axs[1].set_proj_type("persp", focal_length=0.15)


@mpl20("axes3d_ortho")
def _(fig):
    _ax3d(fig).set_proj_type("ortho")


def _plot_cube_edges(ax, r, **kwargs):
    """Every edge of the cube ``r³``."""
    for s, e in itertools.combinations(np.array(list(itertools.product(r, r, r))), 2):
        if np.sum(np.abs(s - e)) == r[1] - r[0]:
            ax.plot3D(*zip(s, e), **kwargs)


@mpl20("axes3d_isometric")
@without_text
def _(fig):
    ax = fig.subplots(subplot_kw=dict(projection="3d", proj_type="ortho", box_aspect=(4, 4, 4)))
    _plot_cube_edges(ax, (-1, 1), c="k")
    ax.view_init(elev=np.degrees(np.arctan(1. / np.sqrt(2))), azim=-45, roll=0)
    ax.grid(True)


@classic("axlim_clip")
def _(fig):
    ax = _ax3d(fig)
    x = np.linspace(0, 1, 11)
    y = np.linspace(0, 1, 11)
    X, Y = np.meshgrid(x, y)
    Z = X + Y
    ax.plot_surface(X, Y, Z, facecolor="C1", edgecolors=None, rcount=50, ccount=50,
                    axlim_clip=True)
    ax.plot([0.5, 0.5], [0, 1], [0.5, 1.5], color="k", linewidth=3, zorder=5, axlim_clip=True)
    ax.scatter(X.ravel(), Y.ravel(), Z.ravel() + 1, axlim_clip=True)
    ax.quiver(X.ravel(), Y.ravel(), Z.ravel() + 2, 0 * X.ravel(), 0 * Y.ravel(),
              0 * Z.ravel() + 1, arrow_length_ratio=0, axlim_clip=True)
    ax.plot(X[0], Y[0], Z[0] + 3, color="C2", axlim_clip=True)
    ax.text(1.1, 0.5, 4, "test", axlim_clip=True)
    ax.set(xlim=(0, 0.5), ylim=(0, 1), zlim=(0, 5))


def _inverted_line(ax):
    ax.plot([1, 1, 10, 10], [1, 10, 10, 10], [1, 1, 1, 10])


@classic("inverted")
def _(fig):
    ax = _ax3d(fig)
    _inverted_line(ax)
    ax.invert_yaxis()


@classic("inverted_ref")
def _(fig):
    ax = _ax3d(fig)
    ax.invert_yaxis()
    _inverted_line(ax)


@classic("ticklabel_format")
def _(fig):
    axs = fig.subplots(4, 5, subplot_kw={"projection": "3d"})
    for ax in axs.flat:
        ax.set_xlim(1e7, 1e7 + 10)
    for row, name in zip(axs, ["x", "y", "z", "both"]):
        row[0].ticklabel_format(axis=name, style="plain")
        row[1].ticklabel_format(axis=name, scilimits=(-2, 2))
        row[2].ticklabel_format(axis=name, useOffset=not matplotlib.rcParams["axes.formatter.useoffset"])
        row[3].ticklabel_format(axis=name, useLocale=not matplotlib.rcParams["axes.formatter.use_locale"])
        row[4].ticklabel_format(axis=name,
                                useMathText=not matplotlib.rcParams["axes.formatter.use_mathtext"])


@mpl20("minor_ticks")
def _(fig):
    ax = _ax3d(fig)
    ax.set_xticks([0.25], minor=True)
    ax.set_xticklabels(["quarter"], minor=True)
    ax.set_yticks([0.33], minor=True)
    ax.set_yticklabels(["third"], minor=True)
    ax.set_zticks([0.50], minor=True)
    ax.set_zticklabels(["half"], minor=True)


@mpl20("equal_box_aspect")
def _(fig):
    ax = _ax3d(fig)
    u = np.linspace(0, 2 * np.pi, 100)
    v = np.linspace(0, np.pi, 100)
    x = np.outer(np.cos(u), np.sin(v))
    y = np.outer(np.sin(u), np.sin(v))
    z = np.outer(np.ones_like(u), np.cos(v))
    ax.plot_surface(x, y, z)
    _plot_cube_edges(ax, [-1, 1], color="b")
    xyzlim = np.column_stack([ax.get_xlim3d(), ax.get_ylim3d(), ax.get_zlim3d()])
    XYZlim = [min(xyzlim[0]), max(xyzlim[1])]
    ax.set_xlim3d(XYZlim)
    ax.set_ylim3d(XYZlim)
    ax.set_zlim3d(XYZlim)
    ax.axis("off")
    ax.set_box_aspect((1, 1, 1))


@classic("shared_view")
def _(fig):
    elev, azim, roll = 5, 20, 30
    ax1 = fig.add_subplot(131, projection="3d")
    ax2 = fig.add_subplot(132, projection="3d", shareview=ax1)
    ax3 = fig.add_subplot(133, projection="3d")
    ax3.shareview(ax1)
    ax2.view_init(elev=elev, azim=azim, roll=roll, share=True)


@mpl20("computed_zorder", rc=AUTOMARGIN)
@without_text
def _(fig):
    ax1 = fig.add_subplot(221, projection="3d")
    ax2 = fig.add_subplot(222, projection="3d")
    ax2.computed_zorder = False
    corners = ((0, 0, 0), (0, 5, 0), (5, 5, 0), (5, 0, 0))
    for ax in (ax1, ax2):
        tri = art3d.Poly3DCollection([corners], facecolors="white", edgecolors="black", zorder=1)
        ax.add_collection3d(tri)
        ax.plot((2, 2), (2, 2), (0, 4), c="red", zorder=2)
        ax.scatter((3, 3), (1, 3), (1, 3), c="red", zorder=10)
        ax.set_xlim(0, 5.0)
        ax.set_ylim(0, 5.0)
        ax.set_zlim(0, 2.5)

    ax3 = fig.add_subplot(223, projection="3d")
    ax4 = fig.add_subplot(224, projection="3d")
    ax4.computed_zorder = False
    dim = 10
    X, Y = np.meshgrid((-dim, dim), (-dim, dim))
    Z = np.zeros((2, 2))
    angle = 0.5
    X2, Y2 = np.meshgrid((-dim, dim), (0, dim))
    Z2 = Y2 * angle
    X3, Y3 = np.meshgrid((-dim, dim), (-dim, 0))
    Z3 = Y3 * angle
    r = 7
    M = 1000
    th = np.linspace(0, 2 * np.pi, M)
    x, y, z = r * np.cos(th), r * np.sin(th), angle * r * np.sin(th)
    for ax in (ax3, ax4):
        ax.plot_surface(X2, Y3, Z3, color="blue", alpha=0.5, linewidth=0, zorder=-1)
        ax.plot(x[y < 0], y[y < 0], z[y < 0], lw=5, linestyle="--", color="green", zorder=0)
        ax.plot_surface(X, Y, Z, color="red", alpha=0.5, linewidth=0, zorder=1)
        ax.plot(r * np.sin(th), r * np.cos(th), np.zeros(M), lw=5, linestyle="--",
                color="black", zorder=2)
        ax.plot_surface(X2, Y2, Z2, color="blue", alpha=0.5, linewidth=0, zorder=3)
        ax.plot(x[y > 0], y[y > 0], z[y > 0], lw=5, linestyle="--", color="green", zorder=4)
        ax.view_init(elev=20, azim=-20, roll=0)
        ax.axis("off")


@mpl20("panecolor_rcparams", figsize=(1, 1),
       rc={"axes3d.xaxis.panecolor": "r", "axes3d.yaxis.panecolor": "g",
           "axes3d.zaxis.panecolor": "b"})
@without_text
def _(fig):
    _ax3d(fig)


# --- plotting ----------------------------------------------------------------


@mpl20("errorbar3d_errorevery")
@without_text
def _(fig):
    t = np.arange(0, 2 * np.pi + .1, 0.01)
    x, y, z = np.sin(t), np.cos(3 * t), np.sin(5 * t)
    ax = _ax3d(fig)
    estep = 15
    i = np.arange(t.size)
    zuplims = (i % estep == 0) & (i // estep % 3 == 0)
    zlolims = (i % estep == 0) & (i // estep % 3 == 2)
    ax.errorbar(x, y, z, 0.2, zuplims=zuplims, zlolims=zlolims, errorevery=estep)


@mpl20("errorbar3d")
@without_text
def _(fig):
    ax = _ax3d(fig)
    d = [1, 2, 3, 4, 5]
    e = [.5, .5, .5, .5, .5]
    ax.errorbar(x=d, y=d, z=d, xerr=e, yerr=e, zerr=e, capsize=3,
                zuplims=[False, True, False, True, True],
                zlolims=[True, False, False, True, False],
                yuplims=True, ecolor="purple", label="Error lines")
    ax.legend()


@mpl20("stem3d", figsize=(8, 6))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 3, subplot_kw={"projection": "3d"})
    theta = np.linspace(0, 2 * np.pi)
    x = np.cos(theta - np.pi / 2)
    y = np.sin(theta - np.pi / 2)
    z = theta
    for ax, zdir in zip(axs[0], ["x", "y", "z"]):
        ax.stem(x, y, z, orientation=zdir)
        ax.set_title(f"orientation={zdir}")
    x = np.linspace(-np.pi / 2, np.pi / 2, 20)
    y = np.ones_like(x)
    z = np.cos(x)
    for ax, zdir in zip(axs[1], ["x", "y", "z"]):
        markerline, stemlines, baseline = ax.stem(
            x, y, z, linefmt="C4-.", markerfmt="C1D", basefmt="C2", orientation=zdir)
        ax.set_title(f"orientation={zdir}")
        markerline.set(markerfacecolor="none", markeredgewidth=2)
        baseline.set_linewidth(3)


@mpl20("scalarmap_update")
def _(fig):
    x, y, z = np.array(list(itertools.product(*[np.arange(0, 5, 1)] * 3))).T
    c = x + y
    ax = fig.add_subplot(111, projection="3d")
    sc = ax.scatter(x, y, z, c=c, s=40, cmap="viridis")
    fig.canvas.draw()
    sc.changed()


@classic("text_3d")
def _(fig):
    ax = _ax3d(fig)
    ax.add_artist(art3d.Text3D(0.5, 0.5, 1, r"Foo bar $\int$"))


@classic("pathpatch_3d")
def _(fig):
    ax = _ax3d(fig)
    ax.add_artist(art3d.PathPatch3D(Path.unit_rectangle(), zs=(0, 0.5, 0.7, 1, 0), zdir="y"))


@mpl20("scatter_spiral", rc=AUTOMARGIN)
@without_text
def _(fig):
    ax = _ax3d(fig)
    th = np.linspace(0, 2 * np.pi * 6, 256)
    ax.scatter(np.sin(th), np.cos(th), th, s=(1 + th * 5), c=th ** 2)
    fig.canvas.draw()


@mpl20("arc_pathpatch")
@without_text
def _(fig):
    ax = _ax3d(fig, 1, 1, 1)
    a = mpatch.Arc((0.5, 0.5), width=0.5, height=0.9, angle=20, theta1=10, theta2=130)
    ax.add_patch(a)
    art3d.pathpatch_2d_to_3d(a, z=0, zdir="z")


@classic("mutating_input_arrays_y_and_z")
def _(fig):
    ax = fig.add_subplot(111, projection="3d")
    x = [1, 2, 3]
    y = [0.0, 0.0, 0.0]
    z = [0.0, 0.0, 0.0]
    ax.plot(x, y, z, "o-")
    y[:] = [1, 2, 3]
    z[:] = [1, 2, 3]


@mpl20("surface3d_zsort_inf", rc=AUTOMARGIN)
@without_text
def _(fig):
    ax = _ax3d(fig)
    x, y = np.mgrid[-2:2:0.1, -2:2:0.1]
    z = np.sin(x)**2 + np.cos(y)**2
    z[x.shape[0] // 2:, x.shape[1] // 2:] = np.inf
    ax.plot_surface(x, y, z, cmap="jet")
    ax.view_init(elev=45, azim=145)


# --- scales ------------------------------------------------------------------


def _make_log_data():
    """Data spanning 1 to ~1000 for log scale."""
    t = np.linspace(0, 2 * np.pi, 50)
    return 10 ** (t / 2), 10 ** (1 + np.sin(t)), 10 ** (2 * (1 + np.cos(t) / 2))


def _make_surface_log_data():
    """Grid data for surface with positive Z."""
    X, Y = np.meshgrid(np.linspace(1, 10, 20), np.linspace(1, 10, 20))
    return X, Y, X * Y


def _make_triangulation_data():
    """Data for trisurf with positive values."""
    r = rng(42)
    x = r.uniform(100, 1, 100)
    y = r.uniform(100, 1, 100)
    return x, y, x * y / 10


@mpl20("scale3d_artists_log", figsize=(16, 12))
def _(fig):
    log_kw = dict(xscale="log", yscale="log", zscale="log")
    line_data = _make_log_data()
    surf_X, surf_Y, surf_Z = _make_surface_log_data()

    ax = _ax3d(fig, 3, 4, 1)
    ax.plot(*line_data)
    ax.set(**log_kw, title="plot")

    ax = _ax3d(fig, 3, 4, 2)
    ax.plot_wireframe(surf_X, surf_Y, surf_Z, rstride=5, cstride=5)
    ax.set(**log_kw, title="wireframe")

    ax = _ax3d(fig, 3, 4, 3)
    ax.scatter(*line_data, c=line_data[2], cmap="viridis")
    ax.set(**log_kw, title="scatter")

    ax = _ax3d(fig, 3, 4, 4)
    bx, by = np.meshgrid([1, 10, 100], [1, 10, 100])
    bx, by = bx.flatten(), by.flatten()
    ax.bar3d(bx, by, np.ones_like(bx, dtype=float), bx * 0.3, by * 0.3, bx * by / 10, alpha=0.8)
    ax.set(**log_kw, title="bar3d")

    ax = _ax3d(fig, 3, 4, 5)
    ax.plot_surface(surf_X, surf_Y, surf_Z, cmap="viridis", alpha=0.8)
    ax.set(**log_kw, title="surface")

    ax = _ax3d(fig, 3, 4, 6)
    ax.plot_trisurf(*_make_triangulation_data(), cmap="viridis", alpha=0.8)
    ax.set(**log_kw, title="trisurf")

    ax = _ax3d(fig, 3, 4, 7)
    ax.contour(surf_X, surf_Y, surf_Z, levels=10)
    ax.set(**log_kw, title="contour")

    ax = _ax3d(fig, 3, 4, 8)
    ax.contourf(surf_X, surf_Y, surf_Z, levels=10, alpha=0.8)
    ax.set(**log_kw, title="contourf")

    ax = _ax3d(fig, 3, 4, 9)
    ax.stem([1, 10, 100], [1, 10, 100], [10, 100, 1000], bottom=1)
    ax.set(**log_kw, title="stem")

    ax = _ax3d(fig, 3, 4, 10)
    qxyz = np.array([1, 10, 100])
    ax.quiver(qxyz, qxyz, qxyz, qxyz * 0.5, qxyz * 0.5, qxyz * 0.5)
    ax.set(**log_kw, title="quiver")

    ax = _ax3d(fig, 3, 4, 11)
    ax.text(1, 1, 1, "Point A")
    ax.text(10, 10, 10, "Point B")
    ax.text(100, 100, 100, "Point C")
    ax.set(**log_kw, title="text", xlim=(0.5, 200), ylim=(0.5, 200), zlim=(0.5, 200))


@mpl20("scale3d_all_scales", figsize=(10, 6))
def _(fig):
    axs = fig.subplots(1, 2, subplot_kw={"projection": "3d"})
    t = np.linspace(0.1, 0.9, 30)
    x = t * 100
    y = (t - 0.5) * 20
    z = t
    axs[0].scatter(x, y, z)
    axs[0].set(xscale="log", yscale="symlog", zscale="logit",
               xlabel="log", ylabel="symlog", zlabel="logit")
    axs[1].scatter(x, y, z)
    axs[1].set_xscale("asinh")
    axs[1].set_zscale("function", functions=(lambda v: v**0.5, lambda v: v**2))
    axs[1].set(xlabel="asinh", ylabel="linear", zlabel="function")


@classic("scale3d_all_clipped")
def _(fig):
    ax = _ax3d(fig)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_zscale("log")
    ax.set(xlim=(0.1, 10), ylim=(0.1, 10), zlim=(0.1, 10))
    ax.plot([-1, -2, -3], [-4, -5, -6], [-7, -8, -9])


def _fmt_e(x, pos=None):
    """Tick labels ``e^n`` at the integer powers of e."""
    if x <= 0:
        return ""
    exp = np.log(x)
    if np.isclose(exp, round(exp)):
        return r"$e^{%d}$" % round(exp)
    return ""


@mpl20("scale3d_log_bases", figsize=(10, 8))
def _(fig):
    axs = fig.subplots(2, 2, subplot_kw={"projection": "3d"})
    x, y, z = _make_log_data()
    for ax, base, title in [(axs[0, 0], 10, "base=10"), (axs[0, 1], 2, "base=2"),
                            (axs[1, 0], np.e, "base=e")]:
        ax.scatter(x, y, z, s=10)
        ax.set_xscale("log", base=base)
        ax.set_yscale("log", base=base)
        ax.set_zscale("log", base=base)
        ax.set_title(title)
        if base == np.e:
            ax.xaxis.set_major_formatter(_fmt_e)
            ax.yaxis.set_major_formatter(_fmt_e)
            ax.zaxis.set_major_formatter(_fmt_e)
    axs[1, 1].scatter(x, y, z, s=10)
    axs[1, 1].set_xscale("log", subs=[2, 5])
    axs[1, 1].set_yscale("log", subs=[2, 5])
    axs[1, 1].set_zscale("log", subs=[2, 5])
    axs[1, 1].set_title("subs=[2,5]")


@mpl20("scale3d_symlog_params")
def _(fig):
    axs = fig.subplots(1, 2, subplot_kw={"projection": "3d"})
    t = np.linspace(-3, 3, 50)
    x = np.sinh(t) * 10
    y = t ** 3
    z = np.sign(t) * np.abs(t) ** 2
    for ax, linthresh in [(axs[0], 0.1), (axs[1], 10)]:
        ax.scatter(x, y, z, c=np.abs(z), cmap="viridis", s=10)
        ax.set_xscale("symlog", linthresh=linthresh)
        ax.set_yscale("symlog", linthresh=linthresh)
        ax.set_zscale("symlog", linthresh=linthresh)
        ax.set_title(f"linthresh={linthresh}")


# --- test_legend3d.py --------------------------------------------------------


@mpl20("legend_plot")
@without_text
def _(fig):
    ax = _ax3d(fig)
    x = np.arange(10)
    ax.plot(x, 5 - x, "o", zdir="y", label="z=1")
    ax.plot(x, x - 5, "o", zdir="y", label="z=-1")
    ax.legend()


@mpl20("legend_bar")
@without_text
def _(fig):
    ax = _ax3d(fig)
    x = np.arange(10)
    b1 = ax.bar(x, x, zdir="y", align="edge", color="m")
    b2 = ax.bar(x, x[::-1], zdir="x", align="edge", color="g")
    ax.legend([b1[0], b2[0]], ["up", "down"])


@mpl20("fancy")
@without_text
def _(fig):
    ax = _ax3d(fig)
    ax.plot(np.arange(10), np.full(10, 5), np.full(10, 5), "o--", label="line")
    ax.scatter(np.arange(10), np.arange(10, 0, -1), label="scatter")
    ax.errorbar(np.full(10, 5), np.arange(10), np.full(10, 10),
                xerr=0.5, zerr=0.5, label="errorbar")
    ax.legend(loc="lower left", ncols=2, title="My legend", numpoints=1)
