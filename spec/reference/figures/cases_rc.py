"""rcParam effects: every visual rcParam set to a non-default value on a
small scene that the value visibly changes.

Each case applies its parameters through ``rc=`` (so that draw-time and
figure-level parameters are active for the whole life of the figure) to one
of a few shared scene builders. Related parameters share a case where their
effects do not interfere (all ``xtick.*`` sizes, all ``boxplot.flierprops.*``,
...). Values are written as rc-file text, exactly as the Rust mirror passes
them, except ``path.effects``, which has no text form.

``font.variant`` and ``font.stretch`` are the only parameters set here
without a visible change: no bundled font has small-caps or condensed faces,
so matplotlib picks the same DejaVu Sans face. The case checks that plt-rs's
font matching agrees. ``figure.frameon`` is checked on subfigures, because
``pyplot.figure`` (which the harness uses) passes ``frameon=True`` and the
root figure never reads it.

Parameters left out, and why:

* GUI, backend and session: ``backend``, ``backend_fallback``,
  ``interactive``, ``toolbar``, ``figure.raise_window``,
  ``macosx.window_mode``, ``tk.window_focus``, ``webagg.*`` and every
  ``keymap.*``; the mouse-rotation parameters ``axes3d.mouserotationstyle``,
  ``axes3d.trackballsize``, ``axes3d.trackballborder`` and
  ``axes3d.snap_rotation``; ``savefig.directory`` (a dialog default).
* ``animation.*``: movie writers and HTML embedding.
* ``timezone``: covered by the date probes.
* ``date.epoch``: matplotlib latches the epoch at the first date conversion
  in the process, so an override in one case would depend on case order.
* ``figure.max_open_warning`` (a warning threshold), ``figure.hooks``
  (callables run by ``pyplot.figure``, with no drawing of their own),
  ``docstring.hardcopy`` and ``_internal.classic_mode`` (set by the
  ``classic`` sheet, covered by ``styles/sheet_classic``).
* ``figure.figsize``, ``figure.dpi`` and ``savefig.format``: the harness
  passes size, resolution and format explicitly, like every case.
  (``savefig.dpi`` rides along in ``rc/savefig_facecolor`` to check that the
  explicit resolution wins.)
* Vector output only, invisible in a PNG: ``svg.fonttype``, ``pdf.fonttype``
  and ``ps.fonttype`` (covered by the vector variants), ``pdf.use14corefonts``
  and ``ps.useafm`` (covered by the ``afm`` group), ``pdf.compression``,
  ``pdf.inheritcolor``, ``svg.image_inline``, ``svg.hashsalt``, ``svg.id``,
  ``image.composite_image``, ``ps.papersize``, ``ps.usedistiller``,
  ``ps.distiller.res`` and ``savefig.orientation``.
* TeX: ``text.usetex``, ``text.latex.engine``, ``text.latex.preamble`` and
  ``pgf.*`` need a TeX installation.
* ``text.hinting``: covered by the ``hinting`` group.
  ``text.hinting_factor`` and ``text.kerning_factor`` are deprecated and
  change nothing in matplotlib 3.11 (a kerning factor of 6 draws the same
  pixels).
* ``image.lut``: matplotlib samples its segmented colormaps once, when it
  builds the colormap registry at import, so an override at run time
  changes nothing.
* ``axes.formatter.use_locale``: depends on the host locale; with the C
  locale the reference run uses it draws exactly as the default.
* ``boxplot.bootstrap``: resamples with numpy's global random state, which
  cannot be reproduced.
"""

from datetime import datetime, timedelta

import numpy as np
from matplotlib import patheffects
from matplotlib.patches import Circle, Polygon, Rectangle
from matplotlib.ticker import FixedLocator

from spec_support import case, rng

T = np.linspace(0, 6, 25)

SMALL = (3, 2)
DATED = (5, 2)
LABELLED = (3.2, 2.4)
SQUARE = (2.6, 2.6)
PANELS = (4, 3)


def _cases(scene, figsize, entries):
    """Register ``rc/<name>`` for each ``(name, rc)`` pair on one scene."""
    for name, rc in entries:
        case(f"rc/{name}", figsize=figsize, rc=rc)(scene)


# --- scenes -------------------------------------------------------------------


def _curve(fig):
    """One curve."""
    ax = fig.subplots()
    ax.plot(T, np.sin(T))


def _curves(fig):
    """Two curves drawn with the line defaults."""
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    ax.plot(T, 0.5 * np.cos(T))


def _reference_lines(fig):
    """Lines that take ``lines.color``: axhline, axvline, hlines, vlines."""
    ax = fig.subplots()
    ax.axhline(0.5)
    ax.axvline(0.25)
    ax.hlines([0.2, 0.8], 0.4, 0.9)
    ax.vlines([0.6, 0.75], 0.1, 0.45)


