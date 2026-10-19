"""matplotlib's own image tests of path simplification and clipping, vector
fields, path effects, arrow patches and button and selector widgets: every
``@image_comparison`` test and the ``fig_test`` side of every
``@check_figures_equal`` test of ``tests/test_simplification.py``,
``tests/test_quiver.py``, ``tests/test_streamplot.py``,
``tests/test_patheffects.py``, ``tests/test_arrow_patches.py`` and
``tests/test_widgets.py``.

Each case is named after its test; parametrized tests contribute one case
per parameter set, suffixed with the parameter. Each case keeps the test's
style. The test suite's default (also used by ``check_figures_equal``) is
``classic`` plus ``_classic_test_patch``: such figures are 8x6 in and saved
at classic's 100 dpi. Tests asking for ``mpl20``/``default`` run in the
reference configuration. A ``savefig`` dpi the test passes becomes the
case's dpi, and ``remove_text=True`` becomes ``remove_ticks_and_titles`` at
the end of the build.

Deviations from the tests:

* Their assertions are dropped (``test_fft_peaks`` counts simplified
  vertices, ``test_maxlength`` checks the autoscaled limits).
* ``test_tickedstroke`` runs under the ``text_placeholders`` fixture, which
  draws text as boxes; the case draws its legend labels as text.
"""

import matplotlib.style
import numpy as np
from matplotlib import patches as mpatches
from matplotlib import patheffects as path_effects
from matplotlib import widgets
from matplotlib.path import Path
from matplotlib.testing.decorators import remove_ticks_and_titles
from matplotlib.ticker import MultipleLocator

from spec_support import case

