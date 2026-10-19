"""matplotlib's own image tests of ``test_axes.py`` (bars and their tick
labels, grouped bars, histograms, stairs, contour hatching and colorbar
lines, hist2d, scatter, stackplot and ``bxp``), ported as figure cases.

Each case is named after the test it ports, without the ``test_`` prefix.
``@image_comparison`` tests draw under ``style=("classic",
"_classic_test_patch")`` unless they name another style, and
``@check_figures_equal`` tests under the same style (matplotlib's test
fixture applies it) unless they are wrapped in ``mpl.style.context``; the
style reaches the case through its ``rc`` (see ``cases_styles.py``), with
classic's 8x6 in canvas saved at 100 dpi. "mpl20"/"default" need no rc.
``remove_text=True`` becomes :func:`remove_ticks_and_titles` at the end of
the build, ``savefig_kwarg={"dpi": ...}`` the case's dpi. Only the
``fig_test`` side of ``@check_figures_equal`` tests is ported. numpy's
random data is replaced by ``spec_support.rng`` streams with the tests'
seeds.
"""

import functools
from decimal import Decimal

import matplotlib as mpl
import matplotlib.lines as mlines
import matplotlib.markers as mmarkers
import matplotlib.style
import numpy as np
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}
CLASSIC_SIZE = (8, 6)


def classic_case(name, **options):
    """A case under the test suite's classic style on classic's canvas."""
    return case(f"mpl_axes_b/{name}", figsize=CLASSIC_SIZE,
                rc={**CLASSIC, **options.pop("rc", {})}, **options)


def mpl20_case(name, **options):
    """A case under matplotlib's default style."""
    return case(f"mpl_axes_b/{name}", **options)


# --- bars ------------------------------------------------------------------


def _bar_tick_label_multiple(fig):
    ax = fig.add_subplot()
    ax.bar([1, 2.5], [1, 2], width=[0.2, 0.5], tick_label=["a", "b"], align="center")


mpl20_case("bar_tick_label_multiple")(_bar_tick_label_multiple)
mpl20_case("bar_tick_label_multiple_old_alignment",
           rc={"ytick.alignment": "center"})(_bar_tick_label_multiple)

DECIMAL_X = [Decimal(v) for v in [1.5, 8.4, 5.3, 4.2]]
DECIMAL_Y = [Decimal(v) for v in [1.1, 2.2, 3.3, 4.4]]
FLOAT_X = [1.5, 8.4, 5.3, 4.2]
FLOAT_Y = [1.1, 2.2, 3.3, 4.4]
DECIMAL_SIZES = [Decimal(v) for v in [0.7, 1.45, 1, 2]]


@classic_case("bar_decimal_center")
def _(fig):
    fig.subplots().bar(DECIMAL_X, DECIMAL_Y, align="center")


@classic_case("barh_decimal_center")
def _(fig):
    fig.subplots().barh(DECIMAL_X, DECIMAL_Y, height=[0.5, 0.5, 1, 1], align="center")


@classic_case("bar_decimal_width")
def _(fig):
    fig.subplots().bar(FLOAT_X, FLOAT_Y, width=DECIMAL_SIZES, align="center")


@classic_case("barh_decimal_height")
def _(fig):
    fig.subplots().barh(FLOAT_X, FLOAT_Y, height=DECIMAL_SIZES, align="center")


@mpl20_case("barh_tick_label")
def _(fig):
    ax = fig.add_subplot()
    ax.barh([1, 2.5], [1, 2], height=[0.2, 0.5], tick_label=["a", "b"], align="center")


@mpl20_case("grouped_bar")
def _(fig):
    data = {"data1": [1, 2, 3], "data2": [1.2, 2.2, 3.2], "data3": [1.4, 2.4, 3.4]}
    ax = fig.subplots()
    ax.grouped_bar(data, tick_labels=["A", "B", "C"], group_spacing=0.5, bar_spacing=0.1,
                   colors=["#1f77b4", "#58a1cf", "#abd0e6"])
    ax.set_yticks([])


GROUPED = {"data1": [1, 1.2], "data2": [2, 2.4], "data3": [3, 3.6]}


