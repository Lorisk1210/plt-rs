"""
C, what the program is called back with as it is used: events and what
they carry, picking, ``format_coord`` and ``fmt_xdata``, widgets'
callbacks, animation frames, legend handlers, the callables that place
Axes, boxes and annotations, and the ``inside`` functions of `bezier`.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import animation, bezier, patches, transforms, widgets
from pltrs.backend_bases import KeyEvent, MouseEvent, NavigationToolbar2
from pltrs.legend_handler import HandlerBase
from pltrs.offsetbox import TextArea, VPacker
from pltrs.path import Path
from pltrs.testing.widgets import click_and_drag


def mouse(rows):
    """Mouse, scroll and key events, picks and the coordinates' texts."""
    seen = rows.seen("C layout and events")

    def offset(width, height, xdescent, ydescent, renderer):
        seen.note("set_offset callable", width, height, xdescent, ydescent)
        return 0, 0

    figo, axo = plt.subplots()
    box = VPacker(children=[TextArea("x")])
    box.set_offset(offset)
    box.set_figure(figo)
    figo.artists.append(box)

    def locator(a, renderer):
        seen.note("axes_locator", a.get_position().bounds)
        return transforms.Bbox.from_bounds(0.5, 0.5, 0.2, 0.2)

    figo.add_axes([0, 0, 1, 1], axes_locator=locator)
    axo.plot([0, 1], picker=lambda artist, ev: seen.note("picker xdata ydata", ev.xdata, ev.ydata)
             or (False, {}))
    figo.canvas.draw()
    MouseEvent("button_press_event", figo.canvas, 100, 100, 1)._process()
    figo.canvas.mpl_connect("motion_notify_event",
                            lambda e: seen.note("event x y xdata ydata", e.x, e.y, e.xdata, e.ydata))
    MouseEvent("motion_notify_event", figo.canvas, 200, 150)._process()
    seen.close()

    events = rows.seen("C events")
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    line, = ax.plot([1, 5, 9], [1, 5, 9], picker=5)
    points = ax.scatter([2, 8], [8, 2], picker=True)
    patch = ax.add_patch(plt.Rectangle((6, 6), 2, 2, picker=True))
    fig.canvas.draw()
    for name in ("button_press_event", "button_release_event", "scroll_event",
                 "motion_notify_event"):
        fig.canvas.mpl_connect(name, lambda e, n=name: events.note(
            f"{n} x y xdata ydata button step", e.x, e.y, e.xdata, e.ydata, e.button,
            getattr(e, "step", None)))
    fig.canvas.mpl_connect("key_press_event", lambda e: events.note(
        "key_press_event x y xdata ydata", e.x, e.y, e.xdata, e.ydata))
    fig.canvas.mpl_connect("pick_event", lambda e: events.note(
        f"pick_event {type(e.artist).__name__} ind, mouse xdata ydata",
        getattr(e, "ind", None), e.mouseevent.xdata, e.mouseevent.ydata))
    for x, y in [ax.transData.transform(point) for point in [(5, 5), (2, 8), (7, 7)]]:
        MouseEvent("button_press_event", fig.canvas, x, y, 1)._process()
        MouseEvent("button_release_event", fig.canvas, x, y, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, 150, 120)._process()
    MouseEvent("motion_notify_event", fig.canvas, 150.5, 120.25)._process()
    MouseEvent("scroll_event", fig.canvas, 150, 120, "up", step=1)._process()
    MouseEvent("scroll_event", fig.canvas, 150, 120, "down", step=-2.0)._process()
    KeyEvent("key_press_event", fig.canvas, "a", 150, 120)._process()
    events.note("line.contains ind", line.contains(MouseEvent(
        "button_press_event", fig.canvas, *ax.transData.transform((5, 5)), 1))[1]["ind"])
    events.note("scatter.contains ind", points.contains(MouseEvent(
        "button_press_event", fig.canvas, *ax.transData.transform((2, 8)), 1))[1]["ind"])
    events.note("patch.contains_point", patch.contains_point(ax.transData.transform((7, 7))))
    ax.fmt_xdata = lambda x: events.note("fmt_xdata", x) or "x"
    ax.fmt_ydata = lambda y: events.note("fmt_ydata", y) or "y"
    ax.format_coord(1.5, 2.5)
    ax.format_coord(np.float64(1.5), 2)
    polar = plt.figure().add_subplot(projection="polar")
    polar.figure.canvas.draw()
    polar.fmt_xdata = lambda x: events.note("polar fmt_xdata", x) or "x"
    polar.fmt_ydata = lambda y: events.note("polar fmt_ydata", y) or "y"
    polar.format_coord(1, 2)
    events.close()


def program_coordinates(rows):
    """``format_coord`` of the program's, called as a toolbar's message
    is made for the cursor over the Axes."""
    seen = rows.seen("C format_coord")
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.format_coord = lambda x, y: seen.note("format_coord", x, y) or ""
    fig.canvas.draw()
    event = MouseEvent("motion_notify_event", fig.canvas, 150, 120)
    NavigationToolbar2._mouse_event_to_message(event)
    seen.close()


