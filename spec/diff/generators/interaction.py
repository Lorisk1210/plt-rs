"""Figures driven as a user at a window drives them, on the headless
canvas, where matplotlib processes the same events: the navigation
toolbar's pan and zoom, the default key bindings, every event a canvas
emits, the widgets dragged and typed into, picking and dragging artists,
blitting and animations.

The events are matplotlib's own (``MouseEvent(...)._process()``,
``KeyEvent(...)._process()``, ``LocationEvent``, ``ResizeEvent``), made
at whole pixels as a window's are. After each step a program reads what
the step changed (limits, the toolbar's mode and history, widget values,
artists' properties), and callbacks print what they receive, numbers to
four significant digits. Every figure is drawn and compared as an image
at the end.

* ``toolbar_<scene>``: ``NavigationToolbar2`` on each scene of
  ``interaction_scenes``: its messages, zoom (each button, the x and y
  keys, rubber bands too small, out of the Axes), pan (each button, the
  x, y and control keys), home, back, forward, ``push_current`` and
  ``update``, Axes that do not navigate;
* ``keys_<scene>``: every default ``keymap.*`` binding through the
  canvas, then the keymaps rebound, the extra mouse buttons and
  ``key_press_handler`` called directly;
* ``events_<scene>``: every event name connected, the attributes each
  callback gets, entering and leaving Axes and the figure, disconnection,
  callbacks failing, the Axes' and axes' own callbacks, resizing;
* ``widgets_*``, ``selectors_*`` (``interaction_widgets``) and
  ``picking_*``, ``draggable_*``, ``blitting_*``, ``animation_*``
  (``interaction_artists``).
"""

from __future__ import annotations

from typing import Iterator

from . import common, interaction_artists, interaction_widgets
from .interaction_scenes import EVENT_SCENES, KEY_SCENES, SCENES, scene

#: The helpers every program defines: events at pixels (``mouse``,
#: ``drag``, ``press``) and in data coordinates (``at``, ``drag_data``),
#: and ``show``, printing numbers to four significant digits.
HELPERS = [
    "from matplotlib.backend_bases import (CloseEvent, KeyEvent, LocationEvent, MouseButton, MouseEvent, "
    "NavigationToolbar2, ResizeEvent, button_press_handler, key_press_handler)",
    "def show(*items):\n"
    "    print(*(f'{v:.4g}' if isinstance(v, float) else v for v in items))",
    "def px(a, fx, fy):\n"
    "    return tuple(round(float(v)) for v in a.transAxes.transform((fx, fy)))",
    "def mouse(name, xy, button=None, key=None, step=0, dblclick=False, buttons=None):\n"
    "    MouseEvent(name, fig.canvas, *xy, button, key, step, dblclick, buttons=buttons)._process()",
    "def drag(start, end, button=MouseButton.LEFT, key=None, steps=2):\n"
    "    mouse('button_press_event', start, button, key)\n"
    "    for i in range(1, steps + 1):\n"
    "        mouse('motion_notify_event', (start[0] + (end[0] - start[0]) * i // steps,"
    " start[1] + (end[1] - start[1]) * i // steps), key=key, buttons={button})\n"
    "    mouse('button_release_event', end, button, key)",
    "def press(key, xy):\n"
    "    KeyEvent('key_press_event', fig.canvas, key, *xy)._process()\n"
    "    KeyEvent('key_release_event', fig.canvas, key, *xy)._process()",
    "def at(a, name, xy, button=None, **kwargs):\n"
    "    MouseEvent._from_ax_coords(name, a, xy, button, **kwargs)._process()",
    "def drag_data(a, start, end, button=1, key=None, steps=2):\n"
    "    if key:\n"
    "        KeyEvent._from_ax_coords('key_press_event', a, start, key)._process()\n"
    "    at(a, 'motion_notify_event', start)\n"
    "    at(a, 'button_press_event', start, button)\n"
    "    for i in range(1, steps + 1):\n"
    "        at(a, 'motion_notify_event', (start[0] + (end[0] - start[0]) * i / steps,"
    " start[1] + (end[1] - start[1]) * i / steps), button)\n"
    "    at(a, 'button_release_event', end, button)\n"
    "    if key:\n"
    "        KeyEvent._from_ax_coords('key_release_event', a, end, key)._process()",
]

