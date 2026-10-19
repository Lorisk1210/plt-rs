"""Behaviour of colorbar ticks and scales, ported from matplotlib's own test
suite (``matplotlib/tests/test_colorbar.py``): automatic, fixed and minor
ticks, tick labels with a format, and the scale and ticks following a new
norm or new limits of the mappable.

Area ``colorbar_behaviour``. One op replays a scenario (see
``probes/_scenario.py`` for the step language).

Ops
---
``colorbar_behaviour.run``  {case, steps, figure?, rc?, style?} -> one entry
    per step: the converted result of query steps, ``null`` for the others.

Cases
-----
Every case is named after the upstream test it mirrors. The tests that set
``_internal.classic_mode`` to false run in the reference configuration,
where it is false already; random data comes from ``spec_support.rng``.

Left out: colorbars of bare ``ScalarMappable``s and ``Colorbar(ax, ...)``
built without a figure method, norms assigned as attributes
(``pcm.norm = ...``), the private ``_values`` and ``locator`` attributes,
and ``cbar.vmin``/``cbar.vmax``, which plt-rs does not expose.
"""

import numpy as np

from probes._scenario import recorder
from spec_support import ProbeSet, rng

probes = ProbeSet("colorbar_behaviour")
_case = recorder(probes)

SUBPLOT = ["fig", "subplots"]
LOG_NORM = {"new": "LogNorm"}


def _grid():
    x = np.arange(-3.0, 4.001)
    y = np.arange(-4.0, 3.001)
    X, Y = np.meshgrid(x, y)
    return X, Y, X * Y


def _bump():
    x, y = np.ogrid[-4:4:31j, -4:4:31j]
    return 120000 * np.exp(-x**2 - y**2)


def test_colorbar_ticks():
    X, Y, Z = _grid()
    clevs = [-12.0, -5.0, 0.0, 5.0, 12.0]
    _case("test_colorbar_ticks", [
        SUBPLOT, [0, "contourf", X.tolist(), Y.tolist(), Z.tolist(), clevs,
                  {"colors": ["r", "g", "b", "c"], "extend": "neither"}],
        ["fig", "colorbar", "@1", {"ax": "@ax0", "orientation": "horizontal", "ticks": clevs}],
        ["@2", "ax.xaxis.get_ticklocs"],
    ])


def test_colorbar_autoticks():
    X, Y, Z = _grid()
    for name, values, norm in [("test_colorbar_autoticks", Z[:-1, :-1], {}),
                               ("test_colorbar_autotickslog", 10 ** Z[:-1, :-1],
                                {"norm": LOG_NORM})]:
        mesh = ["pcolormesh", X.tolist(), Y.tolist(), values.tolist(), *([norm] if norm else [])]
        _case(name, [
            ["fig", "subplots", 2, 1], [0, *mesh], [1, *mesh],
            ["fig", "colorbar", "@1", {"ax": "@ax0", "extend": "both",
                                       "orientation": "vertical"}],
            ["fig", "colorbar", "@2", {"ax": "@ax1", "extend": "both",
                                       "orientation": "vertical", "shrink": 0.4}],
            ["fig", "draw"], ["@3", "ax.yaxis.get_ticklocs"], ["@4", "ax.yaxis.get_ticklocs"],
        ])


def test_colorbar_get_ticks():
    data = np.arange(1200).reshape(30, 40).tolist()
    levels = [0, 200, 400, 600, 800, 1000, 1200]
    _case("test_colorbar_get_ticks", [
        SUBPLOT, [0, "contourf", data, {"levels": levels}],
        ["fig", "colorbar", "@1", {"ticks": [0, 600, 1200]}], ["@2", "get_ticks"],
        ["@2", "set_ticks", [600, 700, 800]], ["@2", "get_ticks"],
        ["fig", "colorbar", "@1", {"orientation": "horizontal"}], ["@6", "get_ticks"],
    ])
    X, Y, Z = _grid()
    _case("test_colorbar_get_ticks/mesh", [
        SUBPLOT, [0, "pcolormesh", X.tolist(), Y.tolist(), Z[:-1, :-1].tolist()],
        ["fig", "colorbar", "@1", {"ax": "@ax0", "extend": "both", "orientation": "vertical"}],
        ["@2", "get_ticks"], ["@2", "get_ticks", {"minor": True}],
    ])
    _case("test_colorbar_get_ticks_2", [
        SUBPLOT, [0, "pcolormesh", [[0.05, 0.95]]], ["fig", "colorbar", "@1"],
        ["@2", "get_ticks"],
    ])


