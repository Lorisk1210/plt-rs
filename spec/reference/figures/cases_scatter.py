"""Scatter plots: sizes, colour mapping, markers, edges and non-finite values."""

import matplotlib.colors as mcolors
import numpy as np
from matplotlib import colormaps

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
