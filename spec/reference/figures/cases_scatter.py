"""Scatter plots: sizes, colour mapping, markers, edges, non-finite values
and legends built with ``legend_elements``."""

import matplotlib.colors as mcolors
import numpy as np
from matplotlib import colormaps
from matplotlib.markers import MarkerStyle
from matplotlib.transforms import Affine2D

from spec_support import case, rng


def _points(seed, n=40):
    r = rng(seed)
    return r.uniform(n), r.uniform(n)


@case("scatter/default")
def _(fig):
    ax = fig.subplots()
    x, y = _points(1, 60)
    ax.scatter(x, y)



@case("scatter/sizes_array", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x, y = _points(3, 30)
    ax.scatter(x, y, s=np.linspace(5, 600, 30), alpha=0.6)


@case("scatter/values_default_viridis", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = _points(4, 80)
    ax.scatter(x, y, c=x + y)


@case("scatter/cmap_vmin_vmax_with_colorbar", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = _points(5, 80)
    sc = ax.scatter(x, y, c=x * 10, cmap="plasma", vmin=2, vmax=8, s=50)
    fig.colorbar(sc, ax=ax, label="value")


@case("scatter/lognorm_with_colorbar", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 1, 50)
    sc = ax.scatter(x, x**2, c=10 ** (4 * x), norm=mcolors.LogNorm(), cmap="cividis", s=60)
    fig.colorbar(sc, ax=ax)


@case("scatter/twoslopenorm_diverging", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = _points(6, 60)
    sc = ax.scatter(x, y, c=10 * x - 3, cmap="RdBu_r", norm=mcolors.TwoSlopeNorm(vcenter=0), s=40)
    fig.colorbar(sc, ax=ax, orientation="horizontal")


@case("scatter/markers", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    for i, m in enumerate(["o", "s", "^", "D", "*", "P", "X", "h", "x", "+", "1", "|"]):
        ax.scatter(np.arange(5), np.full(5, i), marker=m, s=80)


@case("scatter/facecolors_none_edgecolors", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x, y = _points(7, 30)
    ax.scatter(x, y, s=120, facecolors="none", edgecolors="tab:red", linewidths=2)
    ax.scatter(y, x, s=60, edgecolors="k", linewidths=0.5)


@case("scatter/edgecolor_face_and_alpha", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x, y = _points(8, 60)
    ax.scatter(x, y, s=300, c=y, edgecolors="face", alpha=0.4)


@case("scatter/explicit_color_list", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    colors = ["red", "green", "blue", "orange", "purple", "0.3"]
    ax.scatter(np.arange(6), np.arange(6) ** 2, c=colors, s=100)
    ax.scatter(np.arange(6), np.arange(6) * 3, color=(0.1, 0.2, 0.5, 0.5), s=100)


@case("scatter/unfilled_marker_colormapped", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x, y = _points(9, 50)
    ax.scatter(x, y, c=x, marker="x", s=60, cmap="viridis", linewidths=2)


@case("scatter/plotnonfinite_bad_color", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    values = np.array([1.0, 2.0, np.nan, 4.0, np.inf, 6.0, np.nan, 8.0])
    cmap = colormaps["viridis"].with_extremes(bad="red")
    ax1.scatter(np.arange(8), np.arange(8), c=values, cmap=cmap, s=80)
    ax2.scatter(np.arange(8), np.arange(8), c=values, cmap=cmap, s=80, plotnonfinite=True)


@case("scatter/many_small_points")
def _(fig):
    ax = fig.subplots()
    r = rng(10)
    x = r.normal(3000)
    y = 0.5 * x + r.normal(3000, scale=0.5)
    ax.scatter(x, y, s=2, alpha=0.5)


@case("scatter/log_axes", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.logspace(0, 4, 30)
    ax.scatter(x, x ** 0.5, c=np.arange(30))
    ax.set_xscale("log")
    ax.set_yscale("log")


# --- legend elements


@case("scatter/legend_elements_colors_sizes", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    r = rng(11)
    x, y = r.uniform(40), r.uniform(40)
    classes = r.integers(40, 1, 5)
    sizes = r.uniform(40, 20, 300)
    sc = ax.scatter(x, y, c=classes, s=sizes)
    ax.add_artist(ax.legend(*sc.legend_elements(), loc="lower left", title="Classes"))
    handles, labels = sc.legend_elements(prop="sizes", alpha=0.6)
    ax.legend(handles, labels, loc="upper right", title="Sizes")


@case("scatter/legend_elements_num_fmt_func", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 1.0, 30)
    price = 2 + 18 * x**2
    sc = ax.scatter(x, np.sin(4 * x), s=price * 20, c=np.linspace(-1.0, 3.0, 30),
                    cmap="coolwarm", edgecolors="k", linewidths=0.5)
    sizes = sc.legend_elements(prop="sizes", num=4, color="0.4", fmt="$ {x:.2f}",
                               func=lambda s: s / 20)
    ax.add_artist(ax.legend(*sizes, loc="lower left", title="Price"))
    ax.legend(*sc.legend_elements(num=[0, 1.5, 3]), loc="upper right", title="Value")


# --- markers, edges and colours


@case("scatter/marker_variants", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    markers = [(5, 0), (5, 1), (5, 2), (4, 0, 45), [(-1, -1), (1, -1), (0, 1.5)],
               r"$\clubsuit$", "$f$", "8", "p"]
    for i, marker in enumerate(markers):
        ax.scatter(np.arange(4), np.full(4, i), marker=marker, s=[40, 80, 160, 320],
                   color=f"C{i}")
    ax.set_xlim(-0.5, 3.5)


@case("scatter/markerstyle_fillstyle_transform", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    angles = [0, 15, 30, 45, 60, 75, 90]
    for i, angle in enumerate(angles):
        rotated = MarkerStyle("^", transform=Affine2D().rotate_deg(angle))
        ax.scatter([i], [0], marker=rotated, s=300)
        ax.scatter([i], [1], marker=MarkerStyle("s").rotated(deg=angle), s=200, c="tab:green")
    ax.scatter([0, 1, 2, 3], [2] * 4, marker=MarkerStyle("o", fillstyle="none"), s=250,
               c=["C0", "C1", "C2", "C3"], linewidths=2)
    ax.scatter([4, 5, 6], [2] * 3, marker=MarkerStyle("D", fillstyle="none"), s=250, c="k")
    ax.set_ylim(-0.7, 2.7)


@case("scatter/edgecolors_linewidths_per_point", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    n = 8
    ax.scatter(np.arange(n), np.zeros(n), s=300, c="w",
               edgecolors=["C0", "C1", "C2", "C3"], linewidths=np.linspace(0.5, 4.0, n))
    ax.scatter(np.arange(n), np.ones(n), s=200, c=np.arange(n), cmap="viridis",
               edgecolors="k", linewidths=[0.5, 1, 2, 3], linestyle="--")
    ax.set_ylim(-1, 2)


@case("scatter/rgba_array_colors", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0.0, 1.0, 20)
    rgba = np.column_stack([t, np.full(20, 0.2), 1 - t, 0.3 + 0.7 * t])
    ax.scatter(t, t**2, c=rgba, s=150)
    ax.scatter(t, 1 - t**2, c=[(0.9, 0.5, 0.1)], s=80)


@case("scatter/hatched_markers_legend", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.arange(6)
    ax.scatter(x, x % 3, s=600, marker="s", facecolor="none", edgecolor="tab:blue",
               hatch="///", label="hatched")
    ax.scatter(x, x % 3 + 0.5, s=300, c="tab:orange", hatch="..", label="dotted", zorder=1)
    ax.legend(loc="upper left")


# --- colour mapping


@case("scatter/norm_extremes_extend_colorbar", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 1.0, 40)
    cmap = colormaps["viridis"].with_extremes(under="tab:red", over="tab:orange")
    sc = ax.scatter(x, np.cos(5 * x), c=np.linspace(-0.5, 1.5, 40), cmap=cmap, vmin=0, vmax=1,
                    s=60, edgecolors="k", linewidths=0.5)
    fig.colorbar(sc, ax=ax, extend="both")


@case("scatter/boundarynorm_discrete_colorbar", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x, y = _points(12, 60)
    bounds = [0, 0.2, 0.5, 0.8, 1.6, 2]
    cmap = mcolors.ListedColormap(["tab:blue", "tab:green", "gold", "tab:orange", "tab:red"])
    norm = mcolors.BoundaryNorm(bounds, cmap.N)
    sc = ax.scatter(x, y, c=x + y, cmap=cmap, norm=norm, s=70, marker="s")
    fig.colorbar(sc, ax=ax, ticks=bounds, spacing="proportional")


# --- non-finite and degenerate data


@case("scatter/nonfinite_coordinates_sizes", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = np.array([0.0, 1, 2, np.nan, 4, 5, 6, np.inf, 8, 9])
    y = np.array([1.0, np.nan, 3, 4, 5, 6, 7, 8, 9, 10])
    s = np.array([50.0, 100, np.nan, 50, 100, 50, 100, 50, 100, 50])
    c = np.array([0.0, 1, 2, 3, np.nan, 5, 6, 7, -np.inf, 9])
    ax1.scatter(x, y, s=s, c=c)
    cmap = colormaps["cividis"].with_extremes(bad="magenta")
    ax2.scatter(x, y, s=s, c=c, cmap=cmap, plotnonfinite=True, edgecolors="k")


@case("scatter/empty_single_zero_size", figsize=(6, 2.5))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.scatter([], [])
    ax2.scatter([2.5], [-1.0], s=200, marker="*")
    ax3.scatter([0, 1, 2, 3], [0, 1, 0, 1], s=[0, 20, 0, 200], c="tab:red")


@case("scatter/alpha_array", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    t = np.linspace(0.0, 1.0, 20)
    ax1.scatter(t, np.sin(6 * t), s=150, c="tab:blue", alpha=np.linspace(0.1, 1.0, 20),
                edgecolors="k")
    ax2.scatter(t, np.cos(6 * t), s=150, c=t, cmap="plasma", alpha=1 - 0.9 * t)
