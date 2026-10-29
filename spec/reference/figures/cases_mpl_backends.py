"""matplotlib's own image tests of its vector backends, from
``tests/test_backend_pdf.py``, ``tests/test_backend_ps.py`` and
``tests/test_backend_svg.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test.

The tests compare PDF, PostScript and SVG documents rendered to images; the
cases render to PNG like every other case, and the cases whose point is the
vector document also have vector goldens (``VECTOR_CASES`` in
``test_vectors.py``). Case names carry their backend (``pdf_``, ``ps_``,
``svg_``) because the three test modules share test names. The six
``multi_font_*`` tests (PDF and PostScript with Type 3 and Type 42 fonts,
SVG with glyphs as paths and as text) draw the same figure and differ only
in the font type of their document: they share the case ``multi_font``,
whose vector goldens in the default font types plus the ``type42`` and
``astext`` variants (``VECTOR_VARIANTS``) cover all six.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20``/``default`` run in the reference configuration. The tests' own
rcParams changes (and ``savefig(bbox_inches="tight")``) become the case's
``rc``, and ``remove_text=True`` becomes ``remove_ticks_and_titles`` at the
end of the build.

Deviations from the tests: ``np.random`` draws come from ``spec_support.rng``
seeded with the test's seed.

Not ported: ``test_ttc_type3`` and ``test_ttc_type42`` of the PDF and PS
modules (the WenQuanYi Zen Hei font is not bundled),
``test_truetype_conversion`` (``mpltest.ttf`` from matplotlib's test data),
``test_font_heuristica``, ``test_font_dejavusans`` and
``test_font_bitstream_charter`` (usetex); ``test_transparency_tex`` is a unit
test of plt's PostScript output (``figure/postscript``).
"""

import matplotlib.style
import numpy as np
from matplotlib import collections as mcollections
from matplotlib import colors as mcolors
from matplotlib import offsetbox as moffsetbox
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib.testing import _gen_multi_font_text
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

GROUP = "mpl_backends"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: The seed of the tests' ``np.random.default_rng``.
SEED = 19680801


def image_test(name, *, classic=False, figsize=None, remove_text=False, rc=None):
    """Register ``build`` as ``mpl_backends/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, rc=params)(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


# --- shared figures ------------------------------------------------------


def _scatter_on_square(fig, x, y, c, s):
    """The PDF scatter culling tests: a scatter in a 0..10 square."""
    ax = fig.subplots()
    ax.scatter(x, y, c=c, s=s)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)


def _bold_font_output(fig):
    """The ``bold_font_output*`` tests: labels and a title of other weights."""
    ax = fig.subplots()
    ax.plot(np.arange(10), np.arange(10))
    ax.set_xlabel("nonbold-xlabel")
    ax.set_ylabel("bold-ylabel", fontweight="bold")
    ax.set_title("bold-title", fontweight=600)


def _circle_lines():
    """The rasterization tests' points on the unit circle."""
    t = np.arange(0, 100) * 2.3
    return np.cos(t), np.sin(t)


# --- the multi_font_* tests of all three modules -------------------------

MULTI_FONTS, MULTI_FONT_TEXT = _gen_multi_font_text()


@image_test("multi_font", figsize=(8, 6),
            rc={"font.family": MULTI_FONTS, "font.size": 16})
def _(fig):
    """The ``multi_font_*`` tests: text needing a chain of fallback fonts."""
    fig.text(0.5, 0.5, MULTI_FONT_TEXT,
             horizontalalignment="center", verticalalignment="center")


# --- test_backend_pdf.py -------------------------------------------------


@classic("pdf_use14corefonts",
         rc={"pdf.use14corefonts": True, "font.family": "sans-serif",
             "font.size": 8, "font.sans-serif": ["Helvetica"],
             "pdf.compression": 0})
def _(fig):
    text = '''A three-line text positioned just above a blue line
and containing some French characters and the euro symbol:
"Merci pépé pour les 10 €"'''
    ax = fig.subplots()
    ax.set_title("Test PDF backend with option use14corefonts=True")
    ax.text(0.5, 0.5, text, horizontalalignment="center",
            verticalalignment="bottom", fontsize=14)
    ax.axhline(0.5, linewidth=0.5)


