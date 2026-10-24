"""Behaviour of contour sets, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_contour.py`` that the
``contour`` area (levels and geometry) does not already cover.

Area ``contour_behaviour``. Every scene draws on ``Figure()`` (6.4 x 4.8 in
at 100 dpi) with an Agg canvas and ``ax = fig.add_subplot()``.

Ops
---
``contour.style``  {scene} -> {"levels", "colors", "linewidths",
    "dashed"}
    The levels, the colour of each level (``to_rgba_array``), the line
    widths and, per level, whether it is drawn dashed (its dash pattern is
    not solid), of a line contour set:

    ``color:<i>:<extend>``  ``ax.contour([[0, 1], [1, 2]], levels=[0.5,
        0.75, 1, 1.25, 1.5], colors=COLORS[i], extend=extend)``
        (``test_single_color_and_extend``).
    ``linewidth:<case>``  ``ax.contour(arange(12).reshape(4, 3),
        linewidths=...)`` under ``lines.linewidth = 1.23`` and
        ``contour.linewidth`` (``test_contour_linewidth``).
    ``styles:<linestyles>:<negative>:<rc>``  ``ax.contour(X, Y, Z, 6,
        colors="k", linestyles=..., negative_linestyles=...)`` under
        ``contour.negative_linestyle = rc`` for the two-Gaussian field
        ``Z`` on ``arange(-3, 3, 0.25)`` x ``arange(-2, 2, 0.25)``
        (``test_linestyles``, ``test_negative_linestyles``; ``-`` means
        unset).
    ``all_nan``  ``ax.contour([[nan, nan], [nan, nan]])`` (``test_all_nan``).

``contour.labels``  {scene} -> [{"text", "zorder"}, ...]
    The labels ``clabel`` makes:

    ``powerlimits``  ``ax.contour(geomspace(1e-6, 1e-4, 100).reshape(10,
        10), levels=[.25e-5, 1e-5, 4e-5])`` labelled with the default
        format (``test_contour_autolabel_beyond_powerlimits``).
    ``zorder:<clabel zorder>``  ``ax.contour(x, y, max(|x|, |y|),
        zorder=123)`` labelled with ``zorder`` given or not
        (``test_clabel_zorder``).
    ``manual``  ``ax.contour(X, Y, X**2 / Y**2 - 1, levels=[0, 100])`` on
        ``linspace(-10, 10)`` labelled by hand at ``(1.3, 1)``: the nearest
        line, not the one the moveto passes (``test_contour_manual_moveto``).
    ``manual_subset``  ``ax.contour([[1, 2], [3, 4]], levels=[1.5, 2.5,
        3.5])`` labelled at ``(0.5, 0.5)`` for level 2.5 only
        (``test_clabel_manual_subset``).

``contour.legend_elements``  {filled} -> {"labels", "handles"}
    ``ax.contour(f)`` / ``contourf`` of ``h = x * x.T`` (``x =
    arange(1, 10)``) with ``levels=[10, 30, 50]``, three colours and
    ``extend="both"``, then ``legend_elements()``: the labels and the
    number of handles (``test_contour_legend_elements``,
    ``test_contourf_legend_elements``; the handles are opaque in plt-rs).

Left out: ``test_contour_Nlevels``, ``test_contourf_decreasing_levels``,
``test_contourf_symmetric_locator`` (covered by the ``contour`` area), and
the tests the ``mpl_contour`` area ports or lists: shape errors,
``test_lognorm_levels``, ``test_bool_autolevel``, ``test_allsegs_allkinds``,
``test_find_nearest_contour*``, ``test_label_contour_start``,
``test_algorithm_*``, ``test_contour_remove*``, ``test_contour_clip_path``,
``test_negative_linestyles`` (its style attribute), the smoke tests and the
Python-only ones.
"""

import matplotlib as mpl
import matplotlib.colors as mcolors
import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("contour_behaviour")

COLORS = ["darkred", ("r", 0.5), (0.1, 0.2, 0.5, 0.3)]
LINEWIDTHS = {"rc_lines": (None, None), "rc_contour": (4.24, None),
              "explicit": (4.24, 5.02)}


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot()


def _two_gaussians():
    x = np.arange(-3.0, 3.0, 0.25)
    y = np.arange(-2.0, 2.0, 0.25)
    X, Y = np.meshgrid(x, y)
    Z = (np.exp(-X ** 2 - Y ** 2) - np.exp(-(X - 1) ** 2 - (Y - 1) ** 2)) * 2
    return X, Y, Z