@classic_case("grouped_bar_list_of_datasets")
def _(fig):
    ax = fig.subplots()
    ax.grouped_bar(list(GROUPED.values()), tick_labels=["A", "B"], labels=list(GROUPED))
    ax.legend()


@classic_case("grouped_bar_list_of_datasets_ref")
def _(fig):
    ax = fig.subplots()
    label_pos = np.array([0, 1])
    bar_width = 1 / (3 + 1.5)  # 3 bars + 1.5 group_spacing
    data_shift = -1 * bar_width + np.array([0, bar_width, 2 * bar_width])
    for shift, (label, values) in zip(data_shift, GROUPED.items()):
        ax.bar(label_pos + shift, values, width=bar_width, label=label)
    ax.set_xticks(label_pos, ["A", "B"])
    ax.legend()


@classic_case("grouped_bar_dict_of_datasets")
def _(fig):
    ax = fig.subplots()
    ax.grouped_bar(GROUPED, tick_labels=["A", "B"])
    ax.legend()


@classic_case("grouped_bar_array")
def _(fig):
    ax = fig.subplots()
    ax.grouped_bar(np.array([[1, 2, 3], [1.2, 2.4, 3.6]]), tick_labels=["A", "B"],
                   labels=list(GROUPED))
    ax.legend()


@mpl20_case("bar_hatches")
def _(fig):
    fig.subplots().bar([1, 2], [2, 3], hatch=["x", "o"])


# --- histograms --------------------------------------------------------------


@classic_case("hist_log")
def _(fig):
    data0 = np.linspace(0, 1, 200) ** 3
    data = np.concatenate([1 - data0, 1 + data0])
    fig.subplots().hist(data, fill=False, log=True)
    remove_ticks_and_titles(fig)


@classic_case("hist_log_2")
def _(fig):
    axs = fig.subplots(2, 3)
    for i, histtype in enumerate(["bar", "step", "stepfilled"]):
        axs[0, i].set_yscale("log")
        axs[0, i].hist(1, 1, histtype=histtype)
        axs[1, i].hist(1, 1, histtype=histtype)
        axs[1, i].set_yscale("log")


@classic_case("hist_bar_empty")
def _(fig):
    fig.add_subplot().hist([], histtype="bar")
    remove_ticks_and_titles(fig)


@classic_case("hist_step_empty")
def _(fig):
    fig.add_subplot().hist([], histtype="step")
    remove_ticks_and_titles(fig)


@classic_case("hist_step_filled")
def _(fig):
    x = rng(0).normal(1000 * 3).reshape(1000, 3)
    kwargs = [{"fill": True}, {"fill": False}, {"fill": None}, {}] * 2
    types = ["step"] * 4 + ["stepfilled"] * 4
    axs = fig.subplots(nrows=2, ncols=4)
    for kg, histtype, ax in zip(kwargs, types, axs.flat):
        ax.hist(x, 10, histtype=histtype, stacked=True, **kg)
        ax.set_title(f"{kg}/{histtype}")
        ax.set_ylim(bottom=-50)
    remove_ticks_and_titles(fig)


@mpl20_case("hist_density")
def _(fig):
    fig.subplots().hist(rng(19680801).normal(2000), density=True)


# --- stairs ----------------------------------------------------------------


STAIRS_Y = np.array([6, 14, 32, 37, 48, 32, 21, 4])
STAIRS_X = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0])
#: The defaults of ``PathPatch`` for the reference lines.
STEP_STYLE = {"solid_joinstyle": "miter", "solid_capstyle": "butt"}


@classic_case("stairs")
def _(fig):
    y, x = STAIRS_Y, STAIRS_X
    axs = fig.subplots(3, 2).flatten()
    axs[0].stairs(y, x, baseline=None)
    axs[1].stairs(y, x, baseline=None, orientation="horizontal")
    axs[2].stairs(y, x)
    axs[3].stairs(y, x, orientation="horizontal")
    axs[4].stairs(y, x)
    axs[4].semilogy()
    axs[5].stairs(y, x, orientation="horizontal")
    axs[5].semilogx()