@image_test("pdf_hatching_legend", figsize=(1, 2))
def _(fig):
    a = mpatches.Rectangle([0, 0], 0, 0, facecolor="green", hatch="XXXX")
    b = mpatches.Rectangle([0, 0], 0, 0, facecolor="blue", hatch="XXXX")
    fig.legend([a, b, a, b], ["", "", "", ""])


@classic("pdf_grayscale_alpha")
def _(fig):
    x, y = np.ogrid[-2:2:.1, -2:2:.1]
    dd = np.exp(-(x**2 + y**2))
    dd[dd < .1] = np.nan
    ax = fig.subplots()
    ax.imshow(dd, interpolation="none", cmap="gray_r")
    ax.set_xticks([])
    ax.set_yticks([])


@image_test("pdf_eps_savefig_when_color_is_none")
def _(fig):
    ax = fig.add_subplot()
    ax.set_axis_off()
    ax.plot(np.sin(np.linspace(-5, 5, 100)), "v", c="none")


@image_test("pdf_kerning")
def _(fig):
    s = "AVAVAVAVAVAVAVAV€AAVV"
    fig.text(0, .25, s, size=5)
    fig.text(0, .75, s, size=20)


@classic("pdf_scatter_offaxis_colored_visual")
def _(fig):
    r = rng(SEED)
    x = r.uniform(100) * 5
    y = r.uniform(100) * 5
    c = r.uniform(100)
    _scatter_on_square(fig, x, y, c, 50)


@classic("pdf_scatter_mixed_onoff_axis")
def _(fig):
    r = rng(SEED)
    n_points = 50
    x_on = r.uniform(n_points) * 5
    y_on = r.uniform(n_points) * 5
    x_off = r.uniform(n_points) * 5 + 15
    y_off = r.uniform(n_points) * 5 + 15
    c = r.uniform(2 * n_points)
    _scatter_on_square(fig, np.concatenate([x_on, x_off]),
                       np.concatenate([y_on, y_off]), c, 50)


@classic("pdf_scatter_large_markers_partial_clip")
def _(fig):
    x = np.array([-0.5, 10.5, 5])
    y = np.array([5, 5, -0.5])
    c = np.array([0.2, 0.5, 0.8])
    _scatter_on_square(fig, x, y, c, 500)


@classic("pdf_scatter_logscale")
def _(fig):
    r = rng(SEED)
    x = 10 ** (r.uniform(50) * 4)
    y = 10 ** (r.uniform(50) * 4)
    c = r.uniform(50)
    ax = fig.subplots()
    ax.scatter(x, y, c=c, s=50)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(100, 1000)
    ax.set_ylim(100, 1000)


@classic("pdf_scatter_polar")
def _(fig):
    r = rng(SEED)
    theta = r.uniform(50) * 2 * np.pi
    radii = r.uniform(50) * 3
    c = r.uniform(50)
    ax = fig.subplots(subplot_kw={"projection": "polar"})
    ax.scatter(theta, radii, c=c, s=50)
    ax.set_ylim(0, 2)


@classic("pdf_hexbin_negative_offsets")
def _(fig):
    r = rng(SEED)
    x = r.normal(10_000)
    y = r.normal(10_000)
    ax = fig.subplots()
    ax.hexbin(x, y, gridsize=25, edgecolors="none")
    ax.set_xlim(-2.0, 2.0)
    ax.set_ylim(-2.0, 2.0)
    ax.set_axis_off()


# --- test_backend_ps.py --------------------------------------------------


@classic("ps_transparency")
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    ax.plot([0, 1], color="r", alpha=0)
    ax.text(.5, .5, "foo", color="r", alpha=0)


@classic("ps_useafm", rc={"ps.useafm": True})
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    ax.axhline(.5)
    ax.text(.5, .5, "qk")


