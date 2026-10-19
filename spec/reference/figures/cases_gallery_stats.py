"""The gallery's "Statistics" and "Pie and polar charts" sections, one case
per figure of each example (``<example>_<n>`` when the example draws
several): box plots (colours, notches, custom statistics, ``bxp``), violin
plots, error bars (limits, subsampling, error boxes), histograms (histtypes,
several datasets, normalisation, cumulative, bihistogram, side by side),
hexbin, spectral estimates (PSD, CSD, coherence, correlations), confidence
ellipses, error bands, a time-series histogram, pie and donut charts with
their labels, and polar bars, lines, error bars, legends and scatter plots.

Random data comes from ``spec_support.rng`` seeded with the example's seed
instead of ``np.random``; the lognormal, exponential, Gumbel and triangular
draws of ``boxplot_demo`` are built from its uniforms. The box plots leave
out ``bootstrap=``: matplotlib bootstraps the notches' confidence intervals
with ``np.random``, so they would change from run to run (the notches use
the Gaussian-based interval instead).
"""

import math

import matplotlib as mpl
import numpy as np
from matplotlib import cbook, colors, mlab
from matplotlib.artist import setp
from matplotlib.collections import PatchCollection
from matplotlib.patches import ConnectionPatch, Ellipse, PathPatch, Polygon, Rectangle
from matplotlib.path import Path
from matplotlib.ticker import PercentFormatter
from matplotlib.transforms import Affine2D

from spec_support import case, rng

GALLERY = "gallery_stats"
SEED = 19680801

#: The font size of the box and violin plot grids' titles.
FS = 10


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{GALLERY}/{name}", **options)


# --- random draws the spec generator lacks -------------------------------------


def exponential(g, n):
    """``np.random.exponential(1, n)`` by inversion."""
    return np.array([-math.log(1.0 - g.next_f64()) for _ in range(n)])


def gumbel(g, loc, scale, n):
    """``np.random.gumbel(loc, scale, n)`` by inversion."""
    return np.array([loc - scale * math.log(-math.log(1.0 - g.next_f64())) for _ in range(n)])


def triangular(g, left, mode, right, n):
    """``np.random.triangular(left, mode, right, n)`` by inversion."""
    cut = (mode - left) / (right - left)

    def draw(u):
        if u < cut:
            return left + math.sqrt(u * (right - left) * (mode - left))
        return right - math.sqrt((1.0 - u) * (right - left) * (right - mode))

    return np.array([draw(g.next_f64()) for _ in range(n)])


# --- box plots -----------------------------------------------------------------


@gallery("boxplot_color")
def _(fig):
    g = rng(SEED)
    fruit_weights = [g.normal(100, 130, 10), g.normal(100, 125, 20), g.normal(100, 120, 30)]
    ax = fig.subplots()
    ax.set_ylabel("fruit weight (g)")
    bplot = ax.boxplot(fruit_weights, patch_artist=True, tick_labels=["peaches", "oranges", "tomatoes"])
    for patch, color in zip(bplot["boxes"], ["peachpuff", "orange", "tomato"]):
        patch.set_facecolor(color)


def fake_data(g, center):
    """``boxplot_demo``'s data: a uniform spread, a constant centre and
    fliers on both sides."""
    spread = g.uniform(50, 0, 100)
    centre = np.ones(25) * center
    flier_high = g.uniform(10, 100, 200)
    flier_low = g.uniform(10, 0, -100)
    return np.concatenate((spread, centre, flier_high, flier_low))


@gallery("boxplot_demo_1")
def _(fig):
    data = fake_data(rng(SEED), 50)
    axs = fig.subplots(2, 3)
    axs[0, 0].boxplot(data)
    axs[0, 0].set_title("basic plot")
    axs[0, 1].boxplot(data, notch=True)
    axs[0, 1].set_title("notched plot")
    axs[0, 2].boxplot(data, sym="gD")
    axs[0, 2].set_title("change outlier\npoint symbols")
    axs[1, 0].boxplot(data, sym="")
    axs[1, 0].set_title("don't show\noutlier points")
    axs[1, 1].boxplot(data, sym="rs", orientation="horizontal")
    axs[1, 1].set_title("horizontal boxes")
    axs[1, 2].boxplot(data, sym="rs", orientation="horizontal", whis=0.75)
    axs[1, 2].set_title("change whisker length")
    fig.subplots_adjust(left=0.08, right=0.98, bottom=0.05, top=0.9, hspace=0.4, wspace=0.3)


@gallery("boxplot_demo_2")
def _(fig):
    g = rng(SEED)
    data = fake_data(g, 50)
    d2 = fake_data(g, 40)
    ax = fig.subplots()
    ax.boxplot([data, d2, d2[::2]])


RANDOM_DISTS = ["Normal(1, 1)", "Lognormal(1, 1)", "Exp(1)", "Gumbel(6, 4)", "Triangular(2, 9, 11)"]


def bootstrap_pairs(g, n):
    """``boxplot_demo``'s five distributions, each followed by a bootstrap
    resample of itself."""
    dists = [
        g.normal(n, 1, 1),
        np.exp(g.normal(n, 1, 1)),
        exponential(g, n),
        gumbel(g, 6, 4, n),
        triangular(g, 2, 9, 11, n),
    ]
    bootstrap_indices = g.integers(n, 0, n)
    return [sample for dist in dists for sample in (dist, dist[bootstrap_indices])]


@gallery("boxplot_demo_3", figsize=(10, 6))
def _(fig):
    n = 500
    data = bootstrap_pairs(rng(SEED), n)
    ax1 = fig.subplots()
    fig.subplots_adjust(left=0.075, right=0.95, top=0.9, bottom=0.25)
    bp = ax1.boxplot(data, notch=False, sym="+", orientation="vertical", whis=1.5)
    setp(bp["boxes"], color="black")
    setp(bp["whiskers"], color="black")
    setp(bp["fliers"], color="red", marker="+")
    ax1.yaxis.grid(True, linestyle="-", which="major", color="lightgrey", alpha=0.5)
    ax1.set(
        axisbelow=True,
        title="Comparison of IID Bootstrap Resampling Across Five Distributions",
        xlabel="Distribution",
        ylabel="Value",
    )
    box_colors = ["darkkhaki", "royalblue"]
    num_boxes = len(data)
    medians = np.empty(num_boxes)
    for i in range(num_boxes):
        box = bp["boxes"][i]
        box_coords = np.column_stack([box.get_xdata()[:5], box.get_ydata()[:5]])
        ax1.add_patch(Polygon(box_coords, facecolor=box_colors[i % 2]))
        med = bp["medians"][i]
        median_x = []
        median_y = []
        for j in range(2):
            median_x.append(med.get_xdata()[j])
            median_y.append(med.get_ydata()[j])
            ax1.plot(median_x, median_y, "k")
        medians[i] = median_y[0]
        ax1.plot(np.average(med.get_xdata()), np.average(data[i]), color="w", marker="*", markeredgecolor="k")
    ax1.set_xlim(0.5, num_boxes + 0.5)
    ax1.set_ylim(-5, 40)
    ax1.set_xticklabels(np.repeat(RANDOM_DISTS, 2), rotation=45, fontsize=8)
    weights = ["bold", "semibold"]
    for tick in range(num_boxes):
        k = tick % 2
        ax1.text(tick + 1, 0.95, str(round(medians[tick], 2)), transform=ax1.get_xaxis_transform(),
                 horizontalalignment="center", size="x-small", weight=weights[k], color=box_colors[k])
    fig.text(0.80, 0.08, f"{n} Random Numbers", backgroundcolor=box_colors[0], color="black",
             weight="roman", size="x-small")
    fig.text(0.80, 0.045, "IID Bootstrap Resample", backgroundcolor=box_colors[1], color="white",
             weight="roman", size="x-small")
    fig.text(0.80, 0.015, "*", color="white", backgroundcolor="silver", weight="roman", size="medium")
    fig.text(0.815, 0.013, " Average Value", color="black", weight="roman", size="x-small")


@gallery("boxplot_demo_4")
def _(fig):
    g = rng(SEED)
    inc = 0.1
    treatments = [g.normal(500, 0, 1), g.normal(500, 0, 1), g.normal(500, 0, 1 + inc), g.normal(500, 0, 1 + 2 * inc)]
    ax = fig.subplots()
    pos = np.arange(len(treatments)) + 1
    bp = ax.boxplot(treatments, sym="k+", positions=pos, notch=True, usermedians=[None, None, 0.1, 0.2],
                    conf_intervals=[None, None, (-0.25, 0.25), (-0.35, 0.50)])
    ax.set_xlabel("treatment")
    ax.set_ylabel("response")
    setp(bp["whiskers"], color="k", linestyle="-")
    setp(bp["fliers"], markersize=3.0)