def _dashes(fig):
    """Dashed, dash-dot and dotted curves."""
    ax = fig.subplots()
    for i, style in enumerate(["--", "-.", ":"]):
        ax.plot(T, np.sin(T) + i, linestyle=style, linewidth=2)


def _zigzags(fig):
    """A thick solid and a thick dashed zigzag: joins and caps show."""
    ax = fig.subplots()
    x = 1.2 + 0.4 * np.arange(5)
    ax.plot(x, [0, 1, 0, 1, 0], linewidth=12)
    ax.plot(x, [1.6, 2.6, 1.6, 2.6, 1.6], linewidth=12, linestyle="--")
    ax.set_xlim(-0.6, 4.6)
    ax.set_ylim(-0.6, 3.2)


def _markers(fig):
    """Circle markers on a line, and squares and triangles on their own."""
    ax = fig.subplots()
    x = T[::3]
    ax.plot(x, np.sin(x), marker="o")
    ax.plot(x, np.cos(x), marker="s", linestyle="none")
    ax.plot(x, 0.5 * np.sin(x) - 1.2, marker="^", linestyle="none")


def _mesh(fig):
    """A pcolormesh whose coordinates have the shape of the data."""
    ax = fig.subplots()
    x, y = np.meshgrid(np.linspace(0, 1, 7), np.linspace(0, 1, 5))
    ax.pcolormesh(x, y, np.sin(3 * x) + y)


def _patches(fig):
    """A filled rectangle and circle and an unfilled triangle, all with the
    patch defaults."""
    ax = fig.subplots()
    ax.add_patch(Rectangle((0.08, 0.1), 0.3, 0.45))
    ax.add_patch(Circle((0.65, 0.32), 0.2))
    ax.add_patch(Polygon([(0.2, 0.62), (0.5, 0.95), (0.8, 0.62)], fill=False))
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


def _hatches(fig):
    """A hatched patch with an edge colour and one without an edge."""
    ax = fig.subplots()
    ax.add_patch(Rectangle((0.05, 0.1), 0.4, 0.8, hatch="//", facecolor="white", edgecolor="tab:blue"))
    ax.add_patch(Rectangle((0.55, 0.1), 0.4, 0.8, hatch="x", facecolor="lightgray"))
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


def _hist(fig):
    """A histogram of normal samples."""
    fig.subplots().hist(rng(7).normal(300))


def _boxes(fig):
    """Three box plots with outliers."""
    g = rng(11)
    data = [np.append(g.normal(40, i, 1 + 0.3 * i), [i + 4.5, i - 5]) for i in range(3)]
    fig.subplots().boxplot(data)


def _labelled(fig):
    """A curve with a title and axis labels."""
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    ax.set_title("Title")
    ax.set_xlabel("time")
    ax.set_ylabel("value")
    fig.subplots_adjust(left=0.2, bottom=0.22, top=0.85)


def _texts(fig):
    """Plain text, text with a math part, and ligature and minus candidates."""
    fig.text(0.05, 0.72, "Plain AVAT Wo", fontsize=14)
    fig.text(0.05, 0.42, r"Math $\alpha^2+\sqrt{x}$", fontsize=14)
    fig.text(0.05, 0.12, "fi ffl 1-2", fontsize=14)


def _missing_glyph(fig):
    """Characters DejaVu Sans has no glyph for, between two it has."""
    fig.text(0.1, 0.4, "A中กB", fontsize=20)


def _cyrillic(fig):
    """Cyrillic letters with Serbian local forms."""
    fig.text(0.05, 0.35, "бгдпт", fontsize=24)


def _maths(fig):
    """Math text in every font command."""
    fig.text(0.04, 0.72, r"$x_i^2+\alpha\int f\,dx$", fontsize=13)
    fig.text(0.04, 0.42, r"$\mathrm{rm}\ \mathit{it}\ \mathbf{bf}\ \mathsf{sf}$", fontsize=13)
    fig.text(0.04, 0.12, r"$\mathtt{tt}\ \mathcal{CAL}\ \mathbfit{bfit}$", fontsize=13)


def _math_symbols(fig):
    """Math symbols a monospace text font lacks."""
    fig.text(0.05, 0.4, r"$\oint\ \coprod\ \wp\ \bigodot\ x\geq y$", fontsize=16)


def _image(fig):
    """A small array, upsampled."""
    data = np.sin(np.linspace(0, 3, 8))[None, :] * np.cos(np.linspace(0, 2, 5))[:, None]
    fig.subplots().imshow(data)


def _fine_image(fig):
    """A fine pattern, downsampled."""
    x = np.linspace(0, 6, 400)
    fig.subplots().imshow(np.sin(40 * x)[None, :] * np.cos(28 * x)[:, None])


