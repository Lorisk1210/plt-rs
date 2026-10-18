"""Styling through rcParams: individual ``rc=`` overrides and every built-in
style sheet applied to the same small standard plot.

Style sheets are applied through the case's ``rc`` (not ``style.context``
inside the build function) because the framework creates the figure before
``build`` runs and draws it after ``build`` returns: figure-level and
draw-time parameters (figure facecolor, lazily created ticks, ...) must be
active for the whole lifetime of the figure. Two parameters are pinned on
top of each sheet so that goldens do not depend on the host: the font family
(seaborn sheets prefer Arial/Liberation Sans, which may or may not be
installed) and unhinted text (``classic`` asks for ``text.hinting: auto``).
"""

import matplotlib.style
import numpy as np
from cycler import cycler
from matplotlib.patches import Rectangle

from spec_support import case, rng

STYLE_SHEETS = [
    "Solarize_Light2",
    "bmh",
    "classic",
    "dark_background",
    "fast",
    "fivethirtyeight",
    "ggplot",
    "grayscale",
    "petroff6",
    "petroff8",
    "petroff10",
    "seaborn-v0_8",
    "seaborn-v0_8-bright",
    "seaborn-v0_8-colorblind",
    "seaborn-v0_8-dark",
    "seaborn-v0_8-dark-palette",
    "seaborn-v0_8-darkgrid",
    "seaborn-v0_8-deep",
    "seaborn-v0_8-muted",
    "seaborn-v0_8-notebook",
    "seaborn-v0_8-paper",
    "seaborn-v0_8-pastel",
    "seaborn-v0_8-poster",
    "seaborn-v0_8-talk",
    "seaborn-v0_8-ticks",
    "seaborn-v0_8-white",
    "seaborn-v0_8-whitegrid",
    "tableau-colorblind10",
]

PINNED = {"font.family": "DejaVu Sans", "text.hinting": "none"}

X = np.linspace(0, 10, 30)


def _standard_plot(fig):
    """Lines (colour cycle), scatter, legend, labels, bars with error bars."""
    ax1, ax2 = fig.subplots(1, 2)
    for i in range(4):
        ax1.plot(X, np.sin(X + 0.8 * i) + 0.5 * i, label=f"line {i}")
    ax1.scatter(X[::3], np.cos(X[::3]) - 1.5, label="points")
    ax1.set_title("Lines and scatter")
    ax1.set_xlabel("x")
    ax1.set_ylabel("y")
    ax1.legend(fontsize="small", loc="upper right")
    ax2.bar(["a", "b", "c", "d"], [3, 5, 2, 4], yerr=[0.3, 0.5, 0.2, 0.4])
    ax2.set_title("Bars")


def _register_style_sheet(name):
    rc = {**matplotlib.style.library[name], **PINNED}
    case(f"styles/sheet_{name}", figsize=(7, 3.5), rc=rc)(_standard_plot)


for _name in STYLE_SHEETS:
    _register_style_sheet(_name)


# --- individual rcParams -----------------------------------------------------


@case(
    "styles/rc_prop_cycle_color_and_linestyle",
    figsize=(5, 4),
    rc={"axes.prop_cycle": cycler(color=["#e41a1c", "#377eb8", "#4daf4a"]) + cycler(linestyle=["-", "--", ":"])},
)
def _(fig):
    ax = fig.subplots()
    for i in range(5):
        ax.plot(X, X * (i + 1))


@case(
    "styles/rc_font_sizes_and_title",
    rc={
        "font.size": 13,
        "axes.titlesize": "x-large",
        "axes.titleweight": "bold",
        "axes.titlelocation": "left",
        "axes.titlepad": 12,
        "axes.labelsize": "small",
        "xtick.labelsize": 8,
        "ytick.labelsize": 15,
    },
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))
    ax.set_title("Title")
    ax.set_xlabel("x label")
    ax.set_ylabel("y label")


@case(
    "styles/rc_lines_defaults",
    figsize=(5, 4),
    rc={"lines.linewidth": 3, "lines.linestyle": "--", "lines.marker": "o", "lines.markersize": 4, "lines.dashed_pattern": [2, 1]},
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))
    ax.plot(X, np.cos(X))


@case(
    "styles/rc_axes_grid",
    figsize=(5, 4),
    rc={"axes.grid": True, "axes.grid.axis": "y", "grid.color": "tab:purple", "grid.linestyle": ":", "grid.linewidth": 1.5, "axes.axisbelow": True},
)
def _(fig):
    ax = fig.subplots()
    ax.bar(np.arange(5), [3, 5, 2, 4, 6])