def _style_set(scene):
    kind, *params = scene.split(":")
    if kind == "color":
        fig, ax = _axes()
        return ax.contour([[0, 1], [1, 2]], levels=[0.5, 0.75, 1, 1.25, 1.5],
                          colors=[COLORS[int(params[0])]], extend=params[1])
    if kind == "linewidth":
        contour_lw, call_lw = LINEWIDTHS[params[0]]
        with mpl.rc_context({"lines.linewidth": 1.23, "contour.linewidth": contour_lw}):
            fig, ax = _axes()
            return ax.contour(np.arange(12).reshape(4, 3), linewidths=call_lw)
    if kind == "styles":
        linestyles, negative, rc = (None if p == "-" else p for p in params)
        with mpl.rc_context({} if rc is None else {"contour.negative_linestyle": rc}):
            fig, ax = _axes()
            kw = {k: v for k, v in [("linestyles", linestyles),
                                    ("negative_linestyles", negative)] if v is not None}
            return ax.contour(*_two_gaussians(), 6, colors="k", **kw)
    fig, ax = _axes()
    return ax.contour(np.full((2, 2), np.nan))


def _style(scene):
    cs = _style_set(scene)
    return {"levels": cs.levels.tolist(),
            "colors": mcolors.to_rgba_array(cs.get_edgecolor()).tolist(),
            "linewidths": [float(w) for w in cs.get_linewidths()],
            "dashed": [dashes is not None for _, dashes in cs.get_linestyles()]}


def _labels(scene):
    fig, ax = _axes()
    kind, *params = scene.split(":")
    if kind == "powerlimits":
        cs = ax.contour(np.geomspace(1e-6, 1e-4, 100).reshape(10, 10),
                        levels=[.25e-5, 1e-5, 4e-5])
        labels = cs.clabel()
    elif kind == "zorder":
        x, y = np.meshgrid(np.arange(0, 10), np.arange(0, 10))
        cs = ax.contour(x, y, np.max(np.dstack([abs(x), abs(y)]), 2), zorder=123)
        labels = cs.clabel(zorder=None if params[0] == "-" else float(params[0]))
    elif kind == "manual":
        X, Y = np.meshgrid(np.linspace(-10, 10), np.linspace(-10, 10))
        cs = ax.contour(X, Y, X ** 2 * 1 / Y ** 2 - 1, levels=[0, 100])
        labels = cs.clabel(manual=[(1.3, 1)])
    else:
        cs = ax.contour([[1, 2], [3, 4]], levels=[1.5, 2.5, 3.5])
        labels = cs.clabel(levels=[2.5], manual=[(0.5, 0.5)])
    return [{"text": label.get_text(), "zorder": float(label.get_zorder())}
            for label in labels]


def _legend_elements(filled):
    fig, ax = _axes()
    x = np.arange(1, 10)
    h = x * x.reshape(-1, 1)
    if filled:
        cs = ax.contourf(h, levels=[10, 30, 50], colors=["#FFFF00", "#FF00FF", "#00FFFF"],
                         extend="both")
        cs.cmap = cs.cmap.with_extremes(over="red", under="blue")
        cs.changed()
    else:
        cs = ax.contour(h, levels=[10, 30, 50], colors=["blue", "#00FF00", "red"],
                        extend="both")
    handles, labels = cs.legend_elements()
    return {"labels": labels, "handles": len(handles)}


STYLES = [
    *[f"color:{i}:{extend}" for i in range(3) for extend in ["neither", "both"]],
    *[f"linewidth:{case}" for case in LINEWIDTHS],
    "styles:-:-:-",
    *[f"styles:{style}:-:-" for style in ["solid", "dashed", "dashdot", "dotted"]],
    *[f"styles:{style}:dashdot:-" for style in ["solid", "dotted"]],
    *[f"styles:-:-:{style}" for style in ["solid", "dashed", "dashdot", "dotted"]],
    *[f"styles:-:{style}:-" for style in ["solid", "dotted"]],
    *[f"styles:dashdot:{style}:-" for style in ["solid", "dotted"]],
    "all_nan",
]


def test_styles():
    for scene in STYLES:
        probes.probe("contour.style", _style, scene=scene)


def test_labels():
    for scene in ["powerlimits", "zorder:1234", "zorder:-", "manual", "manual_subset"]:
        probes.probe("contour.labels", _labels, scene=scene)


def test_legend_elements():
    for filled in [False, True]:
        probes.probe("contour.legend_elements", _legend_elements, filled=filled)
