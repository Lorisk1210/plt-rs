"""matplotlib's own image tests from the last part of ``test_axes.py``
(``axline``, ``vlines``/``hlines``, steps, collections, spectral plots,
twins, tick resets, pies, tick labels, rc-driven spines and grids, dashes,
log axes, insets, secondary axes, ``axis`` options, clipping, ``ecdf``,
polar spans, ``clear``, orientation keywords and the colorizer keyword),
ported as figure cases.

Each case is named after the test it ports, without the ``test_`` prefix; a
test drawing several figures becomes ``<name>_1``, ``<name>_2``, ... (one
case per figure; figures that only repeat the previous one through the
``data=`` keyword are not repeated), a parametrized test one case per
parameter set. ``@image_comparison`` tests draw under ``style=("classic",
"_classic_test_patch")`` unless they name another style, and
``@check_figures_equal`` tests under the same style (matplotlib's test
fixture applies it) unless they are wrapped in ``mpl.style.context``; the
style reaches the case through its ``rc`` (see ``cases_styles.py``), with
classic's 8x6 in canvas saved at 100 dpi. "mpl20"/"default" need no rc;
``test_bar_all_nan`` switches to "mpl20" before it draws, so its case uses
that style throughout.
``remove_text=True`` becomes :func:`remove_ticks_and_titles` at the end of
the build, ``savefig_kwarg={"dpi": ...}`` the case's dpi. Only the
``fig_test`` side of ``@check_figures_equal`` tests is ported. numpy's
random data is replaced by ``spec_support.rng`` streams with the tests'
seeds.
"""

import functools
from decimal import Decimal
from itertools import product

import matplotlib as mpl
import matplotlib.collections as mcollections
import matplotlib.colorizer as mcolorizer
import matplotlib.colors as mcolors
import matplotlib.dates as mdates
import matplotlib.lines as mlines
import matplotlib.patches as mpatches
import matplotlib.style
import matplotlib.transforms as mtransforms
import numpy as np
from matplotlib import patheffects
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}
CLASSIC_SIZE = (8, 6)


def classic_case(name, **options):
    """A case under the test suite's classic style on classic's canvas."""
    return case(f"mpl_axes_d/{name}", figsize=options.pop("figsize", CLASSIC_SIZE),
                rc={**CLASSIC, **options.pop("rc", {})}, **options)


def mpl20_case(name, **options):
    """A case under matplotlib's default style."""
    return case(f"mpl_axes_d/{name}", **options)


# --- axline ----------------------------------------------------------------


@classic_case("axline_loglog")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(0.1, 10), ylim=(1e-3, 1))
    ax.loglog([.3, .6], [.3, .6], ".-")
    ax.axline((1, 1e-3), (10, 1e-2), c="k")


@classic_case("axline")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(-1, 1), ylim=(-1, 1))
    ax.axline((0, 0), (1, 1))
    ax.axline((0, 0), (1, 0), color="C1")
    ax.axline((0, 0.5), (1, 0.5), color="C2")
    ax.axline((-0.7, -0.5), slope=0, color="C3")
    ax.axline((1, -0.5), slope=-0.5, color="C4")
    ax.axline((-0.5, 1), slope=float("inf"), color="C5")


@classic_case("axline_transaxes")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(-1, 1), ylim=(-1, 1))
    ax.axline((0, 0), slope=1, transform=ax.transAxes)
    ax.axline((1, 0.5), slope=1, color="C1", transform=ax.transAxes)
    ax.axline((0.5, 0.5), slope=0, color="C2", transform=ax.transAxes)
    ax.axline((0.5, 0), (0.5, 1), color="C3", transform=ax.transAxes)


@classic_case("axline_transaxes_panzoom")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(-1, 1), ylim=(-1, 1))
    ax.axline((0, 0), slope=1, transform=ax.transAxes)
    ax.axline((0.5, 0.5), slope=2, color="C1", transform=ax.transAxes)
    ax.axline((0.5, 0.5), slope=0, color="C2", transform=ax.transAxes)
    ax.set(xlim=(0, 5), ylim=(0, 10))
    fig.set_size_inches(3, 3)


# --- vlines and hlines --------------------------------------------------------


LINES_AT = [2, 3, 4, 5, 6, 7]
LINES_LENGTHS = [
    ([2, -6, 3, 8, np.nan, 2], "g", 5, "-"),
    ([np.nan, 2, -6, 3, 8, 2], "r", 3, "--"),
    ([np.nan, 2, -6, 3, 8, np.nan], "k", 2, "-"),
]


def _lines(ax, horizontal, at, lo, hi, **kwargs):
    """``hlines`` or ``vlines``, whichever the test is about."""
    (ax.hlines if horizontal else ax.vlines)(at, lo, hi, **kwargs)