#: A toolbar printing its messages, rubber bands and history updates.
TOOLBAR = [
    "class Bar(NavigationToolbar2):\n"
    "    def set_message(self, s):\n"
    "        print('message:', s)\n"
    "    def draw_rubberband(self, event, x0, y0, x1, y1):\n"
    "        show('rubberband:', x0, y0, x1, y1)\n"
    "    def remove_rubberband(self):\n"
    "        print('rubberband removed')\n"
    "    def set_history_buttons(self):\n"
    "        print('history buttons')",
    "def view():\n"
    "    return [(a.get_xlim(), a.get_ylim(), a.get_navigate_mode(), a.bbox.bounds) for a in axs]",
]

#: Rubber bands of the zoom: (start, end) as fractions of ``ax``, the
#: button and the key held.
ZOOMS = [
    ((0.2, 0.2), (0.6, 0.7), "MouseButton.LEFT", None),
    ((0.3, 0.3), (0.7, 0.6), "MouseButton.LEFT", "'x'"),
    ((0.3, 0.3), (0.6, 0.8), "MouseButton.LEFT", "'y'"),
    ((0.4, 0.4), (0.41, 0.6), "MouseButton.LEFT", None),
    ((0.3, 0.3), (0.31, 0.6), "MouseButton.LEFT", "'y'"),
    ((0.6, 0.6), (0.2, 0.3), "MouseButton.RIGHT", None),
    ((0.5, 0.5), (1.3, -0.2), "MouseButton.LEFT", None),
    ((0.2, 0.2), (0.6, 0.7), "MouseButton.MIDDLE", None),
]
#: Pan drags: (start, end) as fractions of ``ax``, the button and the key.
PANS = [
    ((0.5, 0.5), (0.3, 0.6), "MouseButton.LEFT", None),
    ((0.5, 0.5), (0.6, 0.3), "MouseButton.LEFT", "'x'"),
    ((0.5, 0.5), (0.6, 0.3), "MouseButton.LEFT", "'y'"),
    ((0.5, 0.5), (0.7, 0.6), "MouseButton.LEFT", "'control'"),
    ((0.5, 0.5), (0.7, 0.6), "MouseButton.RIGHT", None),
    ((0.5, 0.5), (0.3, 0.6), "MouseButton.RIGHT", "'control'"),
    ((0.5, 0.5), (0.4, 0.4), "MouseButton.RIGHT", "'x'"),
    ((0.5, 0.5), (0.7, 0.6), "MouseButton.MIDDLE", None),
]


def _gesture(start: tuple[float, float], end: tuple[float, float], button: str, key: str | None) -> str:
    return f"drag(px(ax, {start[0]}, {start[1]}), px(ax, {end[0]}, {end[1]}), {button}, {key}, steps=3)"


