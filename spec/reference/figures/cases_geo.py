"""Geographic projections: aitoff, hammer, lambert and mollweide axes with
their degree grids, and lines, scatter, fills and meshes drawn through the
projection."""

import numpy as np

from spec_support import case, rng


def _track():
    lon = np.linspace(-np.pi, np.pi, 60)
    return lon, 0.6 * np.sin(2 * lon)


@case("geo/aitoff_grid")
def _(fig):
    ax = fig.add_subplot(projection="aitoff")
    ax.grid(True)


@case("geo/hammer_grid")
def _(fig):
    ax = fig.add_subplot(projection="hammer")
    ax.grid(True)


@case("geo/lambert_grid", figsize=(5, 5))
def _(fig):
    ax = fig.add_subplot(projection="lambert")
    ax.grid(True)


@case("geo/mollweide_grid_title")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    ax.grid(True)
    ax.set_title("Mollweide")


@case("geo/plot_scatter")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    ax.plot(*_track(), color="tab:red", linewidth=2)
    g = rng(7)
    lon, lat = g.uniform(40, -3.0, 3.0), g.uniform(40, -1.4, 1.4)
    ax.scatter(lon, lat, c=lat, s=30, cmap="viridis", zorder=3)
    ax.grid(True)


@case("geo/fill_markers")
def _(fig):
    ax = fig.add_subplot(projection="hammer")
    ax.fill([-2.5, -0.5, 0.5, 2.0, 1.0, -1.5], [-0.4, -1.0, -0.2, 0.3, 1.2, 0.9],
            color="tab:green", alpha=0.5)
    lon = np.linspace(-2.8, 2.8, 8)
    ax.plot(lon, np.full(8, -1.2), "o--", color="tab:purple")
    ax.grid(True)


@case("geo/pcolormesh")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    lon = np.linspace(-np.pi, np.pi, 37)
    lat = np.linspace(-np.pi / 2, np.pi / 2, 19)
    lon2, lat2 = np.meshgrid(lon[:-1] + np.pi / 36, lat[:-1] + np.pi / 36)
    ax.pcolormesh(lon, lat, np.cos(lat2) * np.sin(2 * lon2), cmap="RdBu_r")
    ax.grid(True)


@case("geo/custom_grid")
def _(fig):
    ax = fig.add_subplot(projection="aitoff")
    ax.set_longitude_grid(45)
    ax.set_latitude_grid(30)
    ax.set_longitude_grid_ends(60)
    ax.grid(True, color="tab:blue", linestyle="--", linewidth=0.8)


@case("geo/lambert_center", figsize=(5, 5))
def _(fig):
    ax = fig.add_subplot(projection="lambert", center_longitude=np.pi / 4,
                         center_latitude=np.pi / 6)
    ax.plot(*_track(), color="k")
    ax.grid(True)
    ax.set_title("centred at 45°E, 30°N")


@case("geo/subplots_grid_on_off", figsize=(8, 3))
def _(fig):
    left = fig.add_subplot(1, 2, 1, projection="aitoff")
    left.plot(*_track())
    left.set_title("grid off")
    right = fig.add_subplot(1, 2, 2, projection="hammer")
    right.plot(*_track())
    right.grid(True)
    right.set_title("grid on")


# --- grids and tick formatting on every projection


@case("geo/four_projections_grids", figsize=(8, 6))
def _(fig):
    for i, name in enumerate(["aitoff", "hammer", "lambert", "mollweide"]):
        ax = fig.add_subplot(2, 2, i + 1, projection=name)
        ax.plot(*_track(), color="tab:red")
        ax.grid(True)
        ax.set_title(name, fontsize=10)
        ax.tick_params(labelsize=7)