GROUP = "mpl_rendering"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def image_test(name, *, classic=False, figsize=None, dpi=100, remove_text=False):
    """Register ``build`` as ``mpl_rendering/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, dpi=dpi,
             rc=CLASSIC if classic else {})(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


# --- test_simplification.py ----------------------------------------------


@classic("clipping", remove_text=True)
def _(fig):
    t = np.arange(0.0, 2.0, 0.01)
    s = np.sin(2 * np.pi * t)
    ax = fig.subplots()
    ax.plot(t, s, linewidth=1.0)
    ax.set_ylim(-0.20, -0.28)


@classic("overflow", remove_text=True)
def _(fig):
    x = np.array([1.0, 2.0, 3.0, 2.0e5])
    y = np.arange(len(x))
    ax = fig.subplots()
    ax.plot(x, y)
    ax.set_xlim(2, 6)


@classic("diamond", remove_text=True)
def _(fig):
    x = np.array([0.0, 1.0, 0.0, -1.0, 0.0])
    y = np.array([1.0, 0.0, -1.0, 0.0, 1.0])
    ax = fig.subplots()
    ax.plot(x, y)
    ax.set_xlim(-0.6, 0.6)
    ax.set_ylim(-0.6, 0.6)


@classic("simplify_curve", remove_text=True)
def _(fig):
    pp1 = mpatches.PathPatch(
        Path([(0, 0), (1, 0), (1, 1), (np.nan, 1), (0, 0), (2, 0), (2, 2), (0, 0)],
             [Path.MOVETO, Path.CURVE3, Path.CURVE3, Path.CURVE3, Path.CURVE3,
              Path.CURVE3, Path.CURVE3, Path.CLOSEPOLY]),
        fc="none")
    ax = fig.subplots()
    ax.add_patch(pp1)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 2)


M, L, C3, C4, CL = Path.MOVETO, Path.LINETO, Path.CURVE3, Path.CURVE4, Path.CLOSEPOLY

#: The paths of ``test_closed_path_nan_removal``'s test figure, by subplot.
NAN_REMOVAL_PATHS = [
    [
        # NaN on the first point also removes the last point (closed path).
        ([[-3, np.nan], [3, -3], [3, 3], [-3, 3], [-3, -3]], [M, L, L, L, CL]),
        # NaN on the second-last point should not re-close.
        ([[-2, -2], [2, -2], [2, 2], [-2, np.nan], [-2, -2]], [M, L, L, L, CL]),
    ],
    [
        # Both loops in a single path.
        ([[-3, np.nan], [3, -3], [3, 3], [-3, 3], [-3, -3],
          [-2, -2], [2, -2], [2, 2], [-2, np.nan], [-2, -2]],
         [M, L, L, L, CL, M, L, L, L, CL]),
    ],
    [
        # NaN in the first point of a CURVE3 hides the curve.
        ([[-1, -1], [1, -1], [1, np.nan], [0, 1], [-1, 1], [-1, -1]], [M, L, C3, C3, L, CL]),
        # NaN in its second point also hides the next segment.
        ([[-3, -3], [3, -3], [3, 0], [0, np.nan], [-3, 3], [-3, -3]], [M, L, C3, C3, L, L]),
    ],
    [
        # The same for the three points of a CURVE4.
        ([[-1, -1], [1, -1], [1, np.nan], [0, 0], [0, 1], [-1, 1], [-1, -1]],
         [M, L, C4, C4, C4, L, CL]),
        ([[-2, -2], [2, -2], [2, 0], [0, np.nan], [0, 2], [-2, 2], [-2, -2]],
         [M, L, C4, C4, C4, L, L]),
        ([[-3, -3], [3, -3], [3, 0], [0, 0], [0, np.nan], [-3, 3], [-3, -3]],
         [M, L, C4, C4, C4, L, L]),
    ],
]


@classic("closed_path_nan_removal")
def _(fig):
    for ax, paths in zip(fig.subplots(2, 2).flatten(), NAN_REMOVAL_PATHS):
        for vertices, codes in paths:
            ax.add_patch(mpatches.PathPatch(Path(vertices, codes), facecolor="none"))
        ax.set(xlim=(-3.5, 3.5), ylim=(-3.5, 3.5))
    remove_ticks_and_titles(fig)


@classic("closed_path_clipping", figsize=(5, 5))
def _(fig):
    vertices = []
    for roll in range(8):
        offset = 0.1 * roll + 0.1
        # A U-like pattern: the outer square with a notch in the top.
        pattern = [
            [-0.5, 1.5], [-0.5, -0.5], [1.5, -0.5], [1.5, 1.5],
            [1 - offset / 2, 1.5], [1 - offset / 2, offset],
            [offset / 2, offset], [offset / 2, 1.5],
        ]
        pattern = np.roll(pattern, roll, axis=0)
        vertices.append(np.concatenate((pattern, pattern[:1, :])))
    codes = np.full(len(vertices[0]), Path.LINETO)
    codes[0] = Path.MOVETO
    codes[-1] = Path.CLOSEPOLY
    codes = np.tile(codes, len(vertices))
    fig.add_artist(mpatches.PathPatch(Path(np.concatenate(vertices), codes),
                                      facecolor="none"))


@classic("hatch", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.add_patch(mpatches.Rectangle((0, 0), 1, 1, fill=False, hatch="/"))
    ax.set_xlim(0.45, 0.55)
    ax.set_ylim(0.45, 0.55)


@classic("fft_peaks", remove_text=True)
def _(fig):
    ax = fig.subplots()
    t = np.arange(65536)
    ax.plot(abs(np.fft.fft(np.sin(2 * np.pi * .01 * t) * np.blackman(len(t)))))


@classic("clipper", figsize=(2, 1), remove_text=True)
def _(fig):
    dat = (0, 1, 0, 2, 0, 3, 0, 4, 0, 5)
    fig.subplots_adjust(left=0, bottom=0, wspace=0, hspace=0)
    ax = fig.add_axes((0, 0, 1.0, 1.0), ylim=(0, 5), autoscale_on=False)
    ax.plot(dat)
    ax.xaxis.set_major_locator(MultipleLocator(1))
    ax.yaxis.set_major_locator(MultipleLocator(1))
    ax.xaxis.set_ticks_position("bottom")
    ax.yaxis.set_ticks_position("left")
    ax.set_xlim(5, 9)


@classic("para_equal_perp", remove_text=True)
def _(fig):
    x = np.array([0, 1, 2, 1, 0, -1, 0, 1] + [1] * 128)
    y = np.array([1, 1, 2, 1, 0, -1, 0, 0] + [0] * 128)
    ax = fig.subplots()
    ax.plot(x + 1, y + 1)
    ax.plot(x + 1, y + 1, "ro")


@image_test("clipping_with_nans")
def _(fig):
    x = np.linspace(0, 3.14 * 2, 3000)
    y = np.sin(x)
    x[::100] = np.nan
    ax = fig.subplots()
    ax.plot(x, y)
    ax.set_ylim(-0.25, 0.25)


# --- test_quiver.py ------------------------------------------------------


def _draw_quiver(ax, **kwargs):
    """The test module's ``draw_quiver``: cos/sin arrows on a 7x7 grid."""
    X, Y = np.meshgrid(np.arange(0, 2 * np.pi, 1), np.arange(0, 2 * np.pi, 1))
    return ax.quiver(np.cos(X), np.sin(Y), **kwargs)