@gallery("boxplot_demo_5")
def _(fig):
    x = np.hstack([-25, np.linspace(-7, 7, 140), 25])
    ax = fig.subplots()
    ax.boxplot([x, x], notch=True, capwidths=[0.01, 0.2])


def lognormal_columns():
    """``boxplot``'s data: 37 lognormal draws in each of four columns."""
    return np.exp(rng(SEED).normal(37 * 4, 1.5, 1.75)).reshape(37, 4)


def log_grid(fig):
    """The 2×3 grid of ``boxplot``, sharing a log y-axis without labels."""
    axs = fig.subplots(nrows=2, ncols=3, sharey=True)
    for ax in axs.flat:
        ax.set_yscale("log")
        ax.set_yticklabels([])
    fig.subplots_adjust(hspace=0.4)
    return axs


@gallery("boxplot_1", figsize=(6, 6))
def _(fig):
    data = lognormal_columns()
    labels = list("ABCD")
    axs = log_grid(fig)
    axs[0, 0].boxplot(data, tick_labels=labels)
    axs[0, 0].set_title("Default", fontsize=FS)
    axs[0, 1].boxplot(data, tick_labels=labels, showmeans=True)
    axs[0, 1].set_title("showmeans=True", fontsize=FS)
    axs[0, 2].boxplot(data, tick_labels=labels, showmeans=True, meanline=True)
    axs[0, 2].set_title("showmeans=True,\nmeanline=True", fontsize=FS)
    axs[1, 0].boxplot(data, tick_labels=labels, showbox=False, showcaps=False)
    axs[1, 0].set_title("Tufte Style \n(showbox=False,\nshowcaps=False)", fontsize=FS)
    axs[1, 1].boxplot(data, tick_labels=labels, notch=True)
    axs[1, 1].set_title("notch=True,\nbootstrap=10000", fontsize=FS)
    axs[1, 2].boxplot(data, tick_labels=labels, showfliers=False)
    axs[1, 2].set_title("showfliers=False", fontsize=FS)


@gallery("boxplot_2", figsize=(6, 6))
def _(fig):
    data = lognormal_columns()
    boxprops = dict(linestyle="--", linewidth=3, color="darkgoldenrod")
    flierprops = dict(marker="o", markerfacecolor="green", markersize=12, markeredgecolor="none")
    medianprops = dict(linestyle="-.", linewidth=2.5, color="firebrick")
    meanpointprops = dict(marker="D", markeredgecolor="black", markerfacecolor="firebrick")
    meanlineprops = dict(linestyle="--", linewidth=2.5, color="purple")
    axs = log_grid(fig)
    axs[0, 0].boxplot(data, boxprops=boxprops)
    axs[0, 0].set_title("Custom boxprops", fontsize=FS)
    axs[0, 1].boxplot(data, flierprops=flierprops, medianprops=medianprops)
    axs[0, 1].set_title("Custom medianprops\nand flierprops", fontsize=FS)
    axs[0, 2].boxplot(data, whis=(0, 100))
    axs[0, 2].set_title("whis=(0, 100)", fontsize=FS)
    axs[1, 0].boxplot(data, meanprops=meanpointprops, meanline=False, showmeans=True)
    axs[1, 0].set_title("Custom mean\nas point", fontsize=FS)
    axs[1, 1].boxplot(data, meanprops=meanlineprops, meanline=True, showmeans=True)
    axs[1, 1].set_title("Custom mean\nas line", fontsize=FS)
    axs[1, 2].boxplot(data, whis=[15, 85])
    axs[1, 2].set_title("whis=[15, 85]\n#percentiles", fontsize=FS)
    fig.suptitle("I never said they'd be pretty")


@gallery("boxplot_vs_violin", figsize=(9, 4))
def _(fig):
    g = rng(SEED)
    all_data = [g.normal(100, 0, std) for std in range(6, 10)]
    axs = fig.subplots(nrows=1, ncols=2)
    axs[0].violinplot(all_data, showmeans=False, showmedians=True)
    axs[0].set_title("Violin plot")
    axs[1].boxplot(all_data)
    axs[1].set_title("Box plot")
    for ax in axs:
        ax.yaxis.grid(True)
        ax.set_xticks([y + 1 for y in range(len(all_data))], labels=["x1", "x2", "x3", "x4"])
        ax.set_xlabel("Four separate samples")
        ax.set_ylabel("Observed values")


BISQUE = {"facecolor": "bisque"}


@gallery("bxp_1")
def _(fig):
    data = rng(SEED).normal(60).reshape(20, 3)
    ax1, ax2 = fig.subplots(1, 2)
    ax1.boxplot(data, tick_labels=["A", "B", "C"], patch_artist=True, boxprops=BISQUE)
    stats = cbook.boxplot_stats(data, labels=["A", "B", "C"])
    ax2.bxp(stats, patch_artist=True, boxprops=BISQUE)


@gallery("bxp_2")
def _(fig):
    stats = [
        dict(med=0, q1=-1, q3=1, whislo=-2, whishi=2, fliers=[-4, -3, 3, 4], label="A"),
        dict(med=0, q1=-2, q3=2, whislo=-3, whishi=3, fliers=[], label="B"),
        dict(med=0, q1=-3, q3=3, whislo=-4, whishi=4, fliers=[], label="C"),
    ]
    ax = fig.subplots()
    ax.bxp(stats, patch_artist=True, boxprops=BISQUE)


# --- violin plots --------------------------------------------------------------


def adjacent_values(vals, q1, q3):
    """The whisker ends of a sorted sample: 1.5 IQR clipped to the data."""
    upper_adjacent_value = np.clip(q3 + (q3 - q1) * 1.5, q3, vals[-1])
    lower_adjacent_value = np.clip(q1 - (q3 - q1) * 1.5, vals[0], q1)
    return lower_adjacent_value, upper_adjacent_value


def set_axis_style(ax, labels):
    """Labels the violins and leaves room on both sides."""
    ax.set_xticks(np.arange(1, len(labels) + 1), labels=labels)
    ax.set_xlim(0.25, len(labels) + 0.75)
    ax.set_xlabel("Sample name")


@gallery("customized_violin", figsize=(9, 3))
def _(fig):
    g = rng(SEED)
    data = [sorted(g.normal(100, 0, std)) for std in range(1, 5)]
    ax1, ax2, ax3 = fig.subplots(nrows=1, ncols=3, sharey=True)
    ax1.set_title("Default violin plot")
    ax1.set_ylabel("Observed values")
    ax1.violinplot(data)
    ax2.set_title("Set colors of violins")
    ax2.set_ylabel("Observed values")
    ax2.violinplot(data, facecolor=[("yellow", 0.3), ("blue", 0.3), ("red", 0.3), ("green", 0.3)],
                   linecolor="black")
    ax3.set_title("Customized violin plot")
    parts = ax3.violinplot(data, showmeans=False, showmedians=False, showextrema=False,
                           facecolor="#D43F3A", linecolor="black")
    for pc in parts["bodies"]:
        pc.set_edgecolor("black")
        pc.set_linewidth(1)
        pc.set_alpha(1)
    quartile1, medians, quartile3 = np.percentile(data, [25, 50, 75], axis=1)
    whiskers = np.array([
        adjacent_values(sorted_array, q1, q3)
        for sorted_array, q1, q3 in zip(data, quartile1, quartile3)])
    inds = np.arange(1, len(medians) + 1)
    ax3.scatter(inds, medians, marker="o", color="white", s=30, zorder=3)
    ax3.vlines(inds, quartile1, quartile3, color="k", linestyle="-", lw=5)
    ax3.vlines(inds, whiskers[:, 0], whiskers[:, 1], color="k", linestyle="-", lw=1)
    for ax in [ax1, ax2, ax3]:
        set_axis_style(ax, ["A", "B", "C", "D"])
    fig.subplots_adjust(bottom=0.15, wspace=0.05)


