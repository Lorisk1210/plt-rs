"""Histograms: binning, normalisation, histtypes, stacking, orientation and hist2d."""

import matplotlib.colors as mcolors
import numpy as np

from spec_support import case, rng

NORMAL = rng(100).normal(1000, loc=0.0, scale=1.0)
SKEWED = rng(101).normal(800, loc=1.0, scale=0.6) ** 2
THREE = [rng(102).normal(500, loc=-1.0), rng(103).normal(400, loc=0.5, scale=0.7), rng(104).normal(300, loc=2.0, scale=1.2)]


@case("hist/default")
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL)



@case("hist/bins_explicit_edges", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=[-4, -2, -1, -0.5, 0, 0.25, 0.5, 1, 2, 4], edgecolor="k")


@case("hist/bins_auto_and_sqrt", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.hist(SKEWED, bins="auto")
    ax2.hist(SKEWED, bins="sqrt")


@case("hist/range_clips_data", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=20, range=(-1, 2))


@case("hist/density_with_pdf", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=30, density=True, alpha=0.6)
    t = np.linspace(-4, 4, 200)
    ax.plot(t, np.exp(-t**2 / 2) / np.sqrt(2 * np.pi), "k")


@case("hist/cumulative", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.hist(NORMAL, bins=25, cumulative=True)
    ax2.hist(NORMAL, bins=25, cumulative=-1, density=True, histtype="step")


@case("hist/histtype_step", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(THREE, bins=20, histtype="step", linewidth=1.5)


@case("hist/histtype_stepfilled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    for data in THREE:
        ax.hist(data, bins=20, histtype="stepfilled", alpha=0.5)


@case("hist/multiple_side_by_side", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(THREE, bins=10)


@case("hist/barstacked", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(THREE, bins=15, histtype="barstacked")


@case("hist/stacked_step", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(THREE, bins=15, histtype="step", stacked=True, fill=False)



@case("hist/horizontal", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=20, orientation="horizontal")


@case("hist/horizontal_step_multi", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(THREE, bins=15, orientation="horizontal", histtype="step")


@case("hist/weights", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    values = np.arange(10)
    ax.hist(values, bins=np.arange(11) - 0.5, weights=values**2, edgecolor="w")


@case("hist/log", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(SKEWED, bins=30, log=True)


@case("hist/rwidth_align", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    data = rng(105).integers(200, 0, 6)
    ax1.hist(data, bins=np.arange(7), rwidth=0.6, align="left")
    ax2.hist(data, bins=np.arange(7), rwidth=0.6, align="right", color="C1")


@case("hist/colors_edgecolor_linewidth", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=15, color="lightgreen", edgecolor="darkgreen", linewidth=1.5, hatch="/")


@case("hist/bottom_offset", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.hist(NORMAL, bins=12, bottom=50)


@case("hist/hist2d_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = rng(106).normal(5000)
    y = 0.6 * x + rng(107).normal(5000, scale=0.8)
    ax.hist2d(x, y)


@case("hist/hist2d_bins_cmap_colorbar", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = rng(108).normal(5000)
    y = 0.6 * x + rng(109).normal(5000, scale=0.8)
    _, _, _, image = ax.hist2d(x, y, bins=(30, 20), cmap="magma", cmin=1)
    fig.colorbar(image, ax=ax)


@case("hist/hist2d_lognorm_range", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = rng(110).normal(5000)
    y = rng(111).normal(5000)
    _, _, _, image = ax.hist2d(x, y, bins=25, range=[[-3, 3], [-2, 2]], norm=mcolors.LogNorm())
    fig.colorbar(image, ax=ax)
