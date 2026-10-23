"""Property cycles, ported from matplotlib's own ``tests/test_cycles.py``:
``Axes.set_prop_cycle`` with cyclers of one or several properties, the
artists ``plot`` and ``fill`` make from them, properties given explicitly
and invalid cycles.

Area ``mpl_cycles``.

Ops
---
``axes.Axes.set_prop_cycle``  {cycle?, args?, kwargs?, artist?, calls?,
    getters?} -> on the axes of ``plt.subplots()``, ``ax.set_prop_cycle``
    with ``eval(cycle)`` (a cycler expression, ``cycler`` being
    ``cycler.cycler``), else with ``*args, **kwargs``; then one
    ``ax.<artist>(range(10), range(10), *fmt, **kw)`` per call ``{"fmt"?,
    "kw"?}`` (``artist`` is ``"plot"`` or ``"fill"``); returns ``{getter:
    [value per artist]}`` over ``ax.lines`` (plot) or ``ax.patches``
    (fill). Colours are RGBA lists, line styles ``"solid"``, ``"dashed"``,
    ``"dashdot"``, ``"dotted"`` or ``"none"``, markers as matplotlib keeps
    them, and ``get_dash_pattern`` is a line's ``[offset, [on, off, ...]]``
    before scaling by the line width (``_unscaled_dash_pattern``, ``[]`` for
    solid lines). Errors raised by ``set_prop_cycle`` are recorded.

Cases
-----
Named after the upstream tests. Adaptations: numpy arrays and iterators of
values are given as lists (``test_marker_cycle_kwargs_arrays_iterators``,
``test_valid_input_forms``); ``test_cycle_reset`` compares the properties
of the lines instead of ``plt.getp`` printouts; ``test_valid_marker_cycles``
also reads the markers back. Left out: ``set_prop_cycle('linewidth', {1,
2})`` of ``test_invalid_input_forms`` (a Python set).
"""

import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
from cycler import cycler

from spec_support import ProbeSet

probes = ProbeSet("mpl_cycles")

LONG_STYLES = {"-": "solid", "--": "dashed", "-.": "dashdot", ":": "dotted"}


def _style(value):
    return LONG_STYLES.get(value, "none" if value.lower() in {"none", " ", ""} else value)


GETTERS = {
    "get_color": lambda a: list(mcolors.to_rgba(a.get_color())),
    "get_facecolor": lambda a: list(a.get_facecolor()),
    "get_marker": lambda a: a.get_marker(),
    "get_linestyle": lambda a: _style(a.get_linestyle()),
    "get_linewidth": lambda a: a.get_linewidth(),
    "get_hatch": lambda a: a.get_hatch(),
    "get_dash_pattern": lambda a: _dash_pattern(*a._unscaled_dash_pattern),
}


def _dash_pattern(offset, dashes):
    return [offset, list(dashes) if dashes else []]


def _set_prop_cycle(cycle=None, args=(), kwargs=None, artist="plot", calls=(), getters=()):
    fig, ax = plt.subplots()
    try:
        if cycle is not None:
            ax.set_prop_cycle(eval(cycle, {"cycler": cycler}))
        else:
            ax.set_prop_cycle(*args, **(kwargs or {}))
        for call in calls:
            fmt = [call["fmt"]] if "fmt" in call else []
            getattr(ax, artist)(range(10), range(10), *fmt, **call.get("kw", {}))
        artists = ax.lines if artist == "plot" else ax.patches
        return {getter: [GETTERS[getter](a) for a in artists] for getter in getters}
    finally:
        plt.close(fig)


def set_prop_cycle(**args):
    probes.probe("axes.Axes.set_prop_cycle", _set_prop_cycle, **args)


FOUR = [{}] * 4


def test_colorcycle_basic():
    set_prop_cycle(cycle="cycler('color', ['r', 'g', 'y'])", calls=FOUR, getters=["get_color"])


