"""Behaviour of polar axes, ported from the non-image tests of matplotlib's
``lib/matplotlib/tests/test_polar.py``.

Area ``polar_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at
100 dpi) with an Agg canvas and ``ax = fig.add_subplot(projection="polar")``
unless noted.

Ops
---
``polar.limits``  {scene} -> [{"thetalim", "rlim"}, ...]
    The θ limits (radians) and the radial limits after each step of a
    scene:

    ``rlim_zero``  ``ax.plot(arange(10), arange(10) + .01)``: r starts at 0
        (``test_polar_rlim_zero``).
    ``no_data``  nothing plotted (``test_polar_no_data``).
    ``log``  ``ax.set_rscale("log")``: a positive rmin
        (``test_polar_default_log_lims``).
    ``spans``  ``ax.axvspan(0, 1)``, ``ax.axvline(.5)``, ``ax.plot([.1,
        .2])``: the θ spans leave r alone
        (``test_axvline_axvspan_do_not_modify_rlims``).
    ``radial``  the limits initially, after ``set_rticks([1, 2, 3, 4])``,
        ``plot([1, 2], [1, 2])``, ``set_rticks([-1, 0, 1, 2])`` and
        ``plot([1, 2], [-1, -2])`` (``test_radial_limits_behavior``).

``polar.thetalim``  {min, max, degrees} -> [thetamin, thetamax]
    ``ax.set_thetalim(min, max)`` (radians) or ``set_thetalim(thetamin=min,
    thetamax=max)`` (degrees): the limits in degrees, or an error for a
    range wider than a full circle (``test_thetalim_valid_invalid``,
    ``test_thetalim_args``).
``polar.ticks``  {scene} -> [[[loc, label], ...], ...]
    The major θ ticks drawn on each axes (locations in degrees):
    ``negative`` has ``set_thetalim(-pi, pi)``
    (``test_polar_neg_theta_lims``); ``mosaic`` is ``subplot_mosaic("AAAABB.",
    projection="polar")`` with ``set_thetalim(0, pi)`` on both
    (``test_default_thetalocator``).
``polar.tightbbox``  {scene} -> bbox
    ``fig.get_tightbbox()`` in inches of the polar axes as made
    (``default``), with ``set_theta_direction(-1)`` and
    ``set_theta_zero_location("N", offset)`` (``N20``, ``N30``) or with
    ``set_theta_offset(1.570796327)`` (``offset``)
    (``test_get_tightbbox_polar``, ``test_polar_outer_spine_not_collapsed``).
``polar.cursor``  {pixels} -> [str, ...]
    The status bar message at each display pixel: ``θ`` in turns of π and
    degrees with more digits the larger ``r`` is, and ``r``
    (``test_cursor_precision``). The pixels are those nearest to the
    ``(θ, r)`` of the upstream test, the innermost moved out to ``r = 0.015``:
    nearer the centre, a neighbouring pixel lies straight across it, and
    whether the θ resolution rounds to half a turn or just below is a
    matter of the last bit.

Left out: ``test_polar_twice`` (pyplot state), ``test_polar_gridlines``
and ``test_shared_polar_keeps_ticklabels`` (tick artist properties),
``test_polar_not_datalim_adjustable`` (plt-rs polar axes have no
adjustable setting to misuse), ``test_axvspan`` (a private interpolation
attribute), ``test_custom_fmt_data`` (Python formatter callables),
``test_radial_locator_wrapping`` and ``test_set_rticks_ticklabels_no_warning``
(locator classes and warnings).
"""

import numpy as np
from matplotlib.backend_bases import MouseEvent, NavigationToolbar2
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("polar_behaviour")


def _polar():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot(projection="polar")


def _limits_of(ax):
    return {"thetalim": [float(v) for v in ax.get_xlim()],
            "rlim": [float(v) for v in ax.get_ylim()]}


def _limits(scene):
    fig, ax = _polar()
    if scene == "rlim_zero":
        ax.plot(np.arange(10), np.arange(10) + .01)
    elif scene == "log":
        ax.set_rscale("log")
    elif scene == "spans":
        ax.axvspan(0, 1)
        ax.axvline(.5)
        ax.plot([.1, .2])
    elif scene == "radial":
        steps = [lambda: None, lambda: ax.set_rticks([1, 2, 3, 4]),
                 lambda: ax.plot([1, 2], [1, 2]), lambda: ax.set_rticks([-1, 0, 1, 2]),
                 lambda: ax.plot([1, 2], [-1, -2])]
        observed = []
        for step in steps:
            step()
            observed.append(_limits_of(ax))
        return observed
    return [_limits_of(ax)]


def _thetalim(min, max, degrees):
    fig, ax = _polar()
    if degrees:
        ax.set_thetalim(thetamin=min, thetamax=max)
    else:
        ax.set_thetalim(min, max)
    return [float(ax.get_thetamin()), float(ax.get_thetamax())]


def _ticks(scene):
    fig = Figure()
    FigureCanvasAgg(fig)
    if scene == "negative":
        axes = [fig.add_subplot(projection="polar")]
        axes[0].set_thetalim(-np.pi, np.pi)
    else:
        axes = list(fig.subplot_mosaic("AAAABB.", subplot_kw={"projection": "polar"}).values())
        for ax in axes:
            ax.set_thetalim(0, np.pi)
    fig.canvas.draw()
    return [[[float(np.degrees(t.get_loc())), t.label1.get_text()]
             for t in ax.xaxis._update_ticks() if t in ax.xaxis.get_major_ticks()]
            for ax in axes]


def _tightbbox(scene):
    fig, ax = _polar()
    if scene in ("N20", "N30"):
        ax.set_theta_direction(-1)
        ax.set_theta_zero_location("N", int(scene[1:]))
    elif scene == "offset":
        ax.set_theta_offset(1.570796327)
    fig.canvas.draw()
    return fig.get_tightbbox(fig.canvas.get_renderer()).get_points().tolist()


def _cursor(pixels):
    fig, ax = _polar()
    fig.canvas.draw()
    return [NavigationToolbar2._mouse_event_to_message(
        MouseEvent("motion_notify_event", fig.canvas, x, y)) for x, y in pixels]


def test_limits():
    for scene in ["rlim_zero", "no_data", "log", "spans", "radial"]:
        probes.probe("polar.limits", _limits, scene=scene)


def test_thetalims():
    for low, high, degrees in [(0, 2 * np.pi, False), (800, 440, True), (0, 3 * np.pi, False),
                               (800, 400, True), (0, 1, False), (2, 3, False),
                               (-np.pi, np.pi, False), (-90, 270, True), (45, 405.5, True)]:
        probes.probe("polar.thetalim", _thetalim, min=low, max=high, degrees=degrees)


def test_ticks():
    for scene in ["negative", "mosaic"]:
        probes.probe("polar.ticks", _ticks, scene=scene)


def test_tightbbox():
    for scene in ["default", "N20", "N30", "offset"]:
        probes.probe("polar.tightbbox", _tightbbox, scene=scene)


def test_cursor():
    fig, ax = _polar()
    fig.canvas.draw()
    points = [(t, r) for t in [0, 1, 2] for r in [0.015, 0.1, 0.95, 1]] + [(3, 0.5), (5.5, 0.75)]
    pixels = np.round(ax.transData.transform(points)).astype(int).tolist()
    probes.probe("polar.cursor", _cursor, pixels=pixels)
