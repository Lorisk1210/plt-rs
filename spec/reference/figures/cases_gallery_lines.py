"""The gallery's "Lines, bars and markers" section, one case per figure of
each example: bar charts (grouped, stacked, labelled, broken, hat graphs),
fills between curves, line styles, cap and join styles, markers (reference
sheet, markevery, rotated and path markers), masked data, multicoloured
lines, scatter plots with marginal histograms and legends, stackplots,
stairs, stems, steps, a release timeline and reference lines.

The examples draw random data from ``np.random``; here it comes from
``spec_support.rng`` instead, with the gamma, Rayleigh and Poisson draws
built from its uniforms. The examples reading ``goog.npz`` get a synthetic
price series of the same shape.

``spectrum_demo`` is left out: the sign of its angle spectrum at the
Nyquist frequency is the sign of pocketfft's rounding noise, which only a
bit-exact port of numpy's FFT would reproduce."""

import math
from datetime import datetime

import matplotlib as mpl
import numpy as np
from matplotlib import dates as mdates
from matplotlib import path as mpath
from matplotlib import patheffects, ticker
from matplotlib.collections import EventCollection, LineCollection
from matplotlib.colors import Normalize
from matplotlib.lines import Line2D
from matplotlib.markers import CapStyle, JoinStyle, MarkerStyle
from matplotlib.patches import StepPatch
from matplotlib.text import TextPath
from matplotlib.transforms import Affine2D

from spec_support import case, rng

GALLERY = "gallery_lines"


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{GALLERY}/{name}", **options)


# --- random draws the spec generator lacks -------------------------------------


def gamma4(g, n):
    """``np.random.gamma(4, size=n)``: sums of four exponential draws."""
    return np.array([sum(-math.log(1.0 - g.next_f64()) for _ in range(4)) for _ in range(n)])


def rayleigh(g, scale, n):
    """``np.random.rayleigh(scale, size=n)`` by inversion."""
    return np.array([scale * math.sqrt(-2.0 * math.log(1.0 - g.next_f64())) for _ in range(n)])


def poisson(g, lam, n):
    """``np.random.poisson(lam, size=n)`` by Knuth's multiplication method."""
    limit = math.exp(-lam)
    out = []
    for _ in range(n):
        k, p = 0, g.next_f64()
        while p > limit:
            k += 1
            p *= g.next_f64()
        out.append(k)
    return np.array(out)


def price_series(g, n):
    """A synthetic stand-in for ``goog.npz``: dates and a price random walk."""
    dates = np.datetime64("2004-08-19") + np.arange(n)
    close = 100 * np.exp(np.cumsum(g.normal(n, 0.002, 0.02)))
    return dates, close


# --- bars ----------------------------------------------------------------------

SPECIES = ("Adelie", "Chinstrap", "Gentoo")
PEOPLE = ("Tom", "Dick", "Harry", "Slim", "Jim")


@gallery("barchart")
def _(fig):
    penguin_means = {
        "Bill Depth": (18.35, 18.43, 14.98),
        "Bill Length": (38.79, 48.83, 47.50),
        "Flipper Length": (189.95, 195.82, 217.19),
    }
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    res = ax.grouped_bar(penguin_means, tick_labels=SPECIES, group_spacing=1)
    for container in res.bar_containers:
        ax.bar_label(container, padding=3)
    ax.set_ylabel("Length (mm)")
    ax.set_title("Penguin attributes by species")
    ax.legend(loc="upper left", ncols=3)
    ax.set_ylim(0, 250)


@gallery("bar_colors")
def _(fig):
    ax = fig.subplots()
    fruits = ["apple", "blueberry", "cherry", "orange"]
    counts = [40, 100, 30, 55]
    bar_labels = ["red", "blue", "_red", "orange"]
    bar_colors = ["tab:red", "tab:blue", "tab:red", "tab:orange"]
    ax.bar(fruits, counts, label=bar_labels, color=bar_colors)
    ax.set_ylabel("fruit supply")
    ax.set_title("Fruit supply by kind and color")
    ax.legend(title="Fruit color")


@gallery("barh")
def _(fig):
    ax = fig.subplots()
    performance = [5, 7, 6, 4, 9]
    error = [0.2, 0.4, 0.3, 0.6, 0.2]
    ax.barh(PEOPLE, performance, xerr=error, align="center")
    ax.yaxis.set_inverted(True)
    ax.set_xlabel("Performance")
    ax.set_title("How fast do you want to go today?")


@gallery("bar_label_demo_1")
def _(fig):
    sex_counts = {"Male": np.array([73, 34, 61]), "Female": np.array([73, 34, 58])}
    ax = fig.subplots()
    bottom = np.zeros(3)
    for sex, sex_count in sex_counts.items():
        p = ax.bar(SPECIES, sex_count, 0.6, label=sex, bottom=bottom)
        bottom += sex_count
        ax.bar_label(p, label_type="center")
    ax.set_title("Number of penguins by sex")
    ax.legend()


def performance_bars(fig):
    """The horizontal bars with errors that two figures of
    ``bar_label_demo`` label; returns the axes, the bars and the errors."""
    g = rng(19680801)
    y_pos = np.arange(len(PEOPLE))
    performance = 3 + 10 * g.uniform(len(PEOPLE))
    error = g.uniform(len(PEOPLE))
    ax = fig.subplots()
    hbars = ax.barh(y_pos, performance, xerr=error, align="center")
    ax.set_yticks(y_pos, labels=PEOPLE)
    ax.invert_yaxis()
    ax.set_xlabel("Performance")
    ax.set_title("How fast do you want to go today?")
    return ax, hbars, error


@gallery("bar_label_demo_2")
def _(fig):
    ax, hbars, _ = performance_bars(fig)
    ax.bar_label(hbars, fmt="%.2f")
    ax.set_xlim(right=15)


@gallery("bar_label_demo_3")
def _(fig):
    ax, hbars, error = performance_bars(fig)
    ax.bar_label(hbars, labels=[f"±{e:.2f}" for e in error], padding=8, color="b", fontsize=14)
    ax.set_xlim(right=16)


@gallery("bar_label_demo_4")
def _(fig):
    ax = fig.subplots()
    bar_container = ax.bar(["Coffee", "Salted Caramel", "Pistachio"], [4000, 2000, 7000])
    ax.set(ylabel="pints sold", title="Gelato sales by flavor", ylim=(0, 8000))
    ax.bar_label(bar_container, fmt="{:,.0f}")


@gallery("bar_label_demo_5")
def _(fig):
    ax = fig.subplots()
    bar_container = ax.bar(["Lion", "Gazelle", "Cheetah"], [50, 60, 75])
    ax.set(ylabel="speed in MPH", title="Running speeds", ylim=(0, 80))
    ax.bar_label(bar_container, fmt=lambda x: f"{x * 1.61:.1f} km/h")


@gallery("bar_stacked")
def _(fig):
    species = (
        "Adelie\n $\\mu=$3700.66g",
        "Chinstrap\n $\\mu=$3733.09g",
        "Gentoo\n $\\mu=5076.02g$",
    )
    weight_counts = {"Below": np.array([70, 31, 58]), "Above": np.array([82, 37, 66])}
    ax = fig.subplots()
    bottom = np.zeros(3)
    for boolean, weight_count in weight_counts.items():
        ax.bar(species, weight_count, 0.5, label=boolean, bottom=bottom)
        bottom += weight_count
    ax.set_title("Number of penguins with above average body mass")
    ax.legend(loc="upper right")


