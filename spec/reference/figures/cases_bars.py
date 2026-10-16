"""Bar charts: bar/barh geometry, stacking, styling, error bars and labels."""

import numpy as np

from spec_support import case

HEIGHTS = np.array([3.0, 7.0, 5.0, 2.0, 6.0])
X = np.arange(len(HEIGHTS))


@case("bars/default")
def _(fig):
    ax = fig.subplots()
    ax.bar(X, HEIGHTS)


@case("bars/barh_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barh(X, HEIGHTS)


@case("bars/width_scalar_and_array", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.bar(X, HEIGHTS, width=0.4)
    ax2.bar(X, HEIGHTS, width=[0.2, 0.4, 0.6, 0.8, 1.0])


@case("bars/align_edge", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.bar(X, HEIGHTS, width=0.5, align="edge")
    ax2.bar(X, HEIGHTS, width=-0.5, align="edge", color="C1")


@case("bars/stacked_bottom", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    a = np.array([2.0, 3.0, 4.0, 1.0])
    b = np.array([1.0, 2.0, 1.0, 3.0])
    c = np.array([3.0, 1.0, 2.0, 2.0])
    ax.bar(np.arange(4), a)
    ax.bar(np.arange(4), b, bottom=a)
    ax.bar(np.arange(4), c, bottom=a + b)


@case("bars/barh_stacked_left", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    a = np.array([2.0, 3.0, 4.0])
    b = np.array([4.0, 1.0, 2.0])
    ax.barh(np.arange(3), a, height=0.5)
    ax.barh(np.arange(3), b, left=a, height=0.5)


@case("bars/colors_edgecolors_linewidth", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(
        X,
        HEIGHTS,
        color=["tab:red", "tab:green", "tab:blue", "gold", "0.6"],
        edgecolor="k",
        linewidth=[0, 1, 2, 3, 4],
    )


@case("bars/hatches", figsize=(6, 3))
def _(fig):
    ax = fig.subplots()
    hatches = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*", "//", "xx", "/o", "\\|"]
    ax.bar(np.arange(len(hatches)), 1, hatch=hatches, color="white", edgecolor="k")


@case("bars/hatch_colors_and_linewidth", figsize=(5, 3), rc={"hatch.linewidth": 2.0})
def _(fig):
    ax = fig.subplots()
    ax.bar([0, 1, 2], [3, 2, 4], hatch="//", color="lightblue", edgecolor="navy")
    ax.bar([3, 4], [2, 3], hatch="x", facecolor="none", edgecolor="tab:red", hatchcolor="tab:green")


@case("bars/yerr_capsize", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(X, HEIGHTS, yerr=[0.5, 1.0, 0.3, 0.8, 0.6], capsize=5, color="C2", ecolor="k")


@case("bars/asymmetric_yerr_error_kw", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    lower = np.array([0.5, 1.0, 0.2, 0.3, 1.0])
    upper = np.array([1.0, 0.5, 0.8, 1.5, 0.4])
    ax.bar(X, HEIGHTS, yerr=[lower, upper], error_kw={"elinewidth": 2, "capsize": 8, "capthick": 2, "ecolor": "tab:red"})


@case("bars/barh_xerr", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barh(X, HEIGHTS, xerr=0.7, capsize=3)


@case("bars/bar_label_edge_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    bars = ax.bar(X, HEIGHTS * 1.5)
    ax.bar_label(bars)


@case("bars/bar_label_center_stacked", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    a = np.array([20.0, 35.0, 30.0])
    b = np.array([25.0, 32.0, 34.0])
    lower = ax.bar(np.arange(3), a)
    upper = ax.bar(np.arange(3), b, bottom=a)
    ax.bar_label(lower, label_type="center")
    ax.bar_label(upper, label_type="center", color="white")
    ax.bar_label(upper)


@case("bars/bar_label_fmt_padding", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    bars = ax1.bar(X, HEIGHTS / 7)
    ax1.bar_label(bars, fmt="%.2f", padding=3, fontsize=8)
    bars = ax2.barh(X, HEIGHTS / 7)
    ax2.bar_label(bars, fmt="{:.0%}", padding=-25, color="white")


@case("bars/bar_label_custom_labels", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    bars = ax.bar(X, HEIGHTS)
    ax.bar_label(bars, labels=["a", "bb", "ccc", "", "e"], padding=2, rotation=90)


@case("bars/negative_values_labeled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    values = np.array([3.0, -2.0, 4.5, -1.0, 0.5])
    bars = ax.bar(X, values, color=np.where(values < 0, "tab:red", "tab:green"))
    ax.axhline(0, color="k", linewidth=0.8)
    ax.bar_label(bars, fmt="%+.1f")


@case("bars/negative_barh_labeled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    bars = ax.barh(X, [-3.0, 2.0, -1.5, 4.0, 1.0])
    ax.bar_label(bars, padding=4)


@case("bars/log_scale", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(X, [3, 40, 700, 5000, 25], log=True)


@case("bars/categorical_strings", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(["apple", "banana", "cherry", "date"], [4, 6, 3, 5])


@case("bars/categorical_repeated_calls", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barh(["north", "south", "east"], [10, 20, 15])
    ax.barh(["west", "north"], [5, 8], color="C1")


@case("bars/tick_label_param", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar([1, 3, 5], [2, 5, 3], tick_label=["one", "three", "five"], width=1.5)


@case("bars/broken_barh", figsize=(6, 3))
def _(fig):
    ax = fig.subplots()
    ax.broken_barh([(10, 50), (100, 20), (130, 10)], (20, 9), facecolors="tab:blue")
    ax.broken_barh([(10, 50), (100, 20), (130, 10)], (10, 9), facecolors=("tab:orange", "tab:green", "tab:red"))
    ax.set_ylim(5, 35)
    ax.set_xlim(0, 200)
    ax.set_yticks([15, 25], labels=["Bill", "Jim"])
    ax.grid(True)


@case("bars/grouped_via_offsets", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    groups = ["G1", "G2", "G3", "G4"]
    series = {"men": [20, 34, 30, 35], "women": [25, 32, 34, 20], "kids": [10, 15, 12, 18]}
    x = np.arange(len(groups))
    width = 0.25
    for i, (name, values) in enumerate(series.items()):
        bars = ax.bar(x + (i - 1) * width, values, width, label=name)
        ax.bar_label(bars, padding=2, fontsize=7)
    ax.set_xticks(x, groups)
    ax.set_ylim(0, 40)
    ax.legend(loc="upper left", ncols=3)


@case("bars/grouped_bar_lists", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    heights = [[1.0, 2.0, 3.0], [1.5, 2.5, 2.0], [0.5, 1.0, 4.0]]
    ax.grouped_bar(heights, tick_labels=["A", "B", "C"], labels=["x", "y", "z"])
    ax.legend()


@case("bars/grouped_bar_dict_horizontal", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    data = {"2023": [3.0, 5.0], "2024": [4.0, 2.0], "2025": [6.0, 1.0]}
    ax.grouped_bar(data, tick_labels=["east", "west"], orientation="horizontal", group_spacing=0.5, bar_spacing=0.1)
