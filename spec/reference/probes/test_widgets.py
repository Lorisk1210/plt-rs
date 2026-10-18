"""Interactive widgets driven by mouse and key events
(``matplotlib.widgets``: ``Slider``, ``TextBox``, ``SpanSelector`` and
``RectangleSelector``).

Area ``widgets``. Every op builds one widget in ``Figure(figsize=(4, 3),
dpi=100)``, draws the figure once and replays ``events`` (display pixels
from the bottom left):

* ``{"type": "press" | "release", "x", "y", "button"}``
* ``{"type": "motion", "x", "y"}`` (with the pressed buttons held)
* ``{"type": "key_press" | "key_release", "key"}`` (the pointer stays where
  it was; mouse events carry the held key)

Ops
---
``widgets.Slider``  {orientation, valinit, valstep, closedmin, closedmax,
    dragging, events} -> ``{"val": v, "changes": [values given to
    on_changed]}``. The slider spans 0 to 10 in an axes at ``[0.2, 0.4, 0.6,
    0.1]`` (horizontal) or ``[0.45, 0.1, 0.1, 0.8]`` (vertical);
    ``valstep`` is null, a number or a list.
``widgets.TextBox``  {initial, events} -> ``{"text", "changes": [...],
    "submits": [...]}``, the box in an axes at ``[0.2, 0.4, 0.6, 0.15]``.
``widgets.SpanSelector``  {direction, options, events} -> ``{"extents":
    [vmin, vmax], "selects": [[vmin, vmax], ...]}`` on axes from
    ``add_subplot()`` with both limits 0 to 10; ``options`` holds
    ``minspan``, ``interactive``, ``drag_from_anywhere``,
    ``ignore_event_outside`` and ``snap_values`` when given.
``widgets.RectangleSelector``  {options, events} -> ``{"extents": [x0, x1,
    y0, y1], "selects": [[x0, y0, x1, y1], ...]}`` (the press and release
    points given to ``onselect``) on the same axes; ``options`` holds
    ``minspanx``, ``minspany``, ``interactive``, ``drag_from_anywhere`` and
    ``ignore_event_outside`` when given.
"""

from matplotlib.backend_bases import KeyEvent, MouseButton, MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.widgets import RectangleSelector, Slider, SpanSelector, TextBox

from spec_support import ProbeSet

probes = ProbeSet("widgets")


def _canvas():
    fig = Figure(figsize=(4, 3), dpi=100)
    return fig, FigureCanvasAgg(fig)


def _replay(canvas, events):
    held, key, at = set(), None, (0.0, 0.0)
    for event in events:
        kind = event["type"]
        if kind in ("press", "release", "motion"):
            at = (event["x"], event["y"])
        if kind == "press":
            held.add(MouseButton(event["button"]))
            MouseEvent("button_press_event", canvas, *at, button=event["button"],
                       key=key)._process()
        elif kind == "release":
            held.discard(MouseButton(event["button"]))
            MouseEvent("button_release_event", canvas, *at, button=event["button"],
                       key=key)._process()
        elif kind == "motion":
            MouseEvent("motion_notify_event", canvas, *at, key=key, buttons=set(held))._process()
        elif kind == "key_press":
            key = event["key"]
            KeyEvent("key_press_event", canvas, key, *at)._process()
        elif kind == "key_release":
            KeyEvent("key_release_event", canvas, event["key"], *at)._process()
            key = None


def _slider(orientation, valinit, valstep, closedmin, closedmax, dragging, events):
    fig, canvas = _canvas()
    rect = [0.2, 0.4, 0.6, 0.1] if orientation == "horizontal" else [0.45, 0.1, 0.1, 0.8]
    slider = Slider(fig.add_axes(rect), "s", 0.0, 10.0, valinit=valinit, valstep=valstep,
                    closedmin=closedmin, closedmax=closedmax, dragging=dragging,
                    orientation=orientation)
    changes = []
    slider.on_changed(lambda v: changes.append(float(v)))
    canvas.draw()
    _replay(canvas, events)
    return {"val": float(slider.val), "changes": changes}


def _textbox(initial, events):
    fig, canvas = _canvas()
    box = TextBox(fig.add_axes([0.2, 0.4, 0.6, 0.15]), "t", initial=initial)
    changes, submits = [], []
    box.on_text_change(changes.append)
    box.on_submit(submits.append)
    canvas.draw()
    _replay(canvas, events)
    return {"text": box.text, "changes": changes, "submits": submits}


def _axes():
    fig, canvas = _canvas()
    ax = fig.add_subplot()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    return ax, canvas


def _span(direction, options, events):
    ax, canvas = _axes()
    selects = []
    span = SpanSelector(ax, lambda lo, hi: selects.append([float(lo), float(hi)]), direction,
                        **options)
    canvas.draw()
    _replay(canvas, events)
    return {"extents": [float(v) for v in span.extents], "selects": selects}


def _rectangle(options, events):
    ax, canvas = _axes()
    selects = []

    def onselect(press, release):
        selects.append([float(press.xdata), float(press.ydata), float(release.xdata),
                        float(release.ydata)])

    selector = RectangleSelector(ax, onselect, **options)
    canvas.draw()
    _replay(canvas, events)
    return {"extents": [float(v) for v in selector.extents], "selects": selects}