@gallery("violinplot", figsize=(10, 4))
def _(fig):
    g = rng(SEED)
    pos = [1, 2, 4, 5, 7, 8]
    data = [g.normal(100, 0, std) for std in pos]
    quantiles = [[0.1], [], [], [0.175, 0.954], [0.75], [0.25]]
    tails = [0.05, 0.1, 0.8, 0.9]
    common = dict(showmeans=True, showextrema=True, showmedians=True)
    axs = fig.subplots(nrows=2, ncols=6)
    axs[0, 0].violinplot(data, pos, points=20, widths=0.3, **common)
    axs[0, 1].violinplot(data, pos, points=40, widths=0.5, bw_method="silverman", **common)
    axs[0, 2].violinplot(data, pos, points=60, widths=0.7, bw_method=0.5, **common)
    axs[0, 3].violinplot(data, pos, points=60, widths=0.7, bw_method=0.5, quantiles=quantiles, **common)
    axs[0, 4].violinplot(data[-1:], pos[-1:], points=60, widths=0.7, quantiles=tails, bw_method=0.5, **common)
    for side in ("low", "high"):
        axs[0, 5].violinplot(data[-1:], pos[-1:], points=60, widths=0.7, quantiles=tails, bw_method=0.5,
                             side=side, **common)
    horizontal = dict(orientation="horizontal", **common)
    axs[1, 0].violinplot(data, pos, points=80, widths=0.7, **horizontal)
    axs[1, 1].violinplot(data, pos, points=100, widths=0.9, bw_method="silverman", **horizontal)
    axs[1, 2].violinplot(data, pos, points=200, widths=1.1, bw_method=0.5, **horizontal)
    axs[1, 3].violinplot(data, pos, points=200, widths=1.1, quantiles=quantiles, bw_method=0.5, **horizontal)
    axs[1, 4].violinplot(data[-1:], pos[-1:], points=200, widths=1.1, quantiles=tails, bw_method=0.5,
                         **horizontal)
    for side in ("low", "high"):
        axs[1, 5].violinplot(data[-1:], pos[-1:], points=200, widths=1.1, quantiles=tails, bw_method=0.5,
                             side=side, **horizontal)
    for i, ax in enumerate(axs.flat):
        ax.set_title(f"Custom violin {i + 1}", fontsize=FS)
        ax.set_yticklabels([])
    fig.suptitle("Violin Plotting Examples")
    fig.subplots_adjust(hspace=0.4)


# --- error bars ----------------------------------------------------------------


@gallery("errorbar")
def _(fig):
    x = np.arange(0.1, 4, 0.5)
    ax = fig.subplots()
    ax.errorbar(x, np.exp(-x), xerr=0.2, yerr=0.4)


@gallery("errorbar_features")
def _(fig):
    x = np.arange(0.1, 4, 0.5)
    y = np.exp(-x)
    error = 0.1 + 0.2 * x
    ax0, ax1 = fig.subplots(nrows=2, sharex=True)
    ax0.errorbar(x, y, yerr=error, fmt="-o")
    ax0.set_title("variable, symmetric error")
    ax1.errorbar(x, y, xerr=[0.4 * error, error], fmt="o")
    ax1.set_title("variable, asymmetric error")
    ax1.set_yscale("log")


@gallery("errorbar_limits", figsize=(7, 4))
def _(fig):
    x = np.array([0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0])
    y = np.exp(-x)
    xerr = 0.1
    yerr = 0.2
    lolims = np.array([0, 0, 1, 0, 1, 0, 0, 0, 1, 0], dtype=bool)
    uplims = np.array([0, 1, 0, 0, 0, 1, 0, 0, 0, 1], dtype=bool)
    ls = "dotted"
    ax = fig.subplots()
    ax.errorbar(x, y, xerr=xerr, yerr=yerr, linestyle=ls)
    ax.errorbar(x, y + 0.5, xerr=xerr, yerr=yerr, uplims=uplims, linestyle=ls)
    ax.errorbar(x, y + 1.0, xerr=xerr, yerr=yerr, lolims=lolims, linestyle=ls)
    ax.errorbar(x, y + 1.5, xerr=xerr, yerr=yerr, lolims=lolims, uplims=uplims, marker="o", markersize=8,
                linestyle=ls)
    yerr = np.full_like(x, 0.2)
    yerr[[3, 6]] = 0.3
    one_lolim = np.zeros_like(x)
    one_uplim = np.zeros_like(x)
    one_lolim[[6]] = True
    one_uplim[[3]] = True
    ax.errorbar(x, y + 2.1, xerr=0.2, yerr=yerr, xlolims=lolims, xuplims=uplims, uplims=one_uplim,
                lolims=one_lolim, marker="o", markersize=8, linestyle="none")
    ax.set_xlim(0, 5.5)
    ax.set_title("Errorbar upper and lower limits")


UPPER_LIMITS = [True, False] * 5
LOWER_LIMITS = [False, True] * 5


@gallery("errorbar_limits_simple_1")
def _(fig):
    x = np.arange(10)
    y = 2.5 * np.sin(x / 20 * np.pi)
    yerr = np.linspace(0.05, 0.2, 10)
    ax = fig.subplots()
    ax.errorbar(x, y + 3, yerr=yerr, label="both limits (default)")
    ax.errorbar(x, y + 2, yerr=yerr, uplims=True, label="uplims=True")
    ax.errorbar(x, y + 1, yerr=yerr, uplims=True, lolims=True, label="uplims=True, lolims=True")
    ax.errorbar(x, y, yerr=yerr, uplims=UPPER_LIMITS, lolims=LOWER_LIMITS, label="subsets of uplims and lolims")
    ax.legend(loc="lower right")


@gallery("errorbar_limits_simple_2")
def _(fig):
    x = np.arange(10) / 10
    ax = fig.subplots()
    ax.errorbar(x, (x + 0.1) ** 2, xerr=0.1, xlolims=True, label="xlolims=True")
    ax.errorbar(x + 0.6, (x + 0.1) ** 3, xerr=0.1, xuplims=UPPER_LIMITS, xlolims=LOWER_LIMITS,
                label="subsets of xuplims and xlolims")
    ax.errorbar(x + 1.2, (x + 0.1) ** 4, xerr=0.1, xuplims=True, label="xuplims=True")
    ax.legend()


@gallery("errorbar_subsample", figsize=(12, 6))
def _(fig):
    x = np.arange(0.1, 4, 0.1)
    y1 = np.exp(-1.0 * x)
    y2 = np.exp(-0.5 * x)
    y1err = 0.1 + 0.1 * np.sqrt(x)
    y2err = 0.1 + 0.1 * np.sqrt(x / 2)
    ax0, ax1, ax2 = fig.subplots(nrows=1, ncols=3, sharex=True)
    ax0.set_title("all errorbars")
    ax0.errorbar(x, y1, yerr=y1err)
    ax0.errorbar(x, y2, yerr=y2err)
    ax1.set_title("only every 6th errorbar")
    ax1.errorbar(x, y1, yerr=y1err, errorevery=6)
    ax1.errorbar(x, y2, yerr=y2err, errorevery=6)
    ax2.set_title("second series shifted by 3")
    ax2.errorbar(x, y1, yerr=y1err, errorevery=(0, 6))
    ax2.errorbar(x, y2, yerr=y2err, errorevery=(3, 6))
    fig.suptitle("Errorbar subsampling")


@gallery("errorbars_and_boxes")
def _(fig):
    n = 5
    g = rng(SEED)
    x = np.arange(0, n, 1)
    y = g.uniform(n) * 5.0
    xerr = g.uniform(2 * n).reshape(2, n) + 0.1
    yerr = g.uniform(2 * n).reshape(2, n) + 0.2
    ax = fig.subplots(1)
    errorboxes = [Rectangle((xi - xe[0], yi - ye[0]), xe.sum(), ye.sum())
                  for xi, yi, xe, ye in zip(x, y, xerr.T, yerr.T)]
    ax.add_collection(PatchCollection(errorboxes, facecolor="r", alpha=0.5, edgecolor="none"))
    ax.errorbar(x, y, xerr=xerr, yerr=yerr, fmt="none", ecolor="k")


# --- curves, ellipses and bands ------------------------------------------------