def _toolbar() -> Iterator[str]:
    yield "tb = Bar(fig.canvas)"
    yield "fig.canvas.toolbar is tb, str(tb.mode), tb.mode.name"
    yield "[(a.get_navigate(), a.can_pan(), a.can_zoom()) for a in axs]"
    yield "view()"
    for point in ((0.3, 0.4), (0.95, 0.05), (0.02, 0.98), (1.2, 0.5), (0.5, -0.3)):
        yield f"mouse('motion_notify_event', px(ax, {point[0]}, {point[1]}))"
        yield f"NavigationToolbar2._mouse_event_to_message(MouseEvent('motion_notify_event', fig.canvas, " \
              f"*px(ax, {point[0]}, {point[1]})))"
    yield "[a.format_coord(*a.transData.inverted().transform(px(a, 0.25, 0.75))) for a in axs]"
    yield "tb.zoom()"
    yield "str(tb.mode), [a.get_navigate_mode() for a in axs]"
    for gesture in ZOOMS:
        yield _gesture(*gesture)
        yield "view()"
    yield "tb.back()"
    yield "view()"
    yield "tb.back()"
    yield "view()"
    yield "tb.forward()"
    yield "view()"
    yield "tb.home()"
    yield "view()"
    yield "tb.zoom()"
    yield "str(tb.mode)"
    yield "tb.pan()"
    yield "str(tb.mode), [a.get_navigate_mode() for a in axs]"
    for gesture in PANS:
        yield _gesture(*gesture)
        yield "view()"
    yield "tb.back()"
    yield "view()"
    yield "tb.home()"
    yield "view()"
    yield "ax.set_xlim(*ax.get_xlim()[::-1])"
    yield "tb.push_current()"
    yield "tb.back()"
    yield "view()"
    yield "tb.forward()"
    yield "view()"
    yield "tb.update()"
    yield "tb.back()"
    yield "view()"
    yield "ax.set_navigate(False)"
    yield "drag(px(ax, 0.5, 0.5), px(ax, 0.2, 0.2))"
    yield "view()"
    yield "ax.set_navigate(True)"
    yield "tb.zoom()"
    yield "str(tb.mode), [a.get_navigate_mode() for a in axs]"
    yield "drag(px(ax, 0.1, 0.1), px(ax, 0.9, 0.5))"
    yield "view()"
    yield "tb.home()"
    yield "tb.zoom()"
    yield "str(tb.mode), [a.get_navigate_mode() for a in axs]"
    yield "fig.canvas.draw()"
    yield "view()"


#: Keys pressed with the default keymaps, in order (the quitting keys come
#: last, in ``_keys``).
KEYS = ["g", "g", "g", "g", "G", "G", "g", "l", "l", "k", "k", "L", "L", "a", "o", "o", "p", "h", "left", "c",
        "backspace", "right", "v", "r", "home", "s", "ctrl+s", "f", "ctrl+f", "f1", "ctrl+c", "cmd+c", "shift",
        "control", "alt+g"]