@image_test("quiver_animate")
def _(fig):
    ax = fig.subplots()
    Q = _draw_quiver(ax, animated=True)
    ax.quiverkey(Q, 0.5, 0.88, 2, r"$2 \frac{m}{s}$",
                 labelpos="W", fontproperties={"weight": "bold"})


@image_test("quiver_with_key")
def _(fig):
    ax = fig.subplots()
    ax.margins(0.1)
    Q = _draw_quiver(ax)
    ax.quiverkey(Q, 0.5, 0.95, 2, r"$2\, \mathrm{m}\, \mathrm{s}^{-1}$",
                 angle=-10, coordinates="figure", labelpos="W",
                 fontproperties={"weight": "bold", "size": "large"})


@classic("quiver_single", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.margins(0.1)
    ax.quiver([1], [1], [2], [2])


@image_test("quiver_key_pivot", remove_text=True)
def _(fig):
    ax = fig.subplots()
    u, v = np.mgrid[0:2 * np.pi:10j, 0:2 * np.pi:10j]
    q = ax.quiver(np.sin(u), np.cos(v))
    ax.set_xlim(-2, 11)
    ax.set_ylim(-2, 11)
    ax.quiverkey(q, 0.5, 1, 1, "N", labelpos="N")
    ax.quiverkey(q, 1, 0.5, 1, "E", labelpos="E")
    ax.quiverkey(q, 0.5, 0, 1, "S", labelpos="S")
    ax.quiverkey(q, 0, 0.5, 1, "W", labelpos="W")


@classic("quiver_key_xy", remove_text=True)
def _(fig):
    X = np.arange(8)
    Y = np.zeros(8)
    uv = np.exp(1j * X * (np.pi / 4))
    for ax, angle_str in zip(fig.subplots(2), ("uv", "xy")):
        ax.set_xlim(-1, 8)
        ax.set_ylim(-0.2, 0.2)
        q = ax.quiver(X, Y, uv.real, uv.imag, pivot="middle", units="xy", width=0.05,
                      scale=2, scale_units="xy", angles=angle_str)
        for x, angle in zip((0.2, 0.5, 0.8), (0, 45, 90)):
            ax.quiverkey(q, X=x, Y=0.8, U=1, angle=angle, label="", color="b")


def _barb_grid():
    """The barb tests' 5x5 grid and its winds ``12 * (X, Y)``."""
    x = np.linspace(-5, 5, 5)
    X, Y = np.meshgrid(x, x)
    return X, Y, 12 * X, 12 * Y


#: The barb tests' sizes.
BARB_SIZES = dict(emptybarb=0.25, spacing=0.2, height=0.3)


@classic("barbs", remove_text=True)
def _(fig):
    X, Y, U, V = _barb_grid()
    ax = fig.subplots()
    ax.barbs(X, Y, U, V, np.hypot(U, V), fill_empty=True, rounding=False,
             sizes=BARB_SIZES, cmap="viridis")


@classic("barbs_pivot", remove_text=True)
def _(fig):
    X, Y, U, V = _barb_grid()
    ax = fig.subplots()
    ax.barbs(X, Y, U, V, fill_empty=True, rounding=False, pivot=1.7, sizes=BARB_SIZES)
    ax.scatter(X, Y, s=49, c="black")


@classic("barbs_flip", remove_text=True)
def _(fig):
    X, Y, U, V = _barb_grid()
    ax = fig.subplots()
    ax.barbs(X, Y, U, V, fill_empty=True, rounding=False, pivot=1.7, sizes=BARB_SIZES,
             flip_barb=Y < 0)


@classic("quiver_xy", remove_text=True)
def _(fig):
    ax = fig.subplots(subplot_kw=dict(aspect="equal"))
    ax.quiver(0, 0, 1, 1, angles="xy", scale_units="xy", scale=1)
    ax.set_xlim(0, 1.1)
    ax.set_ylim(0, 1.1)
    ax.grid()


def _quiverkey_zorder(zorder):
    """``test_quiverkey_zorder``'s test figure: two keys given ``zorder``."""
    def build(fig):
        X, Y = np.meshgrid(np.arange(1, 6, 1), np.arange(1, 6, 1))
        ax = fig.subplots()
        q = ax.quiver(X, Y, 2, 2, pivot="middle")
        ax.set_xlim(0.5, 5.5)
        ax.set_ylim(0.5, 5.5)
        extra = {} if zorder is None else {"zorder": zorder}
        ax.quiverkey(q, 4, 4, 25, coordinates="data", label="U", color="blue", **extra)
        ax.quiverkey(q, 5.5, 2, 20, coordinates="data", label="V", color="blue", angle=90,
                     **extra)
    return build


for _zorder in (0, 2, 5, None):
    classic(f"quiverkey_zorder_{str(_zorder).lower()}")(_quiverkey_zorder(_zorder))


# --- test_streamplot.py --------------------------------------------------


def _velocity_field():
    """``u = -1 - x² + y``, ``v = 1 + x - y²`` on 200x100 points of [-3, 3]²."""
    Y, X = np.mgrid[-3:3:100j, -3:3:200j]
    return X, Y, -1 - X**2 + Y, 1 + X - Y**2


def _swirl_velocity_field():
    """A rotation turned by 0.1 rad on 200x100 points of [-3, 3]²."""
    x = np.linspace(-3., 3., 200)
    y = np.linspace(-3., 3., 100)
    X, Y = np.meshgrid(x, y)
    a = 0.1
    return x, y, np.cos(a) * (-Y) - np.sin(a) * X, np.sin(a) * (-Y) + np.cos(a) * X


@image_test("startpoints", remove_text=True)
def _(fig):
    X, Y, U, V = _velocity_field()
    start_x, start_y = np.meshgrid(np.linspace(X.min(), X.max(), 5),
                                   np.linspace(Y.min(), Y.max(), 5))
    start_points = np.column_stack([start_x.ravel(), start_y.ravel()])
    ax = fig.add_subplot()
    ax.streamplot(X, Y, U, V, start_points=start_points, num_arrows=4)
    ax.plot(start_x, start_y, "ok")


@image_test("colormap", remove_text=True)
def _(fig):
    X, Y, U, V = _velocity_field()
    stream = fig.add_subplot().streamplot(X, Y, U, V, color=U, density=0.6, linewidth=2,
                                          cmap="autumn")
    fig.colorbar(stream.lines)


@image_test("linewidth", remove_text=True)
def _(fig):
    X, Y, U, V = _velocity_field()
    speed = np.hypot(U, V)
    fig.subplots().streamplot(X, Y, U, V, density=[0.5, 1], color="k",
                              linewidth=5 * speed / speed.max(), num_arrows=2)


@image_test("masks_and_nans", remove_text=True)
def _(fig):
    X, Y, U, V = _velocity_field()
    mask = np.zeros(U.shape, dtype=bool)
    mask[40:60, 80:120] = 1
    U[:20, :40] = np.nan
    U = np.ma.array(U, mask=mask)
    with np.errstate(invalid="ignore"):
        fig.subplots().streamplot(X, Y, U, V, color=U, cmap="Blues")


def _maxlength(broken_streamlines):
    """``test_maxlength`` and ``test_maxlength_no_broken``."""
    def build(fig):
        x, y, U, V = _swirl_velocity_field()
        ax = fig.subplots()
        ax.streamplot(x, y, U, V, maxlength=10., start_points=[[0., 1.5]], linewidth=2,
                      density=2, broken_streamlines=broken_streamlines)
        ax.set(xlim=(None, 3.2555988021882305), ylim=(None, 3.078326760195413))
    return build


image_test("maxlength", remove_text=True)(_maxlength(True))
image_test("maxlength_no_broken", remove_text=True)(_maxlength(False))


@image_test("direction", remove_text=True)
def _(fig):
    x, y, U, V = _swirl_velocity_field()
    fig.add_subplot().streamplot(x, y, U, V, integration_direction="backward",
                                 maxlength=1.5, start_points=[[1.5, 0.]], linewidth=2,
                                 density=2)


@image_test("integration_options", figsize=(6, 14))
def _(fig):
    # Linear potential flow over a lifting cylinder.
    n = 50
    x, y = np.meshgrid(np.linspace(-2, 2, n), np.linspace(-3, 3, n))
    th = np.arctan2(y, x)
    r = np.sqrt(x**2 + y**2)
    vr = -np.cos(th) / r**2
    vt = -np.sin(th) / r**2 - 1 / r
    vx = vr * np.cos(th) - vt * np.sin(th) + 1.0
    vy = vr * np.sin(th) + vt * np.cos(th)
    n_seed = 50
    seed_pts = np.column_stack((np.full(n_seed, -1.75), np.linspace(-2, 2, n_seed)))
    th_circ = np.linspace(0, 2 * np.pi, 100)
    for ax, max_val in zip(fig.subplots(3, 1), [0.05, 1, 5]):
        ax_ins = ax.inset_axes([0.0, 0.7, 0.3, 0.35])
        for ax_curr, is_inset in zip([ax, ax_ins], [False, True]):
            ax_curr.streamplot(x, y, vx, vy, start_points=seed_pts, broken_streamlines=False,
                               arrowsize=1e-10, linewidth=2 if is_inset else 0.6, color="k",
                               integration_max_step_scale=max_val,
                               integration_max_error_scale=max_val)
            ax_curr.fill(np.cos(th_circ), np.sin(th_circ), color="w", ec="k",
                         lw=6 if is_inset else 2)
            ax_curr.set_aspect("equal")
        ax_ins.set_xlim(-1.2, -0.7)
        ax_ins.set_ylim(-0.8, -0.4)
        ax_ins.set_yticks(())
        ax_ins.set_xticks(())
        ax.set_ylim(-1.5, 1.5)
        ax.axis("off")
        ax.indicate_inset_zoom(ax_ins, ec="k")
    fig.tight_layout()


# --- test_patheffects.py -------------------------------------------------


def _white_stroke(width):
    """``withStroke`` in white, ``width`` points wide."""
    return [path_effects.withStroke(linewidth=width, foreground="w")]


@image_test("patheffect1", remove_text=True)
def _(fig):
    ax1 = fig.add_subplot()
    ax1.imshow([[1, 2], [2, 3]])
    txt = ax1.annotate("test", (1., 1.), (0., 0),
                       arrowprops=dict(arrowstyle="->", connectionstyle="angle3", lw=2),
                       size=20, ha="center", path_effects=_white_stroke(3))
    txt.arrow_patch.set_path_effects([path_effects.Stroke(linewidth=5, foreground="w"),
                                      path_effects.Normal()])
    ax1.grid(True, linestyle="-", path_effects=_white_stroke(3))


@image_test("patheffect2", remove_text=True)
def _(fig):
    ax2 = fig.add_subplot()
    arr = np.arange(25).reshape((5, 5))
    ax2.imshow(arr, interpolation="nearest")
    cntr = ax2.contour(arr, colors="k")
    cntr.set(path_effects=_white_stroke(3))
    for label in ax2.clabel(cntr, fmt="%2.0f", use_clabeltext=True):
        label.set_path_effects(_white_stroke(3))


@image_test("patheffect3", figsize=(8, 6))
def _(fig):
    ax = fig.add_subplot()
    p1, = ax.plot([1, 3, 5, 4, 3], "o-b", lw=4)
    p1.set_path_effects([path_effects.SimpleLineShadow(), path_effects.Normal()])
    ax.set_title(r"testing$^{123}$",
                 path_effects=[path_effects.withStroke(linewidth=1, foreground="r")])
    leg = ax.legend([p1], [r"Line 1$^2$"], fancybox=True, loc="upper left")
    leg.legendPatch.set_path_effects([path_effects.withSimplePatchShadow()])

    text = ax.text(2, 3, "Drop test", color="white",
                   bbox={"boxstyle": "circle,pad=0.1", "color": "red"})
    pe = [path_effects.Stroke(linewidth=3.75, foreground="k"),
          path_effects.withSimplePatchShadow((6, -3), shadow_rgbFace="blue")]
    text.set_path_effects(pe)
    text.get_bbox_patch().set_path_effects(pe)

    pe = [path_effects.PathPatchEffect(offset=(4, -4), hatch="xxxx", facecolor="gray"),
          path_effects.PathPatchEffect(edgecolor="white", facecolor="black", lw=1.1)]
    t = fig.text(0.02, 0.1, "Hatch shadow", fontsize=75, weight=1000, va="center")
    t.set_path_effects(pe)


@image_test("patheffects_stroked_text", figsize=(8, 6))
def _(fig):
    text_chunks = [
        "A B C D E F G H I J K L",
        "M N O P Q R S T U V W",
        "X Y Z a b c d e f g h i j",
        "k l m n o p q r s t u v",
        "w x y z 0123456789",
        r"!@#$%^&*()-=_+[]\;'",
        ',./{}|:"<>?',
    ]
    font_size = 50
    ax = fig.add_axes((0, 0, 1, 1))
    for i, chunk in enumerate(text_chunks):
        text = ax.text(x=0.01, y=(0.9 - i * 0.13), s=chunk,
                       fontdict={"ha": "left", "va": "center", "size": font_size,
                                 "color": "white"})
        text.set_path_effects([path_effects.Stroke(linewidth=font_size / 10, foreground="black"),
                               path_effects.Normal()])
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.axis("off")


@image_test("collection")
def _(fig):
    x, y = np.meshgrid(np.linspace(0, 10, 150), np.linspace(-5, 5, 100))
    data = np.sin(x) + np.cos(y)
    ax = fig.add_subplot()
    cs = ax.contour(data)
    cs.set(path_effects=[
        path_effects.PathPatchEffect(edgecolor="black", facecolor="none", linewidth=12),
        path_effects.Stroke(linewidth=5)])
    for text in ax.clabel(cs, colors="white"):
        text.set_path_effects([path_effects.withStroke(foreground="k", linewidth=3)])
        text.set_bbox({"boxstyle": "sawtooth", "facecolor": "none", "edgecolor": "blue"})


@image_test("tickedstroke", figsize=(12, 4), remove_text=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    patch = mpatches.PathPatch(Path.unit_circle(), facecolor="none", lw=2, path_effects=[
        path_effects.withTickedStroke(angle=-90, spacing=10, length=1)])
    ax1.add_patch(patch)
    ax1.axis("equal")
    ax1.set_xlim(-2, 2)
    ax1.set_ylim(-2, 2)

    ax2.plot([0, 1], [0, 1], label="C0",
             path_effects=[path_effects.withTickedStroke(spacing=7, angle=135)])
    x = np.linspace(0.0, 1.0, 101)
    y = 0.3 * np.sin(x * 8) + 0.4
    ax2.plot(x, y, label="C1", path_effects=[path_effects.withTickedStroke()])
    ax2.legend()

    x1, x2 = np.meshgrid(np.linspace(0.001, 4.0, 101), np.linspace(0.001, 4.0, 105))
    g1 = -(3 * x1 + x2 - 5.5)
    g2 = -(x1 + 2 * x2 - 4)
    g3 = .8 + x1 ** -3 - x2
    cg1 = ax3.contour(x1, x2, g1, [0], colors=("k",))
    cg1.set(path_effects=[path_effects.withTickedStroke(angle=135)])
    cg2 = ax3.contour(x1, x2, g2, [0], colors=("r",))
    cg2.set(path_effects=[path_effects.withTickedStroke(angle=60, length=2)])
    cg3 = ax3.contour(x1, x2, g3, [0], colors=("b",))
    cg3.set(path_effects=[path_effects.withTickedStroke(spacing=7)])
    ax3.set_xlim(0, 4)
    ax3.set_ylim(0, 4)


@image_test("patheffects_spaces_and_newlines", remove_text=True)
def _(fig):
    ax = fig.add_subplot()
    text1 = ax.text(0.5, 0.75, "         ", ha="center", va="center", size=20,
                    bbox={"color": "salmon"})
    text2 = ax.text(0.5, 0.25, "\nNewline also causes problems", ha="center", va="center",
                    size=20, bbox={"color": "thistle"})
    text1.set_path_effects([path_effects.Normal()])
    text2.set_path_effects([path_effects.Normal()])


@classic("simple_line_shadow")
def _(fig):
    x = np.linspace(-5, 5, 500)
    fig.add_subplot().plot(x, np.exp(-x**2), linewidth=5, path_effects=[
        path_effects.SimpleLineShadow(offset=(0, 0), shadow_color="blue")])


# --- test_arrow_patches.py -----------------------------------------------


@classic("fancyarrow", figsize=(8, 4.5))
def _(fig):
    # 0 tests the division by zero of matplotlib issue 3930.
    r = [0.4, 0.3, 0.2, 0.1, 0]
    t = ["fancy", "simple", mpatches.ArrowStyle.Fancy()]
    axs = fig.subplots(len(t), len(r), squeeze=False, subplot_kw=dict(aspect=1))
    for i_r, r1 in enumerate(r):
        for i_t, t1 in enumerate(t):
            ax = axs[i_t, i_r]
            ax.annotate("", xy=(0.5, 0.5 + r1), xytext=(0.5, 0.5), size=30,
                        arrowprops=dict(arrowstyle=t1, fc="b", ec="k"))
            ax.tick_params(labelleft=False, labelbottom=False)


#: The font size of the box style tests' labels.
BOX_FONT_SIZE = 0.3 * 72


@image_test("boxarrow", figsize=(4 / 1.5, (10 * 1.2 + .5) / 1.5))
def _(fig):
    styles = mpatches.BoxStyle.get_styles()
    n = len(styles)
    spacing = 1.2
    figheight = n * spacing + .5
    for i, stylename in enumerate(sorted(styles)):
        fig.text(0.5, ((n - i) * spacing - 0.5) / figheight, stylename, ha="center",
                 size=BOX_FONT_SIZE, transform=fig.transFigure,
                 bbox=dict(boxstyle=stylename, fc="w", ec="k"))


@image_test("boxarrow_adjustment", figsize=(3 * 3.75 / 1.5, (6 * 1.6 + .5) / 1.5))
def _(fig):
    styles = ["larrow", "rarrow", "darrow"]
    # [head_width, head_angle] of each arrow.
    cases = [[1.5, 90], [1.5, 170], [0.75, 30], [0.5, -10], [2, -90], [2, -15]]
    spacing_horizontal = 3.75
    spacing_vertical = 1.6
    m = len(styles)
    n = len(cases)
    figwidth = m * spacing_horizontal
    figheight = n * spacing_vertical + .5
    for i, stylename in enumerate(styles):
        for j, (head_width, head_angle) in enumerate(cases):
            fig.text(((m - i) * spacing_horizontal - 1.5) / figwidth,
                     ((n - j) * spacing_vertical - 0.5) / figheight,
                     stylename, ha="center", va="center", size=BOX_FONT_SIZE,
                     transform=fig.transFigure,
                     bbox=dict(boxstyle=f"{stylename}, head_width={head_width}, "
                                        f"head_angle={head_angle}",
                               fc="w", ec="k"))


def _fancyarrow_dpi_cor(fig):
    """``__prepare_fancyarrow_dpi_cor_test``: an arrow whose head must not
    depend on the dpi."""
    ax = fig.add_subplot()
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.add_patch(mpatches.FancyArrowPatch(posA=(0.3, 0.4), posB=(0.8, 0.6), lw=3,
                                          arrowstyle="->", mutation_scale=100))


classic("fancyarrow_dpi_cor_100dpi", figsize=(4, 3), dpi=100,
        remove_text=True)(_fancyarrow_dpi_cor)
classic("fancyarrow_dpi_cor_200dpi", figsize=(4, 3), dpi=200,
        remove_text=True)(_fancyarrow_dpi_cor)


@image_test("fancyarrow_dash", remove_text=True)
def _(fig):
    ax = fig.subplots()
    e = mpatches.FancyArrowPatch((0, 0), (0.5, 0.5), arrowstyle="-|>",
                                 connectionstyle="angle3,angleA=0,angleB=90",
                                 mutation_scale=10.0, linewidth=2, linestyle="dashed",
                                 color="k")
    e2 = mpatches.FancyArrowPatch((0, 0), (0.5, 0.5), arrowstyle="-|>",
                                  connectionstyle="angle3", mutation_scale=10.0,
                                  linewidth=2, linestyle="dotted", color="k")
    ax.add_patch(e)
    ax.add_patch(e2)


@image_test("arrow_styles", figsize=(8, 8), remove_text=True)
def _(fig):
    styles = mpatches.ArrowStyle.get_styles()
    n = len(styles)
    ax = fig.subplots()
    ax.set_xlim(0, 1)
    ax.set_ylim(-1, n)
    fig.subplots_adjust(left=0, right=1, bottom=0, top=1)
    for i, stylename in enumerate(sorted(styles)):
        ax.add_patch(mpatches.FancyArrowPatch((0.1 + (i % 2) * 0.05, i),
                                              (0.45 + (i % 2) * 0.05, i),
                                              arrowstyle=stylename, mutation_scale=25))
    for i, stylename in enumerate([']-[', ']-', '-[', '|-|']):
        style = stylename
        if stylename[0] != '-':
            style += ',angleA=ANGLE'
        if stylename[-1] != '-':
            style += ',angleB=ANGLE'
        for j, angle in enumerate([-30, 60]):
            ax.add_patch(mpatches.FancyArrowPatch((0.55, 2 * i + j), (0.9, 2 * i + j),
                                                  arrowstyle=style.replace('ANGLE', str(angle)),
                                                  mutation_scale=25))


@image_test("connection_styles", figsize=(6, 10), remove_text=True)
def _(fig):
    styles = mpatches.ConnectionStyle.get_styles()
    ax = fig.subplots()
    ax.set_xlim(0, 1)
    ax.set_ylim(-1, len(styles))
    for i, stylename in enumerate(sorted(styles)):
        ax.add_patch(mpatches.FancyArrowPatch((0.1, i), (0.8, i + 0.5), arrowstyle="->",
                                              connectionstyle=stylename, mutation_scale=25))


# --- test_widgets.py -----------------------------------------------------

#: The labels, sizes and colours of the styled buttons.
CHECK_LABELS = ("Check 1", "Check 2", "Check 3")
RADIO_LABELS = ("Radio 1", "Radio 2", "Radio 3")
STYLED_LABELS = {"fontsize": [8, 12, 16], "color": ["red", "green", "blue"]}
STYLED_MARKERS = {"edgecolor": ["red", "green", "blue"],
                  "facecolor": ["mistyrose", "palegreen", "lightblue"]}


@image_test("check_radio_buttons_image", remove_text=True)
def _(fig):
    # ``matplotlib.testing.widgets.get_ax``.
    ax = fig.subplots()
    ax.plot([0, 200], [0, 200])
    ax.set_aspect(1.0)
    fig.canvas.draw()

    fig.subplots_adjust(left=0.3)
    widgets.RadioButtons(fig.add_axes((0.05, 0.7, 0.2, 0.15)), RADIO_LABELS)
    widgets.CheckButtons(fig.add_axes((0.05, 0.5, 0.2, 0.15)), CHECK_LABELS,
                         (False, True, True))
    widgets.RadioButtons(fig.add_axes((0.05, 0.3, 0.2, 0.15)), RADIO_LABELS,
                         label_props=STYLED_LABELS, radio_props=STYLED_MARKERS)
    widgets.CheckButtons(fig.add_axes((0.05, 0.1, 0.2, 0.15)), CHECK_LABELS,
                         (False, True, True), label_props=STYLED_LABELS,
                         frame_props=STYLED_MARKERS,
                         check_props={"color": ["red", "green", "blue"]})


@classic("radio_buttons")
def _(fig):
    widgets.RadioButtons(fig.subplots(), ["tea", "coffee"])


#: The props of ``test_radio_buttons_props`` and ``test_check_button_props``.
PROPS_LABELS = {"color": ["red"], "fontsize": [24]}
PROPS_FRAME = {"facecolor": "green", "edgecolor": "blue", "linewidth": 2}


@classic("radio_buttons_props")
def _(fig):
    cb = widgets.RadioButtons(fig.subplots(), ["tea", "coffee"])
    cb.set_label_props(PROPS_LABELS)
    # A larger label enlarges the default marker, so the setter must too.
    cb.set_radio_props({**PROPS_FRAME, "s": (24 / 2)**2})


@image_test("radio_grid_buttons", remove_text=True)
def _(fig):
    widgets.RadioButtons(fig.add_axes((0.1, 0.05, 0.65, 0.05)),
                         ["tea", "coffee", "chocolate milk", "water", "soda", "coke"],
                         layout="horizontal", active=4)
    widgets.CheckButtons(fig.add_axes((0.1, 0.15, 0.25, 0.05 * 3)),
                         ["Chicken", "Salad", "Rice", "Sushi", "Pizza", "Fries"],
                         layout=(3, 2), actives=[True, True, False, False, False, True])
    widgets.RadioButtons(fig.add_axes((0.1, 0.35, 0.2, 0.05 * 4)),
                         ["Trinity Cream", "Cake", "Ice Cream", "Muhallebi"],
                         layout="vertical", active=3)


@classic("radio_buttons_activecolor_change")
def _(fig):
    cb = widgets.RadioButtons(fig.subplots(), ["tea", "coffee"], activecolor="red")
    cb.activecolor = "green"


@classic("check_buttons")
def _(fig):
    widgets.CheckButtons(fig.subplots(), ["tea", "coffee"], [True, True])


@classic("check_button_props")
def _(fig):
    cb = widgets.CheckButtons(fig.subplots(), ["tea", "coffee"], [True, True])
    cb.set_label_props(PROPS_LABELS)
    cb.set_frame_props({**PROPS_FRAME, "s": (24 / 2)**2})
    # The test passes the cross's facecolor as edgecolor: Collection.update
    # does not promote it like Axes.scatter does for unfilled markers.
    cb.set_check_props({"edgecolor": "red", "linewidth": 2, "s": (24 / 2)**2})


@classic("rect_visibility")
def _(fig):
    tool = widgets.RectangleSelector(fig.subplots(), props={"visible": False})
    tool.extents = (0.2, 0.8, 0.3, 0.7)


def _polygon_selector_verts_setter(draw_bounding_box):
    """``test_polygon_selector_verts_setter``'s test figure."""
    def build(fig):
        tool = widgets.PolygonSelector(fig.add_subplot(), draw_bounding_box=draw_bounding_box)
        tool.verts = [(0.1, 0.4), (0.5, 0.9), (0.3, 0.2)]
    return build


for _box in (False, True):
    classic(f"polygon_selector_verts_setter_{str(_box).lower()}")(
        _polygon_selector_verts_setter(_box))