def test_colorbar_norms():
    z = _bump()
    _case("test_colorbar_renorm", [
        SUBPLOT, [0, "imshow", z.tolist()], ["fig", "colorbar", "@1"],
        ["@2", "ax.yaxis.get_majorticklocs"], ["@2", "set_ticks", [1, 2, 3]],
        ["@1", "set_norm", {"new": "LogNorm", "args": [float(z.min()), float(z.max())]}],
        ["@2", "ax.yaxis.get_majorticklocs"], ["@2", "set_ticks", [1, 2, 3]],
        ["@2", "ax.yaxis.get_majorticklocs"],
    ])
    _case("test_colorbar_scale_reset", [
        SUBPLOT, [0, "pcolormesh", z.tolist(), {"cmap": "RdBu_r"}],
        ["fig", "colorbar", "@1", {"ax": "@ax0"}], ["@2", "ax.yaxis.get_scale"],
        ["@1", "set_norm", {"new": "LogNorm", "args": [1, 100]}], ["@2", "ax.yaxis.get_scale"],
        ["@1", "set_norm", {"new": "Normalize", "args": [-20, 20]}], ["@2", "ax.yaxis.get_scale"],
    ])
    for fmt in ["%4.2e", "{x:.2e}"]:
        _case(f"test_colorbar_format/{fmt}", [
            SUBPLOT, [0, "imshow", z.tolist()], ["fig", "colorbar", "@1", {"format": fmt}],
            ["fig", "draw"], ["@2", "ax.yaxis.get_ticklabels"],
            ["@1", "set_clim", [4, 200]], ["fig", "draw"], ["@2", "ax.yaxis.get_ticklabels"],
            ["@1", "set_norm", {"new": "LogNorm", "args": [0.09999, 10]}], ["fig", "draw"],
            ["@2", "ax.yaxis.get_ticklabels"],
        ])
    _case("test_colorbar_log_minortick_labels", [
        SUBPLOT, [0, "imshow", [[10000, 50000]], {"norm": LOG_NORM}], ["fig", "colorbar", "@1"],
        ["fig", "draw"], ["@2", "ax.yaxis.get_ticklabels", {"which": "both"}],
    ])


def test_colorbar_minor_ticks():
    data = rng(12345).normal(400).reshape(20, 20).tolist()
    _case("test_colorbar_minorticks_on_off", [
        SUBPLOT, [0, "pcolormesh", data, {"vmin": -2.3, "vmax": 3.3}],
        ["fig", "colorbar", "@1", {"extend": "both"}],
        ["@2", "minorticks_on"], ["@2", "ax.yaxis.get_minorticklocs"],
        ["@2", "minorticks_off"], ["@2", "ax.yaxis.get_minorticklocs"],
        ["@1", "set_clim", {"vmin": -1.2, "vmax": 1.2}], ["@2", "minorticks_on"],
        ["@2", "ax.yaxis.get_minorticklocs"],
    ])
    data = rng(13257).uniform(400, 1, 10).reshape(20, 20).tolist()
    _case("test_colorbar_minorticks_on_off/log", [
        SUBPLOT, [0, "pcolormesh", data, {"norm": LOG_NORM}], ["fig", "colorbar", "@1"],
        ["fig", "draw"], ["@2", "ax.yaxis.get_minorticklocs"],
        ["@2", "minorticks_off"], ["@2", "ax.yaxis.get_minorticklocs"],
        ["@2", "minorticks_on"], ["@2", "ax.yaxis.get_minorticklocs"],
        ["@2", "minorticks_off"], ["@2", "set_ticks", [3, 5, 7, 9]],
        ["@2", "ax.yaxis.get_minorticklocs"],
    ])
    # Drawn first: before its first draw a colorbar axes still has its box
    # without the room for the extensions, which plt-rs never shows.
    _case("test_colorbar_inverted_ticks", [
        ["fig", "subplots", 2],
        [0, "pcolormesh", (10 ** np.arange(1, 5).reshape(2, 2)).tolist(), {"norm": LOG_NORM}],
        ["fig", "colorbar", "@1", {"ax": "@ax0", "extend": "both"}], ["fig", "draw"],
        ["@2", "get_ticks"], ["@2", "ax.invert_yaxis"], ["@2", "get_ticks"],
        [1, "pcolormesh", np.arange(1, 5).reshape(2, 2).tolist()],
        ["fig", "colorbar", "@7", {"ax": "@ax1", "extend": "both"}], ["fig", "draw"],
        ["@8", "minorticks_on"], ["@8", "get_ticks"], ["@8", "get_ticks", {"minor": True}],
        ["@8", "ax.invert_yaxis"], ["@8", "get_ticks"], ["@8", "get_ticks", {"minor": True}],
    ])
