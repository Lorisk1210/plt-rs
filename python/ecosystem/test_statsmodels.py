"""statsmodels' graphics (``statsmodels.graphics``) on pltrs as
matplotlib."""

import numpy as np
import pytest

sm = pytest.importorskip("statsmodels.api")


@pytest.fixture
def values():
    """A deterministic sample, roughly normal."""
    return np.sin(np.arange(200) * 0.7) + 0.5 * np.cos(np.arange(200) * 1.3)


def test_qqplot(check, values):
    check(sm.qqplot(values, line="s"))


def test_acf(check, values):
    from statsmodels.graphics.tsaplots import plot_acf
    check(plot_acf(values, lags=20))


def test_boxplots(check, values):
    from statsmodels.graphics.boxplots import violinplot
    import matplotlib.pyplot as plt
    fig, ax = plt.subplots()
    violinplot([values[:100], values[100:]], ax=ax, labels=["a", "b"])
    check(fig)


def test_regression_plots(check, values):
    x = np.linspace(0, 1, 200)
    results = sm.OLS(values + 2 * x, sm.add_constant(x)).fit()
    check(sm.graphics.plot_fit(results, 1))


def test_mosaic(check):
    from statsmodels.graphics.mosaicplot import mosaic
    data = {("a", "x"): 10, ("a", "y"): 15, ("b", "x"): 8, ("b", "y"): 4}
    fig, _ = mosaic(data, title="mosaic")
    check(fig)
