"""pandas' plotting (``DataFrame.plot``, ``Series.plot`` and
``pandas.plotting``) on pltrs as matplotlib."""

import numpy as np
import pytest

pd = pytest.importorskip("pandas")


def test_line(check, frame):
    check(frame[["y", "z"]].plot())


def test_line_time_series(check, series):
    check(series.plot())


def test_line_periods(check):
    check(pd.Series(np.arange(12.0),
                    index=pd.period_range("2024-01", periods=12, freq="M")).plot())


def test_line_styles_grid_and_title(check, frame):
    check(frame[["y", "z"]].plot(style=["--", "o-"], grid=True, title="t"))


def test_line_secondary_y(check, frame):
    check(frame[["y", "z"]].plot(secondary_y=["z"]))


def test_line_log_scale(check, frame):
    check((frame[["w"]] + 2).plot(logy=True))


def test_line_subplots(check, frame):
    check(frame[["y", "z"]].plot(subplots=True))


def test_bar(check, small):
    check(small.plot.bar())


def test_bar_error_bars(check, small):
    check(small.plot.bar(yerr=small * 0.1))


def test_bar_subplots(check, small):
    check(small.plot.bar(subplots=True))


def test_bar_dates(check):
    check(pd.Series(np.arange(12.0),
                    index=pd.date_range("2024-01-01", periods=12, freq="MS")).plot.bar())


def test_barh_stacked(check, small):
    check(small.plot.barh(stacked=True))


def test_hist(check, frame):
    check(frame[["y", "z"]].plot.hist(bins=15, alpha=0.5))


def test_box(check, frame):
    check(frame[["y", "z", "w"]].plot.box())


def test_kde(check, frame):
    pytest.importorskip("scipy")
    check(frame[["y", "z"]].plot.kde())


def test_density(check, series):
    pytest.importorskip("scipy")
    check(series.plot.density())


def test_area(check, small):
    check(small.plot.area())


def test_area_unstacked(check, small):
    check(small.plot.area(stacked=False))


def test_scatter(check, frame):
    check(frame.plot.scatter(x="x", y="y", c="w", colormap="viridis"))


def test_hexbin(check, frame):
    check(frame.plot.hexbin(x="y", y="z", gridsize=10))


def test_pie(check, small):
    check(small.plot.pie(y="p"))


def test_frame_hist(check, frame):
    check(frame[["y", "z"]].hist()[0, 0])


def test_frame_hist_by(check, frame):
    check(frame.hist(column="y", by="cat")[0, 0])


def test_boxplot_by(check, frame):
    check(frame.boxplot(column=["y", "z"], by="cat")[0])


def test_scatter_matrix(check, frame):
    check(pd.plotting.scatter_matrix(frame[["y", "z", "w"]])[0, 0])


def test_table(check, small):
    import matplotlib.pyplot as plt
    pd.plotting.table(plt.gca(), small)
    check(plt.gcf())


def test_andrews_curves(check, frame):
    check(pd.plotting.andrews_curves(frame[["y", "z", "w", "cat"]], "cat"))


def test_parallel_coordinates(check, frame):
    check(pd.plotting.parallel_coordinates(frame[["y", "z", "w", "cat"]], "cat"))


def test_lag_plot(check, series):
    check(pd.plotting.lag_plot(series))


def test_autocorrelation_plot(check, series):
    check(pd.plotting.autocorrelation_plot(series))


def test_radviz(check, frame):
    check(pd.plotting.radviz(frame[["y", "z", "w", "cat"]], "cat"))


def test_bootstrap_plot(check, series):
    check(pd.plotting.bootstrap_plot(series, size=20, samples=50))