@classic_case("stairs_ref")
def _(fig):
    y, x = STAIRS_Y, STAIRS_X
    axs = fig.subplots(3, 2).flatten()
    for ax in axs[0::2]:
        ax.plot(x, np.append(y, y[-1]), drawstyle="steps-post", **STEP_STYLE)
    for ax in axs[1::2]:
        ax.plot(np.append(y[0], y), x, drawstyle="steps-post", **STEP_STYLE)
    for ax in axs[2::2]:
        ax.add_line(mlines.Line2D([x[0], x[0]], [0, y[0]], **STEP_STYLE))
        ax.add_line(mlines.Line2D([x[-1], x[-1]], [0, y[-1]], **STEP_STYLE))
    for ax in axs[3::2]:
        ax.add_line(mlines.Line2D([0, y[0]], [x[0], x[0]], **STEP_STYLE))
        ax.add_line(mlines.Line2D([0, y[-1]], [x[-1], x[-1]], **STEP_STYLE))
    axs[2].set_ylim(0, None)
    axs[3].set_xlim(0, None)
    axs[4].semilogy()
    axs[5].semilogx()


STAIRS_H = [1, 2, 3, 4, 2]
STAIRS_BINS = [0, 1, 2, 3, 4, 5]


@classic_case("stairs_fill")
def _(fig):
    h, bins = STAIRS_H, STAIRS_BINS
    axs = fig.subplots(2, 2).flatten()
    axs[0].stairs(h, bins, fill=True)
    axs[1].stairs(h, bins, orientation="horizontal", fill=True)
    axs[2].stairs(h, bins, baseline=-2, fill=True)
    axs[3].stairs(h, bins, baseline=-2, orientation="horizontal", fill=True)


@classic_case("stairs_fill_ref")
def _(fig):
    bins, top, bottom = STAIRS_BINS, np.append(STAIRS_H, STAIRS_H[-1]), np.ones(6) * -2
    axs = fig.subplots(2, 2).flatten()
    axs[0].fill_between(bins, top, step="post", lw=0)
    axs[0].set_ylim(0, None)
    axs[1].fill_betweenx(bins, top, step="post", lw=0)
    axs[1].set_xlim(0, None)
    axs[2].fill_between(bins, top, bottom, step="post", lw=0)
    axs[2].set_ylim(-2, None)
    axs[3].fill_betweenx(bins, top, bottom, step="post", lw=0)
    axs[3].set_xlim(-2, None)


@classic_case("stairs_update")
def _(fig):
    ax = fig.add_subplot()
    h = ax.stairs([1, 2, 3])
    ax.set_ylim(-3, 4)
    h.set_data([3, 2, 1])
    h.set_data(edges=np.arange(4) + 2)
    h.set_data([1, 2, 1], np.arange(4) / 2)
    h.set_data([1, 2, 3])
    h.set_data(None, np.arange(4))
    h.set_data(baseline=-2)


@classic_case("stairs_baseline_None")
def _(fig):
    x = np.array([0, 2, 3, 5, 10])
    y = np.array([1.148, 1.231, 1.248, 1.25])
    fig.add_subplot().stairs(y, x, baseline=None)


@mpl20_case("stairs_options")
def _(fig):
    x, y = np.array([1, 2, 3, 4, 5]), np.array([1, 2, 3, 4]).astype(float)
    yn = y.copy()
    yn[1] = np.nan
    ax = fig.subplots()
    ax.stairs(y * 3, x, color="green", fill=True, label="A")
    ax.stairs(y, x * 3 - 3, color="red", fill=True, orientation="horizontal", label="B")
    ax.stairs(yn, x, color="orange", ls="--", lw=2, label="C")
    ax.stairs(yn / 3, x * 3 - 2, ls="--", lw=2, baseline=0.5, orientation="horizontal",
              label="D")
    ax.stairs(y[::-1] * 3 + 13, x - 1, color="red", ls="--", lw=2, baseline=None, label="E")
    ax.stairs(y[::-1] * 3 + 14, x, baseline=26, color="purple", ls="--", lw=2, label="F")
    ax.stairs(yn[::-1] * 3 + 15, x + 1, baseline=np.linspace(27, 25, len(y)), color="blue",
              ls="--", label="G", fill=True)
    ax.stairs(y[:-1][::-1] * 2 + 11, x[:-1] + 0.5, color="black", ls="--", lw=2, baseline=12,
              hatch="//", label="H")
    ax.legend(loc=0)
    remove_ticks_and_titles(fig)