@classic("ps_colored_hatch_zero_linewidth")
def _(fig):
    ax = fig.add_subplot()
    ax.add_patch(mpatches.Ellipse((0, 0), 1, 1, hatch="/", facecolor="none",
                                  edgecolor="r", linewidth=0))
    ax.add_patch(mpatches.Ellipse((0.5, 0.5), 0.5, 0.5, hatch="+",
                                  facecolor="none", edgecolor="g",
                                  linewidth=0.2))
    ax.add_patch(mpatches.Ellipse((1, 1), 0.3, 0.8, hatch="\\",
                                  facecolor="none", edgecolor="b",
                                  linewidth=0))
    ax.set_axis_off()


@classic("ps_text_clip")
def _(fig):
    ax = fig.add_subplot()
    ax.text(0, 0, "hello", transform=fig.transFigure, clip_on=True)


@image_test("ps_type42_font_without_prep",
            rc={"ps.fonttype": 42, "mathtext.fontset": "stix"})
def _(fig):
    fig.text(0.5, 0.5, "Mass $m$")


@classic("ps_path_collection")
def _(fig):
    r = rng(SEED)
    xvals = r.uniform(10)
    yvals = r.uniform(10)
    sizes = r.uniform(10, 30, 100)
    ax = fig.subplots()
    ax.scatter(xvals, yvals, sizes, edgecolor=[0.9, 0.2, 0.1], marker="<")
    ax.set_axis_off()
    paths = [mpath.Path.unit_regular_polygon(i) for i in range(3, 7)]
    offsets = r.uniform(20, 0, 200).reshape(10, 2)
    pc = mcollections.PathCollection(paths, [0.02, 0.04], zorder=-1,
                                     facecolors="yellow", offsets=offsets)
    ax.add_collection(pc, autolim=False)
    ax.set_xlim(0, 1)


@image_test("ps_colorbar_shift", rc={"savefig.bbox": "tight"})
def _(fig):
    cmap = mcolors.ListedColormap(["r", "g", "b"])
    norm = mcolors.BoundaryNorm([-1, -0.5, 0.5, 1], cmap.N)
    points = fig.gca().scatter([0, 1], [1, 1], c=[0, 1], cmap=cmap, norm=norm)
    fig.colorbar(points)


# --- test_backend_svg.py -------------------------------------------------


@classic("svg_fill_black_with_alpha", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.scatter(x=[0, 0.1, 1], y=[0, 0, 0], c="k", alpha=0.1, s=10000)


@classic("svg_noscale", remove_text=True)
def _(fig):
    X, Y = np.meshgrid(np.arange(-5, 5, 1), np.arange(-5, 5, 1))
    Z = np.sin(Y ** 2)
    ax = fig.subplots()
    ax.imshow(Z, cmap="gray", interpolation="none")


image_test("svg_bold_font_output")(_bold_font_output)
classic("svg_bold_font_output_with_none_fonttype",
        rc={"svg.fonttype": "none"})(_bold_font_output)


@classic("svg_rasterized")
def _(fig):
    x, y = _circle_lines()
    ax = fig.subplots()
    ax.plot(x, y, "-", c="r", lw=10, rasterized=True)
    ax.plot(x + 1, y, "-", c="b", lw=10, rasterized=True)


@classic("svg_rasterized_ordering")
def _(fig):
    x, y = _circle_lines()
    ax = fig.subplots()
    ax.set_xlim(0, 3)
    ax.set_ylim(-1.1, 1.1)
    ax.plot(x, y, "-", c="r", lw=10, rasterized=True, zorder=1.1)
    ax.plot(x + 2, y, "-", c="g", lw=10, rasterized=True, zorder=1.3)
    ax.plot(x + 3, y, "-", c="m", lw=10, rasterized=True, zorder=1.4)
    ax.plot(x + 1, y, "-", c="b", lw=10, rasterized=False, zorder=1.2)


@classic("svg_prevent_rasterization")
def _(fig):
    loc = [0.05, 0.05]
    ax = fig.subplots()
    ax.plot([loc[0]], [loc[1]], marker="x", c="black", zorder=2,
            rasterized=True)
    abox = moffsetbox.AnnotationBbox(moffsetbox.TextArea("X"), loc, zorder=2.1)
    ax.add_artist(abox)