def test_marker_cycle():
    set_prop_cycle(cycle="cycler('c', ['r', 'g', 'y']) + cycler('marker', ['.', '*', 'x'])",
                   calls=FOUR, getters=["get_color", "get_marker"])


def test_valid_marker_cycles():
    set_prop_cycle(cycle="cycler(marker=[1, '+', '.', 4])", calls=FOUR, getters=["get_marker"])


def test_marker_cycle_kwargs_arrays_iterators():
    set_prop_cycle(kwargs={"c": ["r", "g", "y"], "marker": [".", "*", "x"]}, calls=FOUR,
                   getters=["get_color", "get_marker"])


def test_linestylecycle_basic():
    set_prop_cycle(cycle="cycler('ls', ['-', '--', ':'])", calls=FOUR, getters=["get_linestyle"])


def test_fillcycle_basic():
    set_prop_cycle(cycle="cycler('c', ['r', 'g', 'y']) + cycler('hatch', ['xx', 'O', '|-'])"
                         " + cycler('linestyle', ['-', '--', ':'])",
                   artist="fill", calls=FOUR,
                   getters=["get_facecolor", "get_hatch", "get_linestyle"])


def test_fillcycle_ignore():
    set_prop_cycle(cycle="cycler('color', ['r', 'g', 'y']) + cycler('hatch', ['xx', 'O', '|-'])"
                         " + cycler('marker', ['.', '*', 'D'])",
                   artist="fill",
                   calls=[{"fmt": "r", "kw": {"hatch": "xx"}}, {"kw": {"hatch": "O"}}, {}, {}],
                   getters=["get_facecolor", "get_hatch"])


def test_property_collision_plot():
    set_prop_cycle(args=["linewidth", [2, 4]], calls=[{"kw": {"lw": 0.1}}] * 3 + [{}, {}],
                   getters=["get_linewidth"])


def test_property_collision_fill():
    set_prop_cycle(kwargs={"linewidth": [2, 3, 4, 5, 6], "facecolor": "bgcmy"}, artist="fill",
                   calls=[{"kw": {"lw": 0.1}}] * 3 + [{}, {}],
                   getters=["get_facecolor", "get_linewidth"])


def test_valid_input_forms():
    for args in ([None], ["color", "rgywkbcm"], ["lw", [1, 2]], ["linewidth", [1, 2]],
                 ["color", [[1, 0, 0], [0, 1, 0], [0, 0, 1]]],
                 ["dashes", [[], [13, 2], [8, 3, 1, 3]]]):
        set_prop_cycle(args=args, calls=[{}] * 3,
                       getters=["get_color", "get_linewidth", "get_dash_pattern"])
    set_prop_cycle(cycle="cycler('linewidth', [1, 2])", calls=[{}] * 3,
                   getters=["get_linewidth"])
    set_prop_cycle(kwargs={"lw": [1, 2], "color": ["k", "w"], "ls": ["-", "--"]},
                   calls=[{}] * 3, getters=["get_color", "get_linewidth", "get_linestyle"])


def test_cycle_reset():
    getters = ["get_color", "get_linewidth", "get_linestyle", "get_marker"]
    set_prop_cycle(args=[None], calls=[{"kw": {"label": "label"}}], getters=getters)
    set_prop_cycle(kwargs={"linewidth": [10, 9, 4]}, calls=[{"kw": {"label": "label"}}],
                   getters=getters)


def test_invalid_input_forms():
    for args in ([1], [[1, 2]], ["color", "fish"], ["linewidth", 1], ["foobar", [1, 2]]):
        set_prop_cycle(args=args)
    for kwargs in ({"linewidth": 1, "color": "r"}, {"foobar": [1, 2]}):
        set_prop_cycle(kwargs=kwargs)
    for cycle in ("cycler(foobar=[1, 2])", "cycler(color='rgb', c='cmy')"):
        set_prop_cycle(cycle=cycle)
