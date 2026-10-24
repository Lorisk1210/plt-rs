"""matplotlib's own non-image tests of ``Line2D``
(``matplotlib/tests/test_lines.py``) that the ``line_behaviour`` and
``markers`` areas leave out.

Area ``mpl_lines``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``) on ``plt.subplots()``.

Ops
---
``AxLine.set``  {through, steps} -> what ``test_axline_setters`` reads
    from ``ax.axline((.1, .1), slope=0.6)`` (``through`` is ``"slope"``) or
    ``ax.axline((.1, .1), (.8, .4))`` (``"point"``) after each step of
    ``steps``: ``["xy1", [x, y]]``, ``["xy2", [x, y]]`` or ``["slope", s]``
    calls the setter and records ``{xy1, xy2, slope}`` as the getters
    return them (``null`` for the end that is not set), or the error of
    setting a second point on a line given a slope and the other way round.
``Line2D.get_markerfacecolor``  {} -> ``[fillstyle, facecolor]`` of
    ``plot([1, 3, 2], marker=MarkerStyle("o", fillstyle="none"),
    markerfacecolor="red")``: the face colour (as RGBA) does not override
    the empty fill style (``test_markerfacecolor_fillstyle``).
``lines._mark_every_path``  {markevery, x} -> the x of the vertices a float
    ``markevery`` (a step, or ``[start, step]``) picks on the path of the
    points ``(x, 0)`` scaled by the axes diagonal, so that a unit is one
    diagonal (``test_markevery_float_vertex_selection``,
    ``test_markevery_float_ties_and_repeated_vertices``: on ties the first
    vertex wins), or the error for a step that is not positive
    (``test_markevery_float_nonpositive_spacing``).

``test_linestyle_variants``, which only checks that lines in every named
style and with an empty dash pattern draw, is a case of
``mpl_tests_lines``.

Left out, besides the tests the ``line_behaviour`` area lists:
``test_input_copy`` (that plotting copies the caller's arrays: a plt-rs
line owns its data) and the deprecated one-argument-per-coordinate forms
of the axline setters.
"""

import matplotlib.colors as mcolors
import matplotlib.lines as mlines
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.markers import MarkerStyle
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_lines")
probe = prober(probes)


def _axline_state(line):
    return {"xy1": list(line.get_xy1()),
            "xy2": None if line.get_xy2() is None else list(line.get_xy2()),
            "slope": line.get_slope()}


def _axline_set(through, steps):
    fig, ax = plt.subplots()
    if through == "slope":
        line = ax.axline((.1, .1), slope=0.6)
    else:
        line = ax.axline((.1, .1), (.8, .4))
    out = [_axline_state(line)]
    for name, value in steps:
        getattr(line, f"set_{name}")(tuple(value) if isinstance(value, list) else value)
        out.append(_axline_state(line))
    return out


def test_axline_setters():
    res = probe("AxLine.set", _axline_set, through="slope",
                steps=[["xy1", [.2, .3]], ["slope", 2.4]])
    assert res[-1] == {"xy1": [.2, .3], "xy2": None, "slope": 2.4}
    res = probe("AxLine.set", _axline_set, through="point",
                steps=[["xy1", [.3, .2]], ["xy2", [.6, .8]]])
    assert res[-1] == {"xy1": [.3, .2], "xy2": [.6, .8], "slope": None}
    assert isinstance(probe("AxLine.set", _axline_set, through="slope",
                            steps=[["xy2", [.2, .3]]]), ValueError)
    assert isinstance(probe("AxLine.set", _axline_set, through="point",
                            steps=[["slope", 3]]), ValueError)


def _markerfacecolor():
    line, = plt.plot([1, 3, 2], marker=MarkerStyle("o", fillstyle="none"),
                     markerfacecolor="red")
    return [line.get_fillstyle(), list(mcolors.to_rgba(line.get_markerfacecolor()))]


def test_markerfacecolor_fillstyle():
    assert probe("Line2D.get_markerfacecolor", _markerfacecolor) == ["none", [0, 0, 0, 0]]


def _mark_every_path(markevery, x):
    fig, ax = plt.subplots()
    (x0, y0), (x1, y1) = ax.transAxes.transform([[0, 0], [1, 1]])
    scale = np.hypot(x1 - x0, y1 - y0)
    path = Path(np.column_stack([x, np.zeros(len(x))]))
    every = tuple(markevery) if isinstance(markevery, list) else markevery
    picked = mlines._mark_every_path(every, path, Affine2D().scale(scale), ax)
    return [float(v) for v in picked.vertices[:, 0]]


#: The path of ``test_markevery_float_vertex_selection``.
VERTICES = [0, 0.2, 0.8, 1.6, 2.7, 4.1, 5.8, 7.8, 10]


def test_markevery_float_vertex_selection():
    for markevery, expected in [(1.5, [0, 3, 4, 5, 6, 7, 8]), ([-0.2, 1.5], [0, 3, 4, 5, 6, 7]),
                                ([11.0, 1.5], [])]:
        res = probe("lines._mark_every_path", _mark_every_path, markevery=markevery, x=VERTICES)
        assert res == [VERTICES[i] for i in expected]


def test_markevery_float_ties_and_repeated_vertices():
    assert probe("lines._mark_every_path", _mark_every_path, markevery=1.0,
                 x=[0, 0.5, 1.5]) == [0, 0.5]
    assert probe("lines._mark_every_path", _mark_every_path, markevery=1.0,
                 x=[0, 0, 2]) == [0]


def test_markevery_float_nonpositive_spacing():
    for markevery in [0.0, -0.1]:
        res = probe("lines._mark_every_path", _mark_every_path, markevery=markevery, x=[0, 1])
        assert isinstance(res, ValueError)