def confidence_ellipse(x, y, ax, n_std=3.0, facecolor="none", **kwargs):
    """The covariance ellipse of ``x`` and ``y``, ``n_std`` standard
    deviations wide, drawn into ``ax``."""
    cov = np.cov(x, y)
    pearson = cov[0, 1] / np.sqrt(cov[0, 0] * cov[1, 1])
    ellipse = Ellipse((0, 0), width=np.sqrt(1 + pearson) * 2, height=np.sqrt(1 - pearson) * 2,
                      facecolor=facecolor, **kwargs)
    scale_x = np.sqrt(cov[0, 0]) * n_std
    scale_y = np.sqrt(cov[1, 1]) * n_std
    transf = Affine2D().rotate_deg(45).scale(scale_x, scale_y).translate(np.mean(x), np.mean(y))
    ellipse.set_transform(transf + ax.transData)
    return ax.add_patch(ellipse)


def correlated_dataset(g, n, dependency, mu, scale):
    """``n`` points of two normal variables mixed by ``dependency``."""
    latent = g.normal(2 * n).reshape(n, 2)
    scaled_with_offset = latent.dot(dependency) * scale + mu
    return scaled_with_offset[:, 0], scaled_with_offset[:, 1]


def cross_axes(ax):
    """The grey lines through the origin of ``confidence_ellipse``."""
    ax.axvline(c="grey", lw=1)
    ax.axhline(c="grey", lw=1)


@gallery("confidence_ellipse_1", figsize=(9, 3))
def _(fig):
    parameters = {
        "Positive correlation": [[0.85, 0.35], [0.15, -0.65]],
        "Negative correlation": [[0.9, -0.4], [0.1, -0.6]],
        "Weak correlation": [[1, 0], [0, 1]],
    }
    mu = 2, 4
    g = rng(0)
    axs = fig.subplots(1, 3)
    for ax, (title, dependency) in zip(axs, parameters.items()):
        x, y = correlated_dataset(g, 800, dependency, mu, (3, 5))
        ax.scatter(x, y, s=0.5)
        cross_axes(ax)
        confidence_ellipse(x, y, ax, edgecolor="red")
        ax.scatter(mu[0], mu[1], c="red", s=3)
        ax.set_title(title)


@gallery("confidence_ellipse_2", figsize=(6, 6))
def _(fig):
    ax_nstd = fig.subplots()
    cross_axes(ax_nstd)
    x, y = correlated_dataset(rng(0), 500, [[0.8, 0.75], [-0.2, 0.35]], (0, 0), (8, 5))
    ax_nstd.scatter(x, y, s=0.5)
    confidence_ellipse(x, y, ax_nstd, n_std=1, label=r"$1\sigma$", edgecolor="firebrick")
    confidence_ellipse(x, y, ax_nstd, n_std=2, label=r"$2\sigma$", edgecolor="fuchsia", linestyle="--")
    confidence_ellipse(x, y, ax_nstd, n_std=3, label=r"$3\sigma$", edgecolor="blue", linestyle=":")
    ax_nstd.scatter(0, 0, c="red", s=3)
    ax_nstd.set_title("Different standard deviations")
    ax_nstd.legend()


@gallery("confidence_ellipse_3", figsize=(6, 6))
def _(fig):
    mu = 2, -3
    ax_kwargs = fig.subplots()
    cross_axes(ax_kwargs)
    x, y = correlated_dataset(rng(0), 500, [[-0.8, 0.5], [-0.2, 0.5]], mu, (6, 5))
    confidence_ellipse(x, y, ax_kwargs, alpha=0.5, facecolor="pink", edgecolor="purple", zorder=0)
    ax_kwargs.scatter(x, y, s=0.5)
    ax_kwargs.scatter(mu[0], mu[1], c="red", s=3)
    ax_kwargs.set_title("Using keyword arguments")
    fig.subplots_adjust(hspace=0.25)


def cardioid():
    """``curve_error_band``'s curve: the parameter and the points of a
    limaçon."""
    t = np.linspace(0, 2 * np.pi, 400)
    r = 0.5 + np.cos(t)
    return t, r * np.cos(t), r * np.sin(t)


@gallery("curve_error_band_1")
def _(fig):
    _, x, y = cardioid()
    ax = fig.subplots()
    ax.plot(x, y, "k")
    ax.set(aspect=1)


def draw_error_band(ax, x, y, err, **kwargs):
    """A band ``err`` wide on both sides of the curve, along its normals."""
    dx = np.concatenate([[x[1] - x[0]], x[2:] - x[:-2], [x[-1] - x[-2]]])
    dy = np.concatenate([[y[1] - y[0]], y[2:] - y[:-2], [y[-1] - y[-2]]])
    length = np.hypot(dx, dy)
    nx = dy / length
    ny = -dx / length
    xp = x + nx * err
    yp = y + ny * err
    xn = x - nx * err
    yn = y - ny * err
    vertices = np.block([[xp, xn[::-1]], [yp, yn[::-1]]]).T
    codes = np.full(len(vertices), Path.LINETO)
    codes[0] = codes[len(xp)] = Path.MOVETO
    ax.add_patch(PathPatch(Path(vertices, codes), **kwargs))


@gallery("curve_error_band_2")
def _(fig):
    t, x, y = cardioid()
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2, sharex=True, sharey=True)
    errs = [
        (axs[0], "constant error", 0.05),
        (axs[1], "variable error", 0.05 * np.sin(2 * t) ** 2 + 0.04),
    ]
    for i, (ax, title, err) in enumerate(errs):
        ax.set(title=title, aspect=1, xticks=[], yticks=[])
        ax.plot(x, y, "k")
        draw_error_band(ax, x, y, err=err, facecolor=f"C{i}", edgecolor="none", alpha=0.3)


# --- histograms ----------------------------------------------------------------


@gallery("hexbin_demo", figsize=(9, 4))
def _(fig):
    g = rng(SEED)
    n = 100_000
    x = g.normal(n)
    y = 2.0 + 3.0 * x + 4.0 * g.normal(n)
    xlim = x.min(), x.max()
    ylim = y.min(), y.max()
    ax0, ax1 = fig.subplots(ncols=2, sharey=True)
    hb = ax0.hexbin(x, y, gridsize=50, cmap="inferno")
    ax0.set(xlim=xlim, ylim=ylim)
    ax0.set_title("Hexagon binning")
    fig.colorbar(hb, ax=ax0, label="counts")
    hb = ax1.hexbin(x, y, gridsize=50, bins="log", cmap="inferno")
    ax1.set(xlim=xlim, ylim=ylim)
    ax1.set_title("With a log color scale")
    fig.colorbar(hb, ax=ax1, label="counts")


@gallery("histogram_bihistogram")
def _(fig):
    g = rng(SEED)
    dataset1 = g.normal(10_000, 0, 1)
    dataset2 = g.normal(10_000, 1, 2)
    bin_width = 0.25
    bins = np.arange(np.min([dataset1, dataset2]), np.max([dataset1, dataset2]) + bin_width, bin_width)
    ax = fig.subplots()
    ax.hist(dataset1, bins=bins, label="Dataset 1")
    ax.hist(dataset2, weights=-np.ones_like(dataset2), bins=bins, label="Dataset 2")
    ax.axhline(0, color="k")
    ax.legend()


@gallery("histogram_cumulative", figsize=(9, 4))
def _(fig):
    mu = 200
    sigma = 25
    data = rng(SEED).normal(100, mu, sigma)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 2, sharex=True, sharey=True)
    axs[0].ecdf(data, label="CDF")
    _, bins, _ = axs[0].hist(data, 25, density=True, histtype="step", cumulative=True,
                             label="Cumulative histogram")
    x = np.linspace(data.min(), data.max())
    y = (1 / (np.sqrt(2 * np.pi) * sigma)) * np.exp(-0.5 * (1 / sigma * (x - mu)) ** 2)
    y = y.cumsum()
    y /= y[-1]
    axs[0].plot(x, y, "k--", linewidth=1.5, label="Theory")
    axs[1].ecdf(data, complementary=True, label="CCDF")
    axs[1].hist(data, bins=bins, density=True, histtype="step", cumulative=-1,
                label="Reversed cumulative histogram")
    axs[1].plot(x, 1 - y, "k--", linewidth=1.5, label="Theory")
    fig.suptitle("Cumulative distributions")
    for ax in axs:
        ax.grid(True)
        ax.legend()
        ax.set_xlabel("Annual rainfall (mm)")
        ax.set_ylabel("Probability of occurrence")
        ax.label_outer()


