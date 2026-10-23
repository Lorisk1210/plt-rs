"""Interactive navigation: the toolbar's zoom-to-rectangle and pan modes
(``NavigationToolbar2.press_zoom``/``drag_zoom``/``release_zoom``,
``press_pan``/``drag_pan``/``release_pan`` and the axes' ``_set_view_from_bbox``,
``start_pan``, ``drag_pan``).

Area ``navigation``. Every op builds ``scene`` in ``Figure(figsize=(4, 3),
dpi=100)``, draws it once, turns on ``mode`` and feeds the mouse ``path``
(display pixels from the bottom left): a press at the first point, a motion
with the button held to each later point, and a release at the last.

Ops
---
``navigation.drag``  {scene, mode, button, key, path} -> ``[[xlim, ylim], ...]``
    the view limits of every axes of the figure afterwards, in ``fig.axes``
    order. ``mode`` is ``"zoom"`` or ``"pan"``, ``button`` 1 (left) or 3
    (right), ``key`` the key held (``null``, ``"x"``, ``"y"``, ``"control"``
    or ``"shift"``).
``navigation.format_coord``  {scene, points} -> ``[[coord, x, y], ...]``
    for each data point ``(x, y)`` of the first axes, what the status bar
    shows of it (``ax.format_coord(x, y)``) and of its coordinates
    (``ax.format_xdata(x)``, ``ax.format_ydata(y)``), after one draw.

Scenes
------
``linear``         ``plot([0, 10], [0, 5])``
``logx``           ``semilogx([1, 1000], [0, 5])``
``symlogy``        ``plot([0, 10], [-100, 100])`` on a symlog y axis
``logity``         ``plot([0, 10], [0.01, 0.99])`` on a logit y axis
``equal``          ``linear`` with ``set_aspect("equal")``
``equal_datalim``  ``linear`` with ``set_aspect("equal", adjustable="datalim")``
``inverted``       ``linear`` with the x axis inverted
``twinx``          ``linear`` plus ``twinx().plot([0, 10], [100, 0])``
``sharex``         two rows sharing x, ``plot([0, 10], [0, 5])`` and
                   ``plot([0, 10], [5, 0])``
"""

from matplotlib.backend_bases import MouseButton, MouseEvent, NavigationToolbar2
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("navigation")


def _linear(fig):
    fig.add_subplot().plot([0, 10], [0, 5])


def _logx(fig):
    fig.add_subplot().semilogx([1, 1000], [0, 5])


def _symlogy(fig):
    ax = fig.add_subplot()
    ax.plot([0, 10], [-100, 100])
    ax.set_yscale("symlog")


def _logity(fig):
    ax = fig.add_subplot()
    ax.plot([0, 10], [0.01, 0.99])
    ax.set_yscale("logit")


def _equal(fig, adjustable="box"):
    ax = fig.add_subplot()
    ax.plot([0, 10], [0, 5])
    ax.set_aspect("equal", adjustable=adjustable)


def _inverted(fig):
    ax = fig.add_subplot()
    ax.plot([0, 10], [0, 5])
    ax.invert_xaxis()


def _twinx(fig):
    ax = fig.add_subplot()
    ax.plot([0, 10], [0, 5])
    ax.twinx().plot([0, 10], [100, 0])


def _sharex(fig):
    top, bottom = fig.subplots(2, 1, sharex=True)
    top.plot([0, 10], [0, 5])
    bottom.plot([0, 10], [5, 0])


SCENES = {
    "linear": _linear,
    "logx": _logx,
    "symlogy": _symlogy,
    "logity": _logity,
    "equal": _equal,
    "equal_datalim": lambda fig: _equal(fig, "datalim"),
    "inverted": _inverted,
    "twinx": _twinx,
    "sharex": _sharex,
}


def _drag(scene, mode, button, key, path):
    fig = Figure(figsize=(4, 3), dpi=100)
    canvas = FigureCanvasAgg(fig)
    SCENES[scene](fig)
    canvas.draw()
    toolbar = NavigationToolbar2(canvas)
    getattr(toolbar, mode)()
    held = {MouseButton(button)}
    (x0, y0), *rest = path
    MouseEvent("button_press_event", canvas, x0, y0, button=button, key=key)._process()
    for x, y in rest:
        MouseEvent("motion_notify_event", canvas, x, y, key=key, buttons=held)._process()
    x1, y1 = path[-1]
    MouseEvent("button_release_event", canvas, x1, y1, button=button, key=key)._process()
    return [[[float(v) for v in ax.get_xlim()], [float(v) for v in ax.get_ylim()]]
            for ax in fig.axes]


ZOOMS = [
    (1, None, [[100, 80], [250, 200]]),
    (1, None, [[250, 200], [100, 80]]),
    (1, "x", [[100, 80], [250, 200]]),
    (1, "y", [[100, 80], [250, 200]]),
    (1, None, [[100, 80], [103, 82]]),
    (1, None, [[100, 80], [390, 290]]),
    (3, None, [[100, 80], [250, 200]]),
    (3, "x", [[120, 100], [200, 220]]),
]
PANS = [
    (1, None, [[200, 150], [230, 160]]),
    (1, None, [[200, 150], [230, 160], [260, 120]]),
    (1, "x", [[200, 150], [230, 170]]),
    (1, "y", [[200, 150], [230, 170]]),
    (1, "control", [[200, 150], [230, 170]]),
    (1, "shift", [[200, 150], [240, 155]]),
    (3, None, [[200, 150], [230, 170]]),
    (3, "x", [[200, 150], [170, 120]]),
    (3, "control", [[200, 150], [230, 170]]),
]


def _format_coord(scene, points):
    fig = Figure(figsize=(4, 3), dpi=100)
    canvas = FigureCanvasAgg(fig)
    SCENES[scene](fig)
    canvas.draw()
    ax = fig.axes[0]
    return [[ax.format_coord(x, y), ax.format_xdata(x), ax.format_ydata(y)] for x, y in points]


def test_format_coord():
    points = [[2.5, 1.25], [7.0, 0.5], [1.0, 4.0]]
    for scene in ["linear", "logx", "symlogy", "inverted", "twinx"]:
        probes.probe("navigation.format_coord", _format_coord, scene=scene, points=points)


def test_drags():
    for scene in SCENES:
        for mode, drags in [("zoom", ZOOMS), ("pan", PANS)]:
            for button, key, path in drags:
                probes.probe("navigation.drag", _drag, scene=scene, mode=mode, button=button,
                             key=key, path=path)