def _limit_positions(ax, horizontal, lim):
    """Limit the axis along which the lines are placed (the tests' tweak to
    see the lines better)."""
    (ax.set_ylim if horizontal else ax.set_xlim)(*lim)


def _lines_basic(fig, horizontal):
    ax = fig.subplots()
    _lines(ax, horizontal, [2, 3, 4, 5, 7], 0, [2, -6, 3, 8, 2], colors="g", linewidth=5)
    _limit_positions(ax, horizontal, (0, 10))


def _lines_with_nan(fig, horizontal):
    for ax, (lengths, color, width, style) in zip(fig.subplots(nrows=3), LINES_LENGTHS):
        _lines(ax, horizontal, LINES_AT, 0, lengths, colors=color, linewidth=width,
               linestyle=style)
        _limit_positions(ax, horizontal, (0, 10))


def _lines_masked(fig, horizontal):
    ax = fig.subplots()
    at = np.ma.masked_equal([2, 4, 6, 8, 10, 12], 8)
    lo = np.ma.masked_equal([0, 1, -1, 0, 2, 1], 2)
    hi = np.ma.masked_equal([13, 14, 15, 16, 17, 18], 18)
    _lines(ax, horizontal, at, lo, hi, colors="k", linewidth=2)
    _limit_positions(ax, horizontal, (0, 15))


for _name, _horizontal in (("vlines", False), ("hlines", True)):
    mpl20_case(f"{_name}_1")(functools.partial(_lines_basic, horizontal=_horizontal))
    mpl20_case(f"{_name}_2", figsize=(4, 8))(
        functools.partial(_lines_with_nan, horizontal=_horizontal))
    mpl20_case(f"{_name}_3")(functools.partial(_lines_masked, horizontal=_horizontal))


def _lines_with_colors(fig, data):
    colors = ["red", "green", "blue", "purple", "orange"]
    fig.add_subplot(2, 1, 1).vlines(data, 0, 1, colors=colors, linewidth=5)
    fig.add_subplot(2, 1, 2).hlines(data, 0, 1, colors=colors, linewidth=5)


classic_case("lines_with_colors_nan")(
    functools.partial(_lines_with_colors, data=[1, 2, 3, np.nan, 5]))
classic_case("lines_with_colors_masked")(
    functools.partial(_lines_with_colors, data=np.ma.masked_equal([1, 2, 3, 4, 5], 4)))


@mpl20_case("vlines_hlines_blended_transform", figsize=(6, 6))
def _(fig):
    t = np.arange(5.0, 10.0, 0.1)
    s = np.exp(-t) + np.sin(2 * np.pi * t) + 10
    hax, vax = fig.subplots(2, 1)
    hax.plot(t, s, "^")
    hax.hlines([10, 9], xmin=0, xmax=0.5, transform=hax.get_yaxis_transform(), colors="r")
    vax.plot(t, s, "^")
    vax.vlines([6, 7], ymin=0, ymax=0.15, transform=vax.get_xaxis_transform(), colors="r")


# --- steps and collections ------------------------------------------------------


@classic_case("step_linestyle")
def _(fig):
    x = y = np.arange(10)
    for ax, ls in zip(fig.subplots(2, 2).flatten(), ["-", "--", "-.", ":"]):
        ax.step(x, y, lw=5, linestyle=ls, where="pre")
        ax.step(x, y + 1, lw=5, linestyle=ls, where="mid")
        ax.step(x, y + 2, lw=5, linestyle=ls, where="post")
        ax.set_xlim(-1, 5)
        ax.set_ylim(-1, 7)
    remove_ticks_and_titles(fig)


@classic_case("mixed_collection")
def _(fig):
    ax = fig.subplots()
    c = mpatches.Circle((8, 8), radius=4, facecolor="none", edgecolor="green")
    p1 = mcollections.PatchCollection([c], match_original=True)
    p1.set_offsets([[0, 0], [24, 24]])
    p1.set_linewidths([1, 5])
    p2 = mcollections.PatchCollection([c], match_original=True)
    p2.set_offsets([[48, 0], [-32, -16]])
    p2.set_linewidths([1, 5])
    p2.set_edgecolors([[0, 0, 0.1, 1.0], [0, 0, 0.1, 0.5]])
    ax.patch.set_color("0.5")
    ax.add_collection(p1)
    ax.add_collection(p2)
    ax.set_xlim(0, 16)
    ax.set_ylim(0, 16)
    remove_ticks_and_titles(fig)


# --- spectral plots ---------------------------------------------------------------


def _summed_sines(fstims, x):
    """One sum of sines of ``x`` per row of frequencies in ``fstims``."""
    return np.sin(2 * np.pi * np.multiply.outer(fstims, x)).sum(axis=1)


def _noise(n):
    """The tests' ``standard_normal(n)`` followed by ``rand(n)`` (seed 0)."""
    stream = rng(0)
    return [stream.normal(n), stream.uniform(n)]