@gallery("broken_barh")
def _(fig):
    g = rng(19680801)
    cpu_1 = [(0, 3), (3.5, 1), (5, 5)]
    cpu_2 = np.column_stack([np.linspace(0, 9, 10), np.full(10, 0.5)])
    cpu_3 = np.column_stack([10 * g.uniform(61), np.full(61, 0.05)])
    cpu_4 = [(2, 1.7), (7, 1.2)]
    disk = [(1, 1.5)]
    network = np.column_stack([10 * g.uniform(10), np.full(10, 0.05)])
    ax = fig.subplots()
    ax.broken_barh(cpu_1, ("CPU 1", 0.4), align="center")
    ax.broken_barh(cpu_2, ("CPU 2", 0.4), align="center")
    ax.broken_barh(cpu_3, ("CPU 3", 0.4), align="center")
    ax.broken_barh(cpu_4, ("CPU 4", 0.4), align="center")
    ax.broken_barh(disk, ("disk", 0.4), align="center", color="tab:orange")
    ax.broken_barh(network, ("network", 0.4), align="center", color="tab:green")
    ax.set_xlim(0, 10)
    ax.invert_yaxis()
    ax.set_title("Resource usage")


@gallery("categorical_variables_1", figsize=(9, 3))
def _(fig):
    data = {"apple": 10, "orange": 15, "lemon": 5, "lime": 20}
    names = list(data.keys())
    values = list(data.values())
    axs = fig.subplots(1, 3, sharey=True)
    axs[0].bar(names, values)
    axs[1].scatter(names, values)
    axs[2].plot(names, values)
    fig.suptitle("Categorical Plotting")


@gallery("categorical_variables_2")
def _(fig):
    cat = ["bored", "happy", "bored", "bored", "happy", "bored"]
    dog = ["happy", "happy", "happy", "happy", "bored", "bored"]
    activity = ["combing", "drinking", "feeding", "napping", "playing", "washing"]
    ax = fig.subplots()
    ax.plot(activity, dog, label="dog")
    ax.plot(activity, cat, label="cat")
    ax.legend()


def gradient_image(ax, direction=0.3, cmap_range=(0, 1), **kwargs):
    """The example's gradient image drawn with a colormap."""
    phi = direction * np.pi / 2
    v = np.array([np.cos(phi), np.sin(phi)])
    X = np.array([[v @ [1, 0], v @ [1, 1]], [v @ [0, 0], v @ [0, 1]]])
    a, b = cmap_range
    X = a + (b - a) / X.max() * X
    return ax.imshow(X, interpolation="bicubic", clim=(0, 1), aspect="auto", **kwargs)


@gallery("gradient_bar")
def _(fig):
    ax = fig.subplots()
    ax.set(xlim=(0, 10), ylim=(0, 1))
    gradient_image(ax, direction=1, extent=(0, 1, 0, 1), transform=ax.transAxes,
                   cmap="RdYlGn", cmap_range=(0.2, 0.8), alpha=0.5)
    x = np.arange(10) + 0.15
    y = rng(19680801).uniform(10)
    for left, top in zip(x, y):
        gradient_image(ax, extent=(left, left + 0.7, 0, top), cmap="Blues_r",
                       cmap_range=(0, 0.8))


@gallery("hat_graph")
def _(fig):
    xlabels = ["I", "II", "III", "IV", "V"]
    values = np.array([[5, 15, 22, 20, 25], [25, 32, 34, 30, 27]])
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    color_cycle_colors = mpl.rcParams["axes.prop_cycle"].by_key()["color"]
    bars = ax.grouped_bar((values - values[0]).T, bottom=values[0], tick_labels=xlabels,
                          labels=["Player A", "Player B"], edgecolor="black",
                          group_spacing=0.8, colors=["none"] + color_cycle_colors)
    for bc, heights in zip(bars.bar_containers, values):
        ax.bar_label(bc, heights, padding=4)
    ax.set_xlabel("Games")
    ax.set_ylabel("Score")
    ax.set_ylim(0, 60)
    ax.set_title("Scores by number of game and players")
    ax.legend()


@gallery("horizontal_barchart_distribution", figsize=(9.2, 5))
def _(fig):
    category_names = ["Strongly disagree", "Disagree", "Neither agree nor disagree",
                      "Agree", "Strongly agree"]
    results = {
        "Question 1": [10, 15, 17, 32, 26],
        "Question 2": [26, 22, 29, 10, 13],
        "Question 3": [35, 37, 7, 2, 19],
        "Question 4": [32, 11, 9, 15, 33],
        "Question 5": [21, 29, 5, 5, 40],
        "Question 6": [8, 19, 5, 30, 38],
    }
    labels = list(results.keys())
    data = np.array(list(results.values()))
    data_cum = data.cumsum(axis=1)
    category_colors = mpl.colormaps["RdYlGn"](np.linspace(0.15, 0.85, data.shape[1]))
    ax = fig.subplots()
    ax.invert_yaxis()
    ax.xaxis.set_visible(False)
    ax.set_xlim(0, np.sum(data, axis=1).max())
    for i, (colname, color) in enumerate(zip(category_names, category_colors)):
        widths = data[:, i]
        starts = data_cum[:, i] - widths
        rects = ax.barh(labels, widths, left=starts, height=0.5, label=colname, color=color)
        r, g, b, _ = color
        text_color = "white" if r * g * b < 0.5 else "darkgrey"
        ax.bar_label(rects, label_type="center", color=text_color)
    ax.legend(ncols=len(category_names), bbox_to_anchor=(0, 1), loc="lower left",
              fontsize="small")


# --- fills ---------------------------------------------------------------------


@gallery("fill_between_alpha_1")
def _(fig):
    dates, close = price_series(rng(2004), 1047)
    ax1, ax2 = fig.subplots(1, 2, sharex=True, sharey=True)
    ax1.plot(dates, close, lw=2)
    ax2.fill_between(dates, close.min(), close, alpha=0.7)
    for ax in ax1, ax2:
        ax.grid(True)
        ax.label_outer()
    ax1.set_ylabel("price")
    fig.suptitle("Google (GOOG) daily closing price")
    fig.autofmt_xdate()


@gallery("fill_between_alpha_2")
def _(fig):
    g = rng(19680801)
    nsteps, nwalkers = 100, 250
    t = np.arange(nsteps)
    s1 = g.normal(nsteps * nwalkers, 0.004, 0.02).reshape(nsteps, nwalkers)
    s2 = g.normal(nsteps * nwalkers, 0.002, 0.01).reshape(nsteps, nwalkers)
    x1 = s1.cumsum(axis=0)
    x2 = s2.cumsum(axis=0)
    mu1, sigma1 = x1.mean(axis=1), x1.std(axis=1)
    mu2, sigma2 = x2.mean(axis=1), x2.std(axis=1)
    ax = fig.subplots(1)
    ax.plot(t, mu1, lw=2, label="mean population 1")
    ax.plot(t, mu2, lw=2, label="mean population 2")
    ax.fill_between(t, mu1 + sigma1, mu1 - sigma1, facecolor="C0", alpha=0.4)
    ax.fill_between(t, mu2 + sigma2, mu2 - sigma2, facecolor="C1", alpha=0.4)
    ax.set_title(r"random walkers empirical $\mu$ and $\pm \sigma$ interval")
    ax.legend(loc="upper left")
    ax.set_xlabel("num steps")
    ax.set_ylabel("position")
    ax.grid()