def _contours(fig):
    """Black contour lines; the negative levels are dashed."""
    x, y = np.meshgrid(np.linspace(-2, 2, 30), np.linspace(-2, 2, 30))
    fig.subplots().contour(x, y, np.sin(x) * np.cos(y), levels=7, colors="k")


def _masked_contours(fig):
    """Filled contours of masked data: the corners next to masked points show."""
    x, y = np.meshgrid(np.linspace(-2, 2, 12), np.linspace(-2, 2, 12))
    mask = np.zeros(x.shape, bool)
    mask[3:6, 4:7] = True
    mask[8, 2] = True
    mask[9, 9] = True
    fig.subplots().contourf(x, y, np.ma.masked_array(np.sin(x) * np.cos(y), mask), levels=6)


def _errorbars(fig):
    """Points with x and y error bars."""
    x = np.arange(5.0)
    fig.subplots().errorbar(x, 0.5 * x + 1, yerr=0.4, xerr=0.2, fmt="o")


def _scatter(fig):
    """A scatter plot with the scatter defaults."""
    g = rng(5)
    x = g.uniform(20)
    y = g.uniform(20)
    fig.subplots().scatter(x, y, s=60)


def _axisbelow(fig):
    """Bars and a thick line on a grid: the grid's z-order shows."""
    ax = fig.subplots()
    ax.bar(np.arange(4.0), [3, 5, 2, 4])
    ax.plot([-0.4, 3.4], [1, 4], linewidth=4, color="k")


def _scientific(fig):
    """Values that need scientific notation on both axes."""
    fig.subplots().plot(T * 1e7, np.sin(T) * 1e-6)


def _thousands(fig):
    """Values in the thousands and hundredths."""
    fig.subplots().plot(T * 1000, np.sin(T) / 100)


def _offsets(fig):
    """Values close to 1000 and 100: tick offsets show."""
    fig.subplots().plot(1000 + T / 10, 100 + T / 20)


def _logs(fig):
    """A logarithmic y axis from 10^-2 to 10^4."""
    fig.subplots().semilogy(T, 10 ** (T - 2))


def _polar(fig):
    """A curve on polar axes."""
    fig.add_subplot(projection="polar").plot(T, 1 + 0.5 * np.sin(3 * T))


def _scatter3d(fig):
    """A 3-D scatter plot."""
    ax = fig.add_subplot(projection="3d")
    g = rng(3)
    x = g.uniform(30)
    y = g.uniform(30)
    z = g.uniform(30)
    ax.scatter(x, y, z, s=30)


def _ortho_helix(fig):
    """A 3-D helix, autoscaled in an orthographic projection (perspective
    axes have no z margin)."""
    fig.add_subplot(projection="3d", proj_type="ortho").plot(np.cos(T), np.sin(T), T / 6)


def _limited_helix(fig):
    """A 3-D helix with explicitly set limits."""
    ax = fig.add_subplot(projection="3d")
    ax.plot(np.cos(T), np.sin(T), T / 6)
    ax.set_xlim(-1, 1)
    ax.set_ylim(-1, 1)
    ax.set_zlim(0, 1)


def _dates(start, step, n):
    """A curve over ``n`` dates ``step`` apart from ``start``."""

    def build(fig):
        fig.subplots().plot([start + i * step for i in range(n)], np.sin(np.linspace(0, 6, n)))

    return build


def _legend(fig):
    """A titled legend of a marked line, a line, a scatter and a band."""
    ax = fig.subplots()
    ax.plot(T, np.sin(T), marker="o", markersize=4, label="sine")
    ax.plot(T, 0.5 * np.cos(T), label="cosine")
    ax.scatter(T[::4], 0.3 * np.sin(T[::4]) - 0.4, label="points")
    ax.fill_between(T, -1.1, -0.8, alpha=0.4, label="band")
    ax.legend(title="key")


def _legend_columns(fig):
    """A two-column legend."""
    ax = fig.subplots()
    for i in range(4):
        ax.plot(T, np.sin(T + i), label=f"c{i}")
    ax.legend(ncols=2)


def _minor_labels(fig):
    """A curve with labelled minor ticks."""
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    ax.xaxis.set_minor_locator(FixedLocator([1.5, 4.5]))
    ax.xaxis.set_minor_formatter("{x:.1f}")
    ax.yaxis.set_minor_locator(FixedLocator([-0.25, 0.25]))
    ax.yaxis.set_minor_formatter("{x:.2f}")


def _supertitles(fig):
    """Two panels under a figure title and figure axis labels."""
    for i, ax in enumerate(fig.subplots(1, 2)):
        ax.plot(T, np.sin(T + i))
    fig.suptitle("Suptitle")
    fig.supxlabel("figure x")
    fig.supylabel("figure y")


def _panels(fig):
    """Four titled and labelled panels."""
    for i, ax in enumerate(fig.subplots(2, 2).flat):
        ax.plot(T, np.sin(T + i))
        ax.set_title(f"panel {i}")
        ax.set_xlabel("x")