def _pad_to(nfft):
    """The next power of two of ``nfft``."""
    return int(2 ** np.ceil(np.log2(nfft)))


SPECGRAM_N = 1000
SPECGRAM_FS = 10.
SPECGRAM_FSTIMS = [[SPECGRAM_FS / 4, SPECGRAM_FS / 5, SPECGRAM_FS / 11],
                   [SPECGRAM_FS / 4.7, SPECGRAM_FS / 5.6, SPECGRAM_FS / 11.9]]
SIDES = ["default", "onesided", "twosided"]


def _specgram_freqs(nfft_scale, last_sample=None):
    """``(y, NFFT)``: the tests' summed sines end to end (each row's last
    sample optionally replaced) at ``NFFT = nfft_scale * Fs / min(fstims)``."""
    y = _summed_sines(SPECGRAM_FSTIMS, np.arange(0, SPECGRAM_N, 1 / SPECGRAM_FS))
    if last_sample is not None:
        y[:, -1] = last_sample
    return np.hstack(y), int(nfft_scale * SPECGRAM_FS / np.min(SPECGRAM_FSTIMS))


def _specgram_noise():
    """``(y, NFFT)``: the tests' noise."""
    return np.concatenate(_noise(SPECGRAM_N)), int(10 * SPECGRAM_FS / 11)


def _specgrams(fig, signal, kwargs):
    """Three stacked specgrams of ``signal()``, one per ``sides``."""
    y, nfft = signal()
    for ax, sides in zip(fig.subplots(3), SIDES):
        ax.specgram(y, NFFT=nfft, Fs=SPECGRAM_FS, noverlap=nfft // 2, pad_to=_pad_to(nfft),
                    sides=sides, **kwargs)
    remove_ticks_and_titles(fig)


LINEAR_LOG = {"scale": "linear", "norm": "log"}
#: Per test: its freqs signal and the keywords of its two figures per signal.
SPECGRAM_TESTS = {
    "specgram": (functools.partial(_specgram_freqs, 10), [{}, LINEAR_LOG]),
    "specgram_magnitude": (functools.partial(_specgram_freqs, 100, 1),
                           [{"mode": "magnitude"}, {"mode": "magnitude", **LINEAR_LOG}]),
    "specgram_angle": (functools.partial(_specgram_freqs, 10, 1),
                       [{"mode": "angle"}, {"mode": "phase"}]),
}
for _name, (_freqs, _modes) in SPECGRAM_TESTS.items():
    for _index, _figure in enumerate(product([_freqs, _specgram_noise], _modes), start=1):
        mpl20_case(f"{_name}_{_index}", rc={"image.interpolation": "nearest"})(
            functools.partial(_specgrams, signal=_figure[0], kwargs=_figure[1]))


@classic_case("specgram_origin_rcparam", rc={"image.origin": "lower"})
def _(fig):
    fig.subplots().specgram(np.sin(np.arange(500)))


PSD_N = 10000
PSD_FS = 100.
PSD_FSTIMS = [[PSD_FS / 4, PSD_FS / 5, PSD_FS / 11],
              [PSD_FS / 4.7, PSD_FS / 5.6, PSD_FS / 11.9]]


def _psd_signals(source):
    """``(ys, NFFT)``: two summed-sines signals or two noise signals."""
    if source == "freqs":
        x = np.arange(0, PSD_N, 1 / PSD_FS)
        return _summed_sines(PSD_FSTIMS, x), int(1000 * PSD_FS / np.min(PSD_FSTIMS))
    return _noise(PSD_N), int(1000 * PSD_FS / 11)


def _psd_csd(fig, source, method):
    ys, nfft = _psd_signals(source)
    for ax, sides in zip(fig.subplots(3), SIDES):
        args = (np.concatenate(ys),) if method == "psd" else tuple(ys)
        getattr(ax, method)(*args, NFFT=nfft, Fs=PSD_FS, noverlap=nfft // 2,
                            pad_to=_pad_to(nfft), sides=sides)
        ax.set(xlabel="", ylabel="")
    remove_ticks_and_titles(fig)


for _index, (_source, _method) in enumerate(product(["freqs", "noise"], ["psd", "csd"]),
                                            start=1):
    classic_case(f"psd_csd_{_index}")(
        functools.partial(_psd_csd, source=_source, method=_method))


def _spectrum_signal(source):
    """The tests' three weighted sines or their noise minus one half."""
    if source == "noise":
        return np.hstack(_noise(PSD_N)) - .5
    fstims = [PSD_FS / 4, PSD_FS / 5, PSD_FS / 11]
    x = np.arange(0, PSD_N, 1 / PSD_FS)
    return (np.sin(2 * np.pi * np.outer(x, fstims)) * 10 ** np.arange(3)).sum(axis=1)