@gallery("fill_between_alpha_3")
def _(fig):
    nsteps = 500
    t = np.arange(nsteps)
    mu, sigma = 0.002, 0.01
    x = rng(1).normal(nsteps, mu, sigma).cumsum()
    lower_bound = mu * t - sigma * np.sqrt(t)
    upper_bound = mu * t + sigma * np.sqrt(t)
    ax = fig.subplots(1)
    ax.plot(t, x, lw=2, label="walker position")
    ax.plot(t, mu * t, lw=1, label="population mean", color="C0", ls="--")
    ax.fill_between(t, lower_bound, upper_bound, facecolor="C0", alpha=0.4,
                    label="1 sigma range")
    ax.legend(loc="upper left")
    ax.fill_between(t, upper_bound, x, where=x > upper_bound, fc="red", alpha=0.4)
    ax.fill_between(t, lower_bound, x, where=x < lower_bound, fc="red", alpha=0.4)
    ax.set_xlabel("num steps")
    ax.set_ylabel("position")
    ax.grid()


@gallery("fill_between_demo_1", figsize=(6, 6))
def _(fig):
    x = np.arange(0.0, 2, 0.01)
    y1 = np.sin(2 * np.pi * x)
    y2 = 0.8 * np.sin(4 * np.pi * x)
    ax1, ax2, ax3 = fig.subplots(3, 1, sharex=True)
    ax1.fill_between(x, y1)
    ax1.set_title("fill between y1 and 0")
    ax2.fill_between(x, y1, 1)
    ax2.set_title("fill between y1 and 1")
    ax3.fill_between(x, y1, y2)
    ax3.set_title("fill between y1 and y2")
    ax3.set_xlabel("x")
    fig.tight_layout()


@gallery("fill_between_demo_2")
def _(fig):
    x = np.linspace(0, 10, 11)
    y = [3.9, 4.4, 10.8, 10.3, 11.2, 13.1, 14.1, 9.9, 13.9, 15.1, 12.5]
    a, b = np.polyfit(x, y, deg=1)
    y_est = a * x + b
    y_err = x.std() * np.sqrt(1 / len(x) + (x - x.mean()) ** 2 / np.sum((x - x.mean()) ** 2))
    ax = fig.subplots()
    ax.plot(x, y_est, "-")
    ax.fill_between(x, y_est - y_err, y_est + y_err, alpha=0.2)
    ax.plot(x, y, "o", color="tab:brown")


@gallery("fill_between_demo_3")
def _(fig):
    x = np.array([0, 1, 2, 3])
    y1 = np.array([0.8, 0.8, 0.2, 0.2])
    y2 = np.array([0, 0, 1, 1])
    ax1, ax2 = fig.subplots(2, 1, sharex=True)
    for ax, interpolate in [(ax1, False), (ax2, True)]:
        ax.set_title(f"interpolation={interpolate}")
        ax.plot(x, y1, "o--")
        ax.plot(x, y2, "o--")
        ax.fill_between(x, y1, y2, where=(y1 > y2), color="C0", alpha=0.3,
                        interpolate=interpolate)
        below = (y1 <= y2) if interpolate else (y1 < y2)
        ax.fill_between(x, y1, y2, where=below, color="C1", alpha=0.3, interpolate=interpolate)
    fig.tight_layout()


@gallery("fill_between_demo_4")
def _(fig):
    ax = fig.subplots()
    x = np.arange(0, 4 * np.pi, 0.01)
    y = np.sin(x)
    ax.plot(x, y, color="black")
    threshold = 0.75
    ax.axhline(threshold, color="green", lw=2, alpha=0.7)
    ax.fill_between(x, 0, 1, where=y > threshold, color="green", alpha=0.5,
                    transform=ax.get_xaxis_transform())


def betweenx_curves():
    """``y``, ``x1`` and ``x2`` of ``fill_betweenx_demo``."""
    y = np.arange(0.0, 2, 0.01)
    return y, np.sin(2 * np.pi * y), 1.2 * np.sin(4 * np.pi * y)


@gallery("fill_betweenx_demo_1", figsize=(6, 6))
def _(fig):
    y, x1, x2 = betweenx_curves()
    ax1, ax2, ax3 = fig.subplots(1, 3, sharey=True)
    ax1.fill_betweenx(y, 0, x1)
    ax1.set_title("between (x1, 0)")
    ax2.fill_betweenx(y, x1, 1)
    ax2.set_title("between (x1, 1)")
    ax2.set_xlabel("x")
    ax3.fill_betweenx(y, x1, x2)
    ax3.set_title("between (x1, x2)")


@gallery("fill_betweenx_demo_2", figsize=(6, 6))
def _(fig):
    y, x1, x2 = betweenx_curves()
    ax, ax1 = fig.subplots(1, 2, sharey=True)
    ax.plot(x1, y, x2, y, color="black")
    ax.fill_betweenx(y, x1, x2, where=x2 >= x1, facecolor="green")
    ax.fill_betweenx(y, x1, x2, where=x2 <= x1, facecolor="red")
    ax.set_title("fill_betweenx where")
    x2 = np.ma.masked_greater(x2, 1.0)
    ax1.plot(x1, y, x2, y, color="black")
    ax1.fill_betweenx(y, x1, x2, where=x2 >= x1, facecolor="green")
    ax1.fill_betweenx(y, x1, x2, where=x2 <= x1, facecolor="red")
    ax1.set_title("regions with x2 > 1 are masked")


def koch_snowflake(order, scale=10):
    """The example's Koch snowflake: point coordinates ``x, y``."""
    def snowflake_complex(order):
        if order == 0:
            angles = np.array([0, 120, 240]) + 90
            return scale / np.sqrt(3) * np.exp(np.deg2rad(angles) * 1j)
        zr = 0.5 - 0.5j * np.sqrt(3) / 3
        p1 = snowflake_complex(order - 1)
        p2 = np.roll(p1, shift=-1)
        dp = p2 - p1
        new_points = np.empty(len(p1) * 4, dtype=np.complex128)
        new_points[::4] = p1
        new_points[1::4] = p1 + dp / 3
        new_points[2::4] = p1 + dp * zr
        new_points[3::4] = p1 + dp / 3 * 2
        return new_points

    points = snowflake_complex(order)
    return points.real, points.imag


@gallery("fill_1", figsize=(8, 8))
def _(fig):
    x, y = koch_snowflake(order=5)
    ax = fig.add_subplot()
    ax.axis("equal")
    ax.fill(x, y)


@gallery("fill_2", figsize=(9, 3))
def _(fig):
    x, y = koch_snowflake(order=2)
    ax1, ax2, ax3 = fig.subplots(1, 3, subplot_kw={"aspect": "equal"})
    ax1.fill(x, y)
    ax2.fill(x, y, facecolor="lightsalmon", edgecolor="orangered", linewidth=3)
    ax3.fill(x, y, facecolor="none", edgecolor="purple", linewidth=3)