def _keys() -> Iterator[str]:
    yield "tb = NavigationToolbar2(fig.canvas)"
    yield "tb.save_figure = lambda *args: print('save_figure', len(args))"
    yield "def uniform(ticks):\n" \
          "    states = {t.gridline.get_visible() for t in ticks}\n" \
          "    return states.pop() if len(states) == 1 else sorted(states)"
    yield "def state():\n" \
          "    return (uniform(ax.xaxis.majorTicks), uniform(ax.yaxis.majorTicks), uniform(ax.xaxis.minorTicks), " \
          "uniform(ax.yaxis.minorTicks), ax.get_xscale(), ax.get_yscale(), ax.get_xlim(), ax.get_ylim(), " \
          "str(tb.mode), plt.fignum_exists(fig.number))"
    yield "inside = px(ax, 0.5, 0.5)"
    yield "tb.push_current()"
    yield "ax.set_xlim(*ax.get_xlim()[::-1])"
    yield "tb.push_current()"
    yield "state()"
    for key in KEYS:
        yield f"press({key!r}, inside)"
        yield "state()"
    yield "press('g', (1, 1))"
    yield "state()"
    yield "press('l', (1, 1))"
    yield "state()"
    yield "ax.grid(True, which='minor', axis='x')"
    yield "press('g', inside)"
    yield "state()"
    yield "press('G', inside)"
    yield "state()"
    yield "ax.grid(False, which='both')"
    for button in ("BACK", "FORWARD", "MIDDLE"):
        yield f"mouse('button_press_event', inside, MouseButton.{button})"
        yield f"mouse('button_release_event', inside, MouseButton.{button})"
        yield "state()"
    yield "matplotlib.rcParams['keymap.grid'] = ['x', 'ctrl+g']"
    yield "matplotlib.rcParams['keymap.yscale'] = ['Y']"
    yield "matplotlib.rcParams['keymap.back'] = ['b', 'MouseButton.MIDDLE']"
    yield "matplotlib.rcParams['keymap.home'] = []"
    for key in ("x", "ctrl+g", "g", "Y", "l", "b", "h"):
        yield f"press({key!r}, inside)"
        yield "state()"
    yield "mouse('button_press_event', inside, MouseButton.MIDDLE)"
    yield "state()"
    yield "key_press_handler(KeyEvent('key_press_event', fig.canvas, 'Y', *inside))"
    yield "key_press_handler(KeyEvent('key_press_event', fig.canvas, None, *inside))"
    yield "key_press_handler(KeyEvent('key_press_event', fig.canvas, 'x', *inside), fig.canvas, tb)"
    yield "state()"
    yield "button_press_handler(MouseEvent('button_press_event', fig.canvas, *inside, MouseButton.FORWARD))"
    yield "state()"
    yield "from matplotlib.figure import Figure"
    yield "bare = Figure()"
    yield "bare_ax = bare.subplots()"
    yield "bare.canvas.manager is None, bare.canvas.toolbar"
    yield "KeyEvent('key_press_event', bare.canvas, 'g', 100, 100)._process()"
    yield "bare_ax.xaxis.majorTicks[0].gridline.get_visible()"
    yield "key_press_handler(KeyEvent('key_press_event', bare.canvas, 'g', *bare_ax.transAxes.transform((0.5, 0.5))))"
    yield "bare_ax.xaxis.majorTicks[0].gridline.get_visible()"
    yield "matplotlib.rcParams['keymap.quit_all'] = ['Q']"
    yield "other = plt.figure()"
    yield "plt.get_fignums()"
    yield "press('q', (1, 1))"
    yield "plt.get_fignums()"
    yield "press('Q', (1, 1))"
    yield "plt.get_fignums()"
    yield "fig = plt.figure()"
    yield "plt.plot(x, y)"
    yield "press('ctrl+w', (5, 5))"
    yield "plt.get_fignums()"


#: Every event a canvas emits.
EVENT_NAMES = ("button_press_event", "button_release_event", "motion_notify_event", "scroll_event",
               "key_press_event", "key_release_event", "pick_event", "figure_enter_event", "figure_leave_event",
               "axes_enter_event", "axes_leave_event", "draw_event", "resize_event", "close_event")