SPECTRUM_PAD_TO = _pad_to(int(1000 * PSD_FS / (PSD_FS / 11)))
SPECTRUM_KINDS = [("magnitude_spectrum", {}), ("magnitude_spectrum", {"scale": "dB"}),
                  ("angle_spectrum", {}), ("phase_spectrum", {})]


def _spectrum(fig, source, method, kwargs):
    y = _spectrum_signal(source)
    for ax, sides in zip(fig.subplots(3), SIDES):
        getattr(ax, method)(y, sides=sides, Fs=PSD_FS, pad_to=SPECTRUM_PAD_TO, **kwargs)
        ax.set(xlabel="", ylabel="")
    remove_ticks_and_titles(fig)


for _index, (_source, (_method, _kwargs)) in enumerate(
        product(["freqs", "noise"], SPECTRUM_KINDS), start=1):
    classic_case(f"spectrum_{_index}")(
        functools.partial(_spectrum, source=_source, method=_method, kwargs=_kwargs))


# --- twins and tick resets ------------------------------------------------------


@classic_case("twin_remove")
def _(fig):
    ax = fig.add_subplot()
    ax_twinx = ax.twinx()
    ax_twiny = ax.twiny()
    ax_twinx.remove()
    ax_twiny.remove()


@classic_case("twin_spines", figsize=(4, 3))
def _(fig):
    fig.subplots_adjust(right=0.75)
    host = fig.add_subplot()
    par1 = host.twinx()
    par2 = host.twinx()
    par2.spines.right.set_position(("axes", 1.2))
    par2.set_frame_on(True)
    par2.patch.set_visible(False)
    par2.spines[:].set_visible(False)
    par2.spines.right.set_visible(True)
    p1, = host.plot([0, 1, 2], [0, 1, 2], "b-")
    p2, = par1.plot([0, 1, 2], [0, 3, 2], "r-")
    p3, = par2.plot([0, 1, 2], [50, 30, 15], "g-")
    host.set_xlim(0, 2)
    host.set_ylim(0, 2)
    par1.set_ylim(0, 4)
    par2.set_ylim(1, 65)
    for ax, line in ((host, p1), (par1, p2), (par2, p3)):
        ax.yaxis.label.set_color(line.get_color())
        ax.tick_params(axis="y", colors=line.get_color(), size=4, width=1.5)
    host.tick_params(axis="x", size=4, width=1.5)
    remove_ticks_and_titles(fig)


@classic_case("twin_spines_on_top", rc={"axes.linewidth": 48.0, "lines.linewidth": 48.0})
def _(fig):
    ax1 = fig.add_subplot(1, 1, 1)
    x = np.array([1000, 1100, 1200, 1250])
    y = np.array([310, 301, 360, 400]) / 1E3
    ax2 = ax1.twinx()
    ax1.plot(x, y, color="#BEAED4")
    ax1.fill_between(x, y, color="#BEAED4", alpha=.8)
    ax2.plot(x, y, color="#7FC97F")
    ax2.fill_between(x, y, color="#7FC97F", alpha=.5)
    remove_ticks_and_titles(fig)


@classic_case("reset_ticks")
def _(fig):
    ax = fig.add_subplot()
    ax.grid(True)
    ax.tick_params(
        direction="in", length=10, width=5, color="C0", pad=12,
        labelsize=14, labelcolor="C1", labelrotation=45,
        grid_color="C2", grid_alpha=0.8, grid_linewidth=3,
        grid_linestyle="--")
    fig.draw_without_rendering()
    ax.xaxis.reset_ticks()
    ax.yaxis.reset_ticks()


# --- pies --------------------------------------------------------------------------


PIE_LABELS = "Frogs", "Hogs", "Dogs", "Logs"
PIE_SIZES = [15, 30, 45, 10]
PIE_COLORS = ["yellowgreen", "gold", "lightskyblue", "lightcoral"]
PIE_EXPLODE = (0, 0.1, 0, 0)  # only "explode" the 2nd slice


def _pie(ax, **kwargs):
    """The tests' exploded pie with percentages and a shadow."""
    options = {"explode": PIE_EXPLODE, "labels": PIE_LABELS, "colors": PIE_COLORS,
               "autopct": "%1.1f%%", "shadow": True, "startangle": 90, **kwargs}
    return ax.pie(options.pop("sizes", PIE_SIZES), **options)


@mpl20_case("pie_default", figsize=(8, 6))
def _(fig):
    _pie(fig.subplots())


def _equal_pie(fig, **kwargs):
    """A pie on ``gca`` with equal aspect (``plt.pie``, ``plt.axis("equal")``)."""
    ax = fig.gca()
    _pie(ax, **kwargs)
    ax.axis("equal")
    return ax