@gallery("span_regions")
def _(fig):
    t = np.arange(0.0, 2, 0.01)
    s = np.sin(2 * np.pi * t)
    ax = fig.subplots()
    ax.plot(t, s, color="black")
    ax.axhline(0, color="black")
    ax.fill_between(t, 1, where=s > 0, facecolor="green", alpha=0.5)
    ax.fill_between(t, -1, where=s < 0, facecolor="red", alpha=0.5)


# --- lines ---------------------------------------------------------------------


@gallery("axline_1")
def _(fig):
    t = np.linspace(-10, 10, 100)
    sig = 1 / (1 + np.exp(-t))
    ax = fig.subplots()
    ax.axhline(y=0, color="black", linestyle="--")
    ax.axhline(y=0.5, color="black", linestyle=":")
    ax.axhline(y=1.0, color="black", linestyle="--")
    ax.axvline(color="grey")
    ax.axline((0, 0.5), slope=0.25, color="black", linestyle=(0, (5, 5)))
    ax.plot(t, sig, linewidth=2, label=r"$\sigma(t) = \frac{1}{1 + e^{-t}}$")
    ax.set(xlim=(-10, 10), xlabel="t")
    ax.legend(fontsize=14)


@gallery("axline_2")
def _(fig):
    ax = fig.subplots()
    for pos in np.linspace(-2, 1, 10):
        ax.axline((pos, 0), slope=0.5, color="k", transform=ax.transAxes)
    ax.set(xlim=(0, 1), ylim=(0, 1))


@gallery("capstyle", figsize=(4, 1.2))
def _(fig):
    ax = fig.add_axes((0, 0, 1, 0.8))
    ax.set_title("Cap style")
    for x, style in enumerate(["butt", "round", "projecting"]):
        ax.text(x + 0.25, 0.85, style, ha="center")
        xx = [x, x + 0.5]
        yy = [0, 0]
        ax.plot(xx, yy, lw=12, color="tab:blue", solid_capstyle=style)
        ax.plot(xx, yy, lw=1, color="black")
        ax.plot(xx, yy, "o", color="tab:red", markersize=3)
    ax.set_ylim(-0.5, 1.5)
    ax.set_axis_off()


@gallery("joinstyle", figsize=(5, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.set_title("Join style")
    for x, style in enumerate(["miter", "round", "bevel"]):
        ax.text(x, 5, style)
        for y, angle in enumerate([20, 45, 60, 90, 120]):
            phi = np.radians(angle)
            xx = [x + 0.5, x, x + 0.5 * np.cos(phi)]
            yy = [y, y, y + 0.5 * np.sin(phi)]
            ax.plot(xx, yy, lw=12, color="tab:blue", solid_joinstyle=style)
            ax.plot(xx, yy, lw=1, color="black")
            ax.plot(xx[1], yy[1], "o", color="tab:red", markersize=3)
            if x == 0:
                ax.text(-1.3, y, f"{angle} degrees")
    ax.set_xlim(-1.5, 2.75)
    ax.set_ylim(-0.5, 5.5)
    ax.set_axis_off()


@gallery("eventcollection_demo")
def _(fig):
    g = rng(19680801)
    xdata1 = np.sort(g.uniform(10))
    xdata2 = np.sort(g.uniform(10))
    ydata1 = xdata1 ** 2
    ydata2 = 1 - xdata2 ** 3
    ax = fig.add_subplot(1, 1, 1)
    ax.plot(xdata1, ydata1, color="tab:blue")
    ax.plot(xdata2, ydata2, color="tab:orange")
    ax.add_collection(EventCollection(xdata1, color="tab:blue", linelength=0.05))
    ax.add_collection(EventCollection(xdata2, color="tab:orange", linelength=0.05))
    ax.add_collection(EventCollection(ydata1, color="tab:blue", linelength=0.05,
                                      orientation="vertical"))
    ax.add_collection(EventCollection(ydata2, color="tab:orange", linelength=0.05,
                                      orientation="vertical"))
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.set_title("line plot with data points")


@gallery("eventplot_demo", rc={"font.size": 8.0})
def _(fig):
    g = rng(19680801)
    data1 = g.uniform(6 * 50).reshape(6, 50)
    colors1 = [f"C{i}" for i in range(6)]
    lineoffsets1 = [-15, -3, 1, 1.5, 6, 10]
    linelengths1 = [5, 2, 1, 1, 3, 1.5]
    axs = fig.subplots(2, 2)
    for ax, orientation in [(axs[0, 0], "horizontal"), (axs[1, 0], "vertical")]:
        ax.eventplot(data1, colors=colors1, lineoffsets=lineoffsets1,
                     linelengths=linelengths1, orientation=orientation)
    data2 = gamma4(g, 60 * 50).reshape(60, 50)
    for ax, orientation in [(axs[0, 1], "horizontal"), (axs[1, 1], "vertical")]:
        ax.eventplot(data2, colors="black", lineoffsets=1, linelengths=1,
                     orientation=orientation)


@gallery("line_demo_dash_control", rc={"lines.linewidth": 2.5})
def _(fig):
    x = np.linspace(0, 10, 500)
    y = np.sin(x)
    ax = fig.subplots()
    line1, = ax.plot(x, y, label="Using set_dashes() and set_dash_capstyle()")
    line1.set_dashes([2, 2, 10, 2])
    line1.set_dash_capstyle("round")
    ax.plot(x, y - 0.2, dashes=[6, 2], label="Using the dashes parameter")
    ax.plot(x, y - 0.4, dashes=[4, 4], gapcolor="tab:pink",
            label="Using the dashes and gapcolor parameters")
    ax.legend(handlelength=4)


def plot_linestyles(ax, linestyles, title):
    """The example's table of line styles, annotated with their spelling."""
    X, Y = np.linspace(0, 100, 10), np.zeros(10)
    yticklabels = []
    for i, (name, linestyle) in enumerate(linestyles):
        ax.plot(X, Y + i, linestyle=linestyle, linewidth=1.5, color="black")
        yticklabels.append(name)
    ax.set_title(title)
    ax.set(ylim=(-0.5, len(linestyles) - 0.5), yticks=np.arange(len(linestyles)),
           yticklabels=yticklabels)
    ax.tick_params(left=False, bottom=False, labelbottom=False)
    ax.spines[:].set_visible(False)
    for i, (name, linestyle) in enumerate(linestyles):
        ax.annotate(repr(linestyle), xy=(0.0, i), xycoords=ax.get_yaxis_transform(),
                    xytext=(-6, -12), textcoords="offset points", color="blue", fontsize=8,
                    ha="right", family="monospace")


@gallery("linestyles", figsize=(7, 8))
def _(fig):
    linestyle_str = [
        ("solid", "solid"),
        ("dotted", "dotted"),
        ("dashed", "dashed"),
        ("dashdot", "dashdot"),
    ]
    linestyle_tuple = [
        ("loosely dotted", (0, (1, 10))),
        ("dotted", (0, (1, 5))),
        ("densely dotted", (0, (1, 1))),
        ("long dash with offset", (5, (10, 3))),
        ("loosely dashed", (0, (5, 10))),
        ("dashed", (0, (5, 5))),
        ("densely dashed", (0, (5, 1))),
        ("loosely dashdotted", (0, (3, 10, 1, 10))),
        ("dashdotted", (0, (3, 5, 1, 5))),
        ("densely dashdotted", (0, (3, 1, 1, 1))),
        ("dashdotdotted", (0, (3, 5, 1, 5, 1, 5))),
        ("loosely dashdotdotted", (0, (3, 10, 1, 10, 1, 10))),
        ("densely dashdotdotted", (0, (3, 1, 1, 1, 1, 1))),
    ]
    fig.set_layout_engine("constrained")
    ax0, ax1 = fig.subplots(2, 1, height_ratios=[1, 3])
    plot_linestyles(ax0, linestyle_str[::-1], title="Named linestyles")
    plot_linestyles(ax1, linestyle_tuple[::-1], title="Parametrized linestyles")


@gallery("lines_with_ticks_demo", figsize=(6, 6))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1], label="Line",
            path_effects=[patheffects.withTickedStroke(spacing=7, angle=135)])
    x = np.linspace(0.0, 1.0, 101)
    y = 0.3 * np.sin(x * 8) + 0.4
    ax.plot(x, y, label="Curve", path_effects=[patheffects.withTickedStroke()])
    ax.legend()