@mpl20_case("stairs_datetime")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.stairs(np.arange(36), np.arange(np.datetime64("2001-12-27"), np.datetime64("2002-02-02")))
    for label in ax.get_xticklabels():
        label.set_rotation(30)


@classic_case("stairs_edge_handling")
def _(fig):
    fig.add_subplot().stairs([1, 2, 3], color="red", fill=True)


# --- contours and 2D histograms ------------------------------------------------


def _contour_data():
    x = np.linspace(-3, 5, 150)
    y = np.linspace(-3, 5, 120)
    return x, y, np.cos(x) + np.sin(y[:, np.newaxis])


@mpl20_case("contour_hatching")
def _(fig):
    ax = fig.subplots()
    ax.contourf(*_contour_data(), 7, hatches=["/", "\\", "//", "-"],
                cmap=mpl.colormaps["gray"].with_alpha(0.5), extend="both")
    remove_ticks_and_titles(fig)


@mpl20_case("contour_colorbar")
def _(fig):
    x, y, z = _contour_data()
    ax = fig.subplots()
    cs = ax.contourf(x, y, z, levels=np.arange(-1.8, 1.801, 0.2), cmap=mpl.colormaps["RdBu"],
                     vmin=-0.6, vmax=0.6, extend="both")
    cs1 = ax.contour(x, y, z, levels=np.arange(-2.2, -0.599, 0.2), colors=["y"],
                     linestyles="solid", linewidths=2)
    cs2 = ax.contour(x, y, z, levels=np.arange(0.6, 2.2, 0.2), colors=["c"], linewidths=2)
    cbar = fig.colorbar(cs, ax=ax)
    cbar.add_lines(cs1)
    cbar.add_lines(cs2, erase=False)


@mpl20_case("hist2d", rc={"pcolormesh.snap": False})
def _(fig):
    stream = rng(0)
    x = stream.normal(100, 5.0, 2.0)
    y = stream.normal(100, -2.0, 1.0)
    fig.subplots().hist2d(x, y, bins=10, rasterized=True)
    remove_ticks_and_titles(fig)


@mpl20_case("hist2d_transpose", rc={"pcolormesh.snap": False})
def _(fig):
    x = np.array([5] * 100)
    y = rng(0).normal(100, -2.0, 1.0)
    fig.subplots().hist2d(x, y, bins=10, rasterized=True)
    remove_ticks_and_titles(fig)


# --- scatter (TestScatter) -----------------------------------------------------

SCATTER_X = [3, 4, 2, 6]
SCATTER_Y = [2, 5, 2, 3]
SCATTER_C = ["r", "y", "b", "lime"]
SCATTER_S = [24, 15, 19, 29]


@mpl20_case("scatter_plot")
def _(fig):
    x, y = np.array(SCATTER_X), np.array(SCATTER_Y)
    ax = fig.subplots()
    ax.scatter(x - 1.0, y - 1.0, c=SCATTER_C, s=SCATTER_S)
    ax.scatter(x + 1.0, y + 1.0, c=["0.5", "0.6", "0.7", "0.8"], s=SCATTER_S)
    ax.scatter(x, y, c=SCATTER_C, s=SCATTER_S)
    remove_ticks_and_titles(fig)


@classic_case("scatter_marker")
def _(fig):
    rx, ry = 3, 1
    area = rx * ry * np.pi
    theta = np.linspace(0, 2 * np.pi, 21)
    verts = np.column_stack([np.cos(theta) * rx / area, np.sin(theta) * ry / area])
    markers = ["s", mmarkers.MarkerStyle("o", fillstyle="top"), verts]
    for ax, marker in zip(fig.subplots(ncols=3), markers):
        ax.scatter(SCATTER_X, SCATTER_Y, c=[(1, 0, 0), "y", "b", "lime"], s=[60, 50, 40, 30],
                   edgecolors=["k", "r", "g", "b"], marker=marker)
    remove_ticks_and_titles(fig)