mpl20_case("pie_linewidth_0")(lambda fig: _equal_pie(fig, wedgeprops={"linewidth": 0}))
mpl20_case("pie_linewidth_2")(lambda fig: _equal_pie(fig, wedgeprops={"linewidth": 2}))
mpl20_case("pie_ccw_true")(lambda fig: _equal_pie(fig, counterclock=True))
mpl20_case("pie_rotatelabels_true")(
    lambda fig: _equal_pie(fig, labels=("Hogwarts", "Frogs", "Dogs", "Logs"),
                           rotatelabels=True))


@mpl20_case("pie_center_radius")
def _(fig):
    ax = fig.gca()
    _pie(ax, wedgeprops={"linewidth": 0}, center=(1, 2), radius=1.5)
    ax.annotate("Center point", xy=(1, 2), xytext=(1, 1.3),
                arrowprops=dict(arrowstyle="->", connectionstyle="arc3"),
                bbox=dict(boxstyle="square", facecolor="lightgrey"))
    ax.axis("equal")


@mpl20_case("pie_frame_grid")
def _(fig):
    ax = fig.gca()
    framed = {"wedgeprops": {"linewidth": 0}, "frame": True}
    _pie(ax, center=(2, 2), **framed)
    _pie(ax, sizes=PIE_SIZES[::-1], center=(5, 2), **framed)
    _pie(ax, explode=PIE_EXPLODE[::-1], center=(3, 5), **framed)
    ax.axis("equal")


@mpl20_case("pie_nolabel_but_legend")
def _(fig):
    ax = _equal_pie(fig, labeldistance=None, rotatelabels=True)
    ax.set_ylim(-1.2, 1.2)
    ax.legend()


@mpl20_case("pie_shadow")
def _(fig):
    axes = fig.subplots(2, 2)
    plain = {"explode": PIE_EXPLODE, "colors": PIE_COLORS, "startangle": 90,
             "wedgeprops": {"linewidth": 0}}
    axes[0][0].pie(PIE_SIZES, shadow=True, **plain)
    axes[0][1].pie(PIE_SIZES, shadow=False, **plain)
    axes[1][0].pie(PIE_SIZES, shadow={"ox": -0.05, "oy": -0.05, "shade": 0.9,
                                      "edgecolor": "none"}, **plain)
    axes[1][1].pie(PIE_SIZES, shadow={"ox": 0.05, "linewidth": 2, "shade": 0.2}, **plain)


classic_case("pie_hatch_single")(lambda fig: fig.subplots().pie([0.3, 0.3, 0.1], hatch="+"))
classic_case("pie_hatch_multi")(
    lambda fig: fig.subplots().pie([0.3, 0.3, 0.1], hatch=["/", "+", "."]))


# --- tick labels ---------------------------------------------------------------


@mpl20_case("set_get_ticklabels")
def _(fig):
    ax = fig.subplots(2)
    for axes, title in zip(ax, ["normal", "set_x/yticklabels"]):
        axes.plot(np.arange(10))
        axes.set_title(title)
    ax[0].set_xticks(range(10))
    ax[0].set_yticks(range(10))
    ax[0].set_xticklabels(["a", "b", "c", "d"] + 6 * [""])
    ax[0].set_yticklabels(["11", "12", "13", "14"] + 6 * [""])
    ax[1].set_xticks(ax[0].get_xticks())
    ax[1].set_yticks(ax[0].get_yticks())
    ax[1].set_xticklabels(ax[0].get_xticklabels())
    ax[1].set_yticklabels(ax[0].get_yticklabels())


@classic_case("set_ticks_with_labels")
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([1, 2, 4, 6], ["a", "b", "c", "d"], fontweight="bold")
    ax.set_yticks([1, 3, 5])
    ax.set_yticks([2, 4], ["A", "B"], minor=True)


@mpl20_case("retain_tick_visibility")
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2], [0, -1, 4])
    for label in ax.get_yticklabels():
        label.set_visible(False)
    ax.tick_params(axis="y", which="both", length=0)


@mpl20_case("o_marker_path_snap", dpi=72)
def _(fig):
    ax = fig.subplots()
    ax.margins(.1)
    for ms in range(1, 15):
        ax.plot([1, 2], np.ones(2) + ms, "o", ms=ms)
    for ms in np.linspace(1, 10, 25):
        ax.plot([3, 4], np.ones(2) + ms, "o", ms=ms)


# --- rc-driven spines and grids -------------------------------------------------


@mpl20_case("rc_spines", dpi=40, rc={f"axes.spines.{side}": False
                                     for side in ("left", "right", "top", "bottom")})
def _(fig):
    fig.subplots()


@mpl20_case("rc_grid", dpi=40)
def _(fig):
    for i, axis in enumerate(["both", "x", "y"], 1):
        with mpl.rc_context({"axes.grid": True, "axes.grid.axis": axis}):
            fig.add_subplot(3, 1, i)


# --- dashes, log axes and drawing order -------------------------------------------