@gallery("masked_demo")
def _(fig):
    x = np.linspace(-np.pi / 2, np.pi / 2, 31)
    y = np.cos(x) ** 3
    x2 = x[y <= 0.7]
    y2 = y[y <= 0.7]
    y3 = np.ma.masked_where(y > 0.7, y)
    y4 = y.copy()
    y4[y3 > 0.7] = np.nan
    ax = fig.add_subplot()
    ax.plot(x * 0.1, y, "o-", color="lightgrey", label="No mask")
    ax.plot(x2 * 0.4, y2, "o-", label="Points removed")
    ax.plot(x * 0.7, y3, "o-", label="Masked values")
    ax.plot(x * 1.0, y4, "o-", label="NaN values")
    ax.legend()
    ax.set_title("Masked and NaN data")


def colored_line(x, y, c, ax, **lc_kwargs):
    """A line coloured by ``c`` at each point, from segments joined at the
    midpoints between neighbours."""
    xy = np.stack((x, y), axis=-1)
    xy_mid = np.concat(
        (xy[0, :][None, :], (xy[:-1, :] + xy[1:, :]) / 2, xy[-1, :][None, :]), axis=0
    )
    segments = np.stack((xy_mid[:-1, :], xy, xy_mid[1:, :]), axis=-2)
    lc = LineCollection(segments, array=c, **lc_kwargs)
    ax.add_collection(lc)
    return lc


@gallery("multicolored_line_1")
def _(fig):
    t = np.linspace(-7.4, -0.5, 200)
    x = 0.9 * np.sin(t)
    y = 0.9 * np.cos(1.6 * t)
    color = np.linspace(0, 2, t.size)
    ax1 = fig.subplots()
    lines = colored_line(x, y, color, ax1, linewidth=10, cmap="plasma")
    fig.colorbar(lines)
    ax1.set_title("Color at each point")


@gallery("multicolored_line_2")
def _(fig):
    x = [0, 1, 2, 3, 4]
    y = [0, 1, 2, 1, 1]
    c = [1, 2, 3, 4, 5]
    ax = fig.subplots()
    ax.scatter(x, y, c=c, cmap="rainbow")
    colored_line(x, y, c=c, ax=ax, cmap="rainbow")


@gallery("multicolored_line_3")
def _(fig):
    x = np.linspace(0, 3 * np.pi, 500)
    y = np.sin(x)
    dydx = np.cos(0.5 * (x[:-1] + x[1:]))
    points = np.array([x, y]).T.reshape(-1, 1, 2)
    segments = np.concatenate([points[:-1], points[1:]], axis=1)
    ax2 = fig.subplots()
    lc = LineCollection(segments, linewidth=2, cmap="viridis")
    lc.set_array(dydx)
    line = ax2.add_collection(lc)
    fig.colorbar(line, ax=ax2, label="dy/dx")
    ax2.set_xlim(x.min(), x.max())
    ax2.set_ylim(-1.1, 1.1)
    ax2.set_title("Color between points")


@gallery("simple_plot")
def _(fig):
    t = np.arange(0.0, 2.0, 0.01)
    s = 1 + np.sin(2 * np.pi * t)
    ax = fig.subplots()
    ax.plot(t, s)
    ax.set(xlabel="time (s)", ylabel="voltage (mV)", title="About as simple as it gets, folks")
    ax.grid()


@gallery("vline_hline_demo", figsize=(12, 6))
def _(fig):
    t = np.arange(0.0, 5.0, 0.1)
    s = np.exp(-t) + np.sin(2 * np.pi * t) + 1
    nse = rng(19680801).normal(len(t), 0.0, 0.3) * s
    vax, hax = fig.subplots(1, 2)
    vax.plot(t, s + nse, "^")
    vax.vlines(t, [0], s)
    vax.vlines([1, 2], 0, 1, transform=vax.get_xaxis_transform(), colors="r")
    vax.set_xlabel("time (s)")
    vax.set_title("Vertical lines demo")
    hax.plot(s + nse, t, "^")
    hax.hlines(t, [0], s, lw=2)
    hax.set_xlabel("time (s)")
    hax.set_title("Horizontal lines demo")


# --- markers -------------------------------------------------------------------

TEXT_STYLE = dict(horizontalalignment="right", verticalalignment="center", fontsize=12,
                  fontfamily="monospace")
MARKER_STYLE = dict(linestyle=":", color="0.8", markersize=10, markerfacecolor="tab:blue",
                    markeredgecolor="tab:blue")
FILLED_MARKER_STYLE = dict(marker="o", linestyle=":", markersize=15, color="darkgrey",
                           markerfacecolor="tab:blue", markerfacecoloralt="lightsteelblue",
                           markeredgecolor="brown")
MARKER_ANGLES = [0, 10, 20, 30, 45, 60, 90]


def format_marker_axes(ax):
    """``format_axes`` of the marker reference."""
    ax.margins(0.2)
    ax.set_axis_off()
    ax.invert_yaxis()


def marker_rows(ax, markers, labels, **style):
    """One labelled row of three markers per entry of ``markers``."""
    for y, (marker, label) in enumerate(zip(markers, labels)):
        ax.text(-0.5, y, label, **TEXT_STYLE)
        ax.plot([y] * 3, marker=marker, **style)
    format_marker_axes(ax)


def marker_columns(fig, title, markers):
    """Two columns of labelled marker rows, the first holding the first half."""
    axs = fig.subplots(ncols=2)
    fig.suptitle(title, fontsize=14)
    half = len(markers) // 2
    for ax, column in zip(axs, (markers[:half], markers[half:])):
        marker_rows(ax, column, [repr(m) for m in column], **MARKER_STYLE)


@gallery("marker_reference_1")
def _(fig):
    unfilled = [m for m, func in Line2D.markers.items()
                if func != "nothing" and m not in Line2D.filled_markers]
    marker_columns(fig, "Un-filled markers", unfilled)


@gallery("marker_reference_2")
def _(fig):
    marker_columns(fig, "Filled markers", Line2D.filled_markers)


def marker_sheet(fig, title, left):
    """One axes for a sheet of marker rows, its figure titled ``title``."""
    ax = fig.subplots()
    fig.suptitle(title, fontsize=14)
    fig.subplots_adjust(left=left)
    return ax