@gallery("histogram_histtypes")
def _(fig):
    g = rng(SEED)
    x = g.normal(100, 200, 25)
    w = g.normal(100, 200, 10)
    axs = fig.subplots(nrows=2, ncols=2)
    axs[0, 0].hist(x, 20, density=True, histtype="stepfilled", facecolor="g", alpha=0.75)
    axs[0, 0].set_title("stepfilled")
    axs[0, 1].hist(x, 20, density=True, histtype="step", facecolor="g", alpha=0.75)
    axs[0, 1].set_title("step")
    axs[1, 0].hist(x, density=True, histtype="barstacked", rwidth=0.8)
    axs[1, 0].hist(w, density=True, histtype="barstacked", rwidth=0.8)
    axs[1, 0].set_title("barstacked")
    bins = [100, 150, 180, 195, 205, 220, 250, 300]
    axs[1, 1].hist(x, bins, density=True, histtype="bar", rwidth=0.8)
    axs[1, 1].set_title("bar, unequal bins")
    fig.tight_layout()


#: The bin count of ``histogram_multihist``.
N_BINS = 10
STACK_COLORS = ["green", "red", "blue"]


def three_normals(g):
    """``histogram_multihist``'s data: 1000 draws in each of three columns."""
    return g.normal(3000).reshape(1000, 3)


@gallery("histogram_multihist_1")
def _(fig):
    g = rng(SEED)
    x = three_normals(g)
    (ax0, ax1), (ax2, ax3) = fig.subplots(nrows=2, ncols=2)
    bar_colors = ["red", "tan", "lime"]
    ax0.hist(x, N_BINS, density=True, histtype="bar", color=bar_colors, label=bar_colors)
    ax0.legend(prop={"size": 10})
    ax0.set_title("bars with legend")
    ax1.hist(x, N_BINS, density=True, histtype="bar", stacked=True)
    ax1.set_title("stacked bar")
    ax2.hist(x, N_BINS, histtype="step", stacked=True, fill=False)
    ax2.set_title("stack step (unfilled)")
    x_multi = [g.normal(n) for n in [10000, 5000, 2000]]
    ax3.hist(x_multi, N_BINS, histtype="bar")
    ax3.set_title("different sample sizes")
    fig.tight_layout()


@gallery("histogram_multihist_2")
def _(fig):
    ax = fig.subplots()
    ax.hist(three_normals(rng(SEED)), N_BINS, fill=False, histtype="step", stacked=True,
            edgecolor=STACK_COLORS, label=STACK_COLORS)
    ax.legend()
    ax.set_title("Stacked Steps with Edgecolors")


@gallery("histogram_multihist_3")
def _(fig):
    ax = fig.subplots()
    ax.hist(three_normals(rng(SEED)), N_BINS, histtype="barstacked", facecolor=STACK_COLORS, label=STACK_COLORS)
    ax.legend()
    ax.set_title("Bars with different Facecolors")


@gallery("histogram_multihist_4")
def _(fig):
    hatches = [".", "o", "x"]
    ax = fig.subplots()
    ax.hist(three_normals(rng(SEED)), N_BINS, histtype="barstacked", hatch=hatches, label=hatches)
    ax.legend()
    ax.set_title("Hatches on Stacked Bars")


@gallery("histogram_multihist_5")
def _(fig):
    linewidths = [1, 2, 3]
    ax = fig.subplots()
    ax.hist(three_normals(rng(SEED)), N_BINS, fill=False, histtype="bar", linewidth=linewidths,
            edgecolor=STACK_COLORS, label=linewidths)
    ax.legend()
    ax.set_title("Bars with Linewidths")


@gallery("histogram_multihist_6")
def _(fig):
    linestyles = ["-", ":", "--"]
    ax = fig.subplots()
    ax.hist(three_normals(rng(SEED)), N_BINS, fill=False, histtype="bar", linestyle=linestyles,
            edgecolor=STACK_COLORS, label=linestyles)
    ax.legend()
    ax.set_title("Bars with Linestyles")


#: The nine numbers ``histogram_normalization`` bins first, and its style.
XDATA = np.array([1.2, 2.3, 3.3, 3.1, 1.7, 3.4, 2.1, 1.25, 1.3])
OUTLINE = {"facecolor": "none", "edgecolor": "C0", "linewidth": 3}
DENSITY_LABEL = "Probability density [$V^{-1}$]"


def binned_dots(ax, xlabel, **hist):
    """``XDATA`` binned in outline with its values as diamonds on the axis."""
    ax.hist(XDATA, **hist, **OUTLINE)
    ax.plot(XDATA, 0 * XDATA, "d")
    ax.set_xlabel(xlabel)


@gallery("histogram_normalization_1")
def _(fig):
    ax = fig.subplots()
    binned_dots(ax, "x bins (dx=1.0)", bins=np.array([1, 2, 3, 4]))
    ax.set_ylabel("Number per bin")


@gallery("histogram_normalization_2")
def _(fig):
    ax = fig.subplots()
    binned_dots(ax, "x bins (dx=0.5)", bins=np.arange(1, 4.5, 0.5))
    ax.set_ylabel("Number per bin")


@gallery("histogram_normalization_3")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplot_mosaic([["auto", "n4"]], sharex=True, sharey=True)
    binned_dots(ax["auto"], "x bins (auto)")
    ax["auto"].set_ylabel("Number per bin")
    binned_dots(ax["n4"], 'x bins ("bins=4")', bins=4)


@gallery("histogram_normalization_4")
def _(fig):
    ax = fig.subplots()
    ax.hist(XDATA, bins=np.arange(1, 4.5, 0.5), density=True, **OUTLINE)
    ax.set_ylabel("Probability density [$V^{-1}$])")
    ax.set_xlabel("x bins (dx=0.5 $V$)")


def normal_pdf():
    """The standard normal density on ``[-4, 4)`` in steps of 0.1."""
    xpdf = np.arange(-4, 4, 0.1)
    return xpdf, 1 / (np.sqrt(2 * np.pi)) * np.exp(-xpdf**2 / 2)


def counts_and_density(fig, xbins, scaled_label):
    """The two panels comparing counts with a density for ``xbins``."""
    xdata = rng(SEED).normal(1000)
    xpdf, pdf = normal_pdf()
    fig.set_layout_engine("constrained")
    ax = fig.subplot_mosaic([["False", "True"]])
    ax["False"].hist(xdata, bins=xbins, density=False, histtype="step", label="Counts")
    ax["False"].plot(xpdf, pdf * len(xdata) * 0.1, label=scaled_label)
    ax["False"].set_ylabel("Count per bin")
    ax["False"].set_xlabel("x bins [V]")
    ax["False"].legend()
    ax["True"].hist(xdata, bins=xbins, density=True, histtype="step", label="density")
    ax["True"].plot(xpdf, pdf, label="$f_X(x)$")
    ax["True"].set_ylabel(DENSITY_LABEL)
    ax["True"].set_xlabel("x bins [$V$]")
    ax["True"].legend()


@gallery("histogram_normalization_5")
def _(fig):
    counts_and_density(fig, np.arange(-4, 4, 0.1), r"$N\,f_X(x)\,\delta x$")


@gallery("histogram_normalization_6")
def _(fig):
    dx = 0.1
    xbins = np.hstack([np.arange(-4, -1.25, 6 * dx), np.arange(-1.25, 4, dx)])
    counts_and_density(fig, xbins, r"$N\,f_X(x)\,\delta x_0$")


BIN_WIDTHS = [0.1, 0.4, 1.2]


@gallery("histogram_normalization_7")
def _(fig):
    xdata = rng(SEED).normal(1000)
    xpdf, pdf = normal_pdf()
    fig.set_layout_engine("constrained")
    ax = fig.subplot_mosaic([["False", "True"]])
    ax["True"].plot(xpdf, pdf, "--", label="$f_X(x)$", color="k")
    for nn, dx in enumerate(BIN_WIDTHS):
        xbins = np.arange(-4, 4, dx)
        ax["False"].plot(xpdf, pdf * 1000 * dx, "--", color=f"C{nn}")
        ax["False"].hist(xdata, bins=xbins, density=False, histtype="step")
        ax["True"].hist(xdata, bins=xbins, density=True, histtype="step", label=dx)
    ax["False"].set_xlabel("x bins [$V$]")
    ax["False"].set_ylabel("Count per bin")
    ax["True"].set_ylabel(DENSITY_LABEL)
    ax["True"].set_xlabel("x bins [$V$]")
    ax["True"].legend(fontsize="small", title="bin width:")