def _subfigures(fig):
    """Two coloured subfigures, each with a curve."""
    for i, (sub, color) in enumerate(zip(fig.subfigures(1, 2), ["lightsteelblue", "wheat"])):
        sub.set_facecolor(color)
        sub.subplots().plot(T, np.sin(T + i))


def _framed(fig):
    """A curve in a figure with a thick border."""
    fig.set_linewidth(8)
    fig.subplots().plot(T, np.sin(T))


def _sawtooth(fig):
    """A curve with a sawtooth half a pixel high: simplification at a
    threshold of a pixel flattens it."""
    x = np.linspace(0, 10, 3000)
    fig.subplots().plot(x, np.sin(x) + 0.008 * (-1.0) ** np.arange(3000))


def _lissajous(fig):
    """A dense, thick, translucent Lissajous curve: where separately drawn
    chunks cross, they blend twice."""
    t = np.linspace(0, 2 * np.pi, 3000)
    fig.subplots().plot(np.sin(3 * t), np.sin(4 * t), linewidth=5, alpha=0.5)


# --- lines and markers --------------------------------------------------------

_cases(_curves, SMALL, [
    ("lines_linewidth", {"lines.linewidth": "4"}),
    ("lines_linestyle", {"lines.linestyle": "--"}),
    ("lines_marker", {"lines.marker": "o"}),
    ("lines_antialiased", {"lines.antialiased": "False"}),
])
_cases(_reference_lines, SMALL, [
    ("lines_color", {"lines.color": "tab:red"}),
])
_cases(_markers, SMALL, [
    ("lines_markersize", {"lines.markersize": "10"}),
    ("lines_markeredgewidth", {"lines.markeredgewidth": "3"}),
    ("lines_marker_colors", {"lines.markerfacecolor": "white", "lines.markeredgecolor": "black"}),
    ("markers_fillstyle_left", {"markers.fillstyle": "left"}),
    ("markers_fillstyle_none", {"markers.fillstyle": "none"}),
])
_cases(_zigzags, SMALL, [
    ("lines_solid_joinstyle", {"lines.solid_joinstyle": "miter"}),
    ("lines_solid_capstyle", {"lines.solid_capstyle": "round"}),
    ("lines_dash_joinstyle", {"lines.dash_joinstyle": "bevel"}),
    ("lines_dash_capstyle", {"lines.dash_capstyle": "round"}),
])
_cases(_dashes, SMALL, [
    ("lines_dash_patterns", {
        "lines.dashed_pattern": "6, 2",
        "lines.dashdot_pattern": "8, 2, 2, 2",
        "lines.dotted_pattern": "1, 3",
    }),
    ("lines_scale_dashes", {"lines.scale_dashes": "False"}),
])

# --- meshes, patches and hatches ----------------------------------------------

_cases(_mesh, SMALL, [
    ("pcolor_shading", {"pcolor.shading": "gouraud"}),
    ("pcolormesh_snap", {"pcolormesh.snap": "False"}),
])
_cases(_patches, SMALL, [
    ("patch_facecolor", {"patch.facecolor": "salmon"}),
    ("patch_edgecolor_linewidth", {"patch.edgecolor": "tab:red", "patch.linewidth": "3"}),
    ("patch_force_edgecolor", {"patch.force_edgecolor": "True"}),
    ("patch_antialiased", {"patch.antialiased": "False"}),
])
_cases(_hatches, SMALL, [
    ("hatch_color", {"hatch.color": "tab:red"}),
    ("hatch_linewidth", {"hatch.linewidth": "3"}),
])

# --- statistics ---------------------------------------------------------------