@classic_case("scatter_2D")
def _(fig):
    x, y = np.meshgrid(np.arange(3), np.arange(2))
    fig.subplots().scatter(x, y, c=x + y, s=200, edgecolors="face")
    remove_ticks_and_titles(fig)


@classic_case("scatter_decimal")
def _(fig):
    fig.subplots().scatter(np.array(DECIMAL_X), np.array(DECIMAL_Y), c=SCATTER_C, s=SCATTER_S)


@classic_case("scatter_invalid_color")
def _(fig):
    cmap = mpl.colormaps["viridis"].resampled(16).with_extremes(bad="black")
    fig.subplots().scatter(range(4), range(4), c=[1, np.nan, 2, np.nan], s=[1, 2, 3, 4],
                           cmap=cmap, plotnonfinite=True)


@classic_case("scatter_no_invalid_color")
def _(fig):
    cmap = mpl.colormaps["viridis"].resampled(16).with_extremes(bad="k")
    fig.subplots().scatter(range(4), range(4), c=[1, np.nan, 2, np.nan], s=[1, 2, 3, 4],
                           cmap=cmap, plotnonfinite=False)


@classic_case("scatter_single_point")
def _(fig):
    fig.subplots().scatter(1, 1, c=1)


@classic_case("scatter_different_shapes")
def _(fig):
    x = np.arange(10)
    fig.subplots().scatter(x, x.reshape(2, 5), c=x.reshape(5, 2))


@mpl20_case("scatter_single_color_c")
def _(fig):
    ax = fig.subplots()
    ax.scatter(np.ones(3), range(3), c=[[1, 0.5, 0.05]])
    ax.scatter(np.ones(4) * 2, range(4), c=[[1, 0.5, 0.05, 0.5]])


def _empty_scatter(fig, edgecolor, facecolor, linestyle):
    ax = fig.subplots()
    ax.scatter([], [], ec=edgecolor, fc=facecolor, ls=linestyle, clip_on=False)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


for _params in [("red", "blue", "solid"), ("red", "blue", "dashed"), ("red", "none", "solid"),
                ("none", "blue", "solid")]:
    classic_case("empty_scatter_" + "_".join(_params))(
        functools.partial(_empty_scatter, edgecolor=_params[0], facecolor=_params[1],
                          linestyle=_params[2]))


# --- stackplot -----------------------------------------------------------------

STACK_X = np.linspace(0, 10, 10)
STACK_Y = [1.0 * STACK_X, 2.0 * STACK_X + 1, 3.0 * STACK_X + 2]


def _stackplot(fig, **kwargs):
    ax = fig.add_subplot(1, 1, 1)
    ax.stackplot(STACK_X, *STACK_Y, **kwargs)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 70)


mpl20_case("stackplot_1")(_stackplot)
mpl20_case("stackplot_2")(functools.partial(_stackplot, colors=["C0", "C1", "C2"]))
classic_case("stackplot_hatching")(
    functools.partial(_stackplot, hatch=["x", "//", "\\\\"], colors=["white"]))


@classic_case("stackplot_hatching_ref")
def _(fig):
    y1, y2, y3 = STACK_Y
    ax = fig.subplots()
    ax.fill_between(STACK_X, np.zeros(len(STACK_X)), y1, hatch="x", facecolor="white")
    ax.fill_between(STACK_X, y1, y1 + y2, hatch="//", facecolor="white")
    ax.fill_between(STACK_X, y1 + y2, y1 + y2 + y3, hatch="\\\\", facecolor="white")
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 70)


@classic_case("stackplot_baseline")
def _(fig):
    stream = rng(0)

    def layers(n, m):
        a = np.zeros((m, n))
        for i in range(n):
            for _ in range(5):
                x = 1 / (0.1 + stream.next_f64())
                y = 2 * stream.next_f64() - 0.5
                z = 10 / (0.1 + stream.next_f64())
                a[:, i] += x * np.exp(-((np.arange(m) / m - y) * z) ** 2)
        return a

    d = layers(3, 100)
    d[50, :] = 0
    axs = fig.subplots(2, 2)
    for ax, baseline in zip(axs.flat, ["zero", "sym", "wiggle", "weighted_wiggle"]):
        ax.stackplot(range(100), d.T, baseline=baseline)
    remove_ticks_and_titles(fig)