@classic_case("dash_offset")
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 10)
    y = np.ones_like(x)
    for j in range(0, 100, 2):
        ax.plot(x, j * y, ls=(j, (10, 10)), lw=5, color="k")
    remove_ticks_and_titles(fig)


@classic_case("loglog")
def _(fig):
    ax = fig.subplots()
    x = np.arange(1, 11)
    ax.loglog(x, x ** 3, lw=5)
    ax.tick_params(length=25, width=2)
    ax.tick_params(length=15, width=2, which="minor")
    remove_ticks_and_titles(fig)


@mpl20_case("loglog_nonpos")
def _(fig):
    x = np.arange(1, 11)
    y = x ** 3
    y[7] = -3.
    x[4] = -10
    for (mcy, mcx), ax in zip(product(["mask", "clip", ""], repeat=2), fig.subplots(3, 3).flat):
        if mcx == mcy:
            if mcx:
                ax.loglog(x, y ** 3, lw=2, nonpositive=mcx)
            else:
                ax.loglog(x, y ** 3, lw=2)
        else:
            ax.loglog(x, y ** 3, lw=2)
            if mcx:
                ax.set_xscale("log", nonpositive=mcx)
            if mcy:
                ax.set_yscale("log", nonpositive=mcy)
        ax.set_yticks([1e3, 1e7])
    remove_ticks_and_titles(fig)


@classic_case("axisbelow")
def _(fig):
    axs = fig.subplots(ncols=3, sharex=True, sharey=True)
    for ax, setting in zip(axs, (False, "line", True)):
        ax.plot((0, 10), (0, 10), lw=10, color="m")
        ax.add_patch(mpatches.Circle((3, 3), color="r"))
        ax.grid(color="c", linestyle="-", linewidth=3)
        ax.tick_params(top=False, bottom=False, left=False, right=False)
        ax.spines[:].set_visible(False)
        ax.set_axisbelow(setting)
    remove_ticks_and_titles(fig)


@mpl20_case("stairs_fill_zero_linewidth")
def _(fig):
    fig.subplots().stairs([1, 2, 3, 4], [1, 2, 3, 4, 5], fill=True, ls="--")


@classic_case("plot_decimal")
def _(fig):
    x0 = np.arange(-10, 10, 0.3)
    y0 = [5.2 * x ** 3 - 2.1 * x ** 2 + 7.34 * x + 4.5 for x in x0]
    fig.subplots().plot([Decimal(i) for i in x0], [Decimal(i) for i in y0])


@classic_case("markerfacecolor_none_alpha")
def _(fig):
    fig.subplots().plot(0, "o", mfc="none", alpha=.5)


# --- insets, annotations and secondary axes ------------------------------------------


@mpl20_case("inset_polar")
def _(fig):
    ax = fig.subplots()
    axins = ax.inset_axes([0.5, 0.1, 0.45, 0.45], polar=True)
    r = np.arange(0, 2, 0.01)
    theta = 2 * np.pi * r
    ax.plot(theta, r)
    axins.plot(theta, r)
    remove_ticks_and_titles(fig)


@mpl20_case("annotate_across_transforms", figsize=(3.39, 3))
def _(fig):
    x = np.linspace(0, 10, 200)
    y = np.exp(-x) * np.sin(x)
    ax = fig.subplots()
    ax.plot(x, y)
    axins = ax.inset_axes([0.4, 0.5, 0.3, 0.3])
    axins.set_aspect(0.2)
    axins.xaxis.set_visible(False)
    axins.yaxis.set_visible(False)
    ax.annotate("", xy=(x[150], y[150]), xycoords=ax.transData,
                xytext=(1, 0), textcoords=axins.transAxes,
                arrowprops=dict(arrowstyle="->"))
    remove_ticks_and_titles(fig)


class _Translation(mtransforms.Transform):
    """The test's 1D translation transform."""

    input_dims = 1
    output_dims = 1

    def __init__(self, dx):
        super().__init__()
        self.dx = dx

    def transform(self, values):
        return values + self.dx

    def inverted(self):
        return _Translation(-self.dx)


def _invert(x):
    with np.errstate(divide="ignore"):
        return 1 / x


@mpl20_case("secondary_xy", figsize=(10, 5))
def _(fig):
    fig.set_layout_engine("constrained")
    for nn, ax in enumerate(fig.subplots(1, 2)):
        ax.plot(np.arange(2, 11), np.arange(2, 11))
        secax = ax.secondary_xaxis if nn == 0 else ax.secondary_yaxis
        secax(0.2, functions=(_invert, _invert))
        secax(0.4, functions=(lambda x: 2 * x, lambda x: x / 2))
        secax(0.6, functions=(lambda x: x ** 2, lambda x: x ** (1 / 2)))
        secax(0.8)
        secax("top" if nn == 0 else "right", functions=_Translation(2))
        secax(6.25, transform=ax.transData)


# --- axis options ----------------------------------------------------------------


