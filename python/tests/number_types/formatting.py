"""
R, what tick helpers answer outside a draw: every formatter's
``__call__``, ``format_data`` and ``format_data_short`` on a drawn axis,
for values that are tick locations and values that are not, and every
locator and formatter called without an axis.
"""

import datetime

import pltrs.pyplot as plt
from pltrs import dates, ticker

#: The formatters, by name, with the scale of the axis they label.
FORMATTERS = (
    ("ScalarFormatter", "linear", ticker.ScalarFormatter),
    ("ScalarFormatter(useOffset)", "linear", lambda: ticker.ScalarFormatter(useOffset=True)),
    ("EngFormatter", "linear", lambda: ticker.EngFormatter(unit="m")),
    ("PercentFormatter", "linear", lambda: ticker.PercentFormatter(10)),
    ("StrMethodFormatter", "linear", lambda: ticker.StrMethodFormatter("{x:.1f}")),
    ("FormatStrFormatter", "linear", lambda: ticker.FormatStrFormatter("%.2f")),
    ("FuncFormatter", "linear", lambda: ticker.FuncFormatter(lambda x, pos: f"{x}|{pos}")),
    ("FixedFormatter", "linear", lambda: ticker.FixedFormatter(list("abcdefghij"))),
    ("NullFormatter", "linear", ticker.NullFormatter),
    ("LogFormatter", "log", ticker.LogFormatter),
    ("LogFormatterExponent", "log", ticker.LogFormatterExponent),
    ("LogFormatterMathtext", "log", ticker.LogFormatterMathtext),
    ("LogFormatterSciNotation", "log", ticker.LogFormatterSciNotation),
    ("LogitFormatter", "logit", ticker.LogitFormatter),
)

#: Values each formatter is asked about, on and off its ticks.
VALUES = {"linear": (2, 2.5, 3.7, 1000.25), "log": (10, 100, 30, 0.5),
          "logit": (0.1, 0.5, 0.33, 0.999)}

#: The data each scale is drawn with.
DATA = {"linear": ([0, 10], [1, 100]), "log": ([1, 1000], [1, 2]),
        "logit": ([0.01, 0.99], [1, 2])}


def formatters(rows):
    """Each formatter on a drawn axis, asked about values."""
    for label, scale, make in FORMATTERS:
        fig, ax = plt.subplots()
        ax.plot(*DATA[scale])
        ax.set_xscale(scale)
        formatter = make()
        ax.xaxis.set_major_formatter(formatter)
        fig.canvas.draw()
        for value in VALUES[scale]:
            rows.row(f"R {label} on a drawn axis", f"__call__({value})", lambda v=value: formatter(v))
            rows.row(f"R {label} on a drawn axis", f"__call__({value}, 1)",
                     lambda v=value: formatter(v, 1))
            rows.row(f"R {label} on a drawn axis", f"format_data({value})",
                     lambda v=value: formatter.format_data(v))
            rows.row(f"R {label} on a drawn axis", f"format_data_short({value})",
                     lambda v=value: formatter.format_data_short(v))
        rows.row(f"R {label} on a drawn axis", "get_offset()", formatter.get_offset)
        rows.row(f"R {label} on a drawn axis", "locs", lambda f=formatter: f.locs)
        plt.close(fig)
    day = datetime.datetime(2020, 1, 1)
    for label, make in (("AutoDateFormatter", lambda loc: dates.AutoDateFormatter(loc)),
                        ("ConciseDateFormatter", lambda loc: dates.ConciseDateFormatter(loc)),
                        ("DateFormatter", lambda loc: dates.DateFormatter("%Y-%m-%d"))):
        fig, ax = plt.subplots()
        ax.plot([day, day + datetime.timedelta(days=10)], [1, 2])
        formatter = make(ax.xaxis.get_major_locator())
        ax.xaxis.set_major_formatter(formatter)
        fig.canvas.draw()
        tick = ax.get_xticks()[1]
        for value in (tick, tick + 0.3):
            rows.row(f"R {label} on a drawn axis", f"__call__(tick{'+0.3' if value != tick else ''})",
                     lambda v=value: formatter(v))
            rows.row(f"R {label} on a drawn axis",
                     f"format_data_short(tick{'+0.3' if value != tick else ''})",
                     lambda v=value: formatter.format_data_short(v))
        rows.row(f"R {label} on a drawn axis", "get_offset()", formatter.get_offset)
        plt.close(fig)


#: Every locator of `ticker`, made with its arguments.
LOCATORS = (
    ("MaxNLocator", ticker.MaxNLocator), ("AutoLocator", ticker.AutoLocator),
    ("MultipleLocator", lambda: ticker.MultipleLocator(2)),
    ("LinearLocator", ticker.LinearLocator),
    ("FixedLocator", lambda: ticker.FixedLocator([1, 2, 3])),
    ("IndexLocator", lambda: ticker.IndexLocator(1, 0)),
    ("LogLocator", ticker.LogLocator), ("SymmetricalLogLocator",
                                        lambda: ticker.SymmetricalLogLocator(base=10, linthresh=1)),
    ("AsinhLocator", lambda: ticker.AsinhLocator(1)), ("LogitLocator", ticker.LogitLocator),
    ("AutoMinorLocator", ticker.AutoMinorLocator), ("NullLocator", ticker.NullLocator),
)


def without_axis(rows):
    """Every locator and formatter of `ticker` called without an axis."""
    for label, make in LOCATORS:
        for what, call in (("__call__()", lambda m: m()()),
                           ("tick_values(1, 10)", lambda m: m().tick_values(1, 10)),
                           ("view_limits(1, 10)", lambda m: m().view_limits(1, 10)),
                           ("nonsingular(1, 10)", lambda m: m().nonsingular(1, 10)),
                           ("nonsingular(-inf, inf)",
                            lambda m: m().nonsingular(-float("inf"), float("inf")))):
            rows.row(f"R {label} without an axis", what, lambda m=make, c=call: c(m))
    for label, _, make in FORMATTERS:
        for what, call in (("__call__(10)", lambda f: f()(10)),
                           ("__call__(10, 0)", lambda f: f()(10, 0)),
                           ("format_data(10)", lambda f: f().format_data(10)),
                           ("format_data_short(10)", lambda f: f().format_data_short(10)),
                           ("format_ticks([1, 10])", lambda f: f().format_ticks([1, 10])),
                           ("get_offset()", lambda f: f().get_offset())):
            rows.row(f"R {label} without an axis", what, lambda m=make, c=call: c(m))


SCENES = (formatters, without_axis)
