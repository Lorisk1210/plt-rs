"""Projections the program registers, navigated by the toolbar: the
pan/zoom and zoom-to-rectangle tools on axes whose projection overrides
``start_pan``, ``drag_pan``, ``end_pan`` and ``_set_view_from_bbox``.

Area ``projections``. The axes are ``RangeFrameAxes`` (``range_frame``,
from ``figures/cases_projections.py``): the right button of the pan/zoom
tool changes their ``pad``, the left one pans as usual; the
zoom-to-rectangle tool zooms into a box along x alone, and out of it as
usual. Every op builds ``scene`` and feeds the mouse ``path`` as
``navigation.drag`` does (``probes/test_navigation.py``).

Ops
---
``projections.drag``  {scene, mode, button, key, path} -> ``[[xlim, ylim, pad], ...]``
    the view limits and pad of every axes of the figure afterwards, in
    ``fig.axes`` order. ``mode``, ``button`` and ``key`` as for
    ``navigation.drag``.

Scenes
------
``single``  ``add_subplot(projection="range_frame", pad=0.1)`` with
            ``plot([0, 10], [0, 5])``
``twinx``   ``single`` plus ``twinx(projection="range_frame", pad=0.2)``
            with ``plot([0, 10], [100, 0])``
``sharey``  ``subplots(1, 2, sharey=True,
            subplot_kw={"projection": "range_frame"})`` with
            ``plot([0, 10], [0, 5])`` and ``plot([0, 10], [5, 0])``
"""

from figures.cases_projections import RangeFrameAxes
from probes.test_navigation import drag, view_limits
from spec_support import ProbeSet

probes = ProbeSet("projections")


def _single(fig):
    ax = fig.add_subplot(projection=RangeFrameAxes.name, pad=0.1)
    ax.plot([0, 10], [0, 5])
    return ax


def _twinx(fig):
    twin = _single(fig).twinx(projection=RangeFrameAxes.name, pad=0.2)
    twin.plot([0, 10], [100, 0])


def _sharey(fig):
    left, right = fig.subplots(1, 2, sharey=True, subplot_kw={"projection": RangeFrameAxes.name})
    left.plot([0, 10], [0, 5])
    right.plot([0, 10], [5, 0])


SCENES = {
    "single": _single,
    "twinx": _twinx,
    "sharey": _sharey,
}


def _drag(scene, mode, button, key, path):
    fig = drag(SCENES[scene], mode, button, key, path)
    return [[*view_limits(ax), float(ax.pad)] for ax in fig.axes]


PANS = [
    (1, None, [[120, 150], [150, 160]]),
    (1, "x", [[120, 150], [150, 170]]),
    (3, None, [[120, 150], [170, 160]]),
    (3, "y", [[120, 150], [170, 160], [90, 140]]),
    (3, None, [[120, 150], [0, 150]]),
]
ZOOMS = [
    (1, None, [[100, 80], [150, 200]]),
    (1, None, [[150, 200], [100, 80]]),
    (1, "x", [[100, 80], [150, 200]]),
    (1, "y", [[100, 80], [150, 200]]),
    (3, None, [[100, 80], [150, 200]]),
]


def test_drags():
    for scene in SCENES:
        for mode, drags in [("pan", PANS), ("zoom", ZOOMS)]:
            for button, key, path in drags:
                probes.probe("projections.drag", _drag, scene=scene, mode=mode, button=button,
                             key=key, path=path)
