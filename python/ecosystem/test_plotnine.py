"""plotnine's grammar of graphics (``ggplot`` with geoms, facets and
themes) on pltrs as matplotlib."""

import numpy as np
import pytest

p9 = pytest.importorskip("plotnine")


@pytest.fixture
def data():
    """A frame of deterministic data."""
    pd = pytest.importorskip("pandas")
    x = np.linspace(0, 10, 60)
    return pd.DataFrame({"x": x, "y": np.sin(x) + 0.1 * x,
                         "cat": np.array(["a", "b", "c"])[np.arange(60) % 3]})


def test_points_and_smooth(check, data):
    plot = (p9.ggplot(data, p9.aes("x", "y", color="cat")) + p9.geom_point()
            + p9.geom_smooth(method="lm"))
    check(plot.draw())


def test_bars(check, data):
    check((p9.ggplot(data, p9.aes("cat")) + p9.geom_bar()).draw())


def test_facets_and_theme(check, data):
    plot = (p9.ggplot(data, p9.aes("x", "y")) + p9.geom_line()
            + p9.facet_wrap("cat") + p9.theme_bw() + p9.labs(title="facets"))
    check(plot.draw())


def test_histogram_fill(check, data):
    check((p9.ggplot(data, p9.aes("y", fill="cat")) + p9.geom_histogram(bins=10)).draw())
