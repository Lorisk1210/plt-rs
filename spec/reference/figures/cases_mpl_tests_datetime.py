"""matplotlib's own tests of plotting dates (``tests/test_datetime.py``): every
``TestDatetimePlotting`` method that draws, one case per figure, showing which
Axes methods accept ``datetime`` and ``numpy.datetime64`` values (and
``timedelta`` extents) in matplotlib.

The tests pick the ``default`` style, which needs no rc; most set
``date.converter`` to ``"concise"``, which becomes the case's rc. A test
drawing several figures becomes one case per figure (``hist_1``,
``hist_2``), a parametrised one a case per parameter (``violin_vertical``,
``violin_horizontal``). ``layout="constrained"`` and
``constrained_layout=True`` set the constrained layout engine and ``figsize``
the case's canvas.

Deviations from the tests: ``np.random`` data comes from
``spec_support.rng`` with the tests' seeds (``test_hist`` draws unseeded
integers upstream), and the random dates of ``test_eventplot`` are whole
seconds after its start date (``rng.integers``) rather than fractions of its
date range, so that they are exact.

Not ported: the ``xfail`` placeholders (``test_arrow``, ``test_axline``,
``test_boxplot``, ``test_hexbin``, ``test_hist2d``, ``test_loglog``,
``test_pcolor``, ``test_pcolorfast``, ``test_pcolormesh``, ``test_quiver``,
``test_semilogx``, ``test_semilogy``, ``test_streamplot``,
``test_tricontour``, ``test_tricontourf``, ``test_tripcolor``,
``test_triplot``, ``test_violinplot``), which call their method with
``...``.
"""

import datetime

import matplotlib as mpl
import numpy as np

from spec_support import case, rng

GROUP = "mpl_tests_datetime"

#: What the tests setting ``date.converter`` change.
CONCISE = {"date.converter": "concise"}


def datetime_test(name, *, rc=None, figsize=(6.4, 4.8), constrained=False):
    """Register ``build`` as ``mpl_tests_datetime/<name>`` on a canvas of
    ``figsize``, with the constrained layout engine when ``constrained``."""

    def register(build):
        def run(fig):
            if constrained:
                fig.set_layout_engine("constrained")
            build(fig)

        case(f"{GROUP}/{name}", figsize=figsize, rc=rc or {})(run)
        return build

    return register


def _day(year, month, day, hour=0):
    return datetime.datetime(year, month, day, hour)


def _days(n):
    return datetime.timedelta(days=n)