@gallery("marker_reference_3")
def _(fig):
    ax = marker_sheet(fig, "Marker fillstyle", 0.4)
    for y, fill_style in enumerate(Line2D.fillStyles):
        ax.text(-0.5, y, repr(fill_style), **TEXT_STYLE)
        ax.plot([y] * 3, fillstyle=fill_style, **FILLED_MARKER_STYLE)
    format_marker_axes(ax)


@gallery("marker_reference_4")
def _(fig):
    ax = marker_sheet(fig, "Mathtext markers", 0.4)
    markers = ["$1$", r"$\frac{1}{2}$", "$f$", "$♫$", r"$\mathcal{A}$"]
    labels = [repr(m).replace("$", r"\$") for m in markers]
    marker_rows(ax, markers, labels, **{**MARKER_STYLE, "markeredgecolor": "none",
                                        "markersize": 15})


@gallery("marker_reference_5")
def _(fig):
    ax = marker_sheet(fig, "Path markers", 0.4)
    star = mpath.Path.unit_regular_star(6)
    circle = mpath.Path.unit_circle()
    cut_star = mpath.Path(vertices=np.concatenate([circle.vertices, star.vertices[::-1, ...]]),
                          codes=np.concatenate([circle.codes, star.codes]))
    markers = {"star": star, "circle": circle, "cut_star": cut_star}
    marker_rows(ax, markers.values(), markers.keys(), **MARKER_STYLE)


@gallery("marker_reference_6")
def _(fig):
    ax = fig.subplots()
    fig.suptitle("Rotated markers", fontsize=14)
    common_style = {k: v for k, v in FILLED_MARKER_STYLE.items() if k != "marker"}
    rows = [("Filled marker", "o"), ("Un-filled marker", "1"),
            ("Equation marker", r"$\frac{1}{x}$")]
    for y, (label, marker) in enumerate(rows):
        ax.text(-0.5, y, label, **TEXT_STYLE)
        for x, theta in enumerate(MARKER_ANGLES):
            t = Affine2D().rotate_deg(theta)
            ax.plot(x, y, marker=MarkerStyle(marker, "left", t), **common_style)
    for x, theta in enumerate(MARKER_ANGLES):
        ax.text(x, 2.5, f"{theta}°", horizontalalignment="center")
    format_marker_axes(ax)
    fig.tight_layout()


MARKER_INNER = dict(markersize=35, markerfacecolor="tab:blue",
                    markerfacecoloralt="lightsteelblue", markeredgecolor="brown",
                    markeredgewidth=8)
MARKER_OUTER = dict(markersize=35, markerfacecolor="tab:blue",
                    markerfacecoloralt="lightsteelblue", markeredgecolor="white",
                    markeredgewidth=1)


@gallery("marker_reference_7")
def _(fig):
    ax = marker_sheet(fig, "Marker CapStyle", 0.1)
    for y, cap_style in enumerate(CapStyle):
        ax.text(-0.5, y, cap_style.name, **TEXT_STYLE)
        for x, theta in enumerate(MARKER_ANGLES):
            t = Affine2D().rotate_deg(theta)
            m = MarkerStyle("1", transform=t, capstyle=cap_style)
            ax.plot(x, y, marker=m, **MARKER_INNER)
            ax.plot(x, y, marker=m, **MARKER_OUTER)
            ax.text(x, len(CapStyle) - 0.5, f"{theta}°", ha="center")
    format_marker_axes(ax)


@gallery("marker_reference_8")
def _(fig):
    ax = marker_sheet(fig, "Marker JoinStyle", 0.05)
    for y, join_style in enumerate(JoinStyle):
        ax.text(-0.5, y, join_style.name, **TEXT_STYLE)
        for x, theta in enumerate(MARKER_ANGLES):
            t = Affine2D().rotate_deg(theta)
            m = MarkerStyle("*", transform=t, joinstyle=join_style)
            ax.plot(x, y, marker=m, **MARKER_INNER)
            ax.text(x, len(JoinStyle) - 0.5, f"{theta}°", ha="center")
    format_marker_axes(ax)


MARKEVERY_CASES = [None, 8, (30, 8), [16, 24, 32], [0, -1], slice(100, 200, 3), 0.1, 0.4,
                   (0.2, 0.4)]


def markevery_grid(fig, **subplot_kw):
    """The 3 × 3 grid of axes, one per ``markevery`` case, titled with it."""
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 3, subplot_kw=subplot_kw)
    for ax, markevery in zip(axs.flat, MARKEVERY_CASES):
        ax.set_title(f"markevery={markevery}")
        yield ax, markevery


def markevery_curve():
    """``x`` and ``y`` of the cartesian ``markevery`` figures."""
    delta = 0.11
    x = np.linspace(0, 10 - 2 * delta, 200) + delta
    return x, np.sin(x) + 1.0 + delta


@gallery("markevery_demo_1", figsize=(10, 6))
def _(fig):
    x, y = markevery_curve()
    for ax, markevery in markevery_grid(fig):
        ax.plot(x, y, "o", ls="-", ms=4, markevery=markevery)


@gallery("markevery_demo_2", figsize=(10, 6))
def _(fig):
    x, y = markevery_curve()
    for ax, markevery in markevery_grid(fig):
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.plot(x, y, "o", ls="-", ms=4, markevery=markevery)


@gallery("markevery_demo_3", figsize=(10, 6))
def _(fig):
    x, y = markevery_curve()
    for ax, markevery in markevery_grid(fig):
        ax.plot(x, y, "o", ls="-", ms=4, markevery=markevery)
        ax.set_xlim(6, 6.7)
        ax.set_ylim(1.1, 1.7)


@gallery("markevery_demo_4", figsize=(10, 6))
def _(fig):
    r = np.linspace(0, 3.0, 200)
    theta = 2 * np.pi * r
    for ax, markevery in markevery_grid(fig, projection="polar"):
        ax.plot(theta, r, "o", ls="-", ms=4, markevery=markevery)


@gallery("multivariate_marker_plot")
def _(fig):
    success_symbols = [TextPath((0, 0), "☹"), TextPath((0, 0), "😒"), TextPath((0, 0), "☺")]
    n = 25
    g = rng(42)
    skills = g.uniform(n, 5, 80) * 0.1 + 5
    takeoff_angles = g.normal(n, 0, 90)
    thrusts = g.uniform(n)
    successful = g.integers(n, 0, 3)
    positions = g.normal(2 * n).reshape(n, 2) * 5
    cmap = mpl.colormaps["plasma"]
    ax = fig.subplots()
    fig.suptitle("Throwing success", size=14)
    for skill, takeoff, thrust, mood, pos in zip(skills, takeoff_angles, thrusts, successful,
                                                 positions):
        t = Affine2D().scale(skill).rotate_deg(takeoff)
        m = MarkerStyle(success_symbols[mood], transform=t)
        ax.plot(pos[0], pos[1], marker=m, color=cmap(thrust))
    fig.colorbar(mpl.cm.ScalarMappable(norm=Normalize(0, 1), cmap="plasma"), ax=ax,
                 label="Normalized Thrust [a.u.]")
    ax.set_xlabel("X position [m]")
    ax.set_ylabel("Y position [m]")


# --- scatter -------------------------------------------------------------------