def press(x, y, button=1):
    return {"type": "press", "x": x, "y": y, "button": button}


def release(x, y, button=1):
    return {"type": "release", "x": x, "y": y, "button": button}


def motion(x, y):
    return {"type": "motion", "x": x, "y": y}


def key(name, up=False):
    return {"type": "key_release" if up else "key_press", "key": name}


def click(x, y, button=1):
    return [press(x, y, button), release(x, y, button)]


def drag(points, button=1):
    (x0, y0), *rest = points
    return [press(x0, y0, button), *[motion(x, y) for x, y in rest],
            release(*points[-1], button)]


SLIDES = {
    "horizontal": [click(140, 135), click(300, 135), click(81, 135), click(330, 135),
                   drag([(100, 135), (200, 140), (250, 160), (400, 140)]),
                   click(200, 50), click(200, 135, 3)],
    "vertical": [click(150, 100), click(150, 270), drag([(150, 60), (150, 200), (150, 10)])],
}


def test_sliders():
    for orientation, interactions in SLIDES.items():
        for events in interactions:
            for valstep in [None, 1.5, [0.0, 2.0, 2.5, 7.0, 10.0]]:
                probes.probe("widgets.Slider", _slider, orientation=orientation, valinit=5.0,
                             valstep=valstep, closedmin=True, closedmax=True, dragging=True,
                             events=events)
        for closed in [(False, True), (True, False), (False, False)]:
            probes.probe("widgets.Slider", _slider, orientation=orientation, valinit=5.0,
                         valstep=None, closedmin=closed[0], closedmax=closed[1], dragging=True,
                         events=interactions[-1] if orientation == "vertical" else
                         drag([(100, 135), (60, 135), (350, 135)]))
        probes.probe("widgets.Slider", _slider, orientation=orientation, valinit=2.0,
                     valstep=None, closedmin=True, closedmax=True, dragging=False,
                     events=interactions[-1] if orientation == "vertical" else
                     drag([(100, 135), (200, 140), (250, 135)]))


TYPING = [
    [*click(200, 140), key("a"), key("b"), key("c"), key("enter")],
    [*click(200, 140), key("backspace"), key("backspace"), key("x"), key("escape")],
    [*click(200, 140), key("left"), key("left"), key("Z"), key("right"), key("delete"),
     key("enter")],
    [*click(200, 140), key("home"), key("q"), key("end"), key("r"), key("enter")],
    [key("a"), *click(200, 140), key("b"), *click(20, 20), key("c")],
    [*click(200, 140), key("shift"), key("A"), key("shift", up=True), key(" "), key("enter"),
     key("x")],
]


def test_textboxes():
    for initial in ["", "hello"]:
        for events in TYPING:
            probes.probe("widgets.TextBox", _textbox, initial=initial, events=events)


SPANS = [
    drag([(80, 150), (150, 160), (200, 150)]),
    drag([(200, 150), (120, 140)]),
    drag([(100, 150), (102, 150)]),
    [*drag([(100, 150), (200, 150)]), *drag([(200, 150), (250, 150)])],
    [*drag([(100, 150), (200, 150)]), *drag([(150, 150), (180, 150)])],
    [*drag([(100, 150), (200, 150)]), *drag([(100, 150), (60, 150)])],
    [*drag([(100, 150), (200, 150)]), *click(300, 150)],
    drag([(100, 150), (500, 150)]),
    drag([(100, 150), (200, 150)], button=3),
]
SPAN_OPTIONS = [
    {},
    {"minspan": 1.0},
    {"interactive": True},
    {"interactive": True, "drag_from_anywhere": True},
    {"interactive": True, "ignore_event_outside": True},
    {"snap_values": [0.0, 1.0, 2.5, 4.0, 5.0, 7.5, 10.0]},
]


def test_spans():
    for direction in ["horizontal", "vertical"]:
        for options in SPAN_OPTIONS:
            for events in SPANS:
                if direction == "vertical":
                    events = [{**e, "x": e["y"], "y": e["x"]} if "x" in e else e for e in events]
                probes.probe("widgets.SpanSelector", _span, direction=direction,
                             options=options, events=events)


RECTANGLES = [
    drag([(100, 80), (150, 120), (200, 200)]),
    drag([(200, 200), (100, 80)]),
    drag([(100, 80), (103, 82)]),
    [key("shift"), *drag([(100, 80), (200, 150)]), key("shift", up=True)],
    [key("control"), *drag([(150, 150), (200, 200)]), key("control", up=True)],
    [*drag([(100, 80), (200, 200)]), *drag([(150, 140), (180, 160)])],
    [*drag([(100, 80), (200, 200)]), *drag([(200, 200), (250, 240)])],
    [*drag([(100, 80), (200, 200)]), *drag([(200, 140), (240, 140)])],
    [*drag([(100, 80), (200, 200)]), *click(300, 50)],
    drag([(100, 80), (500, 400)]),
]
RECTANGLE_OPTIONS = [
    {},
    {"minspanx": 1.0, "minspany": 1.0},
    {"interactive": True},
    {"interactive": True, "drag_from_anywhere": True},
    {"interactive": True, "ignore_event_outside": True},
]


def test_rectangles():
    for options in RECTANGLE_OPTIONS:
        for events in RECTANGLES:
            probes.probe("widgets.RectangleSelector", _rectangle, options=options, events=events)