@datetime_test("annotate", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3, ax4 = fig.subplots(4, 1)
    start_date = _day(2023, 10, 1)
    dates = [start_date + _days(i) for i in range(31)]
    data = list(range(1, 32))
    test_text = "Test Text"
    ax1.plot(dates, data)
    ax1.annotate(text=test_text, xy=(dates[15], data[15]))
    ax2.plot(data, dates)
    ax2.annotate(text=test_text, xy=(data[5], dates[26]))
    ax3.plot(dates, dates)
    ax3.annotate(text=test_text, xy=(dates[15], dates[3]))
    ax4.plot(dates, dates)
    ax4.annotate(text=test_text, xy=(dates[5], dates[30]),
                 xytext=(dates[1], dates[7]), arrowprops=dict(facecolor="red"))


@datetime_test("axhline", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    ax1.set_ylim(bottom=_day(2020, 4, 1), top=_day(2020, 8, 1))
    ax2.set_ylim(bottom=np.datetime64("2005-01-01"), top=np.datetime64("2005-04-01"))
    ax3.set_ylim(bottom=_day(2023, 9, 1), top=_day(2023, 11, 1))
    ax1.axhline(y=_day(2020, 6, 3), xmin=0.5, xmax=0.7)
    ax2.axhline(np.datetime64("2005-02-25T03:30"), xmin=0.1, xmax=0.9)
    ax3.axhline(y=_day(2023, 10, 24), xmin=0.4, xmax=0.7)


def _spans(fig, horizontal):
    """``test_axhspan`` (``horizontal``) and ``test_axvspan``."""
    start_date = _day(2023, 1, 1)
    dates = [start_date + _days(i) for i in range(31)]
    numbers = list(range(1, 32))
    ax1, ax2, ax3 = fig.subplots(3, 1)
    span = (lambda ax, low, high: ax.axhspan(ymin=low, ymax=high, facecolor="green",
                                             alpha=0.5)) if horizontal else (
        lambda ax, low, high: ax.axvspan(xmin=low, xmax=high, facecolor="red", alpha=0.5))
    date_spans = [(start_date + _days(i), start_date + _days(i + 1)) for i in range(0, 31, 2)]
    number_spans = [(i + 1, i + 2) for i in range(0, 31, 2)]
    for ax, x, y, spans, title, xlabel, ylabel in [
            (ax1, dates, numbers, number_spans if horizontal else date_spans,
             "Datetime vs. Number", "Date", "Number"),
            (ax2, numbers, dates, date_spans if horizontal else number_spans,
             "Number vs. Datetime", "Number", "Date"),
            (ax3, dates, dates, date_spans, "Datetime vs. Datetime", "Date", "Date")]:
        ax.plot(x, y, marker="o", color="blue")
        for low, high in spans:
            span(ax, low, high)
        ax.set_title(title)
        ax.set_xlabel(xlabel)
        ax.set_ylabel(ylabel)


@datetime_test("axhspan", rc=CONCISE, figsize=(10, 12), constrained=True)
def _(fig):
    _spans(fig, horizontal=True)


@datetime_test("axvline", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    ax1.set_xlim(left=_day(2020, 4, 1), right=_day(2020, 8, 1))
    ax2.set_xlim(left=np.datetime64("2005-01-01"), right=np.datetime64("2005-04-01"))
    ax3.set_xlim(left=_day(2023, 9, 1), right=_day(2023, 11, 1))
    ax1.axvline(x=_day(2020, 6, 3), ymin=0.5, ymax=0.7)
    ax2.axvline(np.datetime64("2005-02-25T03:30"), ymin=0.1, ymax=0.9)
    ax3.axvline(x=_day(2023, 10, 24), ymin=0.4, ymax=0.7)


@datetime_test("axvspan", rc=CONCISE, figsize=(10, 12), constrained=True)
def _(fig):
    _spans(fig, horizontal=False)


@datetime_test("bar", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2 = fig.subplots(2, 1)
    x_dates = np.array([_day(2020, 6, 30), _day(2020, 7, 22), _day(2020, 8, 3),
                        _day(2020, 9, 14)], dtype=np.datetime64)
    x_ranges = [8800, 2600, 8500, 7400]
    x = np.datetime64(_day(2020, 6, 1))
    ax1.bar(x_dates, x_ranges, width=np.timedelta64(4, "D"))
    ax2.bar(np.arange(4), x_dates - x, bottom=x)


@datetime_test("bar_label", figsize=(10, 8), constrained=True)
def _(fig):
    date_list = [_day(2023, 1, 1) + _days(i) for i in range(5)]
    values = [10, 20, 15, 25, 30]
    ax = fig.subplots(1, 1)
    bars = ax.bar(date_list, values)
    ax.bar_label(bars, labels=[f"{val}%" for val in values], label_type="edge",
                 color="black")


@datetime_test("barbs", rc=CONCISE, figsize=(12, 6))
def _(fig):
    start_date = _day(2022, 2, 8, 22)
    dates = [start_date + datetime.timedelta(hours=i) for i in range(12)]
    numbers = np.sin(np.linspace(0, 2 * np.pi, 12))
    u = np.ones(12) * 10
    v = np.arange(0, 120, 10)
    axes = fig.subplots(nrows=1, ncols=2)
    axes[0].barbs(dates, numbers, u, v, length=7)
    axes[0].set_title("Datetime vs. Numeric Data")
    axes[0].set_xlabel("Datetime")
    axes[0].set_ylabel("Numeric Data")
    axes[1].barbs(numbers, dates, u, v, length=7)
    axes[1].set_title("Numeric vs. Datetime Data")
    axes[1].set_xlabel("Numeric Data")
    axes[1].set_ylabel("Datetime")


@datetime_test("barh", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2 = fig.subplots(2, 1)
    birth_date = np.array([_day(2020, 4, 10), _day(2020, 5, 30), _day(2020, 10, 12),
                           _day(2020, 11, 15)])
    year_start = _day(2020, 1, 1)
    year_end = _day(2020, 12, 31)
    age = [21, 53, 20, 24]
    ax1.set_xlabel("Age")
    ax1.set_ylabel("Birth Date")
    ax1.barh(birth_date, width=age, height=_days(10))
    ax2.set_xlim(left=year_start, right=year_end)
    ax2.set_xlabel("Birth Date")
    ax2.set_ylabel("Order of Birth Dates")
    ax2.barh(np.arange(4), birth_date - year_start, left=year_start)


@datetime_test("broken_barh", rc=CONCISE)
def _(fig):
    ax = fig.subplots()
    ax.broken_barh([(_day(2023, 1, 4), _days(2)), (_day(2023, 1, 8), _days(3))],
                   (10, 9), facecolors="tab:blue")
    ax.broken_barh([(_day(2023, 1, 2), _days(1)), (_day(2023, 1, 4), _days(4))],
                   (20, 9), facecolors=("tab:red"))


@datetime_test("bxp", rc=CONCISE)
def _(fig):
    ax = fig.subplots()
    data = [{
        "med": _day(2020, 1, 15),
        "q1": _day(2020, 1, 10),
        "q3": _day(2020, 1, 20),
        "whislo": _day(2020, 1, 5),
        "whishi": _day(2020, 1, 25),
        "fliers": [_day(2020, 1, 3), _day(2020, 1, 27)],
    }]
    ax.bxp(data, orientation="horizontal")
    ax.xaxis.set_major_formatter(mpl.dates.DateFormatter("%Y-%m-%d"))
    ax.set_title("Box plot with datetime data")


@datetime_test("clabel")
def _(fig):
    dates = [_day(2023, 10, 1) + _days(i) for i in range(10)]
    x = np.arange(-10.0, 5.0, 0.5)
    X, Y = np.meshgrid(x, dates)
    Z = np.arange(X.size).reshape(X.shape)
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z)
    ax.clabel(CS, manual=[(x[0], dates[0])])


def _contours(fig, filled):
    """``test_contour`` and ``test_contourf``."""
    range_threshold = 10
    ax1, ax2, ax3 = fig.subplots(3, 1)
    x_dates = np.array([_day(2023, 10, delta) for delta in range(1, range_threshold)])
    y_dates = np.array([_day(2023, 10, delta) for delta in range(1, range_threshold)])
    x_ranges = np.array(range(1, range_threshold))
    y_ranges = np.array(range(1, range_threshold))
    X_dates, Y_dates = np.meshgrid(x_dates, y_dates)
    X_ranges, Y_ranges = np.meshgrid(x_ranges, y_ranges)
    Z_ranges = np.cos(X_ranges / 4) + np.sin(Y_ranges / 4)
    for ax, X, Y in [(ax1, X_dates, Y_dates), (ax2, X_dates, Y_ranges),
                     (ax3, X_ranges, Y_dates)]:
        (ax.contourf if filled else ax.contour)(X, Y, Z_ranges)


@datetime_test("contour", rc=CONCISE, constrained=True)
def _(fig):
    _contours(fig, filled=False)


@datetime_test("contourf", rc=CONCISE, constrained=True)
def _(fig):
    _contours(fig, filled=True)


@datetime_test("errorbar", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3, ax4 = fig.subplots(4, 1)
    limit = 7
    x_dates = np.array([_day(2023, 10, d) for d in range(1, limit)])
    y_dates = np.array([_day(2023, 10, d) for d in range(1, limit)])
    x_date_error = _days(1)
    y_date_error = _days(1)
    x_values = list(range(1, limit))
    y_values = list(range(1, limit))
    x_value_error = 0.5
    y_value_error = 0.5
    ax1.errorbar(x_dates, y_values, yerr=y_value_error, capsize=10, barsabove=True,
                 label="Data")
    ax2.errorbar(x_values, y_dates, xerr=x_value_error, yerr=y_date_error,
                 errorevery=(1, 2), fmt="-o", label="Data")
    ax3.errorbar(x_dates, y_dates, xerr=x_date_error, yerr=y_date_error, lolims=True,
                 xlolims=True, label="Data")
    ax4.errorbar(x_dates, y_values, xerr=x_date_error, yerr=y_value_error, uplims=True,
                 xuplims=True, label="Data")


@datetime_test("eventplot", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    x_dates1 = np.array([_day(2020, 6, 30), _day(2020, 7, 22), _day(2020, 8, 3),
                         _day(2020, 9, 14)], dtype=np.datetime64)
    ax1.eventplot(x_dates1)
    gen = rng(19680801)
    start_date = _day(2020, 7, 1)
    end_date = _day(2020, 10, 15)
    seconds = int((end_date - start_date).total_seconds())
    dates1, dates2, dates3 = (
        [start_date + datetime.timedelta(seconds=int(s)) for s in gen.integers(n, 0, seconds)]
        for n in [30, 10, 50])
    colors1 = ["C1", "C2", "C3"]
    lineoffsets1 = np.array([1, 6, 8])
    linelengths1 = [5, 2, 3]
    ax2.eventplot([dates1, dates2, dates3], colors=colors1, lineoffsets=lineoffsets1,
                  linelengths=linelengths1)
    lineoffsets2 = np.array([_day(2020, 7, 1), _day(2020, 7, 15), _day(2020, 8, 1)],
                            dtype=np.datetime64)
    ax3.eventplot([dates1, dates2, dates3], colors=colors1, lineoffsets=lineoffsets2,
                  linelengths=linelengths1)


def _date_walk(gen, start, steps, high):
    """``start`` followed by ``steps`` dates, each a whole number of days in
    ``[1, high)`` after the previous one."""
    dates = [start]
    for step in gen.integers(steps, 1, high):
        dates.append(dates[-1] + _days(int(step)))
    return dates


@datetime_test("fill", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3, ax4 = fig.subplots(4, 1)
    gen = rng(19680801)
    x_dates = _date_walk(gen, _day(2023, 1, 1), 4, 5)
    y_dates = _date_walk(gen, _day(2023, 1, 1), 4, 5)
    x_values = gen.uniform(5) * 5
    y_values = gen.uniform(5) * 5 - 2
    ax1.fill(x_dates, y_values)
    ax2.fill(x_values, y_dates)
    ax3.fill(x_values, y_values)
    ax4.fill(x_dates, y_dates)


def _between_data(gen):
    """The dates and values of ``test_fill_between`` and
    ``test_fill_betweenx``: two walks of dates, the second from the end of
    the first, sorted values, and a slower walk of dates."""
    dates1 = _date_walk(gen, _day(2023, 1, 1), 9, 5)
    dates2 = _date_walk(gen, dates1[-1], 9, 5)
    values = np.sort(gen.uniform(10) * 10)
    values1 = gen.uniform(10) * 10
    values2 = values1 + gen.uniform(10) * 10
    slow_dates = _date_walk(gen, _day(2023, 1, 1), 9, 10)
    return dates1, dates2, values, np.sort(values1), np.sort(values2), slow_dates


@datetime_test("fill_between", rc=CONCISE, constrained=True)
def _(fig):
    y_dates1, y_dates2, x_values, y_values1, y_values2, x_dates = _between_data(
        rng(19680801))
    ax1, ax2, ax3 = fig.subplots(3, 1)
    ax1.fill_between(x_values, y_dates1, y_dates2)
    ax2.fill_between(x_dates, y_values1, y_values2)
    ax3.fill_between(x_dates, y_dates1, y_dates2)


@datetime_test("fill_betweenx", rc=CONCISE, constrained=True)
def _(fig):
    x_dates1, x_dates2, y_values, x_values1, x_values2, y_dates = _between_data(
        rng(19680801))
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.fill_betweenx(y_values, x_dates1, x_dates2)
    ax2.fill_betweenx(y_dates, x_values1, x_values2)
    ax3.fill_betweenx(y_dates, x_dates1, x_dates2)


def _hist(fig, bins):
    """One figure of ``test_hist``: the same 30 days, weighted three ways,
    in ``bins``."""
    start_date = _day(2023, 10, 1)
    gen = rng(7)
    weights = [gen.integers(30, 1, 10) for _ in range(3)]
    days = [start_date + i * _days(1) for i in range(30)]
    for ax, values in zip(fig.subplots(3, 1), weights):
        ax.hist(days, bins=bins, weights=values)


@datetime_test("hist_1", rc=CONCISE, constrained=True)
def _(fig):
    _hist(fig, 10)


@datetime_test("hist_2", rc=CONCISE, constrained=True)
def _(fig):
    _hist(fig, [_day(2023, 10, 1) + i * _days(1) for i in range(31)])


@datetime_test("hlines", rc=CONCISE, constrained=True)
def _(fig):
    axs = fig.subplots(2, 4)
    date_strs = ["2023-03-08", "2023-04-09", "2023-05-13", "2023-07-28", "2023-12-24"]
    dates = [_day(2023, m * 2, 10) for m in range(1, 6)]
    date_start = [_day(2023, 6, d) for d in range(5, 30, 5)]
    date_end = [_day(2023, 7, d) for d in range(5, 30, 5)]
    np_dates = [np.datetime64(s) for s in date_strs]
    axs[0, 0].hlines(y=dates, xmin=[0.1, 0.2, 0.3, 0.4, 0.5], xmax=[0.5, 0.6, 0.7, 0.8, 0.9])
    axs[0, 1].hlines(dates, xmin=_day(2020, 5, 10), xmax=_day(2020, 5, 31))
    axs[0, 2].hlines(dates, xmin=date_start, xmax=date_end)
    axs[0, 3].hlines(dates, xmin=0.45, xmax=0.65)
    axs[1, 0].hlines(y=np_dates, xmin=[0.5, 0.6, 0.7, 0.8, 0.9], xmax=[0.1, 0.2, 0.3, 0.4, 0.5])
    axs[1, 2].hlines(y=np_dates, xmin=date_start, xmax=date_end)
    axs[1, 1].hlines(np_dates, xmin=_day(2020, 5, 10), xmax=_day(2020, 5, 31))
    axs[1, 3].hlines(np_dates, xmin=0.45, xmax=0.65)


@datetime_test("imshow")
def _(fig):
    ax = fig.subplots()
    a = np.diag(range(5))
    dt_start = _day(2010, 11, 1)
    dt_end = _day(2010, 11, 11)
    ax.imshow(a, extent=(dt_start, dt_end, dt_start, dt_end))
    ax.tick_params(axis="x", labelrotation=90)


@datetime_test("matshow")
def _(fig):
    a = np.diag(range(5))
    dt_start = _day(1980, 4, 15)
    dt_end = _day(2020, 11, 11)
    ax = fig.subplots()
    ax.matshow(a, extent=(dt_start, dt_end, dt_start, dt_end))
    for label in ax.get_xticklabels():
        label.set_rotation(90)


def _lines(fig, method):
    """``test_plot`` and ``test_step``."""
    n = 6
    ax1, ax2, ax3 = fig.subplots(3, 1)
    x = np.array([_day(2023, 9, i) for i in range(1, n)])
    getattr(ax1, method)(x, range(1, n))
    getattr(ax2, method)(range(1, n), x)
    getattr(ax3, method)(x, x)


@datetime_test("plot", rc=CONCISE, constrained=True)
def _(fig):
    _lines(fig, "plot")


@datetime_test("scatter", rc=CONCISE, figsize=(6, 6), constrained=True)
def _(fig):
    base = _day(2005, 2, 1)
    dates = [base + datetime.timedelta(hours=(2 * i)) for i in range(10)]
    y = np.cumsum(rng(19680801).normal(len(dates)))
    axs = fig.subplots(3, 1)
    axs[0].scatter(dates, y)
    for label in axs[0].get_xticklabels():
        label.set_rotation(40)
        label.set_horizontalalignment("right")
    axs[1].scatter(y, dates)
    axs[2].scatter(dates, dates)
    for label in axs[2].get_xticklabels():
        label.set_rotation(40)
        label.set_horizontalalignment("right")


@datetime_test("stackplot", rc=CONCISE, constrained=True)
def _(fig):
    n = 10
    stacked_nums = np.tile(np.arange(1, n), (4, 1))
    dates = np.array([_day(2020 + i, 1, 1) for i in range(n - 1)])
    ax = fig.subplots()
    ax.stackplot(dates, stacked_nums)


@datetime_test("stairs", rc=CONCISE, constrained=True)
def _(fig):
    start_date = _day(2023, 12, 1)
    baseline_date = _day(1980, 1, 1)
    bin_edges = [start_date + i * _days(1) for i in range(31)]
    edge_int = np.arange(31)
    gen = rng(123456)
    values1 = gen.integers(30, 1, 100)
    values2 = [start_date + _days(int(i)) for i in gen.integers(30, 1, 10000)]
    values3 = [start_date + _days(int(i)) for i in gen.integers(30, -10000, 10000)]
    ax1, ax2, ax3 = fig.subplots(3, 1)
    ax1.stairs(values1, edges=bin_edges)
    ax2.stairs(values2, edges=edge_int, baseline=baseline_date)
    ax3.stairs(values3, edges=bin_edges, baseline=baseline_date)


@datetime_test("stem", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3, ax4, ax5, ax6 = fig.subplots(6, 1)
    limit_value = 10
    above = _day(2023, 9, 18)
    below = _day(2023, 11, 18)
    x_ranges = np.arange(1, limit_value)
    y_ranges = np.arange(1, limit_value)
    x_dates = np.array([_day(2023, 10, n) for n in range(1, limit_value)])
    y_dates = np.array([_day(2023, 10, n) for n in range(1, limit_value)])
    ax1.stem(x_dates, y_dates, bottom=above)
    ax2.stem(x_dates, y_ranges, bottom=5)
    ax3.stem(x_ranges, y_dates, bottom=below)
    ax4.stem(x_ranges, y_dates, orientation="horizontal", bottom=above)
    ax5.stem(x_dates, y_ranges, orientation="horizontal", bottom=5)
    ax6.stem(x_ranges, y_dates, orientation="horizontal", bottom=below)


@datetime_test("step", rc=CONCISE, constrained=True)
def _(fig):
    _lines(fig, "step")


@datetime_test("text", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    limit_value = 10
    font_properties = {"family": "serif", "size": 12, "weight": "bold"}
    test_date = _day(2023, 10, 1)
    x_data = np.array(range(1, limit_value))
    y_data = np.array(range(1, limit_value))
    x_dates = np.array([_day(2023, 10, n) for n in range(1, limit_value)])
    y_dates = np.array([_day(2023, 10, n) for n in range(1, limit_value)])
    ax1.plot(x_dates, y_data)
    ax1.text(test_date, 5, "Inserted Text", **font_properties)
    ax2.plot(x_data, y_dates)
    ax2.text(7, test_date, "Inserted Text", **font_properties)
    ax3.plot(x_dates, y_dates)
    ax3.text(test_date, test_date, "Inserted Text", **font_properties)


def _violin(fig, orientation):
    ax = fig.subplots()
    datetimes = [_day(2023, 2, 10), _day(2023, 5, 18), _day(2023, 6, 6)]
    ax.violin([{"coords": datetimes, "vals": [0.1, 0.5, 0.2], "mean": datetimes[1],
                "median": datetimes[1], "min": datetimes[0], "max": datetimes[-1],
                "quantiles": datetimes}], orientation=orientation)


@datetime_test("violin_vertical")
def _(fig):
    _violin(fig, "vertical")


@datetime_test("violin_horizontal")
def _(fig):
    _violin(fig, "horizontal")


@datetime_test("vlines", rc=CONCISE, constrained=True)
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    ax1.set_xlim(left=_day(2023, 1, 1), right=_day(2023, 6, 30))
    ax1.vlines(x=[_day(2023, 2, 10), _day(2023, 5, 18), _day(2023, 6, 6)],
               ymin=[0, 0.25, 0.5], ymax=[0.25, 0.5, 0.75])
    ax2.set_xlim(left=0, right=0.5)
    ax2.vlines(x=[0.3, 0.35],
               ymin=[np.datetime64("2023-03-20"), np.datetime64("2023-03-31")],
               ymax=[np.datetime64("2023-05-01"), np.datetime64("2023-05-16")])
    ax3.set_xlim(left=_day(2023, 7, 1), right=_day(2023, 12, 31))
    ax3.vlines(x=[_day(2023, 9, 1), _day(2023, 12, 10)], ymin=_day(2023, 1, 15),
               ymax=_day(2023, 1, 30))