@case("geo/hammer_coarse_grid_styled_ticks")
def _(fig):
    ax = fig.add_subplot(projection="hammer")
    ax.set_longitude_grid(60)
    ax.set_latitude_grid(20)
    ax.grid(True, color="0.6", linestyle=":")
    ax.tick_params(axis="x", labelcolor="tab:blue", labelsize=8)
    ax.tick_params(axis="y", labelcolor="tab:green")


@case("geo/mollweide_fine_grid_ends")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    ax.set_longitude_grid(20)
    ax.set_latitude_grid(10)
    ax.set_longitude_grid_ends(80)
    ax.grid(True, linewidth=0.5)
    ax.tick_params(labelsize=6)


@case("geo/lambert_south_center_grid", figsize=(5, 5))
def _(fig):
    ax = fig.add_subplot(projection="lambert", center_longitude=-np.pi / 3,
                         center_latitude=-np.pi / 4)
    ax.set_longitude_grid(45)
    ax.set_latitude_grid(15)
    ax.plot(*_track(), color="tab:purple")
    ax.grid(True)


@case("geo/aitoff_custom_ticklabels")
def _(fig):
    ax = fig.add_subplot(projection="aitoff")
    ax.set_xticklabels([f"{abs(d)}°{'W' if d < 0 else 'E' if d > 0 else ''}"
                        for d in range(-150, 151, 30)])
    ax.set_yticklabels([f"{abs(d)}°{'S' if d < 0 else 'N' if d > 0 else ''}"
                        for d in range(-75, 76, 15)])
    ax.grid(True)


@case("geo/mollweide_hour_formatter")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    ax.xaxis.set_major_formatter(lambda x, pos: f"{int(round(np.degrees(x) / 15)) % 24}h")
    ax.plot(*_track())
    ax.grid(True)


# --- artists through the projections


@case("geo/hammer_contourf_colorbar", figsize=(6.4, 4))
def _(fig):
    ax = fig.add_subplot(projection="hammer")
    lon = np.linspace(-np.pi, np.pi, 73)
    lat = np.linspace(-np.pi / 2, np.pi / 2, 37)
    lon2, lat2 = np.meshgrid(lon, lat)
    cs = ax.contourf(lon2, lat2, np.cos(lat2) ** 2 * np.cos(3 * lon2), levels=8, cmap="viridis")
    fig.colorbar(cs, ax=ax, orientation="horizontal", shrink=0.6, pad=0.08)
    ax.grid(True)


@case("geo/aitoff_scatter_text_annotate")
def _(fig):
    ax = fig.add_subplot(projection="aitoff")
    lon = np.radians([-120, -60, 0, 45, 100, 150])
    lat = np.radians([40, -20, 51, 10, -35, 60])
    ax.scatter(lon, lat, s=[30, 60, 90, 120, 150, 180], c="tab:orange", edgecolors="k", zorder=3)
    ax.text(lon[2], lat[2] + 0.12, "Greenwich", ha="center", fontsize=8)
    ax.annotate("far east", xy=(lon[5], lat[5]), xytext=(1.5, 1.2),
                arrowprops=dict(arrowstyle="->", color="0.3"), fontsize=8)
    ax.grid(True)


@case("geo/mollweide_lines_beyond_edge")
def _(fig):
    ax = fig.add_subplot(projection="mollweide")
    lon = np.linspace(-4.0, 4.0, 100)
    ax.plot(lon, 0.5 * np.cos(lon), lw=2)
    ax.plot([2.8, 3.6], [-1.0, -0.6], "o-", color="tab:red")
    ax.fill_between(np.linspace(-1.0, 1.0, 20), -0.4, 0.4, color="tab:green", alpha=0.3)
    ax.grid(True)


@case("geo/hammer_facecolor_no_labels")
def _(fig):
    ax = fig.add_subplot(projection="hammer")
    ax.set_facecolor("#dceefc")
    ax.set_xticklabels([])
    ax.set_yticklabels([])
    ax.grid(True, color="w", linewidth=1.5)
    ax.plot(*_track(), color="tab:blue")