@gallery("scatter_demo2")
def _(fig):
    g = rng(250)
    adj_close = 400 * np.exp(np.cumsum(g.normal(250, 0.0, 0.03)))
    volume = g.uniform(250, 2e6, 8e6)
    open_price = adj_close * (1 + g.normal(250, 0.0, 0.01))
    delta1 = np.diff(adj_close) / adj_close[:-1]
    volume = (15 * volume[:-2] / volume[0]) ** 2
    close = 0.003 * adj_close[:-2] / 0.003 * open_price[:-2]
    ax = fig.subplots()
    ax.scatter(delta1[:-1], delta1[1:], c=close, s=volume, alpha=0.5)
    ax.set_xlabel(r"$\Delta_i$", fontsize=15)
    ax.set_ylabel(r"$\Delta_{i+1}$", fontsize=15)
    ax.set_title("Volume and percent change")
    ax.grid(True)
    fig.tight_layout()


def scatter_hist(x, y, ax, ax_histx, ax_histy):
    """A scatter plot with the histograms of its coordinates beside it."""
    ax_histx.tick_params(axis="x", labelbottom=False)
    ax_histy.tick_params(axis="y", labelleft=False)
    ax.scatter(x, y)
    binwidth = 0.25
    xymax = max(np.max(np.abs(x)), np.max(np.abs(y)))
    lim = (int(xymax / binwidth) + 1) * binwidth
    bins = np.arange(-lim, lim + binwidth, binwidth)
    ax_histx.hist(x, bins=bins)
    ax_histy.hist(y, bins=bins, orientation="horizontal")


def scatter_hist_data():
    """The 1000 normal ``x`` and ``y`` of ``scatter_hist``."""
    g = rng(19680801)
    return g.normal(1000), g.normal(1000)


@gallery("scatter_hist_1", figsize=(6, 6))
def _(fig):
    x, y = scatter_hist_data()
    fig.set_layout_engine("constrained")
    axs = fig.subplot_mosaic([["histx", "."], ["scatter", "histy"]], width_ratios=(4, 1),
                             height_ratios=(1, 4))
    scatter_hist(x, y, axs["scatter"], axs["histx"], axs["histy"])


@gallery("scatter_hist_2")
def _(fig):
    x, y = scatter_hist_data()
    fig.set_layout_engine("constrained")
    ax = fig.add_subplot()
    ax.set_aspect("equal")
    ax_histx = ax.inset_axes([0, 1.05, 1, 0.25], sharex=ax)
    ax_histy = ax.inset_axes([1.05, 0, 0.25, 1], sharey=ax)
    scatter_hist(x, y, ax, ax_histx, ax_histy)


@gallery("scatter_masked")
def _(fig):
    g = rng(19680801)
    n = 100
    r0 = 0.6
    x = 0.9 * g.uniform(n)
    y = 0.9 * g.uniform(n)
    area = (20 * g.uniform(n)) ** 2
    c = np.sqrt(area)
    r = np.sqrt(x ** 2 + y ** 2)
    area1 = np.ma.masked_where(r < r0, area)
    area2 = np.ma.masked_where(r >= r0, area)
    ax = fig.add_subplot()
    ax.scatter(x, y, s=area1, marker="^", c=c)
    ax.scatter(x, y, s=area2, marker="o", c=c)
    theta = np.arange(0, np.pi / 2, 0.01)
    ax.plot(r0 * np.cos(theta), r0 * np.sin(theta))


@gallery("scatter_star_poly")
def _(fig):
    g = rng(19680801)
    x = g.uniform(10)
    y = g.uniform(10)
    z = np.sqrt(x ** 2 + y ** 2)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 3, sharex=True, sharey=True)
    markers = [
        (">", "marker='>'"),
        (r"$\clubsuit$", r"marker=r'\$\clubsuit\$'"),
        ([[-1, -1], [1, -1], [1, 1], [-1, -1]], "marker=verts"),
        ((5, 0), "marker=(5, 0)"),
        ((5, 1), "marker=(5, 1)"),
        ((5, 2), "marker=(5, 2)"),
    ]
    for ax, (marker, title) in zip(axs.flat, markers):
        ax.scatter(x, y, s=80, c=z, marker=marker)
        ax.set_title(title)


@gallery("scatter_with_legend_1")
def _(fig):
    g = rng(19680801)
    ax = fig.subplots()
    for color in ["tab:blue", "tab:orange", "tab:green"]:
        n = 750
        x, y = g.uniform(n), g.uniform(n)
        scale = 200.0 * g.uniform(n)
        ax.scatter(x, y, c=color, s=scale, label=color, alpha=0.3, edgecolors="none")
    ax.legend()
    ax.grid(True)


@gallery("scatter_with_legend_2")
def _(fig):
    g = rng(19680801)
    n = 45
    x, y = g.uniform(n), g.uniform(n)
    c = g.integers(n, 1, 5)
    s = g.integers(n, 10, 220)
    ax = fig.subplots()
    scatter = ax.scatter(x, y, c=c, s=s)
    legend1 = ax.legend(*scatter.legend_elements(), loc="lower left", title="Classes")
    ax.add_artist(legend1)
    handles, labels = scatter.legend_elements(prop="sizes", alpha=0.6)
    ax.legend(handles, labels, loc="upper right", title="Sizes")


@gallery("scatter_with_legend_3")
def _(fig):
    g = rng(19680801)
    volume = rayleigh(g, 27, 40)
    amount = poisson(g, 10, 40)
    ranking = g.normal(40)
    price = g.uniform(40, 1, 10)
    ax = fig.subplots()
    scatter = ax.scatter(volume, amount, c=ranking, s=0.3 * (price * 3) ** 2, vmin=-3, vmax=3,
                         cmap="Spectral")
    legend1 = ax.legend(*scatter.legend_elements(num=5), loc="upper left", title="Ranking")
    ax.add_artist(legend1)
    kw = dict(prop="sizes", num=5, color=scatter.cmap(0.7), fmt="$ {x:.2f}",
              func=lambda s: np.sqrt(s / .3) / 3)
    ax.legend(*scatter.legend_elements(**kw), loc="lower right", title="Price")


# --- stacks, stairs, stems, steps and timelines ---------------------------------


@gallery("stackplot_demo_1")
def _(fig):
    year = [1950, 1960, 1970, 1980, 1990, 2000, 2010, 2018]
    population_by_continent = {
        "Africa": [.228, .284, .365, .477, .631, .814, 1.044, 1.275],
        "the Americas": [.340, .425, .519, .619, .727, .840, .943, 1.006],
        "Asia": [1.394, 1.686, 2.120, 2.625, 3.202, 3.714, 4.169, 4.560],
        "Europe": [.220, .253, .276, .295, .310, .303, .294, .293],
        "Oceania": [.012, .015, .019, .022, .026, .031, .036, .039],
    }
    ax = fig.subplots()
    ax.stackplot(year, population_by_continent.values(),
                 labels=population_by_continent.keys(), alpha=0.8)
    ax.legend(loc="upper left", reverse=True)
    ax.set_title("World population")
    ax.set_xlabel("Year")
    ax.set_ylabel("Number of people (billions)")
    ax.yaxis.set_minor_locator(ticker.MultipleLocator(.2))


