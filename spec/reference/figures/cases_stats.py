"""Statistical plots: boxplot, violinplot, errorbar, eventplot, stem, stackplot,
pie and fill_between/fill_betweenx."""

import numpy as np

from spec_support import case, rng

SAMPLES = [
    rng(200).normal(100, loc=0.0, scale=1.0),
    rng(201).normal(80, loc=1.0, scale=0.5),
    np.concatenate([rng(202).normal(60, loc=-0.5, scale=1.5), [6.0, -5.5, 5.0]]),
]
SHARES = [35, 25, 20, 12, 8]
NAMES = ["alpha", "beta", "gamma", "delta", "epsilon"]


# --- boxplot ---------------------------------------------------------------


@case("stats/boxplot_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.boxplot(SAMPLES)


@case("stats/boxplot_notch", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.boxplot(SAMPLES, notch=True)


@case("stats/boxplot_horizontal", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.boxplot(SAMPLES, orientation="horizontal")


@case("stats/boxplot_whis_percentiles_and_range", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    ax1.boxplot(SAMPLES, whis=(5, 95))
    ax2.boxplot(SAMPLES, whis=(0, 100))


@case("stats/boxplot_no_fliers_custom_sym", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    ax1.boxplot(SAMPLES, showfliers=False)
    ax2.boxplot(SAMPLES, sym="r+")


@case("stats/boxplot_showmeans_meanline", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    ax1.boxplot(SAMPLES, showmeans=True)
    ax2.boxplot(SAMPLES, showmeans=True, meanline=True)


@case("stats/boxplot_patch_artist_colored", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    parts = ax.boxplot(SAMPLES, patch_artist=True, medianprops={"color": "k", "linewidth": 2})
    for box, color in zip(parts["boxes"], ["pink", "lightblue", "lightgreen"]):
        box.set_facecolor(color)


@case("stats/boxplot_labels_positions_widths", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.boxplot(SAMPLES, positions=[1, 2.5, 6], widths=[0.3, 0.8, 1.5], tick_labels=["low", "mid", "wide"])


@case("stats/boxplot_props_and_caps", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.boxplot(
        SAMPLES,
        boxprops={"color": "tab:blue", "linewidth": 2},
        whiskerprops={"linestyle": "--"},
        capprops={"color": "tab:red"},
        flierprops={"marker": "D", "markerfacecolor": "tab:orange", "markersize": 5},
        capwidths=0.4,
    )


# --- violinplot ------------------------------------------------------------


@case("stats/violin_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.violinplot(SAMPLES)


@case("stats/violin_means_medians_no_extrema", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.violinplot(SAMPLES, showmeans=True, showmedians=True, showextrema=False)


@case("stats/violin_quantiles_points_bw", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.violinplot(SAMPLES, quantiles=[[0.25, 0.75], [0.1, 0.5, 0.9], [0.05]], points=50, bw_method=0.3)


@case("stats/violin_split_sides", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.violinplot(SAMPLES[:2], positions=[1, 2], side="low", showmedians=True)
    ax.violinplot(SAMPLES[1:], positions=[1, 2], side="high", showmedians=True)


@case("stats/violin_horizontal_widths_colors", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.violinplot(SAMPLES, orientation="horizontal", widths=[0.3, 0.6, 0.9], facecolor="tab:red", linecolor="k")


# --- errorbar --------------------------------------------------------------


@case("stats/errorbar_yerr_symmetric", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(8)
    ax.errorbar(x, np.sqrt(x + 1), yerr=0.1 * (x + 1))


@case("stats/errorbar_xerr_yerr_asymmetric", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(1, 7, dtype=float)
    ax.errorbar(x, x**1.5, xerr=[0.1 * x, 0.3 * np.ones(6)], yerr=[0.5 * x, x], fmt="o")


@case("stats/errorbar_capsize_fmt_elinewidth", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(6)
    ax.errorbar(x, x, yerr=0.8, fmt="s--", capsize=6, capthick=2, elinewidth=3, ecolor="tab:gray", markersize=7)
    ax.errorbar(x, x + 4, yerr=0.5, fmt="none", ecolor="tab:red", elinestyle=":")


@case("stats/errorbar_limits", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(10, dtype=float)
    ax.errorbar(x, np.sin(x / 2), yerr=0.2, uplims=True)
    ax.errorbar(x, np.sin(x / 2) + 1, yerr=0.2, lolims=True)
    ax.errorbar(x, np.sin(x / 2) + 2, yerr=0.2, xerr=0.3, lolims=x % 2 == 0, xuplims=x > 5)


@case("stats/errorbar_errorevery", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 4, 40)
    ax.errorbar(x, np.exp(-x), yerr=0.05, errorevery=5)
    ax.errorbar(x, np.exp(-x) + 0.5, yerr=0.05, errorevery=(2, 6), capsize=3)



# --- eventplot -------------------------------------------------------------


@case("stats/eventplot_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.eventplot([rng(210 + i).uniform(20, 0, 10) for i in range(4)])


@case("stats/eventplot_vertical_colors_offsets", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    events = [rng(220 + i).uniform(15, 0, 1) for i in range(3)]
    ax.eventplot(
        events,
        orientation="vertical",
        lineoffsets=[1, 3, 4],
        linelengths=[0.5, 1.5, 0.8],
        colors=["C0", "C1", "C2"],
        linewidths=[1, 2, 3],
    )


# --- stem ------------------------------------------------------------------


@case("stats/stem_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.1, 2 * np.pi, 25)
    ax.stem(x, np.cos(x))


@case("stats/stem_formats_bottom", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(12)
    ax.stem(x, np.sin(x / 2) + 1, linefmt="g:", markerfmt="D", basefmt="k-", bottom=1)


@case("stats/stem_horizontal", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.arange(10)
    ax.stem(x, x**0.5, orientation="horizontal", markerfmt="ro")


# --- stackplot -------------------------------------------------------------

STACK_X = np.arange(12)
STACK_Y = [
    2 + np.sin(STACK_X / 2),
    1.5 + np.cos(STACK_X / 3),
    1 + 0.1 * STACK_X,
]


@case("stats/stackplot_zero", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.stackplot(STACK_X, STACK_Y)





@case("stats/stackplot_baselines_sym_wiggle_weighted", figsize=(9, 3))
def _(fig):
    axs = fig.subplots(1, 3)
    for ax, baseline in zip(axs, ["sym", "wiggle", "weighted_wiggle"]):
        ax.stackplot(STACK_X, STACK_Y, baseline=baseline)
        ax.set_title(baseline)


@case("stats/stackplot_colors_hatch_edges", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.stackplot(STACK_X, *STACK_Y, colors=["#8dd3c7", "#ffffb3", "#bebada"], hatch=["", "//", ".."], edgecolor="k")


# --- pie -------------------------------------------------------------------


@case("stats/pie_default", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES)


@case("stats/pie_labels_autopct", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES, labels=NAMES, autopct="%1.1f%%")


@case("stats/pie_explode_shadow_startangle", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES, labels=NAMES, explode=[0.1, 0, 0, 0.2, 0], shadow=True, startangle=90)


@case("stats/pie_clockwise_label_and_pct_distance", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES, labels=NAMES, counterclock=False, labeldistance=0.7, autopct="%d%%", pctdistance=1.15)


@case("stats/pie_donut_nested", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES, radius=1, wedgeprops={"width": 0.3, "edgecolor": "w"})
    ax.pie([60, 40], radius=0.7, colors=["0.5", "0.8"], wedgeprops={"width": 0.3, "edgecolor": "w"})


@case("stats/pie_not_normalized", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.pie([0.2, 0.3, 0.1], normalize=False, labels=["a", "b", "c"])


@case("stats/pie_wedgeprops_textprops_colors", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(
        SHARES,
        labels=NAMES,
        colors=["#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854"],
        wedgeprops={"linewidth": 2, "edgecolor": "k", "linestyle": "--"},
        textprops={"fontsize": 14, "color": "darkblue"},
        hatch=["", "/", "x", "", "."],
    )


@case("stats/pie_label", figsize=(8, 4))
def _(fig):
    left, right = fig.subplots(1, 2)
    pie = left.pie([15, 30, 45, 10])
    left.pie_label(pie, "{absval:d}", distance=0.7)
    left.pie_label(pie, ["frogs", "hogs", "dogs", "logs"], distance=1.15)
    partial = right.pie([0.2, 0.3, 0.1], normalize=False)
    right.pie_label(partial, "{frac:.1%}", rotate=True, distance=1.1,
                    textprops=dict(color="tab:red", size=8))
    right.pie_label(partial, ["a", "b", "c"], alignment="outer", distance=0.5)


@case("stats/pie_rotatelabels_frame_center", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.pie(SHARES, labels=NAMES, rotatelabels=True, frame=True, center=(2, 1), radius=1.5)


# --- fill_between ----------------------------------------------------------

FB_X = np.linspace(0, 4 * np.pi, 60)


@case("stats/fill_between_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.fill_between(FB_X, np.sin(FB_X))


@case("stats/fill_between_two_curves_alpha", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    y = np.sin(FB_X)
    ax.plot(FB_X, y, "k")
    ax.fill_between(FB_X, y - 0.3, y + 0.3, alpha=0.3, color="tab:blue", edgecolor="tab:blue", linewidth=1)


@case("stats/fill_between_where_interpolate", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    x = np.linspace(0, 2 * np.pi, 13)
    y1, y2 = np.sin(x), np.cos(x)
    for ax, interpolate in [(ax1, False), (ax2, True)]:
        ax.plot(x, y1, "k", x, y2, "k--")
        ax.fill_between(x, y1, y2, where=y1 >= y2, interpolate=interpolate, color="green", alpha=0.4)
        ax.fill_between(x, y1, y2, where=y1 < y2, interpolate=interpolate, color="red", alpha=0.4)


@case("stats/fill_between_step", figsize=(6, 3))
def _(fig):
    axs = fig.subplots(1, 3, sharey=True)
    x = np.arange(8)
    y = np.array([1, 3, 2, 4, 3, 5, 2, 3], dtype=float)
    for ax, step in zip(axs, ["pre", "mid", "post"]):
        ax.fill_between(x, y, step=step, alpha=0.5)
        ax.step(x, y, where=step)


@case("stats/fill_betweenx", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    y = np.linspace(0, 2, 50)
    ax.fill_betweenx(y, np.sin(4 * y), 1.5 * np.sin(4 * y) + 1, where=y < 1.5, hatch="\\\\", facecolor="lightyellow", edgecolor="k")
