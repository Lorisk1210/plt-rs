r"""matplotlib's own image tests of ``tests/test_text.py`` and
``tests/test_mathtext.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test (the ``fig_ref``
side too, as ``<name>_ref``, where it draws something else). Of the
parametrised mathtext tests, ``test_mathtext_rendering`` and
``test_mathfont_rendering`` contribute the expressions the ``mathtext``
group does not draw, stacked the same way (one 0.75 in row each, ten rows to
a case) in each of the tests' five fontsets (``math_tests_<n>_<fontset>``,
``font_tests_<n>_<fontset>``), and ``test_mathtext_rendering_svgastext``
its 16 pt rows with SVG text as text (``svgastext_<fontset>``); the
lightweight test is the ``mathtext`` group's ``lightweight`` case.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20`` run in the reference configuration on the test's figure size. A
``savefig.dpi`` the test sets becomes the case's dpi, and
``remove_text=True`` becomes ``remove_ticks_and_titles`` at the end of the
build.

Deviations from the tests:

* ``np.random`` draws come from ``spec_support.rng`` with the test's seed.
* ``test_annotation_units`` plots a fixed date instead of
  ``datetime.now()``.
* ``test_large_subscript_title`` is left out: the right edge of its left axes
  falls on a pixel boundary (445.5 px), and only the rounding noise of
  kiwisolver's solution (445.49999999999983) decides on which side its spine
  is snapped.
* ``test_mathtext_rendering_svgastext`` hides the figure background to
  make its SVG smaller; the cases keep it, because on a transparent canvas
  the slightest difference in glyph anti-aliasing (alpha 0 against 1) shows
  as a full colour difference in the PNG comparison.
* ``test_draw_text_as_path_fallback`` deletes ``RendererAgg.draw_text`` so
  that texts go through ``RendererBase.draw_text``, which draws them as
  paths. A ``Normal`` path effect on every text takes the same route
  (``PathEffectRenderer`` inherits that ``draw_text``) without patching the
  renderer.
"""

import datetime

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import patches as mpatches
from matplotlib import patheffects
from matplotlib.backend_bases import MouseEvent
from matplotlib.font_manager import FontProperties, findfont
from matplotlib.testing.decorators import remove_ticks_and_titles
from matplotlib.text import OffsetFrom
from matplotlib.ticker import NullLocator

from spec_support import case, rng

GROUP = "mpl_text"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}

#: The long string of the wrapping tests.
WRAPPED = ("This is a really long string that I'd rather have wrapped so that"
           " it doesn't go outside of the figure, but if it's long enough it"
           " will go off the top or bottom!")