# --- bxp -----------------------------------------------------------------------


def _label_abcd(stats):
    for s, label in zip(stats, "ABCD"):
        s["label"] = label
    return stats


def _drop_fliers(stats):
    for s in stats:
        s.pop("fliers", None)
    return stats


def _bxp(fig, stats_kwargs=None, transform_stats=None, remove_text=True, **bxp_kwargs):
    """``_bxp_test_helper``: four log-normal samples on a log axis."""
    data = np.exp(rng(937).normal(37 * 4, 1.25, 1.0)).reshape(37, 4)
    stats = mpl.cbook.boxplot_stats(data, **(stats_kwargs or {}))
    ax = fig.subplots()
    if bxp_kwargs.get("orientation", "vertical") == "vertical":
        ax.set_yscale("log")
    else:
        ax.set_xscale("log")
    ax.bxp(transform_stats(stats) if transform_stats else stats, **bxp_kwargs)
    if remove_text:
        remove_ticks_and_titles(fig)


BXP_CASES = {
    "bxp_baseline": dict(remove_text=False),
    "bxp_rangewhis": dict(remove_text=False, stats_kwargs=dict(whis=[0, 100])),
    "bxp_percentilewhis": dict(remove_text=False, stats_kwargs=dict(whis=[5, 95])),
    "bxp_with_xlabels": dict(remove_text=False, transform_stats=_label_abcd),
    "bxp_horizontal": dict(orientation="horizontal"),
    "bxp_with_ylabels": dict(remove_text=False, transform_stats=_label_abcd,
                             orientation="horizontal"),
    "bxp_patchartist": dict(patch_artist=True),
    "bxp_custompatchartist": dict(
        patch_artist=True, boxprops=dict(facecolor="yellow", edgecolor="green", ls=":")),
    "bxp_customoutlier": dict(flierprops=dict(linestyle="none", marker="d", mfc="g")),
    "bxp_showcustommean": dict(
        showmeans=True, meanprops=dict(linestyle="none", marker="d", mfc="green")),
    "bxp_custombox": dict(boxprops=dict(linestyle="--", color="b", lw=3)),
    "bxp_custommedian": dict(medianprops=dict(linestyle="--", color="b", lw=3)),
    "bxp_customcap": dict(capprops=dict(linestyle="--", color="g", lw=3)),
    "bxp_customwhisker": dict(whiskerprops=dict(linestyle="-", color="m", lw=3)),
    "bxp_shownotches": dict(shownotches=True),
    "bxp_nocaps": dict(showcaps=False),
    "bxp_nobox": dict(showbox=False),
    "bxp_no_flier_stats": dict(transform_stats=_drop_fliers, showfliers=False),
    "bxp_showmean": dict(showmeans=True, meanline=False),
    "bxp_showmeanasline": dict(showmeans=True, meanline=True),
    "bxp_scalarwidth": dict(widths=0.25),
}

for _name, _kwargs in BXP_CASES.items():
    # The baselines predate bxp honouring boxplot.boxprops.linewidth without
    # patch_artist, so the tests set it to lines.linewidth.
    _rc = {} if _kwargs.get("patch_artist") else {"boxplot.boxprops.linewidth": 1.5}
    _dpi = 100 if _name == "bxp_custompatchartist" else 40
    mpl20_case(_name, dpi=_dpi, rc=_rc)(functools.partial(_bxp, **_kwargs))


@classic_case("boxplot_median_bound_by_box")
def _(fig):
    fig.subplots().boxplot(np.arange(3), medianprops={"linewidth": 12})


@classic_case("boxplot_median_bound_by_box_ref")
def _(fig):
    fig.subplots().boxplot(np.arange(3), medianprops={"linewidth": 12, "solid_capstyle": "butt"})