_cases(_hist, SMALL, [
    ("hist_bins", {"hist.bins": "25"}),
    ("hist_bins_auto", {"hist.bins": "auto"}),
    ("hist_bins_edges", {"hist.bins": "-3, -1, -0.5, 0, 0.5, 1, 3"}),
])
_cases(_boxes, SMALL, [
    ("boxplot_notch", {"boxplot.notch": "True"}),
    ("boxplot_vertical", {"boxplot.vertical": "False"}),
    ("boxplot_whiskers", {"boxplot.whiskers": "0.5"}),
    ("boxplot_whiskers_percentiles", {"boxplot.whiskers": "5, 95"}),
    ("boxplot_patchartist", {"boxplot.patchartist": "True"}),
    ("boxplot_showmeans", {"boxplot.showmeans": "True"}),
    ("boxplot_meanline", {"boxplot.showmeans": "True", "boxplot.meanline": "True"}),
    ("boxplot_hidden_parts", {
        "boxplot.showcaps": "False",
        "boxplot.showbox": "False",
        "boxplot.showfliers": "False",
    }),
    ("boxplot_flierprops", {
        "boxplot.flierprops.color": "tab:red",
        "boxplot.flierprops.marker": "D",
        "boxplot.flierprops.markerfacecolor": "yellow",
        "boxplot.flierprops.markeredgecolor": "tab:green",
        "boxplot.flierprops.markeredgewidth": "2",
        "boxplot.flierprops.markersize": "8",
        "boxplot.flierprops.linestyle": "-",
        "boxplot.flierprops.linewidth": "2",
    }),
    ("boxplot_boxprops", {
        "boxplot.boxprops.color": "tab:blue",
        "boxplot.boxprops.linewidth": "2.5",
        "boxplot.boxprops.linestyle": "--",
    }),
    ("boxplot_whiskerprops", {
        "boxplot.whiskerprops.color": "tab:orange",
        "boxplot.whiskerprops.linewidth": "2.5",
        "boxplot.whiskerprops.linestyle": ":",
    }),
    ("boxplot_capprops", {
        "boxplot.capprops.color": "tab:purple",
        "boxplot.capprops.linewidth": "3",
        "boxplot.capprops.linestyle": "-.",
    }),
    ("boxplot_medianprops", {
        "boxplot.medianprops.color": "black",
        "boxplot.medianprops.linewidth": "3",
        "boxplot.medianprops.linestyle": "--",
    }),
    ("boxplot_meanprops_marker", {
        "boxplot.showmeans": "True",
        "boxplot.meanprops.marker": "D",
        "boxplot.meanprops.markerfacecolor": "white",
        "boxplot.meanprops.markeredgecolor": "black",
        "boxplot.meanprops.markersize": "9",
    }),
    ("boxplot_meanprops_line", {
        "boxplot.showmeans": "True",
        "boxplot.meanline": "True",
        "boxplot.meanprops.color": "tab:purple",
        "boxplot.meanprops.linestyle": ":",
        "boxplot.meanprops.linewidth": "2.5",
    }),
])
_cases(_errorbars, SMALL, [
    ("errorbar_capsize", {"errorbar.capsize": "5"}),
    ("errorbar_capthick", {"errorbar.capsize": "4", "errorbar.capthick": "3"}),
    ("errorbar_elinewidth", {"errorbar.elinewidth": "3"}),
])
_cases(_scatter, SMALL, [
    ("scatter_marker", {"scatter.marker": "^"}),
    ("scatter_edgecolors", {"scatter.edgecolors": "black"}),
])

# --- fonts and text -----------------------------------------------------------

_cases(_texts, SMALL, [
    ("font_family_serif", {"font.family": "serif"}),
    ("font_family_monospace", {"font.family": "monospace"}),
    ("font_serif", {"font.family": "serif", "font.serif": "STIXGeneral"}),
    ("font_sans_serif", {"font.family": "sans-serif", "font.sans-serif": "DejaVu Sans Mono"}),
    ("font_cursive", {"font.family": "cursive", "font.cursive": "DejaVu Serif"}),
    ("font_fantasy", {"font.family": "fantasy", "font.fantasy": "STIXGeneral"}),
    ("font_monospace", {"font.family": "monospace", "font.monospace": "DejaVu Serif"}),
    ("font_style", {"font.style": "italic"}),
    ("font_weight", {"font.weight": "bold"}),
    ("font_variant_stretch", {"font.variant": "small-caps", "font.stretch": "condensed"}),
    ("text_antialiased", {"text.antialiased": "False"}),
    ("text_parse_math", {"text.parse_math": "False"}),
])
_cases(_labelled, LABELLED, [
    ("font_size", {"font.size": "13"}),
    ("text_color", {"text.color": "tab:blue"}),
])
_cases(_missing_glyph, SMALL, [
    ("font_enable_last_resort", {"font.enable_last_resort": "False"}),
])
_cases(_cyrillic, SMALL, [
    ("text_language", {"text.language": "sr"}),
])

# --- mathtext -----------------------------------------------------------------

_cases(_maths, SMALL, [
    ("mathtext_fontset_cm", {"mathtext.fontset": "cm"}),
    ("mathtext_fontset_stix", {"mathtext.fontset": "stix"}),
    ("mathtext_fontset_stixsans", {"mathtext.fontset": "stixsans"}),
    ("mathtext_fontset_dejavuserif", {"mathtext.fontset": "dejavuserif"}),
    ("mathtext_custom", {
        "mathtext.fontset": "custom",
        "mathtext.rm": "serif",
        "mathtext.it": "serif:italic",
        "mathtext.bf": "serif:bold",
        "mathtext.bfit": "serif:italic:bold",
        "mathtext.sf": "monospace",
        "mathtext.tt": "serif",
        "mathtext.cal": "STIXGeneral",
    }),
    ("mathtext_default", {"mathtext.default": "rm"}),
])
_cases(_math_symbols, SMALL, [
    ("mathtext_fallback", {
        "mathtext.fontset": "custom",
        "mathtext.rm": "DejaVu Sans Mono",
        "mathtext.it": "DejaVu Sans Mono:italic",
        "mathtext.fallback": "stix",
    }),
])