def image_test(name, *, classic=False, figsize=None, dpi=100, remove_text=False,
               rc=None):
    """Register ``build`` as ``mpl_text/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, dpi=dpi,
             rc=params)(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


# --- test_text.py: fonts, lines and alignment ----------------------------------


@image_test("font_styles")
def _(fig):
    def find_matplotlib_font(**kw):
        path = findfont(FontProperties(**kw), directory=matplotlib.get_data_path())
        return FontProperties(fname=path)

    ax = fig.subplots()
    requests = [
        ("Normal Font", dict(family="sans-serif")),
        ("Bold Font", dict(family="Foo", weight="bold", stretch=500)),
        ("Bold Italic Font", dict(family="sans serif", style="italic",
                                  weight=750, stretch=500)),
        ("Light Font", dict(family="sans-serif", weight=200, stretch=500)),
        ("Condensed Font", dict(family="sans-serif", weight=500, stretch=100)),
    ]
    for i, (text, request) in enumerate(requests):
        font = find_matplotlib_font(variant="normal", size=14,
                                    **{"style": "normal", **request})
        ax.annotate(text, (0.1, 0.1 * (i + 1)), xycoords="axes fraction",
                    fontproperties=font)
    ax.set_xticks([])
    ax.set_yticks([])


@image_test("multiline")
def _(fig):
    ax = fig.add_subplot(1, 1, 1)
    ax.set_title("multiline\ntext alignment")
    for x, math in [(0.2, "$M$"), (0.5, "$M^{M^{M^{M}}}$"),
                    (0.8, "$M_{q_{q_{q}}}$")]:
        ax.text(x, 0.5, f"TpTpTp\n{math}\nTpTpTp", size=20, ha="center",
                va="top")
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 0.8)
    ax.set_xticks([])
    ax.set_yticks([])


@image_test("multiline2")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 1.4)
    ax.set_ylim(0, 2)
    sts = ["Line", "2 Lineg\n 2 Lg", "$\\sum_i x $", "hi $\\sum_i x $\ntest",
           "test\n $\\sum_i x $", "$\\sum_i x $\n $\\sum_i x $"]
    renderer = fig.canvas.get_renderer()
    rows = [(0.5, "bottom", 0, "Bottom align"), (1.3, "top", 0, "Top align"),
            (1.8, "baseline", 0, "Baseline align"),
            (0.1, "bottom", 20, "Bot align, rot20")]
    for y, va, rotation, label in rows:
        ax.axhline(y, color="C2", linewidth=0.3)
        for nn, st in enumerate(sts):
            tt = ax.text(0.2 * nn + 0.1, y, st, horizontalalignment="left",
                         verticalalignment=va, rotation=rotation)
            box = mpatches.Rectangle((0, 0), 1, 1, clip_on=False,
                                     transform=ax.transAxes)
            box.set_bounds(tt.get_window_extent(renderer)
                           .transformed(ax.transAxes.inverted()).bounds)
            ax.add_patch(box)
        ax.text(1.2, y, label, color="C2")


@image_test("antialiasing", figsize=(5.25, 0.75))
def _(fig):
    for x, text, antialiased in [(0.3, "antialiased", True),
                                 (0.7, "not antialiased", False)]:
        matplotlib.rcParams["text.antialiased"] = not antialiased
        for y, s in [(0.75, text), (0.25, r"$\sqrt{x}$")]:
            fig.text(x, y, s, horizontalalignment="center",
                     verticalalignment="center", antialiased=antialiased)
    matplotlib.rcParams["text.antialiased"] = False


@image_test("contains", figsize=(8, 6))
def _(fig):
    ax = fig.add_subplot()
    mevent = MouseEvent("button_press_event", fig.canvas, 0.5, 0.5, 1, None)
    xs, ys = np.meshgrid(np.linspace(0.25, 0.75, 30), np.linspace(0.25, 0.75, 30))
    txt = ax.text(0.5, 0.4, "hello world", ha="center", fontsize=30, rotation=30)
    fig.canvas.draw()
    for x, y in zip(xs.flat, ys.flat):
        mevent.x, mevent.y = ax.transAxes.transform([x, y])
        contains, _ = txt.contains(mevent)
        vl = ax.viewLim.frozen()
        ax.plot(x, y, "o", color="yellow" if contains else "red")
        ax.viewLim.set(vl)


@image_test("titles")
def _(fig):
    ax = fig.add_subplot(1, 1, 1)
    ax.set_title("left title", loc="left")
    ax.set_title("right title", loc="right")
    ax.set_xticks([])
    ax.set_yticks([])


@image_test("text_alignment")
def _(fig):
    ax = fig.add_subplot(1, 1, 1)
    x = 0.1
    for rotation in (0, 30):
        for alignment in ("top", "bottom", "baseline", "center"):
            ax.text(x, 0.5, alignment + " Tj", va=alignment, rotation=rotation,
                    bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))
            ax.text(x, 1.0, r"$\sum_{i=0}^{j}$", va=alignment, rotation=rotation)
            x += 0.1
    ax.plot([0, 1], [0.5, 0.5])
    ax.plot([0, 1], [1.0, 1.0])
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1.5)
    ax.set_xticks([])
    ax.set_yticks([])


@image_test("rotation_anchor", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], lw=0)
    ax.axvline(.5, linewidth=.5, color=".5")
    ax.axhline(.5, linewidth=.5, color=".5")
    n = 4
    for r in range(n):
        ax.text(.5, .5, "pP", color=f"C{r}", size=100, rotation=r / n * 360,
                rotation_mode="anchor", verticalalignment="center_baseline")


@image_test("axes_titles")
def _(fig):
    ax = fig.add_subplot(1, 1, 1)
    ax.set_title("center", loc="center", fontsize=20, fontweight=700)
    ax.set_title("left", loc="left", fontsize=12, fontweight=400)
    ax.set_title("right", loc="right", fontsize=12, fontweight=400)


@image_test("text_bboxclip")
def _(fig):
    ax = fig.add_subplot()
    ax.text(0.9, 0.2, "Is bbox clipped?", backgroundcolor="r", clip_on=True)
    t = ax.text(0.9, 0.5, "Is fancy bbox clipped?", clip_on=True)
    t.set_bbox({"boxstyle": "round, pad=0.1"})


# --- test_text.py: annotations and clipping ------------------------------------


def _signed_annotations(ax, unit, placements):
    """The annotations of the negative coordinate tests: for each
    ``(name, positive, negative)`` an annotation at the positive point and a
    top-aligned one at the negative point, in ``unit`` coordinates."""
    for name, positive, negative in placements:
        coords = f"{unit} {name}"
        ax.annotate(f"+ {name}", xytext=positive, textcoords=coords,
                    xy=positive, xycoords=coords, fontsize=32)
        ax.annotate(f"- {name}", xytext=negative, textcoords=coords,
                    xy=negative, xycoords=coords, fontsize=32, va="top")


@image_test("annotation_negative_ax_coords")
def _(fig):
    ax = fig.subplots()
    _signed_annotations(ax, "axes", [
        ("points", [30, 20], [30, -20]),
        ("fraction", [0.75, 0.05], [0.75, -0.05]),
        ("pixels", [160, 25], [160, -25]),
    ])


@image_test("annotation_negative_fig_coords")
def _(fig):
    ax = fig.subplots()
    _signed_annotations(ax, "figure", [
        ("points", [10, 250], [-10, 310]),
        ("fraction", [0.05, 0.5], [-0.05, 0.45]),
        ("pixels", [50, 50], [-50, 150]),
    ])


@image_test("agg_text_clip")
def _(fig):
    ax1, ax2 = fig.subplots(2)
    for x, y in rng(1).uniform(20).reshape(10, 2):
        ax1.text(x, y, "foo", clip_on=True)
        ax2.text(x, y, "foo")


@image_test("font_scaling", figsize=(6.4, 12.4), rc={"pdf.fonttype": 42})
def _(fig):
    ax = fig.subplots()
    ax.xaxis.set_major_locator(NullLocator())
    ax.yaxis.set_major_locator(NullLocator())
    ax.set_ylim(-10, 600)
    for i, fs in enumerate(range(4, 43, 2)):
        ax.text(0.1, i * 30, f"{fs} pt font size", fontsize=fs)


def _opacity(fig, texts):
    """The opacity tests: texts at ``x = 0.25`` on an axes without axis
    lines; ``texts`` holds ``(y, text, color, alpha)``."""
    ax = fig.add_subplot()
    ax.set_axis_off()
    for y, text, color, alpha in texts:
        ax.text(0.25, y, text, color=color, alpha=alpha)


@classic("text_as_path_opacity")
def _(fig):
    _opacity(fig, [(0.25, "c", (0, 0, 0, 0.5), None),
                   (0.5, "a", None, 0.5),
                   (0.75, "x", (0, 0, 0, 1), 0.5)])


@classic("text_as_text_opacity", rc={"svg.fonttype": "none"})
def _(fig):
    _opacity(fig, [(0.25, "50% using `color`", (0, 0, 0, 0.5), None),
                   (0.5, "50% using `alpha`", None, 0.5),
                   (0.75, "50% using `alpha` and 100% `color`", (0, 0, 0, 1), 0.5)])


@classic("annotation_units")
def _(fig):
    when = datetime.datetime(2026, 10, 19, 12, 30)
    ax = fig.add_subplot()
    ax.plot(when, 1, "o")
    ax.annotate("x", (when, 0.5), xycoords=("data", "axes fraction"),
                xytext=(0, 0), textcoords="offset points")


@classic("buffer_size")
def _(fig):
    ax = fig.add_subplot()
    ax.set_yticks([0, 1])
    ax.set_yticklabels(["€", "a"])
    ax.yaxis.majorTicks[1].label1.set_color("w")


@classic("annotate_and_offsetfrom_copy_input")
def _(fig):
    ax = fig.add_subplot()
    line, = ax.plot([0, 2], [0, 2])
    of_xy = np.array([.5, .5])
    ax.annotate("foo", textcoords=OffsetFrom(line, of_xy), xytext=(10, 0),
                xy=(0, 0))
    of_xy[:] = 1


@classic("annotate_and_offsetfrom_copy_input_ref")
def _(fig):
    ax = fig.add_subplot()
    line, = ax.plot([0, 2], [0, 2])
    an_xy = np.array([.5, .5])
    ax.annotate("foo", xy=an_xy, xycoords=line, xytext=(10, 0),
                textcoords="offset points")
    an_xy[:] = 2


@classic("text_antialiased_off_default_vs_manual")
def _(fig):
    fig.text(0.5, 0.5, "6 inches x 2 inches", antialiased=False)


@classic("text_antialiased_off_default_vs_manual_ref",
         rc={"text.antialiased": False})
def _(fig):
    fig.text(0.5, 0.5, "6 inches x 2 inches")


@classic("text_antialiased_on_default_vs_manual")
def _(fig):
    fig.text(0.5, 0.5, "6 inches x 2 inches", antialiased=True)


# --- test_text.py: wrapping and tick label rotation ----------------------------


@image_test("basictext_wrap", figsize=(8, 6))
def _(fig):
    ax = fig.add_subplot()
    ax.axis([0, 10, 0, 10])
    ax.text(4, 1, WRAPPED, ha="left", rotation=15, wrap=True)
    ax.text(6, 5, WRAPPED, ha="left", rotation=15, wrap=True)
    ax.text(5, 5, WRAPPED, ha="right", rotation=-15, wrap=True)
    ax.text(5, 10, WRAPPED, fontsize=18, style="oblique", ha="center",
            va="top", wrap=True)
    ax.text(3, 4, WRAPPED, family="serif", style="italic", ha="right", wrap=True)
    ax.text(-1, 0, WRAPPED, ha="left", rotation=-15, wrap=True)


@image_test("fonttext_wrap", figsize=(8, 6))
def _(fig):
    ax = fig.add_subplot()
    ax.axis([0, 10, 0, 10])
    ax.text(4, -1, WRAPPED, fontsize=18, family="serif", ha="left",
            rotation=15, wrap=True)
    ax.text(6, 5, WRAPPED, family="sans serif", ha="left", rotation=15,
            wrap=True)
    ax.text(5, 10, WRAPPED, weight="heavy", ha="center", va="top", wrap=True)
    ax.text(3, 4, WRAPPED, family="monospace", ha="right", wrap=True)
    ax.text(-1, 0, WRAPPED, fontsize=14, style="italic", ha="left",
            rotation=-15, wrap=True)


def _rotated_tick_labels(axes, axis, mode):
    """37 ``foo`` tick labels on ``axis`` ("x" or "y") of each axes, turned
    by 0, 10, ..., 360 degrees in rotation mode ``mode``."""
    angles = np.linspace(0, 360, 37)
    for ax in axes:
        getattr(ax, f"set_{axis}ticks")(range(37), ["foo"] * 37,
                                        rotation_mode=mode)
        for tick, angle in zip(getattr(ax, f"get_{axis}ticklabels")(), angles):
            tick.set_rotation(angle)


@image_test("xtick_rotation_mode", figsize=(12, 1))
def _(fig):
    ax = fig.subplots()
    ax.set_yticks([])
    _rotated_tick_labels([ax, ax.twiny()], "x", "xtick")
    fig.subplots_adjust(left=0.01, right=0.99, top=.6, bottom=.4)


@image_test("ytick_rotation_mode", figsize=(1, 12))
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    _rotated_tick_labels([ax, ax.twinx()], "y", "ytick")
    fig.subplots_adjust(left=0.4, right=0.6, top=.99, bottom=.01)


# --- test_text.py: shaping -------------------------------------------------------


def _complex_shaping(fig, **text_kw):
    """``_test_complex_shaping``: Arabic between n-ary operators, then the
    same with ``cmr10`` and ``DejaVu Sans Display`` ahead of the fallback."""
    math_signs = "\N{N-ary Product}\N{N-ary Coproduct}\N{N-ary summation}\N{Integral}"
    text = (math_signs
            + "Arabic: \N{Arabic Letter REH}\N{Arabic FATHA}\N{Arabic Letter QAF}"
              "\N{Arabic SUKUN}\N{Arabic Letter MEEM}"
            + math_signs)
    fig.text(0.5, 0.75, text, size=32, ha="center", va="center", **text_kw)
    fig.text(0.5, 0.25, text, size=32, ha="center", va="center",
             family=["cmr10", "DejaVu Sans Display", "DejaVu Sans"], **text_kw)


def _text_features(fig, **text_kw):
    """``_test_text_features``: ligatures by default, disabled, and
    discretionary."""
    fig.text(1, 0.7, "Default: fi ffi fl st", fontsize=32,
             horizontalalignment="right", **text_kw)
    fig.text(1, 0.4, "Disabled: fi ffi fl st", fontsize=32,
             horizontalalignment="right", fontfeatures=["-liga"], **text_kw)
    t = fig.text(1, 0.1, "Discretionary: fi ffi fl st", fontsize=32,
                 horizontalalignment="right", **text_kw)
    t.set_fontfeatures(["dlig"])


def _text_language(fig, **text_kw):
    """``_test_text_language``: Serbian forms of a Cyrillic letter and Sámi
    forms of a Latin one, by text, by range, and from ``text.language``."""
    for y, label in [(0.8, "Default"), (0.55, "Lang A"), (0.3, "Lang B"),
                     (0.05, "Mixed")]:
        fig.text(0, y, label, fontsize=32, **text_kw)
    cyrillic = "\U00000431"
    fig.text(0.4, 0.8, cyrillic, fontsize=32, **text_kw)
    fig.text(0.4, 0.55, cyrillic, fontsize=32, language="sr", **text_kw)
    t = fig.text(0.4, 0.3, cyrillic, fontsize=32, **text_kw)
    t.set_language("ru")
    fig.text(0.4, 0.05, cyrillic * 4, fontsize=32,
             language=[("ru", 0, 1), ("sr", 1, 2), ("ru", 2, 3), ("sr", 3, 4)],
             **text_kw)
    latin = "\U0000014a"
    fig.text(0.7, 0.8, latin, fontsize=32, **text_kw)
    with matplotlib.rc_context({"text.language": "en"}):
        fig.text(0.7, 0.55, latin, fontsize=32, **text_kw)
    fig.text(0.7, 0.3, latin, fontsize=32, language="smn", **text_kw)
    t = fig.text(0.7, 0.05, latin * 4, fontsize=32, **text_kw)
    t.set_language((("en", 0, 1), ("smn", 1, 2), ("en", 2, 3), ("smn", 3, 4)))


@image_test("complex", figsize=(6, 2))
def _(fig):
    _complex_shaping(fig)


@image_test("features", figsize=(5, 1.5))
def _(fig):
    _text_features(fig)


@image_test("language", figsize=(5, 3))
def _(fig):
    _text_language(fig)


@image_test("draw_text_fallback", figsize=(6, 6.5))
def _(fig):
    heights = [2, 1.5, 3]
    subfigs = fig.subfigures(3, 1, height_ratios=heights)
    as_paths = {"path_effects": [patheffects.Normal()]}
    _complex_shaping(subfigs[0], **as_paths)
    _text_features(subfigs[1], **as_paths)
    _text_language(subfigs[2], **as_paths)


# --- test_mathtext.py --------------------------------------------------------------


def _texts(*texts, **text_kw):
    """A case drawing ``(x, y, s)`` texts on the figure."""
    def build(fig):
        for x, y, s in texts:
            fig.text(x, y, s, **text_kw)

    return build


#: ``test_short_long_accents``: every one-character accent command and the
#: long command with the same accent, built from ``Parser._accent_map``.
classic("short_long_accents")(_texts((0, .5, r"$\"a\`a\'a\~a\.a\^a$")))
classic("short_long_accents_ref")(
    _texts((0, .5, r"$\ddot a\grave a\acute a\tilde a\dot a\hat a$")))
classic("spaces")(_texts((.5, .5, r"$1\,2\>3\ 4$")))
classic("spaces_ref")(_texts((.5, .5, r"$1\/2\:3~4$")))
classic("operator_space")(_texts(*[
    (0.1, 0.1 * (i + 1), s) for i, s in enumerate([
        r"$\log 6$", r"$\log(6)$", r"$\arcsin 6$", r"$\arcsin|6|$",
        r"$\operatorname{op} 6$", r"$\operatorname{op}[6]$", r"$\cos^2$",
        r"$\log_2$", r"$\sin^2 \max \cos$",
    ])
]))
classic("operator_space_ref")(_texts(*[
    (0.1, 0.1 * (i + 1), s) for i, s in enumerate([
        r"$\mathrm{log\,}6$", r"$\mathrm{log}(6)$", r"$\mathrm{arcsin\,}6$",
        r"$\mathrm{arcsin}|6|$", r"$\mathrm{op\,}6$", r"$\mathrm{op}[6]$",
        r"$\mathrm{cos}^2$", r"$\mathrm{log}_2$",
        r"$\mathrm{sin}^2 \mathrm{\,max} \mathrm{\,cos}$",
    ])
]))
classic("inverted_delimiters")(
    _texts((.5, .5, r"$\left)\right($"), math_fontfamily="dejavusans"))
classic("genfrac_displaystyle")(_texts((0.1, 0.1, r"$\dfrac{2x}{3y}$")))
#: The ``fig_ref`` side: the underline thickness of 12 pt text at 100 dpi
#: as the rule.
classic("genfrac_displaystyle_ref")(
    _texts((0.1, 0.1, r"$\genfrac{}{}{1.041667}{0}{2x}{3y}$")))
classic("text_nested_braces")(
    _texts((0.1, 0.2, r"$\text{{example}}$"), (0.1, 0.5, r"$\text{a{b}{{c}}d}$")))
classic("text_escaped_braces")(_texts((0.1, 0.2, r"$\text{{\{example\}}}$")))
classic("boldsymbol")(_texts((0.1, 0.2, r"$\boldsymbol{\mathrm{abc0123\alpha}}$")))
classic("mathnormal")(_texts((0.1, 0.2, r"$\mathnormal{0123456789}$")))


@classic("phantoms")
def _(fig):
    fig.text(0.5, 0.9, r"$\rlap{rlap}extra$", ha="left")
    fig.text(0.5, 0.8, r"$extra\llap{llap}$", ha="right")
    fig.text(0.5, 0.7, r"$\phantom{phantom}$")


@image_test("math_fontfamily_image", figsize=(10, 3), dpi=40)
def _(fig):
    fig.text(0.2, 0.7, r"$This\ text\ should\ have\ one\ font$", size=24,
             math_fontfamily="dejavusans")
    fig.text(0.2, 0.3, r"$This\ text\ should\ have\ another$", size=24,
             math_fontfamily="stix")


# --- test_mathtext.py: the rest of the parametrised expressions ---------------

#: The ``math_tests`` of ``test_mathtext_rendering`` the ``mathtext`` group
#: does not draw, in matplotlib's order.
MATH_TESTS = [
    r"$x\hspace{-0.2}\doteq\hspace{-0.2}y$",
    r"\$100.00 $\alpha \_$",
    r"$\frac{\$100.00}{y}$",
    r"$x   y$",
    r"$x+y\ x=y\ x<y\ x:y\ x,y\ x@y$",
    r"$100\%y\ x*y\ x/y x\$y$",
    r"$x\leftarrow y\ x\forall y\ x-y$",
    r"$x \sf x \bf x {\cal X} \rm x$",
    r"$\{ \rm braces \}$",
    r"$\left(x\right)$",
    r"$\sin(x)$",
    r"$x_2$",
    r"$x^2$",
    r"$x^2_y$",
    r"$x_y^2$",
    r"$dz/dt = \gamma x^2 + {\rm sin}(2\pi y+\phi)$",
    r"Variable $i$ is good",
    r"$\Delta_i^j$",
    r"$\Delta^j_{i+1}$",
    r"$\arccos((x^i))$",
    r"$\gamma = \frac{x=\frac{6}{8}}{y} \delta$",
    r"$f'\quad f'''(x)\quad ''/\mathrm{yr}$",
    r"$\frac{x_2888}{y}$",
    r"$\sqrt[3]{x}=5$",
    r"$\frac{X}{\frac{X}{Y}}$",
    r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2} \int^{\alpha_2}_{\alpha_2} d \alpha^\prime_2 \left[\frac{ U^{2\beta}_{\delta_1 \rho_1} - \alpha^\prime_2U^{1\beta}_{\rho_1 \sigma_2} }{U^{0\beta}_{\rho_1 \sigma_2}}\right]$",
    r"$\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega$",
    r"$\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi$",
    r"${x}^{2}{y}^{2}$",
    r"${}_{2}F_{3}$",
    r"$\frac{x+{y}^{2}}{k+1}$",
    r"$x+{y}^{\frac{2}{k+1}}$",
    r"$\frac{a}{b/2}$",
    r"${a}_{0}+\dfrac{1}{{a}_{1}+\dfrac{1}{{a}_{2}+\dfrac{1}{{a}_{3}+\dfrac{1}{{a}_{4}}}}}$",
    r"$\binom{n}{k/2}$",
    r"${x}^{2y}$",
    r"$\sum _{i=1}^{p}\sum _{j=1}^{q}\sum _{k=1}^{r}{a}_{ij}{b}_{jk}{c}_{ki}$",
    r"${2}^{{2}^{{2}^{x}}}$",
    r"${\int }_{1}^{x}\frac{\mathrm{dt}}{t}$",
    r"$\int {\int }_{D}\mathrm{dx} \mathrm{dy}$",
    r"${y}_{{x}^{2}}$",
    r"${y}_{{x}_{2}}$",
    r"${x}_{92}^{31415}+\pi $",
    r"${y}_{3}^{\prime \prime \prime }$",
    r"$\left( \xi \left( 1 - \xi \right) \right)$",
    r"$\left(2 \, a=b\right)$",
    r"$? ! &$",
    r"$\mathring{A}  \AA$",
    r"$\hspace{-0.2}\dotplus\hspace{-0.2}$ $\hspace{-0.2}\doteq\hspace{-0.2}$ $\hspace{-0.2}\doteqdot\hspace{-0.2}$ $\ddots$",
    r"$xyz^kx_kx^py^{p-2} d_i^jb_jc_kd x^j_i E^0 E^0_u$",
    r"${xyz}^k{x}_{k}{x}^{p}{y}^{p-2} {d}_{i}^{j}{b}_{j}{c}_{k}{d} {x}^{j}_{i}{E}^{0}{E}^0_u$",
    r"${\int}_x^x x\oint_x^x x\int_{X}^{X}x\int_x x \int^x x \int_{x} x\int^{x}{\int}_{x} x{\int}^{x}_{x}x$",
    r"testing$^{123}$",
    r"$\overline{\omega}^x \frac{1}{2}_0^x$",
    r"$,$ $.$ $1{,}234{, }567{ , }890$ and $1,234,567,890$",
    r"$\left(X\right)_{a}^{b}$",
    r"$\dfrac{\$100.00}{y}$",
    r"$a=-b-c$",
]

#: The ``font_tests`` of ``test_mathfont_rendering`` the ``mathtext`` group
#: does not draw, in matplotlib's order.
FONT_TESTS = [
    r" $0123456789$",
    r" $ABCDEFGHIJKLMNOPQRSTUVWXYZ$",
    r" $abcdefghijklmnopqrstuvwxyz$",
    r" $\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega$",
    r" $\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi$",
    r"mathrm $\mathrm{0123456789}$",
    r"mathrm $\mathrm{abcdefghijklmnopqrstuvwxyz}$",
    r"mathrm $\mathrm{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathrm $\mathrm{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
    r"mathbf $\mathbf{0123456789}$",
    r"mathbf $\mathbf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbf $\mathbf{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbf $\mathbf{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathit $\mathit{0123456789}$",
    r"mathit $\mathit{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathit $\mathit{abcdefghijklmnopqrstuvwxyz}$",
    r"mathit $\mathit{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
    r"mathtt $\mathtt{0123456789}$",
    r"mathtt $\mathtt{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbb $\mathbb{0123456789}$",
    r"mathbb $\mathbb{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbb $\mathbb{\Gamma \Pi \Sigma \gamma \pi}$",
    r"mathrm mathbb $\mathrm{\mathbb{0123456789}}$",
    r"mathrm mathbb $\mathrm{\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathrm mathbb $\mathrm{\mathbb{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathbb $\mathbf{\mathbb{0123456789}}$",
    r"mathbf mathbb $\mathbf{\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathbb $\mathbf{\mathbb{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathbb $\mathbf{\mathbb{\Gamma \Pi \Sigma \gamma \pi}}$",
    r"mathfrak $\mathfrak{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbf mathfrak $\mathbf{\mathfrak{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathfrak $\mathbf{\mathfrak{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathscr $\mathscr{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathsf $\mathsf{0123456789}$",
    r"mathsf $\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathsf $\mathsf{abcdefghijklmnopqrstuvwxyz}$",
    r"mathrm mathsf $\mathrm{\mathsf{0123456789}}$",
    r"mathrm mathsf $\mathrm{\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathrm mathsf $\mathrm{\mathsf{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathsf $\mathbf{\mathsf{0123456789}}$",
    r"mathbf mathsf $\mathbf{\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathsf $\mathbf{\mathsf{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbfit $\mathbfit{0123456789}$",
    r"mathbfit $\mathbfit{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbfit $\mathbfit{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbfit $\mathbfit{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathbfit $\mathbfit{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
]

#: The fontsets both tests run in.
FONTSETS = ["cm", "stix", "stixsans", "dejavusans", "dejavuserif"]

#: Expressions per case.
CHUNK = 10

#: ``svgastext_math_tests``: every ``AutoHeightChar`` substitution, which
#: ``test_mathtext_rendering_svgastext`` saves as SVG with text as text.
SVGASTEXT = [
    r"$-$-",
    *[
        r"$\left" + lc + r" M \middle/ ? \middle\backslash ? \right" + rc + " " +
        r"\left" + lc + r" \frac{M}{B} \middle/ ? \middle\backslash ? \right" + rc + " " +
        r"\left" + lc + r" \frac{\frac{M}{I}}{B} \middle/ ? \middle\backslash ? \right" + rc + " " +
        r"\left" + lc + r" \frac{\frac{M}{I}}{\frac{B}{U}} \middle/ ? \middle\backslash ? \right" + rc + " " +
        r"\left" + lc + r"\frac{\frac{\frac{M}{I}}{N}}{\frac{\frac{B}{U}}{G}} \middle/ ? \middle\backslash ? \right" + rc + "$"
        for lc, rc in ["()", "[]", "<>", (r"\{", r"\}"), (r"\lfloor", r"\rfloor"),
                       (r"\lceil", r"\rceil")]
    ],
]


def _math_rows(expressions, fontsize=12):
    """The tests' 5.25 x 0.75 in figure of each expression, stacked: the
    expression centred in its row."""
    def build(fig):
        n = len(expressions)
        for i, s in enumerate(expressions):
            fig.text(0.5, 1 - (i + 0.5) / n, s, fontsize=fontsize, ha="center",
                     va="center")

    return build


for _name, _tests in [("math_tests", MATH_TESTS), ("font_tests", FONT_TESTS)]:
    for _start in range(0, len(_tests), CHUNK):
        _chunk = _tests[_start:_start + CHUNK]
        for _fontset in FONTSETS:
            image_test(f"{_name}_{_start // CHUNK + 1}_{_fontset}",
                       figsize=(5.25, 0.75 * len(_chunk)),
                       rc={"mathtext.fontset": _fontset})(_math_rows(_chunk))


for _fontset in ["cm", "dejavusans"]:
    image_test(f"svgastext_{_fontset}", figsize=(5.25, 0.75 * len(SVGASTEXT)),
               rc={"mathtext.fontset": _fontset, "svg.fonttype": "none"})(
        _math_rows(SVGASTEXT, fontsize=16))