@mpl20_case("axis_options")
def _(fig):
    axes = fig.subplots(2, 3)
    for i, option in enumerate(("scaled", "tight", "image")):
        axes[0, i].plot((1, 2), (1, 3.2))
        axes[0, i].axis(option)
        axes[0, i].add_artist(mpatches.Circle((1.5, 1.5), radius=0.5,
                                              facecolor="none", edgecolor="k"))
        axes[1, i].plot((1, 2.25), (1, 1.75))
        axes[1, i].axis(option)
        axes[1, i].add_artist(mpatches.Circle((1.5, 1.25), radius=0.25,
                                              facecolor="none", edgecolor="k"))
    remove_ticks_and_titles(fig)


@classic_case("axis_bool_arguments")
def _(fig):
    fig.add_subplot(211).axis(False)
    ax = fig.add_subplot(212)
    ax.axis(False)
    ax.axis(True)


@classic_case("polar_interpolation_steps_variable_r")
def _(fig):
    line, = fig.add_subplot(projection="polar").plot([0, np.pi / 2], [1, 2])
    line.get_path()._interpolation_steps = 100


@classic_case("2dcolor_plot")
def _(fig):
    color = np.array([0.1, 0.2, 0.3])
    axs = fig.subplots(5)
    axs[0].plot([1, 2], [1, 2], c=color)
    axs[1].scatter([1, 2], [1, 2], c=color.reshape((1, -1)))
    axs[2].step([1, 2], [1, 2], c=color)
    axs[3].hist(np.arange(10), color=color)
    axs[4].bar(np.arange(10), np.arange(10), color=color)


@classic_case("shared_axes_clear")
def _(fig):
    x = np.arange(0.0, 2 * np.pi, 0.01)
    for ax in fig.subplots(2, 2, sharex=True, sharey=True).flat:
        ax.clear()
        ax.plot(x, np.sin(x))


# --- bars, images and clipping ----------------------------------------------------


@mpl20_case("centered_bar_label_nonlinear")
def _(fig):
    ax = fig.subplots()
    bar_container = ax.barh(["c", "b", "a"], [1_000, 5_000, 7_000])
    ax.set_xscale("log")
    ax.set_xlim(1, None)
    ax.bar_label(bar_container, label_type="center")
    ax.set_axis_off()


@mpl20_case("bar_all_nan")
def _(fig):
    ax = fig.subplots()
    ax.bar([np.nan], [np.nan])
    ax.bar([1], [1])


@mpl20_case("extent_units")
def _(fig):
    axs = fig.subplots(2, 2)
    date_first = np.datetime64("2020-01-01", "D")
    date_last = np.datetime64("2020-01-11", "D")
    arr = [[i + j for i in range(10)] for j in range(10)]
    plasma = mpl.colormaps["plasma"]
    day = mdates.DateFormatter("%d")

    axs[0, 0].set_title("Date extents on y axis")
    axs[0, 0].imshow(arr, origin="lower", extent=[1, 11, date_first, date_last], cmap=plasma)

    axs[0, 1].set_title("Date extents on x axis (Day of Jan 2020)")
    axs[0, 1].imshow(arr, origin="lower", extent=[date_first, date_last, 1, 11], cmap=plasma)
    axs[0, 1].xaxis.set_major_formatter(day)

    axs[1, 0].imshow(arr, origin="lower", cmap=plasma,
                     extent=[date_first, date_last, date_first, date_last])
    axs[1, 0].xaxis.set_major_formatter(day)
    axs[1, 0].set(xlabel="Day of Jan 2020")

    im = axs[1, 1].imshow(arr, origin="lower", cmap=plasma)
    im.set_extent([date_last, date_first, date_last, date_first])
    axs[1, 1].xaxis.set_major_formatter(day)
    axs[1, 1].set(xlabel="Day of Jan 2020")


@mpl20_case("preset_clip_paths")
def _(fig):
    ax = fig.subplots()
    poly = mpatches.Polygon(
        [[1, 0], [0, 1], [-1, 0], [0, -1]], facecolor="#ddffdd",
        edgecolor="#00ff00", linewidth=2, alpha=0.5)
    ax.add_patch(poly)
    line = mlines.Line2D((-1, 1), (0.5, 0.5), clip_on=True, clip_path=poly)
    line.set_path_effects([patheffects.withTickedStroke()])
    ax.add_artist(line)
    line = mlines.Line2D((-1, 1), (-0.5, -0.5), color="r", clip_on=True, clip_path=poly)
    ax.add_artist(line)
    poly2 = mpatches.Polygon(
        [[-1, 1], [0, 1], [0, -0.25]], facecolor="#beefc0", alpha=0.3,
        edgecolor="#faded0", linewidth=2, clip_on=True, clip_path=poly)
    ax.add_artist(poly2)
    ax.annotate("Annotation", (-0.75, -0.75), xytext=(0.1, 0.75),
                arrowprops={"color": "k"}, clip_on=True, clip_path=poly)
    poly3 = mpatches.Polygon(
        [[0, 0], [0, 0.5], [0.5, 0.5], [0.5, 0]], facecolor="g", edgecolor="y",
        linewidth=2, alpha=0.3, clip_on=True, clip_path=poly)
    fig.add_artist(poly3, clip=True)
    ax.set_xlim(-1, 1)
    ax.set_ylim(-1, 1)
    remove_ticks_and_titles(fig)