@case(
    "styles/rc_ticks_inward_mirrored_minor",
    figsize=(5, 4),
    rc={
        "xtick.direction": "in",
        "ytick.direction": "in",
        "xtick.top": True,
        "ytick.right": True,
        "xtick.minor.visible": True,
        "ytick.minor.visible": True,
        "xtick.major.size": 7,
        "ytick.major.size": 7,
        "xtick.major.pad": 6,
    },
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))


@case(
    "styles/rc_spines_and_colors",
    rc={
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.facecolor": "#eeeeee",
        "axes.edgecolor": "gray",
        "axes.linewidth": 2,
        "axes.labelcolor": "darkred",
        "xtick.color": "tab:blue",
        "ytick.labelcolor": "tab:green",
        "text.color": "navy",
    },
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))
    ax.set_title("colored title")
    ax.set_xlabel("x")
    ax.set_ylabel("y")


@case(
    "styles/rc_figure_facecolor_subplot_params",
    figsize=(5, 4),
    rc={
        "figure.facecolor": "lightgray",
        "figure.subplot.left": 0.2,
        "figure.subplot.right": 0.95,
        "figure.subplot.top": 0.95,
        "figure.subplot.wspace": 0.5,
    },
)
def _(fig):
    for i, ax in enumerate(fig.subplots(1, 2)):
        ax.plot(X, np.sin(X + i))


@case(
    "styles/rc_margins_and_formatter",
    figsize=(5, 4),
    rc={
        "axes.xmargin": 0,
        "axes.ymargin": 0.2,
        "axes.formatter.limits": (-2, 2),
        "axes.formatter.use_mathtext": True,
        "axes.unicode_minus": False,
    },
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X * 100, np.sin(X) * 1e-3)


@case(
    "styles/rc_patch_and_hist",
    figsize=(5, 4),
    rc={"patch.force_edgecolor": True, "patch.edgecolor": "k", "patch.linewidth": 1.5, "patch.facecolor": "salmon", "hist.bins": 20, "hatch.color": "white"},
)
def _(fig):
    ax = fig.subplots()
    ax.hist(rng(400).normal(500), color="C2")
    ax.add_patch(Rectangle((-3, 40), 1.5, 15, hatch="//"))


@case(
    "styles/rc_legend",
    figsize=(5, 4),
    rc={
        "legend.frameon": True,
        "legend.loc": "upper left",
        "legend.fontsize": "large",
        "legend.handlelength": 3,
        "legend.facecolor": "lightyellow",
        "legend.edgecolor": "k",
        "legend.fancybox": False,
        "legend.framealpha": 1,
    },
)
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X), label="sine")
    ax.plot(X, np.cos(X), label="cosine")
    ax.legend()


@case(
    "styles/rc_scatter_errorbar_cmap",
    figsize=(5, 4),
    rc={"scatter.marker": "s", "scatter.edgecolors": "k", "errorbar.capsize": 4, "image.cmap": "plasma", "lines.markeredgewidth": 2},
)
def _(fig):
    ax = fig.subplots()
    ax.scatter(X, np.sin(X), c=X)
    ax.errorbar(X[::3], np.cos(X[::3]), yerr=0.2, fmt="o")


@case("styles/rc_path_snap_off", figsize=(4, 3), rc={"path.snap": False, "axes.grid": True})
def _(fig):
    ax = fig.subplots()
    ax.plot([0.13, 0.13, 0.87, 0.87, 0.13], [0.21, 0.79, 0.79, 0.21, 0.21], linewidth=1)
    ax.bar([0.3, 0.55], [0.4, 0.6], width=0.17)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)


@case("styles/rc_agg_path_chunksize", figsize=(5, 3), rc={"agg.path.chunksize": 200})
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 10, 2000)
    ax.plot(x, np.sin(x) + 0.3 * np.sin(37 * x), linewidth=2)
    ax.fill_between(x[:400], np.sin(x[:400]) - 1, -1.5, alpha=0.3)


@case("styles/rc_boxplot_horizontal", figsize=(5, 3), rc={"boxplot.vertical": False})
def _(fig):
    ax = fig.subplots()
    data = [np.sin(X * k) * k for k in (1, 2, 3)]
    ax.boxplot(data)
