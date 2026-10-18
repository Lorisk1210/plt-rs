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


# --- orientation, sectors and radial origins


@case("polar/theta_zero_locations_directions", figsize=(6, 6))
def _(fig):
    settings = [("NE", 1, 0), ("SW", -1, 0), ("E", -1, 30), ("S", 1, -15)]
    theta = np.linspace(0.0, 2 * np.pi, 8, endpoint=False)
    for i, (loc, direction, offset) in enumerate(settings):
        ax = fig.add_subplot(2, 2, i + 1, projection="polar")
        ax.bar(theta, 1 + np.arange(8) % 3, width=0.5, color="tab:blue", alpha=0.6)
        ax.plot([0, 0], [0, 3], color="tab:red", lw=2)
        ax.set_theta_zero_location(loc, offset=offset)
        ax.set_theta_direction(direction)
        ax.set_title(f"{loc}, {direction:+d}, {offset}°", fontsize=9)
    fig.subplots_adjust(hspace=0.4)


@case("polar/sector_gauge_clockwise", figsize=(5, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.set_theta_zero_location("N")
    ax.set_theta_direction(-1)
    ax.set_thetamin(-135)
    ax.set_thetamax(135)
    theta = np.radians(np.linspace(-135, 60, 50))
    ax.plot(theta, np.full(50, 0.8), lw=8, color="tab:green", solid_capstyle="butt")
    ax.set_rorigin(-1.0)
    ax.set_rlim(0, 1)
    ax.set_rticks([])
    ax.set_thetagrids(np.arange(-135, 136, 45))


@case("polar/sector_narrow_thetalim", figsize=(5, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.radians(np.linspace(10, 40, 30))
    ax.plot(theta, 5 + np.sin(theta * 20), "o-", ms=3)
    ax.set_thetalim(np.radians(10), np.radians(40))
    ax.set_rlim(3, 7)


@case("polar/sector_bars_rorigin_west", figsize=(5, 3.5))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.set_theta_zero_location("W")
    ax.set_theta_direction(-1)
    ax.set_thetamin(0)
    ax.set_thetamax(180)
    theta = np.radians(np.arange(10, 180, 20))
    ax.bar(theta, [3, 5, 2, 6, 4, 7, 1, 5, 3], width=np.radians(15), bottom=1,
           color=["C0", "C1", "C2"] * 3, edgecolor="k", linewidth=0.5)
    ax.set_rorigin(-2)
    ax.set_rlabel_position(90)


@case("polar/negative_radii_autoscale", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 200)
    ax.plot(theta, np.cos(3 * theta) + 0.5)
    ax.plot(theta, -0.5 * np.ones(200), "--", color="0.5")


@case("polar/rlim_negative_rorigin", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 200)
    ax.plot(theta, 2 * np.sin(2 * theta))
    ax.set_rlim(-2, 2)
    ax.set_rorigin(-3)


@case("polar/radial_data_offset", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 100)
    ax.plot(theta, 7.5 + 2.5 * np.sin(5 * theta))
    ax.fill_between(theta, 5, 7.5 + 2.5 * np.sin(5 * theta), alpha=0.3)


# --- radial scales and grids


@case("polar/rscale_log_rlim_rgrids", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 4 * np.pi, 300)
    ax.plot(theta, 10 ** (theta / (4 * np.pi) * 3))
    ax.set_rscale("log")
    ax.set_rlim(1, 1000)
    ax.set_rgrids([1, 10, 100, 1000], angle=135)


@case("polar/rscale_symlog", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 300)
    ax.plot(theta, 50 * (1 + np.sin(4 * theta)))
    ax.set_rscale("symlog", linthresh=1)


@case("polar/rgrids_labels_thetagrids_fmt", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    ax.plot(*_spiral(), color="tab:purple")
    ax.set_rgrids([0.5, 1.0, 1.5], labels=["low", "mid", "high"], angle=200)
    ax.set_thetagrids(np.arange(0, 360, 30), fmt="%d°")
    ax.tick_params(axis="y", labelcolor="tab:purple", labelsize=8)


@case("polar/spines_grid_off_sector", figsize=(5, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 3 * np.pi / 2, 60)
    ax.plot(theta, 1 + theta)
    ax.set_thetamax(270)
    ax.set_rorigin(-1)
    ax.grid(False)
    ax.spines["polar"].set_color("tab:red")
    ax.spines["polar"].set_linewidth(2)
    ax.spines["inner"].set_color("tab:blue")
    ax.spines["start"].set_linestyle("--")
    ax.spines["end"].set_visible(False)


# --- artists in polar coordinates


@case("polar/errorbar_annotate_text", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 8, endpoint=False)
    r = 2 + np.cos(2 * theta)
    ax.errorbar(theta, r, yerr=0.3, xerr=0.1, fmt="o", capsize=3)
    ax.annotate("peak", xy=(0, 3), xytext=(np.pi / 4, 3.6),
                arrowprops=dict(arrowstyle="->"), ha="center")
    ax.text(np.pi, 1.5, "west", ha="center", va="center", color="tab:red")
    ax.set_rmax(4)


@case("polar/theta_offset_radians_stem", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0.0, 2 * np.pi, 12, endpoint=False)
    ax.stem(theta, 1 + theta / np.pi, basefmt=" ")
    ax.set_theta_offset(np.pi / 3)
    ax.set_theta_direction("clockwise")