# --- images and contours ------------------------------------------------------

_cases(_image, SMALL, [
    ("image_aspect", {"image.aspect": "auto"}),
    ("image_interpolation", {"image.interpolation": "bilinear"}),
    ("image_interpolation_stage", {"image.interpolation": "bilinear", "image.interpolation_stage": "rgba"}),
    ("image_cmap", {"image.cmap": "plasma"}),
    ("image_origin", {"image.origin": "lower"}),
])
_cases(_fine_image, SMALL, [
    ("image_resample", {"image.resample": "False"}),
])
_cases(_contours, SMALL, [
    ("contour_negative_linestyle", {"contour.negative_linestyle": "dashdot"}),
    ("contour_linewidth", {"contour.linewidth": "2.5"}),
])
_cases(_masked_contours, SMALL, [
    ("contour_corner_mask", {"contour.corner_mask": "False"}),
    ("contour_algorithm", {"contour.algorithm": "mpl2005"}),
])

# --- axes ---------------------------------------------------------------------

_cases(_curves, SMALL, [
    ("axes_facecolor", {"axes.facecolor": "lightyellow"}),
    ("axes_edge", {"axes.edgecolor": "tab:red", "axes.linewidth": "3"}),
    ("axes_spines_top_right", {"axes.spines.top": "False", "axes.spines.right": "False"}),
    ("axes_spines_left_bottom", {"axes.spines.left": "False", "axes.spines.bottom": "False"}),
    ("axes_grid", {"axes.grid": "True"}),
    ("axes_grid_axis", {"axes.grid": "True", "axes.grid.axis": "x"}),
    ("axes_grid_which", {
        "axes.grid": "True",
        "axes.grid.which": "minor",
        "xtick.minor.visible": "True",
        "ytick.minor.visible": "True",
    }),
    ("axes_unicode_minus", {"axes.unicode_minus": "False"}),
    ("axes_prop_cycle", {"axes.prop_cycle": "cycler(color=['tab:red', 'k']) + cycler(marker=['o', 's'])"}),
    ("axes_autolimit_mode", {"axes.autolimit_mode": "round_numbers"}),
    ("axes_margins", {"axes.xmargin": "0", "axes.ymargin": "0.3"}),
])
_cases(_labelled, LABELLED, [
    ("axes_title_style", {"axes.titlesize": "small", "axes.titleweight": "bold", "axes.titlecolor": "tab:green"}),
    ("axes_titlelocation", {"axes.titlelocation": "right"}),
    ("axes_titley", {"axes.titley": "0.8"}),
    ("axes_titlepad", {"axes.titlepad": "14"}),
    ("axes_label_style", {"axes.labelsize": "large", "axes.labelweight": "bold", "axes.labelcolor": "tab:purple"}),
    ("axes_labelpad", {"axes.labelpad": "10"}),
    ("axis_labellocation", {"xaxis.labellocation": "right", "yaxis.labellocation": "top"}),
])
_cases(_axisbelow, SMALL, [
    ("axes_axisbelow_true", {"axes.grid": "True", "axes.axisbelow": "True"}),
    ("axes_axisbelow_false", {"axes.grid": "True", "axes.axisbelow": "False"}),
])
_cases(_thousands, SMALL, [
    ("axes_formatter_limits", {"axes.formatter.limits": "-2, 2"}),
])
_cases(_scientific, SMALL, [
    ("axes_formatter_use_mathtext", {"axes.formatter.use_mathtext": "True"}),
])
_cases(_logs, SMALL, [
    ("axes_formatter_min_exponent", {"axes.formatter.min_exponent": "2"}),
])
_cases(_offsets, SMALL, [
    ("axes_formatter_useoffset", {"axes.formatter.useoffset": "False"}),
    ("axes_formatter_offset_threshold", {"axes.formatter.offset_threshold": "2"}),
])

# --- polar and 3-D axes -------------------------------------------------------

_cases(_polar, SQUARE, [
    ("polaraxes_grid", {"polaraxes.grid": "False"}),
])
_cases(_scatter3d, SQUARE, [
    ("axes3d_grid", {"axes3d.grid": "False"}),
    ("axes3d_panecolor", {
        "axes3d.xaxis.panecolor": "lightsteelblue",
        "axes3d.yaxis.panecolor": "(1.0, 0.9, 0.7, 0.8)",
        "axes3d.zaxis.panecolor": "#c8e6c8",
    }),
    ("axes3d_depthshade", {"axes3d.depthshade": "False"}),
    ("axes3d_depthshade_minalpha", {"axes3d.depthshade_minalpha": "0.8"}),
])
_cases(_limited_helix, SQUARE, [
    ("axes3d_automargin", {"axes3d.automargin": "True"}),
])
_cases(_ortho_helix, SQUARE, [
    ("axes_zmargin", {"axes.zmargin": "0.4"}),
])