def controls(rows):
    """The callbacks of sliders, selectors and text boxes."""
    seen = rows.seen("C widgets")
    fig, axs = plt.subplots(5, figsize=(4, 6), dpi=100)
    for ax in axs:
        ax.set_xlim(0, 10)
        ax.set_ylim(0, 10)
    fig.canvas.draw()
    slider = widgets.Slider(axs[0], "s", 0, 10, valinit=5)
    slider.on_changed(lambda v: seen.note("Slider on_changed", v))
    slider.set_val(3)
    slider.set_val(np.float32(4))
    seen.note("Slider val", slider.val)
    steps = widgets.Slider(axs[1], "i", 0, 10, valinit=2, valstep=1)
    steps.on_changed(lambda v: seen.note("Slider valstep=1 on_changed", v))
    steps.set_val(3.4)
    seen.note("Slider valstep=1 val", steps.val)
    ranged = widgets.RangeSlider(axs[2], "r", 0, 10, valinit=(2, 8))
    ranged.on_changed(lambda v: seen.note("RangeSlider on_changed", v))
    ranged.set_val((3, 7))
    ranged.set_min(1)
    seen.note("RangeSlider val", ranged.val)
    span = widgets.SpanSelector(axs[3], lambda a, b: seen.note("SpanSelector onselect", a, b),
                                "horizontal", interactive=True)
    click_and_drag(span, (2, 5), (6, 5))
    seen.note("SpanSelector extents", span.extents)
    rect = widgets.RectangleSelector(axs[4], lambda press, release: seen.note(
        "RectangleSelector onselect xdata ydata", press.xdata, press.ydata, release.xdata, release.ydata),
        interactive=True)
    click_and_drag(rect, (2, 2), (6, 7))
    seen.note("RectangleSelector extents", rect.extents)
    seen.note("RectangleSelector corners", rect.corners)
    seen.note("RectangleSelector center", rect.center)
    tf, tax = plt.subplots()
    box = widgets.TextBox(tax, "t", initial="1")
    box.on_submit(lambda text: seen.note("TextBox on_submit", text))
    box.set_val("2")
    seen.close()


def frames(rows):
    """The frame arguments of an animation's function for each kind of
    ``frames``."""
    seen = rows.seen("C animation")
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])

    def generator():
        yield 1
        yield np.int64(2)

    for label, given in [("int", 3), ("range", range(2)), ("list", [1, 2]),
                         ("ndarray", np.arange(2)), ("float array", np.linspace(0, 1, 2)),
                         ("generator", generator)]:
        anim = animation.FuncAnimation(
            fig, lambda frame, k=label: seen.note(f"FuncAnimation frame (frames={k})", frame) or (line,),
            frames=given, repeat=False)
        anim._init_draw()
        for frame in anim.new_saved_frame_seq():
            anim._draw_next_frame(frame, blit=False)
    seen.close()


def handlers(rows):
    """A legend handler of the program's."""
    seen = rows.seen("C legend handler")

    class Handler(HandlerBase):
        def create_artists(self, legend, orig_handle, xdescent, ydescent, width, height,
                           fontsize, trans):
            seen.note("create_artists xdescent ydescent width height fontsize",
                      xdescent, ydescent, width, height, fontsize)
            return [plt.Rectangle((-xdescent, -ydescent), width, height, transform=trans)]

    class Plain:
        pass

    fig, ax = plt.subplots()
    handle = Plain()
    ax.legend([handle], ["x"], handler_map={Plain: Handler()}, fontsize=10)
    ax.legend([handle], ["y"], handler_map={Plain: Handler()}, fontsize=np.int64(11),
              handlelength=2, handleheight=1)
    fig.canvas.draw()
    seen.close()


def placement(rows):
    """Callables placing annotations by the renderer."""
    seen = rows.seen("C annotation coordinates")
    fig, ax = plt.subplots()

    def coordinates(renderer):
        seen.note("xycoords callable renderer dpi", renderer.dpi)
        return ax.bbox

    ax.annotate("a", (0.5, 0.5), xycoords=coordinates)
    ax.annotate("b", (1, 1), xycoords=lambda renderer: ax.transAxes)
    fig.canvas.draw()
    seen.close()


def connections(rows):
    """``connect`` of a connection style of the program's, registered and
    given to an arrow and an annotation."""
    seen = rows.seen("C connection style")

    class Straight(patches.ConnectionStyle._Base):
        def connect(self, posA, posB):
            seen.note("connect posA posB", posA, posB)
            return Path([posA, posB])

    patches.ConnectionStyle._style_list["num1straight"] = Straight
    try:
        fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
        ax.add_patch(patches.FancyArrowPatch((0.1, 0.1), (0.9, 0.9),
                                             connectionstyle="num1straight", arrowstyle="->"))
        ax.annotate("a", (0.2, 0.8), (0.6, 0.3),
                    arrowprops=dict(arrowstyle="->", connectionstyle=Straight()))
        fig.canvas.draw()
    finally:
        del patches.ConnectionStyle._style_list["num1straight"]
    seen.close()


def curves(rows):
    """The ``inside`` functions of `bezier`."""
    seen = rows.seen("C bezier")
    bezier.split_bezier_intersecting_with_closedpath(
        [(0, 0), (1, 2), (2, 0)], lambda xy: seen.note("inside_closedpath", xy) or xy[0] < 1)
    bezier.split_path_inout(Path([(0, 0), (1, 1), (2, 0)], [1, 3, 3]),
                            lambda xy: seen.note("split_path_inout inside", xy) or xy[0] < 1)
    bezier.split_path_inout(Path([(0, 0), (1, 1), (2, 0)]),
                            lambda xy: seen.note("split_path_inout inside of lines", xy) or xy[0] < 1)
    bezier.find_bezier_t_intersecting_with_closedpath(
        bezier.BezierSegment(np.array([[0, 0], [1, 2], [2, 0]])).point_at_t,
        lambda xy: seen.note("find_bezier_t inside", xy) or xy[0] < 1)
    seen.close()


SCENES = (mouse, program_coordinates, controls, frames, handlers, placement, connections,
          curves)