@gallery("histogram_normalization_8", figsize=(3.5, 3))
def _(fig):
    xdata = rng(SEED).normal(1000)
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    for dx in BIN_WIDTHS:
        ax.hist(xdata, bins=np.arange(-4, 4, dx), weights=1 / len(xdata) * np.ones(len(xdata)),
                histtype="step", label=f"{dx}")
    ax.set_xlabel("x bins [$V$]")
    ax.set_ylabel("Bin count / N")
    ax.legend(fontsize="small", title="bin width:")


@gallery("histogram_normalization_9", figsize=(8, 4))
def _(fig):
    g = rng(SEED)
    xdata = g.normal(1000)
    xdata2 = g.normal(100)
    fig.set_layout_engine("constrained")
    ax = fig.subplot_mosaic([["no_norm", "density", "weight"]])
    xbins = np.arange(-4, 4, 0.25)
    ax["no_norm"].hist(xdata, bins=xbins, histtype="step")
    ax["no_norm"].hist(xdata2, bins=xbins, histtype="step")
    ax["no_norm"].set_ylabel("Counts")
    ax["no_norm"].set_xlabel("x bins [$V$]")
    ax["no_norm"].set_title("No normalization")
    ax["density"].hist(xdata, bins=xbins, histtype="step", density=True)
    ax["density"].hist(xdata2, bins=xbins, histtype="step", density=True)
    ax["density"].set_ylabel(DENSITY_LABEL)
    ax["density"].set_title("Density=True")
    ax["density"].set_xlabel("x bins [$V$]")
    ax["weight"].hist(xdata, bins=xbins, histtype="step", weights=1 / len(xdata) * np.ones(len(xdata)),
                      label="N=1000")
    ax["weight"].hist(xdata2, bins=xbins, histtype="step", weights=1 / len(xdata2) * np.ones(len(xdata2)),
                      label="N=100")
    ax["weight"].set_xlabel("x bins [$V$]")
    ax["weight"].set_ylabel("Counts / N")
    ax["weight"].legend(fontsize="small")
    ax["weight"].set_title("Weight = 1/N")


def two_distributions():
    """``hist``'s data: a standard normal and a narrow one around 5."""
    g = rng(SEED)
    dist1 = g.normal(100_000)
    dist2 = 0.4 * g.normal(100_000) + 5
    return dist1, dist2


@gallery("hist_1")
def _(fig):
    dist1, dist2 = two_distributions()
    fig.set_layout_engine("tight")
    axs = fig.subplots(1, 2, sharey=True)
    axs[0].hist(dist1, bins=20)
    axs[1].hist(dist2, bins=20)


@gallery("hist_2")
def _(fig):
    dist1, _ = two_distributions()
    fig.set_layout_engine("tight")
    axs = fig.subplots(1, 2)
    counts, _, patches = axs[0].hist(dist1, bins=20)
    fracs = counts / counts.max()
    norm = colors.Normalize(fracs.min(), fracs.max())
    for thisfrac, thispatch in zip(fracs, patches):
        thispatch.set_facecolor(mpl.colormaps["viridis"](norm(thisfrac)))
    axs[1].hist(dist1, bins=20, density=True)
    axs[1].yaxis.set_major_formatter(PercentFormatter(xmax=1))


@gallery("hist_3")
def _(fig):
    dist1, dist2 = two_distributions()
    fig.set_layout_engine("tight")
    ax = fig.subplots()
    ax.hist2d(dist1, dist2)


@gallery("hist_4", figsize=(5, 15))
def _(fig):
    dist1, dist2 = two_distributions()
    fig.set_layout_engine("tight")
    axs = fig.subplots(3, 1, sharex=True, sharey=True)
    axs[0].hist2d(dist1, dist2, bins=40)
    axs[1].hist2d(dist1, dist2, bins=40, norm=colors.LogNorm())
    axs[2].hist2d(dist1, dist2, bins=(80, 10), norm=colors.LogNorm())


@gallery("multiple_histograms_side_by_side")
def _(fig):
    g = rng(SEED)
    number_of_bins = 20
    data_sets = [g.normal(387, 0, 1), g.normal(387, 6, 1), g.normal(387, -3, 1)]
    hist_range = (np.min(data_sets), np.max(data_sets))
    binned_data_sets = [np.histogram(d, range=hist_range, bins=number_of_bins)[0] for d in data_sets]
    binned_maximums = np.max(binned_data_sets, axis=1)
    x_locations = np.arange(0, sum(binned_maximums), np.max(binned_maximums))
    bin_edges = np.linspace(hist_range[0], hist_range[1], number_of_bins + 1)
    heights = np.diff(bin_edges)
    centers = bin_edges[:-1] + heights / 2
    ax = fig.subplots()
    for x_loc, binned_data in zip(x_locations, binned_data_sets):
        ax.barh(centers, binned_data, height=heights, left=x_loc - 0.5 * binned_data)
    ax.set_xticks(x_locations, ["A", "B", "C"])
    ax.set_ylabel("Data values")
    ax.set_xlabel("Data sets")


@gallery("time_series_histogram", figsize=(6, 8))
def _(fig):
    g = rng(SEED)
    num_series = 1000
    num_points = 100
    num_signal = round(0.10 * num_series)
    x = np.linspace(0, 4 * np.pi, num_points)
    Y = np.cumsum(g.normal(num_series * num_points).reshape(num_series, num_points), axis=-1)
    phi = (np.pi / 8) * g.normal(num_signal).reshape(num_signal, 1)
    noise = 0.05 * g.normal(num_signal * num_points).reshape(num_signal, num_points)
    Y[-num_signal:] = np.sqrt(np.arange(num_points)) * (np.sin(x - phi) + noise)

    fig.set_layout_engine("constrained")
    axes = fig.subplots(nrows=3)
    axes[0].plot(x, Y.T, color="C0", alpha=0.1)
    axes[0].set_title("Line plot with alpha")

    num_fine = 800
    x_fine = np.linspace(x.min(), x.max(), num_fine)
    y_fine = np.concatenate([np.interp(x_fine, x, y_row) for y_row in Y])
    x_fine = np.broadcast_to(x_fine, (num_series, num_fine)).ravel()
    cmap = mpl.colormaps["plasma"]
    cmap = cmap.with_extremes(bad=cmap(0))
    h, xedges, yedges = np.histogram2d(x_fine, y_fine, bins=[400, 100])
    pcm = axes[1].pcolormesh(xedges, yedges, h.T, cmap=cmap, norm="log", vmax=1.5e2, rasterized=True)
    fig.colorbar(pcm, ax=axes[1], label="# points", pad=0)
    axes[1].set_title("2d histogram and log color scale")
    pcm = axes[2].pcolormesh(xedges, yedges, h.T, cmap=cmap, vmax=1.5e2, rasterized=True)
    fig.colorbar(pcm, ax=axes[2], label="# points", pad=0)
    axes[2].set_title("2d histogram and linear color scale")


# --- spectra and correlations --------------------------------------------------


def noisy_sines(g, t, amplitude, noise):
    """Two 10 Hz sines of ``amplitude`` plus two noises made by ``noise``
    from white noise (``cohere`` and ``csd_demo``)."""
    sine = amplitude * np.sin(2 * np.pi * 10 * t)
    nse1 = g.normal(len(t))
    nse2 = g.normal(len(t))
    return sine + noise(nse1), sine + noise(nse2)


def signals_panel(ax, t, s1, s2, xmax):
    """The time-domain panel of ``cohere`` and ``csd_demo``."""
    ax.plot(t, s1, t, s2)
    ax.set_xlim(0, xmax)
    ax.set_xlabel("Time (s)")
    ax.set_ylabel("s1 and s2")
    ax.grid(True)


@gallery("cohere")
def _(fig):
    dt = 0.01
    t = np.arange(0, 30, dt)
    s1, s2 = noisy_sines(rng(SEED), t, 1, lambda nse: nse)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1)
    signals_panel(axs[0], t, s1, s2, 2)
    axs[1].cohere(s1, s2, NFFT=256, Fs=1.0 / dt)
    axs[1].set_ylabel("Coherence")


@gallery("csd_demo")
def _(fig):
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(2, 1)
    dt = 0.01
    t = np.arange(0, 30, dt)
    r = np.exp(-t / 0.05)
    s1, s2 = noisy_sines(rng(SEED), t, 0.01, lambda nse: np.convolve(nse, r, mode="same") * dt)
    signals_panel(ax1, t, s1, s2, 5)
    ax2.csd(s1, s2, NFFT=256, Fs=1.0 / dt)
    ax2.set_ylabel("CSD (dB)")


