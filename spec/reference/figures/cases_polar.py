"""Polar axes: lines, scatter, bars, fills, theta orientation/direction,
sectors, r origin/limits/scale, label positions, grids, contourf/pcolormesh."""

import numpy as np

from spec_support import case, rng


def _spiral():
    r = np.linspace(0.0, 2.0, 200)
    return 2 * np.pi * r, r


@case("polar/plot_default")
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())


@case("polar/plot_markers_multiple", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 13)
    ax.plot(theta, 1 + 0.5 * np.cos(3 * theta), "o-", label="trefoil")
    ax.plot(theta, np.full(13, 0.8), "s--", label="circle")
    ax.legend(loc="lower left")


@case("polar/scatter", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    g = rng(31)
    theta, r = g.uniform(80, 0.0, 2 * np.pi), g.uniform(80, 0.0, 2.0)
    ax.scatter(theta, r, c=theta, s=200 * r**2, cmap="hsv", alpha=0.75)


@case("polar/bars", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 12, endpoint=False)
    radii = 1 + np.arange(12) % 5
    ax.bar(theta, radii, width=0.45, bottom=1.0, color=["C0", "C1", "C2"] * 4, alpha=0.6,
           edgecolor="k")


@case("polar/fill_and_fill_between", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 7)
    ax.fill(theta, [3, 2, 3.5, 1.5, 2.5, 3, 3], alpha=0.4, color="tab:green")
    t = np.linspace(0.0, np.pi / 2, 30)
    ax.fill_between(t, 0.5, 1.0 + np.sin(4 * t), color="tab:purple", alpha=0.5)


@case("polar/theta_zero_north_clockwise", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_theta_zero_location("N")
    ax.set_theta_direction(-1)


@case("polar/theta_zero_offset", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_theta_zero_location("W", offset=-20)


@case("polar/thetagrids_labels", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_thetagrids([0, 90, 180, 270], labels=["E", "N", "W", "S"])


@case("polar/sector", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_thetamin(45)
    ax.set_thetamax(135)


@case("polar/half_sector_rorigin", figsize=(5, 3))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(-np.pi / 2, np.pi / 2, 50)
    ax.plot(theta, 2 + np.cos(3 * theta))
    ax.set_thetamin(-90)
    ax.set_thetamax(90)
    ax.set_rorigin(-1.5)


@case("polar/rorigin_hole", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_rorigin(-1.0)


@case("polar/rmin_rmax", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_rmin(0.5)
    ax.set_rmax(1.5)


@case("polar/rticks_rlabel_position", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral())
    ax.set_rticks([0.5, 1.0, 1.5, 2.0])
    ax.set_rlabel_position(-22.5)


@case("polar/rscale_log", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 200)
    ax.plot(theta, 10 ** (1 + 2 * theta / (2 * np.pi)))
    ax.set_rscale("log")


@case("polar/grid_styling", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral(), color="k")
    ax.grid(color="tab:orange", linestyle=":", linewidth=1.5)
    ax.set_facecolor("#f0f0ff")
    ax.tick_params(axis="x", colors="tab:blue", labelsize=8)
    ax.set_title("styled grid")


@case("polar/contourf", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta, r = np.meshgrid(np.linspace(0.0, 2 * np.pi, 60), np.linspace(0.0, 2.0, 30))
    ax.contourf(theta, r, r * np.cos(3 * theta), levels=10, cmap="RdBu")


@case("polar/pcolormesh", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta, r = np.meshgrid(np.linspace(0.0, 2 * np.pi, 25), np.linspace(0.0, 2.0, 9))
    ax.pcolormesh(theta, r, (np.sin(theta) * r)[:-1, :-1], edgecolors="k", linewidth=0.3)