@classic_case("ecdf")
def _(fig):
    data = np.array([0, -np.inf, -np.inf, np.inf, 1, 1, 2])
    weights = range(len(data))
    for ax, orientation in zip(fig.subplots(1, 2), ["vertical", "horizontal"]):
        ax.ecdf(data, orientation=orientation)
        ax.ecdf("d", "w", data={"d": np.ma.array(data), "w": weights},
                orientation=orientation, complementary=True, compress=True, ls=":")


@mpl20_case("axhvlinespan_interpolation")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.set_axis_off()
    ax.axvline(.1, c="C0")
    ax.axvspan(.2, .3, fc="C1")
    ax.axvspan(.4, .5, .1, .2, fc="C2")
    ax.axhline(1, c="C0", alpha=.5)
    ax.axhspan(.8, .9, fc="C1", alpha=.5)
    ax.axhspan(.6, .7, .8, .9, fc="C2", alpha=.5)


# --- clear, orientation keywords, colorizers and positions ------------------------


CLEAR_TICK_PARAMS = {
    "direction": "in", "length": 10, "width": 10, "color": "xkcd:wine red", "pad": 0,
    "labelfontfamily": "serif", "zorder": 7, "labelrotation": 45,
    "labelcolor": "xkcd:shocking pink", "grid_color": "xkcd:fluorescent green",
    "grid_alpha": 0.5, "grid_linewidth": 3, "grid_linestyle": ":",
    "bottom": False, "top": True, "left": False, "right": True,
    "labelbottom": True, "labeltop": True, "labelleft": True, "labelright": True,
}


def _axes_clear_behavior(fig, which):
    ax = fig.subplots()
    ax.tick_params(axis=which, **CLEAR_TICK_PARAMS)
    ax.clear()
    ax.grid(True)


for _which in ("x", "y"):
    classic_case(f"axes_clear_behavior_{_which}")(
        functools.partial(_axes_clear_behavior, which=_which))


def _orientation_data():
    """The tests' four normal samples of 100 with std 6 to 9."""
    stream = rng(19680801)
    return [stream.normal(100, 0, std) for std in range(6, 10)]


@classic_case("violinplot_orientation")
def _(fig):
    fig.subplots().violinplot(_orientation_data(), orientation="horizontal")


@classic_case("boxplot_orientation")
def _(fig):
    fig.subplots().boxplot(_orientation_data(), orientation="horizontal")


@classic_case("use_colorizer_keyword")
def _(fig):
    stream = rng(0)
    rand_x = stream.uniform(100)
    rand_y = stream.uniform(100)
    c = np.arange(25, dtype="float32").reshape((5, 5))
    axes = fig.subplots(3, 4)
    cl = mcolorizer.Colorizer(norm=mcolors.Normalize(4, 20), cmap="RdBu")
    axes[0, 0].scatter(c, c, c=c, colorizer=cl)
    axes[0, 1].hexbin(rand_x, rand_y, colorizer=cl, gridsize=(2, 2))
    axes[0, 2].imshow(c, colorizer=cl)
    axes[0, 3].pcolor(c, colorizer=cl)
    axes[1, 0].pcolormesh(c, colorizer=cl)
    axes[1, 1].pcolorfast(c, colorizer=cl)
    axes[1, 2].pcolorfast((0, 1, 2, 3, 4, 5), (0, 1, 2, 3, 5, 6), c, colorizer=cl)
    axes[1, 3].pcolorfast(c.T, c, c[:4, :4], colorizer=cl)
    axes[2, 0].contour(c, colorizer=cl)
    axes[2, 1].contourf(c, colorizer=cl)
    axes[2, 2].tricontour(c.T.ravel(), c.ravel(), c.ravel(), colorizer=cl)
    axes[2, 3].tricontourf(c.T.ravel(), c.ravel(), c.ravel(), colorizer=cl)
    fig.figimage(np.repeat(np.repeat(c, 15, axis=0), 15, axis=1), colorizer=cl)
    remove_ticks_and_titles(fig)


@classic_case("axes_set_position_external_bbox_unchanged")
def _(fig):
    bbox = mtransforms.Bbox([[0.0, 0.0], [1.0, 1.0]])
    ax = fig.add_axes(bbox)
    ax.set_position([0.25, 0.25, 0.5, 0.5])