@gallery("stackplot_demo_2")
def _(fig):
    g = rng(19680801)
    x = np.linspace(0, 100, 101)

    def gaussian_mixture(n=5):
        a = np.zeros_like(x)
        for _ in range(n):
            amplitude = 1 / (.1 + g.next_f64())
            dx = x[-1] - x[0]
            x0 = (2 * g.next_f64() - .5) * dx
            z = 10 / (.1 + g.next_f64()) / dx
            a += amplitude * np.exp(-(z * (x - x0)) ** 2)
        return a

    ys = [gaussian_mixture() for _ in range(3)]
    ax = fig.subplots()
    ax.stackplot(x, ys, baseline="wiggle")


@gallery("stairs_demo_1", figsize=(7, 15))
def _(fig):
    h, edges = np.histogram(rng(0).normal(5000, 5, 3), bins=np.linspace(0, 10, 20))
    axs = fig.subplots(3, 1)
    axs[0].stairs(h, edges, label="Simple histogram")
    axs[0].stairs(h, edges + 5, baseline=50, label="Modified baseline")
    axs[0].stairs(h, edges + 10, baseline=None, label="No edges")
    axs[0].set_title("Step Histograms")
    axs[1].stairs(np.arange(1, 6, 1), fill=True, label="Filled histogram\nw/ automatic edges")
    axs[1].stairs(np.arange(1, 6, 1) * 0.3, np.arange(2, 8, 1), orientation="horizontal",
                  hatch="//", label="Hatched histogram\nw/ horizontal orientation")
    axs[1].set_title("Filled histogram")
    patch = StepPatch(values=[1, 2, 3, 2, 1], edges=range(1, 7),
                      label=("Patch derived underlying object\n"
                             "with default edge/facecolor behaviour"))
    axs[2].add_patch(patch)
    axs[2].set_xlim(0, 7)
    axs[2].set_ylim(-1, 5)
    axs[2].set_title("StepPatch artist")
    for ax in axs:
        ax.legend()


@gallery("stairs_demo_2")
def _(fig):
    a = [[0, 0, 0], [1, 2, 3], [2, 4, 6], [3, 6, 9]]
    ax = fig.add_subplot()
    for i in range(len(a) - 1):
        ax.stairs(a[i + 1], baseline=a[i], fill=True)


@gallery("stairs_demo_3")
def _(fig):
    bins = np.arange(14)
    centers = bins[:-1] + np.diff(bins) / 2
    y = np.sin(centers / 2)
    ax = fig.add_subplot()
    ax.step(bins[:-1], y, where="post", label='step(where="post")')
    ax.plot(bins[:-1], y, "o--", color="grey", alpha=0.3)
    ax.stairs(y - 1, bins, baseline=None, label="stairs()")
    ax.plot(centers, y - 1, "o--", color="grey", alpha=0.3)
    ax.plot(np.repeat(bins, 2), np.hstack([y[0], np.repeat(y, 2), y[-1]]) - 1, "o",
            color="red", alpha=0.2)
    ax.legend()
    ax.set_title("step() vs. stairs()")


def stem_curve():
    """``x`` and ``y`` of ``stem_plot``."""
    x = np.linspace(0.1, 2 * np.pi, 41)
    return x, np.exp(np.sin(x))


@gallery("stem_plot_1")
def _(fig):
    fig.add_subplot().stem(*stem_curve())


@gallery("stem_plot_2")
def _(fig):
    markerline, _, _ = fig.add_subplot().stem(*stem_curve(), linefmt="grey", markerfmt="D",
                                              bottom=1.1)
    markerline.set_markerfacecolor("none")


def step_rows(ax, steps, title):
    """The three step curves of ``step_demo``, each over its data points;
    ``steps`` draws one curve given its offset and its label's index."""
    x = np.arange(14)
    y = np.sin(x / 2)
    for i, offset in enumerate([2, 1, 0]):
        steps(x, y + offset, i)
        ax.plot(x, y + offset, "o--", color="grey", alpha=0.3)
    ax.grid(axis="x", color="0.95")
    ax.legend(title=title)


@gallery("step_demo_1")
def _(fig):
    ax = fig.add_subplot()
    labels = [("pre", "pre (default)"), ("mid", "mid"), ("post", "post")]
    step_rows(ax, lambda x, y, i: ax.step(x, y, where=labels[i][0], label=labels[i][1]),
              "Parameter where:")
    ax.set_title("plt.step(where=...)")


@gallery("step_demo_2")
def _(fig):
    ax = fig.add_subplot()
    labels = [("steps", "steps (=steps-pre)"), ("steps-mid", "steps-mid"),
              ("steps-post", "steps-post")]
    step_rows(ax, lambda x, y, i: ax.plot(x, y, drawstyle=labels[i][0], label=labels[i][1]),
              "Parameter drawstyle:")
    ax.set_title("plt.plot(drawstyle=...)")


@gallery("timeline", figsize=(8.8, 4))
def _(fig):
    releases = ["2.2.4", "3.0.3", "3.0.2", "3.0.1", "3.0.0", "2.2.3", "2.2.2", "2.2.1",
                "2.2.0", "2.1.2", "2.1.1", "2.1.0", "2.0.2", "2.0.1", "2.0.0", "1.5.3",
                "1.5.2", "1.5.1", "1.5.0", "1.4.3", "1.4.2", "1.4.1", "1.4.0"]
    dates = ["2019-02-26", "2019-02-26", "2018-11-10", "2018-11-10", "2018-09-18",
             "2018-08-10", "2018-03-17", "2018-03-16", "2018-03-06", "2018-01-18",
             "2017-12-10", "2017-10-07", "2017-05-10", "2017-05-02", "2017-01-17",
             "2016-09-09", "2016-07-03", "2016-01-10", "2015-10-29", "2015-02-16",
             "2014-10-26", "2014-10-18", "2014-08-26"]
    dates = [datetime.strptime(d, "%Y-%m-%d") for d in dates]
    releases = [tuple(release.split(".")) for release in releases]
    dates, releases = zip(*sorted(zip(dates, releases)))
    levels = []
    macro_meso_releases = sorted({release[:2] for release in releases})
    for release in releases:
        h = 1 + 0.8 * (5 - int(release[2]))
        levels.append(h if macro_meso_releases.index(release[:2]) % 2 == 0 else -h)

    def is_feature(release):
        return release[-1] == "0"

    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.set(title="Matplotlib release dates")
    ax.vlines(dates, 0, levels,
              color=[("tab:red", 1 if is_feature(release) else .5) for release in releases])
    ax.axhline(0, c="black")
    meso_dates = [date for date, release in zip(dates, releases) if is_feature(release)]
    micro_dates = [date for date, release in zip(dates, releases) if not is_feature(release)]
    ax.plot(micro_dates, np.zeros_like(micro_dates), "ko", mfc="white")
    ax.plot(meso_dates, np.zeros_like(meso_dates), "ko", mfc="tab:red")
    for date, level, release in zip(dates, levels, releases):
        ax.annotate(".".join(release), xy=(date, level), xytext=(-3, np.sign(level) * 3),
                    textcoords="offset points",
                    verticalalignment="bottom" if level > 0 else "top",
                    weight="bold" if is_feature(release) else "normal",
                    bbox=dict(boxstyle="square", pad=0, lw=0, fc=(1, 1, 1, 0.7)))
    ax.xaxis.set(major_locator=mdates.YearLocator(), major_formatter=mdates.DateFormatter("%Y"))
    ax.yaxis.set_visible(False)
    ax.spines[["left", "top", "right"]].set_visible(False)
    ax.margins(y=0.1)