# --- dates --------------------------------------------------------------------

_YEARS = _dates(datetime(2012, 1, 1), timedelta(days=60), 61)
_MONTHS = _dates(datetime(2023, 11, 15), timedelta(days=7), 30)
_DAYS = _dates(datetime(2024, 2, 20, 7), timedelta(hours=6), 33)
_HOURS = _dates(datetime(2024, 2, 20, 7, 20), timedelta(minutes=30), 25)
_cases(_YEARS, DATED, [("date_autoformatter_year", {"date.autoformatter.year": "Y%y"})])
_cases(_MONTHS, DATED, [("date_autoformatter_month", {"date.autoformatter.month": "%b %y"})])
_cases(_DAYS, DATED, [
    ("date_autoformatter_day", {"date.autoformatter.day": "%d.%m."}),
    ("date_converter", {"date.converter": "concise"}),
])
_cases(_HOURS, DATED, [
    ("date_autoformatter_hour", {"date.autoformatter.hour": "%Hh"}),
    ("date_interval_multiples", {"date.interval_multiples": "False"}),
])
_cases(_dates(datetime(2024, 3, 1, 12), timedelta(minutes=2), 21), DATED, [
    ("date_autoformatter_minute", {"date.autoformatter.minute": "%H.%M"}),
])
_cases(_dates(datetime(2024, 3, 1, 12), timedelta(seconds=3), 21), DATED, [
    ("date_autoformatter_second", {"date.autoformatter.second": "%Mm%Ss"}),
])
_cases(_dates(datetime(2024, 3, 1, 12), timedelta(milliseconds=25), 21), DATED, [
    ("date_autoformatter_microsecond", {"date.autoformatter.microsecond": "%S.%f"}),
])

# --- legends ------------------------------------------------------------------

_cases(_legend, LABELLED, [
    ("legend_loc", {"legend.loc": "lower left"}),
    ("legend_frameon", {"legend.frameon": "False"}),
    ("legend_frame_style", {
        "legend.framealpha": "1",
        "legend.facecolor": "lightyellow",
        "legend.edgecolor": "tab:red",
        "legend.linewidth": "2",
    }),
    ("legend_box_shape", {"legend.fancybox": "False", "legend.shadow": "True"}),
    ("legend_points", {"legend.numpoints": "3", "legend.scatterpoints": "4"}),
    ("legend_markerscale", {"legend.markerscale": "2"}),
    ("legend_fontsizes", {"legend.fontsize": "x-small", "legend.title_fontsize": "large"}),
    ("legend_labelcolor", {"legend.labelcolor": "linecolor"}),
    ("legend_paddings", {"legend.borderpad": "1.5", "legend.labelspacing": "1.2"}),
    ("legend_handle_spacing", {"legend.handletextpad": "2", "legend.borderaxespad": "2"}),
    ("legend_handle_size", {"legend.handlelength": "4", "legend.handleheight": "2"}),
])
_cases(_legend_columns, LABELLED, [
    ("legend_columnspacing", {"legend.columnspacing": "5"}),
])

# --- ticks and grids ----------------------------------------------------------

