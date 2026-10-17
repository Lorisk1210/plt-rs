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
