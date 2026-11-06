"""geopandas' plotting (``GeoDataFrame.plot`` and ``GeoSeries.plot``) on
pltrs as matplotlib."""

import pytest

gpd = pytest.importorskip("geopandas")
shapely = pytest.importorskip("shapely")


@pytest.fixture
def frame():
    """Squares in a row with a value each, and points and lines."""
    from shapely.geometry import LineString, Point, box
    squares = [box(i, 0, i + 0.9, 1 + 0.2 * i) for i in range(5)]
    return gpd.GeoDataFrame({"value": [3.0, 1.0, 4.0, 1.5, 5.0],
                             "kind": list("ababa")}, geometry=squares), [
        Point(i, 2) for i in range(5)] + [LineString([(0, 3), (4, 3.5)])]


def test_choropleth(check, frame):
    check(frame[0].plot(column="value", legend=True))


def test_categories(check, frame):
    check(frame[0].plot(column="kind", legend=True, edgecolor="k"))


def test_mixed_geometries(check, frame):
    check(gpd.GeoSeries(frame[1]).plot(color="tab:green", markersize=20))


def test_boundary(check, frame):
    check(frame[0].boundary.plot(linewidth=2))