@gallery("psd_demo_1")
def _(fig):
    dt = 0.01
    t = np.arange(0, 10, dt)
    nse = rng(SEED).normal(len(t))
    r = np.exp(-t / 0.05)
    cnse = np.convolve(nse, r) * dt
    s = 0.1 * np.sin(2 * np.pi * t) + cnse[: len(t)]
    fig.set_layout_engine("constrained")
    ax0, ax1 = fig.subplots(2, 1)
    ax0.plot(t, s)
    ax0.set_xlabel("Time (s)")
    ax0.set_ylabel("Signal")
    ax1.psd(s, NFFT=512, Fs=1 / dt)


@gallery("psd_demo_2")
def _(fig):
    dt = np.pi / 100.0
    fs = 1.0 / dt
    t = np.arange(0, 8, dt)
    y = 10.0 * np.sin(2 * np.pi * 4 * t) + 5.0 * np.sin(2 * np.pi * 4.25 * t)
    y = y + rng(SEED).normal(len(t))
    n = len(t)
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic([
        ["signal", "signal", "signal"],
        ["zero padding", "block size", "overlap"],
    ])
    axs["signal"].plot(t, y)
    axs["signal"].set_xlabel("Time (s)")
    axs["signal"].set_ylabel("Signal")
    for pad_to in (n, n * 2, n * 4):
        axs["zero padding"].psd(y, NFFT=n, pad_to=pad_to, Fs=fs)
    for nfft in (n, n // 2, n // 4):
        axs["block size"].psd(y, NFFT=nfft, pad_to=n, Fs=fs)
    axs["block size"].set_ylabel("")
    for noverlap in (0, int(0.025 * n), int(0.1 * n)):
        axs["overlap"].psd(y, NFFT=n // 2, pad_to=n, noverlap=noverlap, Fs=fs)
    axs["overlap"].set_ylabel("")
    axs["overlap"].set_title("overlap")
    for title, ax in axs.items():
        if title == "signal":
            continue
        ax.set_title(title)
        ax.sharex(axs["zero padding"])
        ax.sharey(axs["zero padding"])


def periodogram_and_welch(fig, xn, xticks):
    """``psd_demo``'s comparison of a periodogram with Welch's estimate."""
    fig.set_layout_engine("constrained")
    ax0, ax1 = fig.subplots(ncols=2)
    yticks = np.arange(-50, 30, 10)
    yrange = (yticks[0], yticks[-1])
    ax0.psd(xn, NFFT=301, Fs=1000, window=mlab.window_none, pad_to=1024, scale_by_freq=True)
    ax0.set_title("Periodogram")
    ax0.set_yticks(yticks)
    ax0.set_xticks(xticks)
    ax0.grid(True)
    ax0.set_ylim(yrange)
    ax1.psd(xn, NFFT=150, Fs=1000, window=mlab.window_none, pad_to=512, noverlap=75, scale_by_freq=True)
    ax1.set_title("Welch")
    ax1.set_xticks(xticks)
    ax1.set_yticks(yticks)
    ax1.set_ylabel("")
    ax1.grid(True)
    ax1.set_ylim(yrange)


#: The amplitudes and frequencies of ``psd_demo``'s two close tones.
TONES = ((2, 150), (8, 140))


@gallery("psd_demo_3")
def _(fig):
    t = np.linspace(0, 0.3, 301)
    xn = sum(a * np.sin(2 * np.pi * f * t) for a, f in TONES) + 5 * rng(SEED).normal(len(t))
    periodogram_and_welch(fig, xn, np.arange(0, 550, 100))


@gallery("psd_demo_4")
def _(fig):
    t = np.linspace(0, 0.3, 301)
    xn = sum(a * np.exp(2j * np.pi * f * t) for a, f in TONES) + 5 * rng(SEED).normal(len(t))
    periodogram_and_welch(fig, xn, np.arange(-500, 550, 200))


@gallery("xcorr_acorr_demo")
def _(fig):
    x, y = rng(SEED).normal(200).reshape(2, 100)
    ax1, ax2 = fig.subplots(2, 1, sharex=True)
    ax1.xcorr(x, y, usevlines=True, maxlags=50, normed=True, lw=2)
    ax1.grid(True)
    ax1.set_title("Cross-correlation (xcorr)")
    ax2.acorr(x, usevlines=True, normed=True, maxlags=50, lw=2)
    ax2.grid(True)
    ax2.set_title("Auto-correlation (acorr)")


# --- pie charts ----------------------------------------------------------------


@gallery("bar_of_pie", figsize=(9, 5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    fig.subplots_adjust(wspace=0)
    overall_ratios = [0.27, 0.56, 0.17]
    pie = ax1.pie(overall_ratios, autopct="%1.1f%%", startangle=-180 * overall_ratios[0],
                  labels=["Approve", "Disapprove", "Undecided"], explode=[0.1, 0, 0])
    age_ratios = [0.33, 0.54, 0.07, 0.06]
    age_labels = ["Under 35", "35-49", "50-65", "Over 65"]
    bottom = 1
    width = 0.2
    for j, (height, label) in enumerate(reversed([*zip(age_ratios, age_labels)])):
        bottom -= height
        bc = ax2.bar(0, height, width, bottom=bottom, color="C0", label=label, alpha=0.1 + 0.25 * j)
        ax2.bar_label(bc, labels=[f"{height:.0%}"], label_type="center")
    ax2.set_title("Age of approvers")
    ax2.legend()
    ax2.axis("off")
    ax2.set_xlim(-2.5 * width, 2.5 * width)
    wedge = pie.wedges[0]
    center, r = wedge.center, wedge.r
    bar_height = sum(age_ratios)
    for theta, bar_y in ((wedge.theta2, bar_height), (wedge.theta1, 0)):
        x = r * np.cos(np.pi / 180 * theta) + center[0]
        y = r * np.sin(np.pi / 180 * theta) + center[1]
        con = ConnectionPatch(xyA=(-width / 2, bar_y), coordsA=ax2.transData, xyB=(x, y), coordsB=ax1.transData)
        con.set_color([0, 0, 0])
        con.set_linewidth(4)
        ax2.add_artist(con)


#: ``nested_pie``'s values: three groups of two.
NESTED_VALS = np.array([[60.0, 32.0], [37.0, 40.0], [29.0, 10.0]])
RING = 0.3


@gallery("nested_pie_1")
def _(fig):
    ax = fig.subplots()
    tab20c = mpl.color_sequences["tab20c"]
    outer_colors = [tab20c[i] for i in [0, 4, 8]]
    inner_colors = [tab20c[i] for i in [1, 2, 5, 6, 9, 10]]
    ax.pie(NESTED_VALS.sum(axis=1), radius=1, colors=outer_colors, wedgeprops=dict(width=RING, edgecolor="w"))
    ax.pie(NESTED_VALS.flatten(), radius=1 - RING, colors=inner_colors,
           wedgeprops=dict(width=RING, edgecolor="w"))
    ax.set(aspect="equal", title="Pie plot with `ax.pie`")


@gallery("nested_pie_2")
def _(fig):
    ax = fig.subplots(subplot_kw=dict(projection="polar"))
    valsnorm = NESTED_VALS / np.sum(NESTED_VALS) * 2 * np.pi
    valsleft = np.cumsum(np.append(0, valsnorm.flatten()[:-1])).reshape(NESTED_VALS.shape)
    cmap = mpl.colormaps["tab20c"]
    outer_colors = cmap(np.arange(3) * 4)
    inner_colors = cmap([1, 2, 5, 6, 9, 10])
    ax.bar(x=valsleft[:, 0], width=valsnorm.sum(axis=1), bottom=1 - RING, height=RING,
           color=outer_colors, edgecolor="w", linewidth=1, align="edge")
    ax.bar(x=valsleft.flatten(), width=valsnorm.flatten(), bottom=1 - 2 * RING, height=RING,
           color=inner_colors, edgecolor="w", linewidth=1, align="edge")
    ax.set(title="Pie plot with `ax.bar` and polar coordinates")
    ax.set_axis_off()


@gallery("pie_and_donut_labels_1", figsize=(6, 3))
def _(fig):
    ax = fig.subplots()
    recipe = ["375 g flour", "75 g sugar", "250 g butter", "300 g berries"]
    data = [int(x.split()[0]) for x in recipe]
    ingredients = [x.split()[-1] for x in recipe]
    pie = ax.pie(data)
    ax.pie_label(pie, "{frac:.1%}\n({absval:d}g)", textprops=dict(color="w", size=8, weight="bold"))
    ax.legend(pie.wedges, ingredients, title="Ingredients", loc="center left", bbox_to_anchor=(1, 0, 0.5, 1))
    ax.set_title("Matplotlib bakery: A pie")


@gallery("pie_and_donut_labels_2", figsize=(6, 3))
def _(fig):
    ax = fig.subplots(subplot_kw=dict(aspect="equal"))
    recipe = ["225 g flour", "90 g sugar", "1 egg", "60 g butter", "100 ml milk", "1/2 package of yeast"]
    pie = ax.pie([225, 90, 50, 60, 100, 5], wedgeprops=dict(width=0.5), startangle=-40)
    bbox_props = dict(boxstyle="square,pad=0.3", fc="w", ec="k", lw=0.72)
    kw = dict(arrowprops=dict(arrowstyle="-"), bbox=bbox_props, zorder=0, va="center")
    for i, p in enumerate(pie.wedges):
        ang = (p.theta2 - p.theta1) / 2.0 + p.theta1
        y = np.sin(np.deg2rad(ang))
        x = np.cos(np.deg2rad(ang))
        horizontalalignment = {-1: "right", 1: "left"}[int(np.sign(x))]
        kw["arrowprops"].update({"connectionstyle": f"angle,angleA=0,angleB={ang}"})
        ax.annotate(recipe[i], xy=(x, y), xytext=(1.35 * np.sign(x), 1.4 * y),
                    horizontalalignment=horizontalalignment, **kw)
    ax.set_title("Matplotlib bakery: A donut")


PIE_LABELS = "Frogs", "Hogs", "Dogs", "Logs"
PIE_SIZES = [15, 30, 45, 10]
#: Explodes the second slice, 'Hogs'.
HOGS = (0, 0.1, 0, 0)


@gallery("pie_features_1")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS)


@gallery("pie_features_2")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS, autopct="%1.1f%%")


@gallery("pie_features_3")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS, colors=["olivedrab", "rosybrown", "gray", "saddlebrown"])


@gallery("pie_features_4")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS, hatch=["**O", "oO", "O.O", ".||."])


