"""Picking: ``Artist.set_picker``, ``Figure.pick`` and ``PickEvent``.

Area ``pick``. Every op builds ``scene`` in ``Figure(figsize=(4, 3),
dpi=100)``, draws it once and processes one mouse event at each display
pixel of ``pixels`` (whole pixels from the bottom left): a press of button
1, or one scroll step up when ``scroll`` is true. Every artist has a
``gid``.

Ops
---
``pick.events``  {scene, pixels, scroll?} -> [[[gid, ind], ...], ...]
    per pixel, the pick events in the order they fire; ``ind`` is empty
    for artists picked whole.

Scenes
------
Unless noted, the axes are ``fig.add_subplot(xlim=(0, 4), ylim=(0, 3))``.

``lines``        a polyline (``picker=5``), markers without a line
                 (``picker=3``), a ``steps-mid`` line (``picker=4``) and a
                 dashed line given ``set_picker(True)`` afterwards (a pick
                 radius of 1 point)
``collections``  scatter points of five sizes (``picker=True``), a
                 ``LineCollection`` (``picker=2``) and a ``PolyCollection``
                 with ``pickradius=0`` (``picker=True``)
``patches``      a ``Rectangle`` (``picker=True``), a ``Circle``
                 (``picker=3``), an ``Annulus`` (``picker=True``) and an open,
                 unfilled ``Polygon`` (``picker=2``)
``texts``        a text in a round box, the title and an annotation with an
                 arrow (``picker=True``)
``layers``       the axes, an image (``aspect="auto"``), a line over it and a
                 figure text (``picker=True``, the line ``picker=5``)
``twoaxes``      two subplots side by side with unit limits, each with an
                 unclipped line reaching across the other (``picker=5``)
"""

from matplotlib.backend_bases import MouseEvent
from matplotlib.collections import LineCollection, PolyCollection
from matplotlib.figure import Figure
from matplotlib.patches import Annulus, Circle, Polygon, Rectangle

from spec_support import ProbeSet

probes = ProbeSet("pick")

XS = [0.5, 1.5, 2.5, 3.5]


def _lines(fig):
    ax = fig.add_subplot(xlim=(0, 4), ylim=(0, 3))
    ax.plot(XS, [0.5, 2.5, 0.5, 2.5], gid="zigzag", picker=5)
    ax.plot(XS, [1.5, 1.5, 1.5, 1.5], "o", gid="markers", picker=3)
    ax.plot(XS, [0.2, 1.0, 0.4, 2.8], drawstyle="steps-mid", gid="steps", picker=4)
    dashed, = ax.plot([0, 4], [2.8, 0.2], "--", gid="dashed")
    dashed.set_picker(True)


def _collections(fig):
    ax = fig.add_subplot(xlim=(0, 4), ylim=(0, 3))
    ax.scatter([0.5, 1.0, 1.5, 2.0, 3.5], [2.5, 2.4, 2.6, 2.5, 0.5], s=[20, 80, 200, 20, 400],
               gid="scatter", picker=True)
    segments = [[(0.2, 0.2), (1.8, 1.8)], [(0.2, 1.8), (1.8, 0.2)],
                [(2.2, 1.0), (3.8, 1.2), (3.0, 2.0)]]
    ax.add_collection(LineCollection(segments, gid="segments", picker=2))
    polygons = [[(2.2, 2.0), (3.0, 2.0), (2.6, 2.9)],
                [(0.2, 0.2), (0.8, 0.2), (0.8, 0.8), (0.2, 0.8)]]
    ax.add_collection(PolyCollection(polygons, pickradius=0, gid="polygons", picker=True))


def _patches(fig):
    ax = fig.add_subplot(xlim=(0, 4), ylim=(0, 3))
    ax.add_patch(Rectangle((0.3, 0.3), 1.0, 0.8, gid="rectangle", picker=True))
    ax.add_patch(Circle((2.5, 0.8), 0.5, gid="circle", picker=3))
    ax.add_patch(Annulus((1.0, 2.2), 0.6, 0.2, gid="annulus", picker=True))
    ax.add_patch(Polygon([(2.2, 1.8), (3.8, 2.0), (3.2, 2.8)], closed=False, fill=False,
                         gid="polygon", picker=2))


def _texts(fig):
    ax = fig.add_subplot(xlim=(0, 4), ylim=(0, 3))
    ax.text(0.5, 2.0, "boxed", bbox={"boxstyle": "round"}, gid="boxed", picker=True)
    ax.set_title("title", gid="title", picker=True)
    ax.annotate("note", xy=(3.0, 0.5), xytext=(2.0, 1.5), arrowprops={"arrowstyle": "->"},
                gid="note", picker=True)


def _layers(fig):
    ax = fig.add_subplot(xlim=(0, 4), ylim=(0, 3), gid="axes", picker=True)
    ax.imshow([[0.0, 1.0], [1.0, 0.0]], extent=(1, 3, 0.5, 2.5), aspect="auto", gid="image",
              picker=True)
    ax.plot([0, 4], [1.5, 1.5], gid="line", picker=5)
    fig.text(0.02, 0.02, "figure text", gid="figtext", picker=True)


def _twoaxes(fig):
    left = fig.add_subplot(1, 2, 1, xlim=(0, 1), ylim=(0, 1))
    right = fig.add_subplot(1, 2, 2, xlim=(0, 1), ylim=(0, 1))
    left.plot([0, 3], [0.5, 0.5], clip_on=False, gid="left", picker=5)
    right.plot([-2, 1], [0.3, 0.3], clip_on=False, gid="right", picker=5)


SCENES = {
    "lines": _lines,
    "collections": _collections,
    "patches": _patches,
    "texts": _texts,
    "layers": _layers,
    "twoaxes": _twoaxes,
}


def _events(scene, pixels, scroll=False):
    fig = Figure(figsize=(4, 3), dpi=100)
    SCENES[scene](fig)
    fig.draw_without_rendering()
    picks = []
    fig.canvas.mpl_connect(
        "pick_event",
        lambda event: picks[-1].append(
            [event.artist.get_gid(), [int(i) for i in getattr(event, "ind", [])]]))
    for x, y in pixels:
        picks.append([])
        if scroll:
            event = MouseEvent("scroll_event", fig.canvas, x, y, button="up", step=1)
        else:
            event = MouseEvent("button_press_event", fig.canvas, x, y, button=1)
        event._process()
    return picks


COLUMNS = range(4, 400, 7)
ROWS = range(5, 300, 9)


def test_presses():
    for scene in SCENES:
        for y in ROWS:
            probes.probe("pick.events", _events, scene=scene, pixels=[[x, y] for x in COLUMNS])


def test_scrolls():
    for scene in SCENES:
        probes.probe("pick.events", _events, scene=scene, pixels=[[x, 149] for x in COLUMNS],
                     scroll=True)