_cases(_curve, SMALL, [
    ("xtick_top_labeltop", {
        "xtick.top": "True",
        "xtick.bottom": "False",
        "xtick.labeltop": "True",
        "xtick.labelbottom": "False",
    }),
    ("ytick_right_labelright", {
        "ytick.left": "False",
        "ytick.right": "True",
        "ytick.labelleft": "False",
        "ytick.labelright": "True",
    }),
    ("xtick_sizes", {
        "xtick.minor.visible": "True",
        "xtick.major.size": "8",
        "xtick.minor.size": "4",
        "xtick.major.width": "2",
        "xtick.minor.width": "1.5",
    }),
    ("ytick_sizes", {
        "ytick.minor.visible": "True",
        "ytick.major.size": "8",
        "ytick.minor.size": "4",
        "ytick.major.width": "2",
        "ytick.minor.width": "1.5",
    }),
    ("tick_colors", {
        "xtick.color": "tab:red",
        "xtick.labelcolor": "tab:blue",
        "ytick.color": "tab:green",
        "ytick.labelcolor": "tab:purple",
    }),
    ("tick_colors_inherited", {"xtick.color": "tab:red", "ytick.color": "tab:green"}),
    ("xtick_major_minor_sides", {
        "xtick.top": "True",
        "xtick.minor.visible": "True",
        "xtick.major.top": "False",
        "xtick.minor.bottom": "False",
    }),
    ("xtick_major_minor_sides_swapped", {
        "xtick.top": "True",
        "xtick.minor.visible": "True",
        "xtick.major.bottom": "False",
        "xtick.minor.top": "False",
    }),
    ("ytick_major_minor_sides", {
        "ytick.right": "True",
        "ytick.minor.visible": "True",
        "ytick.major.right": "False",
        "ytick.minor.left": "False",
    }),
    ("ytick_major_minor_sides_swapped", {
        "ytick.right": "True",
        "ytick.minor.visible": "True",
        "ytick.major.left": "False",
        "ytick.minor.right": "False",
    }),
    ("tick_minor_ndivs", {
        "xtick.minor.visible": "True",
        "ytick.minor.visible": "True",
        "xtick.minor.ndivs": "2",
        "ytick.minor.ndivs": "3",
    }),
    ("tick_labelsize", {"xtick.labelsize": "small", "ytick.labelsize": "13"}),
    ("tick_direction", {"xtick.direction": "in", "ytick.direction": "inout"}),
    ("tick_alignment", {"xtick.alignment": "left", "ytick.alignment": "top"}),
    ("grid_style", {
        "axes.grid": "True",
        "grid.color": "tab:orange",
        "grid.linestyle": "--",
        "grid.linewidth": "1.5",
        "grid.alpha": "0.5",
    }),
    ("grid_major", {
        "axes.grid": "True",
        "axes.grid.which": "both",
        "xtick.minor.visible": "True",
        "ytick.minor.visible": "True",
        "grid.major.color": "tab:blue",
        "grid.major.linestyle": "-.",
        "grid.major.linewidth": "2",
        "grid.major.alpha": "0.6",
    }),
    ("grid_minor", {
        "axes.grid": "True",
        "axes.grid.which": "both",
        "xtick.minor.visible": "True",
        "ytick.minor.visible": "True",
        "grid.minor.color": "tab:red",
        "grid.minor.linestyle": ":",
        "grid.minor.linewidth": "1.2",
        "grid.minor.alpha": "0.7",
    }),
])
_cases(_minor_labels, SMALL, [
    ("xtick_pads", {"xtick.major.pad": "9", "xtick.minor.pad": "1"}),
    ("ytick_pads", {"ytick.major.pad": "9", "ytick.minor.pad": "1"}),
])

# --- figure and saving --------------------------------------------------------

_cases(_supertitles, LABELLED, [
    ("figure_title_label_style", {
        "figure.titlesize": "small",
        "figure.titleweight": "bold",
        "figure.labelsize": "x-small",
        "figure.labelweight": "bold",
    }),
])
_cases(_curve, SMALL, [
    ("figure_facecolor", {"figure.facecolor": "lightsteelblue"}),
    ("savefig_facecolor", {"savefig.facecolor": "lightgreen", "savefig.dpi": "50"}),
    ("savefig_transparent", {"savefig.transparent": "True"}),
    ("savefig_bbox", {"savefig.bbox": "tight"}),
    ("savefig_pad_inches", {"savefig.bbox": "tight", "savefig.pad_inches": "0.4"}),
])
_cases(_subfigures, SMALL, [
    ("figure_frameon", {"figure.frameon": "False"}),
])
_cases(_framed, SMALL, [
    ("figure_edgecolor", {"figure.edgecolor": "tab:red"}),
    ("savefig_edgecolor", {"savefig.edgecolor": "tab:blue"}),
])
_cases(_panels, PANELS, [
    ("figure_autolayout", {"figure.autolayout": "True"}),
    ("figure_subplot", {
        "figure.subplot.left": "0.2",
        "figure.subplot.right": "0.95",
        "figure.subplot.bottom": "0.15",
        "figure.subplot.top": "0.9",
        "figure.subplot.wspace": "0.5",
        "figure.subplot.hspace": "0.8",
    }),
    ("figure_constrained_layout", {"figure.constrained_layout.use": "True"}),
    ("figure_constrained_layout_pads", {
        "figure.constrained_layout.use": "True",
        "figure.constrained_layout.h_pad": "0.15",
        "figure.constrained_layout.w_pad": "0.15",
    }),
    ("figure_constrained_layout_spaces", {
        "figure.constrained_layout.use": "True",
        "figure.constrained_layout.hspace": "0.3",
        "figure.constrained_layout.wspace": "0.3",
    }),
])

# --- paths --------------------------------------------------------------------

_cases(_sawtooth, SMALL, [
    ("path_simplify_threshold", {"path.simplify_threshold": "1.0"}),
    ("path_simplify", {"path.simplify": "False", "path.simplify_threshold": "1.0"}),
])
_cases(_lissajous, SMALL, [
    ("agg_path_chunksize", {"agg.path.chunksize": "300"}),
])
_cases(_curves, SMALL, [
    ("path_snap", {"path.snap": "False"}),
    ("path_sketch", {"path.sketch": "3, 30, 2"}),
    ("path_effects", {"path.effects": [patheffects.withStroke(linewidth=4, foreground="0.8")]}),
])