@gallery("pie_features_5")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS, autopct="%1.1f%%", pctdistance=1.25, labeldistance=0.6)


@gallery("pie_features_6")
def _(fig):
    fig.subplots().pie(PIE_SIZES, explode=HOGS, labels=PIE_LABELS, autopct="%1.1f%%", shadow=True, startangle=90)


@gallery("pie_features_7")
def _(fig):
    fig.subplots().pie(PIE_SIZES, labels=PIE_LABELS, autopct="%.0f%%", textprops={"size": "small"}, radius=0.5)


@gallery("pie_features_8")
def _(fig):
    fig.subplots().pie(PIE_SIZES, explode=HOGS, labels=PIE_LABELS, autopct="%1.1f%%",
                       shadow={"ox": -0.04, "edgecolor": "none", "shade": 0.9}, startangle=90)


PIE_DATA = [36, 24, 8, 12]
BREAKFAST = ["spam", "eggs", "bacon", "sausage"]


def labelled_pie(fig, *labellings):
    """A pie of ``PIE_DATA`` labelled once per ``(labels, options)``."""
    ax = fig.subplots()
    pie = ax.pie(PIE_DATA)
    for labels, options in labellings:
        ax.pie_label(pie, labels, **options)


@gallery("pie_label_1")
def _(fig):
    labelled_pie(fig, (BREAKFAST, {}))


@gallery("pie_label_2")
def _(fig):
    labelled_pie(fig, (BREAKFAST, {"distance": 1.1}))


@gallery("pie_label_3")
def _(fig):
    labelled_pie(fig, (BREAKFAST, {"rotate": True}))


@gallery("pie_label_4")
def _(fig):
    labelled_pie(fig, ("{absval:.1f}", {}))


@gallery("pie_label_5")
def _(fig):
    labelled_pie(fig, ("{frac:.1%}", {}))


@gallery("pie_label_6")
def _(fig):
    labelled_pie(fig, ("{absval:d}\n{frac:.1%}", {}))


@gallery("pie_label_7")
def _(fig):
    labelled_pie(fig, (BREAKFAST, {"textprops": {"fontsize": "large", "color": "white"}}))


@gallery("pie_label_8")
def _(fig):
    labelled_pie(fig, (BREAKFAST, {"distance": 1.1}), ("{frac:.1%}", {"distance": 0.7}),
                 ("{absval:d}", {"distance": 0.4}))


# --- polar charts --------------------------------------------------------------


@gallery("polar_bar")
def _(fig):
    g = rng(SEED)
    n = 20
    theta = np.linspace(0.0, 2 * np.pi, n, endpoint=False)
    radii = 10 * g.uniform(n)
    width = np.pi / 4 * g.uniform(n)
    ax = fig.add_subplot(projection="polar")
    ax.bar(theta, radii, width=width, bottom=0.0, color=mpl.colormaps["viridis"](radii / 10.0), alpha=0.5)


@gallery("polar_demo", figsize=(5, 8))
def _(fig):
    r = np.arange(0, 2, 0.01)
    theta = 2 * np.pi * r
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 1, subplot_kw={"projection": "polar"})
    ax = axs[0]
    ax.plot(theta, r)
    ax.set_rmax(2)
    ax.set_rticks([0.5, 1, 1.5, 2])
    ax.set_rlabel_position(-22.5)
    ax.grid(True)
    ax.set_title("A line plot on a polar axis", va="bottom")
    ax = axs[1]
    ax.plot(theta, r)
    ax.set_rmax(2)
    ax.set_rmin(1)
    ax.set_rorigin(0)
    ax.set_thetamin(0)
    ax.set_thetamax(225)
    ax.set_rticks([1, 1.5, 2])
    ax.set_rlabel_position(-22.5)
    ax.grid(True)
    ax.set_title("Same plot, but with reduced axis limits", va="bottom")


def polar_errorbars(fig, xerr, yerr, color, title):
    """``polar_error_caps``: eight points with capped theta and r errors."""
    theta = np.arange(0, 2 * np.pi, np.pi / 4)
    r = theta / np.pi / 2 + 0.5
    ax = fig.add_subplot(projection="polar")
    ax.errorbar(theta, r, xerr=xerr, yerr=yerr, capsize=7, fmt="o", c=color)
    ax.set_title(title)


@gallery("polar_error_caps_1", figsize=(10, 10))
def _(fig):
    polar_errorbars(fig, 0.25, 0.1, "seagreen", "Pretty polar error bars")


@gallery("polar_error_caps_2", figsize=(10, 10))
def _(fig):
    polar_errorbars(fig, 5.25, 0.1, "darkred", "Overlapping theta error bars")


@gallery("polar_error_caps_3", figsize=(10, 10))
def _(fig):
    polar_errorbars(fig, 0.25, 10.1, "orangered", "Large radius error bars")


@gallery("polar_legend")
def _(fig):
    ax = fig.add_subplot(projection="polar", facecolor="lightgoldenrodyellow")
    r = np.linspace(0, 3, 301)
    theta = 2 * np.pi * r
    ax.plot(theta, r, color="tab:orange", lw=3, label="a line")
    ax.plot(0.5 * theta, r, color="tab:blue", ls="--", lw=3, label="another line")
    ax.tick_params(grid_color="palegoldenrod")
    angle = np.deg2rad(67.5)
    ax.legend(loc="lower left", bbox_to_anchor=(0.5 + np.cos(angle) / 2, 0.5 + np.sin(angle) / 2))


def polar_scatter(fig):
    """``polar_scatter``'s points: size grows with r, colour with theta."""
    g = rng(SEED)
    n = 150
    r = 2 * g.uniform(n)
    theta = 2 * np.pi * g.uniform(n)
    ax = fig.add_subplot(projection="polar")
    ax.scatter(theta, r, c=theta, s=200 * r**2, cmap="hsv", alpha=0.75)
    return ax


@gallery("polar_scatter_1")
def _(fig):
    polar_scatter(fig)


@gallery("polar_scatter_2")
def _(fig):
    ax = polar_scatter(fig)
    ax.set_rorigin(-2.5)
    ax.set_theta_zero_location("W", offset=10)


@gallery("polar_scatter_3")
def _(fig):
    ax = polar_scatter(fig)
    ax.set_thetamin(45)
    ax.set_thetamax(135)