def _events() -> Iterator[str]:
    yield "seen = []"
    yield "def report(event):\n" \
          "    fields = [event.name, type(event).__name__]\n" \
          "    for attr in ('x', 'y', 'xdata', 'ydata', 'button', 'key', 'step', 'dblclick', 'guiEvent'):\n" \
          "        if hasattr(event, attr):\n" \
          "            value = getattr(event, attr)\n" \
          "            fields.append(f'{attr}={value:.4g}' if isinstance(value, float) else f'{attr}={value!r}')\n" \
          "    if hasattr(event, 'inaxes'):\n" \
          "        fields.append(f'inaxes={axs.index(event.inaxes) if event.inaxes in axs else event.inaxes}')\n" \
          "    if hasattr(event, 'artist'):\n" \
          "        fields.append(f'artist={type(event.artist).__name__} ind={getattr(event, \"ind\", None)}')\n" \
          "    if hasattr(event, 'renderer'):\n" \
          "        fields.append(type(event.renderer).__name__)\n" \
          "    if hasattr(event, 'width'):\n" \
          "        fields.append(f'size={event.width}x{event.height}')\n" \
          "    seen.append((event.name, getattr(event, 'x', None), getattr(event, 'y', None), " \
          "getattr(event, 'xdata', None), getattr(event, 'ydata', None)))\n" \
          "    print(*fields)"
    yield f"cids = [fig.canvas.mpl_connect(name, report) for name in {EVENT_NAMES!r}]"
    yield "len(set(cids)), sorted(fig.canvas.callbacks.callbacks)"
    yield "LocationEvent('figure_enter_event', fig.canvas, 2, 2)._process()"
    for point in ((0.5, 0.5), (0.8, 0.9), (1.08, 0.5), (0.6, 0.5), (0.5, 1.2), (-0.15, -0.15)):
        yield f"mouse('motion_notify_event', px(ax, {point[0]}, {point[1]}))"
    yield "seen[-4:]"
    yield "mouse('button_press_event', px(ax, 0.3, 0.3), MouseButton.LEFT)"
    yield "mouse('motion_notify_event', px(ax, 0.35, 0.3), buttons={MouseButton.LEFT})"
    yield "mouse('button_release_event', px(ax, 0.35, 0.3), MouseButton.LEFT)"
    yield "mouse('button_press_event', px(ax, 0.35, 0.3), MouseButton.LEFT, dblclick=True)"
    yield "mouse('button_release_event', px(ax, 0.35, 0.3), MouseButton.LEFT)"
    yield "mouse('button_press_event', px(ax, 0.6, 0.6), MouseButton.RIGHT)"
    yield "mouse('button_release_event', px(ax, 1.5, 1.5), MouseButton.RIGHT)"
    yield "mouse('scroll_event', px(ax, 0.5, 0.5), 'up', step=1)"
    yield "mouse('scroll_event', px(ax, 0.5, 0.5), 'down', step=-2)"
    yield "KeyEvent('key_press_event', fig.canvas, 'shift', *px(ax, 0.4, 0.4))._process()"
    yield "mouse('motion_notify_event', px(ax, 0.45, 0.4))"
    yield "mouse('button_press_event', px(ax, 0.45, 0.4), MouseButton.LEFT)"
    yield "mouse('button_release_event', px(ax, 0.45, 0.4), MouseButton.LEFT)"
    yield "KeyEvent('key_release_event', fig.canvas, 'shift', *px(ax, 0.45, 0.4))._process()"
    yield "KeyEvent('key_press_event', fig.canvas, 'ctrl+a', 3, 3)._process()"
    yield "KeyEvent('key_release_event', fig.canvas, 'ctrl+a', 3, 3)._process()"
    yield "LocationEvent('figure_leave_event', fig.canvas, -5, -5)._process()"
    yield "seen[-6:]"
    # (Data within the limits: on polar Axes, a line changing them after a
    # draw is drawn by matplotlib through its transformed path cached then.)
    yield "line = ax.plot(x, values / 2, 'o', picker=6)[0]"
    yield "fig.canvas.draw()"
    yield "mouse('button_press_event', " \
          "tuple(round(float(v)) for v in ax.transData.transform((x[2], values[2] / 2))), MouseButton.LEFT)"
    yield "mouse('button_release_event', (1, 1), MouseButton.LEFT)"
    yield "fig.set_size_inches(5, 3.5)"
    yield "fig.canvas.get_width_height(), fig.canvas.get_width_height(physical=True)"
    yield "ResizeEvent('resize_event', fig.canvas)._process()"
    yield "fig.set_size_inches(4.25, 3.33)"
    yield "fig.canvas.get_width_height(), fig.canvas.get_width_height(physical=True)"
    yield "fig.set_dpi(72)"
    yield "fig.canvas.get_width_height(), fig.bbox.bounds"
    yield "ResizeEvent('resize_event', fig.canvas)._process()"
    yield "fig.set_dpi(100)"
    yield "fig.set_size_inches(4, 3)"
    yield "for cid in cids[2:5]:\n    fig.canvas.mpl_disconnect(cid)"
    yield "fig.canvas.mpl_disconnect(10 ** 6)"
    yield "mouse('motion_notify_event', px(ax, 0.2, 0.2))"
    yield "mouse('scroll_event', px(ax, 0.2, 0.2), 'up', step=1)"
    yield "mouse('button_press_event', px(ax, 0.2, 0.2), MouseButton.LEFT)"
    yield "mouse('button_release_event', px(ax, 0.2, 0.2), MouseButton.LEFT)"
    yield "def failing(event):\n    raise ValueError(f'failed on {event.name}')"
    yield "failing_cid = fig.canvas.mpl_connect('button_press_event', failing)"
    yield "mouse('button_press_event', px(ax, 0.2, 0.2), MouseButton.LEFT)"
    yield "fig.canvas.callbacks.exception_handler = lambda exc: print('handled', type(exc).__name__, exc)"
    yield "mouse('button_press_event', px(ax, 0.2, 0.2), MouseButton.LEFT)"
    yield "fig.canvas.mpl_disconnect(failing_cid)"
    yield "registry = matplotlib.cbook.CallbackRegistry(exception_handler=None, signals=['ping'])"
    yield "registry.connect('ping', failing)"
    yield "registry.process('ping', type('Ping', (), {'name': 'ping'})())"
    yield "registry.connect('pong', print)"
    yield "limits = [a.callbacks.connect(signal, lambda a, signal=signal: show(signal, axs.index(a), " \
          "*(a.get_xlim() if signal == 'xlim_changed' else a.get_ylim()))) for a in axs " \
          "for signal in ('xlim_changed', 'ylim_changed')]"
    yield "ax.set_xlim(ax.get_xlim()[0], ax.get_xlim()[1])"
    yield "ax.set_ylim(*ax.get_ylim()[::-1])"
    yield "ax.set_xlim(*ax.get_xlim()[::-1], emit=False)"
    yield "ax.autoscale()"
    yield "ax.margins(0.2)"
    yield "ax.relim()"
    yield "ax.autoscale_view()"
    yield "fig.canvas.draw()"
    yield "len(seen)"
    yield "ax.xaxis.callbacks.connect('units', lambda: print('units changed'))"
    yield "closing = plt.figure()"
    yield "closing.canvas.mpl_connect('close_event', report)"
    yield "plt.close(closing)"
    yield "closing.canvas.callbacks.process('close_event', CloseEvent('close_event', closing.canvas))"


