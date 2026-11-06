"""xarray's plotting (``DataArray.plot``, ``Dataset.plot`` and its facet
grids) on pltrs as matplotlib."""

import numpy as np
import pytest

xr = pytest.importorskip("xarray")


@pytest.fixture
def field():
    """A 2-D field over named coordinates with units."""
    x = np.linspace(0, 4, 30)
    y = np.linspace(-1, 1, 20)
    data = np.sin(x)[None, :] * np.cos(3 * y)[:, None]
    return xr.DataArray(data, dims=("y", "x"), coords={"x": x, "y": y},
                        name="amplitude", attrs={"units": "m"})


def test_pcolormesh(check, field):
    check(field.plot())


def test_contourf_levels(check, field):
    check(field.plot.contourf(levels=7, cmap="viridis"))


def test_contour_colors(check, field):
    check(field.plot.contour(levels=[-0.5, 0, 0.5], colors=["k", "r", "b"]))


def test_lines_hue(check, field):
    check(field.isel(y=slice(0, 20, 5)).plot.line(x="x", hue="y"))


def test_facet_grid(check, field):
    stacked = xr.concat([field, 2 * field], dim="scale").assign_coords(scale=[1, 2])
    check(stacked.plot(col="scale"))


def test_dataset_scatter(check, field):
    ds = xr.Dataset({"a": field, "b": field ** 2})
    check(ds.plot.scatter(x="a", y="b", hue="b"))


def test_histogram(check, field):
    check(field.plot.hist(bins=12))