def _units() -> Iterator[str]:
    """Axes' units callbacks, on a figure of its own."""
    yield "units_fig, units_ax = plt.subplots()"
    yield "units_ax.xaxis.callbacks.connect('units', lambda: print('x units'))"
    yield "units_ax.yaxis.callbacks.connect('units', lambda: print('y units'))"
    yield "units_ax.plot(np.arange(np.datetime64('2024-01-01'), np.datetime64('2024-01-05')), [1, 2, 3, 4])"
    yield "units_ax.plot(['a', 'b'], [1, 2])"
    yield "units_ax.yaxis.set_units(None)"
    yield "units_ax.xaxis.converter is not None, units_ax.yaxis.get_units()"


def programs() -> Iterator[tuple[str, list[str]]]:
    """The toolbar, key and event programs on their scenes, then the
    widget and artist programs."""
    header = common.HEADER + common.DATA + HELPERS
    for name in SCENES:
        yield f"toolbar_{name}", header + scene(name) + TOOLBAR + list(_toolbar())
    for name in KEY_SCENES:
        yield f"keys_{name}", header + scene(name) + list(_keys())
    for name in EVENT_SCENES:
        units = list(_units()) if name == "plain" else []
        yield f"events_{name}", header + scene(name) + list(_events()) + units
    for name, statements in interaction_widgets.programs():
        yield name, header + statements
    for name, statements in interaction_artists.programs():
        yield name, header + statements
