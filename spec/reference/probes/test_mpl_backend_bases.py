"""matplotlib's own tests of ``matplotlib.backend_bases``
(``matplotlib/tests/test_backend_bases.py``): how collections reuse their
paths, canvas helpers, location, key and pick events, the coordinates a
status bar shows, the navigation toolbar's zoom, pan, home and widget lock,
layout on every canvas, and canvas sizes.

Area ``mpl_backend_bases``. Unless noted, an op draws on a
``Figure()`` (6.4 x 4.8 in at 100 dpi) with an Agg canvas. Mouse input is
given in whole display pixels, which the test functions compute as the
upstream tests do (``transData`` of data points, truncated) on a scratch
copy of the op's figure: before a draw, matplotlib's ``transData`` still
maps the view of a fresh axes, ``(0, 1)``, whatever was plotted, and the
ops take the pixels as they are. The ops replaying toolbar input draw the
figure once first, as a window does before input reaches it (the upstream
tests do not, which leaves the aspect of ``test_interactive_pan``
unapplied).

Ops
---
``RendererBase._iter_collection``  {paths, transforms, offsets, facecolors,
    edgecolors} -> ``[uses, seen]``: ``_iter_collection_uses_per_path`` of a
    collection with that many regular polygons, rotations, offsets and
    colours, and how often ``_iter_collection`` draws each raw path
    (``test_uses_per_path``, which requires every count to be ``uses`` or
    ``uses - 1``).
``FigureCanvasBase.get_default_filename``  {title} -> the default file name
    of a pyplot figure, then after ``manager.set_window_title(title)``
    (``test_get_default_filename``).
``FigureCanvasBase.grab_mouse``  {} -> whether the canvas' mouse grabber is
    the axes grabbing it, and whether it is ``None`` after ``fig.clear()``
    (``test_grab_clear``).
``LocationEvent``  {x, y} -> ``[event.x, event.y]`` of a location event at
    ``(x, y)`` (``test_location_event_position``).
``Axes.format_coord``  {x, y, fmt?} -> ``[format_coord(x, y),
    format_xdata(x), format_ydata(y)]`` on fresh axes, with ``fmt_xdata``
    and ``fmt_ydata`` returning ``fmt`` when given
    (``test_location_event_position``).
``Axes.format_coord``  {twins} -> ``format_coord(5, 5)`` of axes with limits
    ``(0, 10)`` and ``(0, 20)``, after each twin of ``twins`` (``"x"``: a
    ``twinx`` with y limits ``(0, 40)``, ``"y"``: a ``twiny`` with x limits
    ``(0, 5)``) is added (``test_location_event_position_twin``).
``pick_event``  {key} -> the number of pick events a press over a picker
    text fires while ``key`` is held, and the key of their mouse event
    (``test_pick``).
``NavigationToolbar2.zoom``  {scale, start, stop} -> on axes with an x
    ``scale``: the navigate mode before and after ``zoom()``, the limits and
    the data coordinates of the pixels after zooming in from pixel ``start``
    to pixel ``stop``, the limits after zooming out from ``stop`` to
    ``start``, the navigate mode after ``zoom()`` again and whether x and y
    autoscale (``test_interactive_zoom``: the pixels of ``(1e-6, 0.1)`` and
    ``(1 - 1e-5, 0.8)``).
``NavigationToolbar2.widgetlock``  {} -> the navigate mode after ``zoom()``
    and ``pan()`` with the widget lock taken (``test_widgetlock_zoompan``).
``NavigationToolbar2.home``  {} -> the limits of ``plot(range(11),
    range(11))`` after both axes go logarithmic, after they go linear again,
    after a zoom from ``(2, 2)`` to ``(5, 5)`` and ``home()``, and after they
    go logarithmic once more (``test_toolbar_home_restores_autoscale``).
``FigureCanvasBase.draw``  {backend} -> the positions of the axes of
    ``Figure(constrained_layout=True).subplots(2, 2)`` before and after a
    draw on that backend's canvas (``test_draw``).
``NavigationToolbar2.pan``  {key, start, end} -> the limits of
    ``plot(arange(10))`` with an equal aspect after panning from pixel
    ``start`` to pixel ``end`` with ``key`` held (``test_interactive_pan``:
    the pixels of ``(0.5, 0.5)`` and of its ``mouseend``).
``NavigationToolbar2.events``  {tool, button, patch_visible, forward,
    twin_or_share, start, stop} -> the limits of the top and bottom axes and
    of their twins after a drag from pixel ``start`` to pixel ``stop`` with
    ``tool`` and ``button`` (``test_interactive_pan_zoom_events``: the
    pixels of ``(1, 1)`` and ``(2, 2)`` of the top axes).
``Colorbar.navigation``  {plot_func, orientation, tool, button, start, end}
    -> for the colorbar of ``plot_func(arange(12).reshape(4, 3), vmin=0,
    vmax=10)`` (``imshow`` or ``contourf``) in ``orientation``: whether its
    axes navigate, and if they do the norm's ``vmin`` and ``vmax`` after a
    drag from pixel ``start`` to pixel ``end`` with ``tool`` and ``button``
    (``test_interactive_colorbar``: the pixels of the values 4 and 6 on the
    colorbar).
``FigureCanvasBase.get_width_height``  {figsize, dpi} -> the figure's
    ``bbox.height`` and the canvas' ``get_width_height()``
    (``test_get_width_height_floating_point_precision``).

Adaptations: the key presses ``k`` and ``l`` of
``test_toolbar_home_restores_autoscale`` become the scale changes
matplotlib's ``key_press_handler`` makes for them (pltrs's keyboard
shortcuts belong to its figure windows), and ``h`` the toolbar's ``home``.

Left out: Python machinery (``test_canvas_ctor``, ``test_canvas_change``
replacing a figure's canvas object); ``test_non_gui_warning``, a warning
pltrs has no counterpart of (showing on a non-interactive backend does
nothing); the ``None`` positions of ``test_location_event_position``
(pltrs events always have a position); the toolmanager tests
(``test_toolbar_zoompan``, ``test_toolmanager_remove``,
``test_toolmanager_get_tool``, ``test_toolmanager_update_keymap``): the
toolmanager is experimental matplotlib API, and pltrs's own tools have no
built-in pan and zoom tools to trigger or remove; and the PGF variant of
``test_draw`` (it needs XeLaTeX).
"""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib import path as mpath
from matplotlib import transforms as mtransforms
from matplotlib.backend_bases import (
    FigureCanvasBase, KeyEvent, LocationEvent, MouseButton, MouseEvent, NavigationToolbar2,
    RendererBase)
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_backend_bases")


def _canvas_figure(**options):
    fig = Figure(**options)
    FigureCanvasAgg(fig)
    return fig


def _pixels(ax, point):
    return [int(v) for v in ax.transData.transform(point).astype(int)]


def _limits(ax):
    return [[float(v) for v in ax.get_xlim()], [float(v) for v in ax.get_ylim()]]


def _mode(ax):
    return ax.get_navigate_mode()


def _iter_collection(paths, transforms, offsets, facecolors, edgecolors):
    master = mtransforms.Affine2D()
    polygons = [mpath.Path.unit_regular_polygon(i) for i in range(3, 3 + paths)]
    matrices = [master.rotate(i).get_matrix().copy() for i in range(1, 1 + transforms)]
    points = np.arange(20).reshape((10, 2))[:offsets]
    faces = ["red", "green"][:facecolors]
    edges = ["red", "green"][:edgecolors]
    rb = RendererBase()
    raw_paths = list(rb._iter_collection_raw_paths(master, polygons, matrices))
    gc = rb.new_gc()
    ids = [path_id for _, _, path_id, _, _ in rb._iter_collection(
        gc, range(len(raw_paths)), points, mtransforms.AffineDeltaTransform(master),
        faces, edges, [], [], [False], [], "screen", hatchcolors=[])]
    uses = rb._iter_collection_uses_per_path(polygons, matrices, points, faces, edges)
    seen = np.bincount(ids, minlength=len(raw_paths)).tolist() if raw_paths else []
    return [uses, seen]


def _default_filename(title):
    plt.close("all")
    try:
        fig = plt.figure()
        before = fig.canvas.get_default_filename()
        fig.canvas.manager.set_window_title(title)
        return [before, fig.canvas.get_default_filename()]
    finally:
        plt.close("all")


def _grab_mouse():
    fig = _canvas_figure()
    ax = fig.add_subplot()
    fig.canvas.grab_mouse(ax)
    grabbed = fig.canvas.mouse_grabber is ax
    fig.clear()
    return [grabbed, fig.canvas.mouse_grabber is None]


def _location_event(x, y):
    fig = _canvas_figure()
    fig.add_subplot()
    event = LocationEvent("test_event", fig.canvas, x, y)
    return [event.x, event.y]


def _format_coord(x, y, fmt=None):
    ax = _canvas_figure().add_subplot()
    if fmt is not None:
        ax.fmt_xdata = ax.fmt_ydata = lambda value: fmt
    return [ax.format_coord(x, y), ax.format_xdata(x), ax.format_ydata(y)]


def _format_coord_twins(twins):
    ax = _canvas_figure().add_subplot()
    ax.set(xlim=(0, 10), ylim=(0, 20))
    seen = [ax.format_coord(5., 5.)]
    for twin in twins:
        if twin == "x":
            ax.twinx().set(ylim=(0, 40))
        else:
            ax.twiny().set(xlim=(0, 5))
        seen.append(ax.format_coord(5., 5.))
    return seen


def _pick(key):
    fig = _canvas_figure()
    fig.text(.5, .5, "hello", ha="center", va="center", picker=True)
    fig.canvas.draw()
    keys = []
    fig.canvas.mpl_connect("pick_event", lambda event: keys.append(event.mouseevent.key))
    KeyEvent("key_press_event", fig.canvas, key)._process()
    MouseEvent("button_press_event", fig.canvas, *fig.transFigure.transform((.5, .5)),
               MouseButton.LEFT)._process()
    KeyEvent("key_release_event", fig.canvas, key)._process()
    return [len(keys), keys[0] if keys else None]


def _press_release(fig, start, stop, button):
    for name, (x, y) in [("button_press_event", start), ("button_release_event", stop)]:
        event = MouseEvent(name, fig.canvas, x, y, button)
        fig.canvas.callbacks.process(event.name, event)


def _zoom_scene(scale):
    fig = _canvas_figure()
    ax = fig.add_subplot()
    ax.set(xscale=scale)
    return fig, ax


def _zoom(scale, start, stop):
    fig, ax = _zoom_scene(scale)
    fig.canvas.draw()
    seen = [_mode(ax)]
    tb = NavigationToolbar2(fig.canvas)
    tb.zoom()
    seen.append(_mode(ax))
    start_event = MouseEvent("button_press_event", fig.canvas, *start, MouseButton.LEFT)
    stop_event = MouseEvent("button_release_event", fig.canvas, *stop, MouseButton.LEFT)
    for event in [start_event, stop_event]:
        fig.canvas.callbacks.process(event.name, event)
    seen += [_limits(ax), [[start_event.xdata, stop_event.xdata],
                           [start_event.ydata, stop_event.ydata]]]
    _press_release(fig, stop, start, MouseButton.RIGHT)
    seen.append(_limits(ax))
    tb.zoom()
    return seen + [_mode(ax), ax.get_autoscalex_on(), ax.get_autoscaley_on()]


def _widgetlock():
    fig = _canvas_figure()
    ax = fig.add_subplot()
    ax.plot([0, 1], [0, 1])
    fig.canvas.widgetlock(ax)
    tb = NavigationToolbar2(fig.canvas)
    tb.zoom()
    zoomed = _mode(ax)
    tb.pan()
    return [zoomed, _mode(ax)]


def _toggle_logs(ax, scale):
    ax.set_xscale(scale)
    ax.set_yscale(scale)


def _home():
    fig = _canvas_figure()
    ax = fig.add_subplot()
    ax.plot(range(11), range(11))
    fig.canvas.draw()
    tb = NavigationToolbar2(fig.canvas)
    tb.zoom()
    _toggle_logs(ax, "log")
    seen = [_limits(ax)]
    _toggle_logs(ax, "linear")
    seen.append(_limits(ax))
    start, stop = ax.transData.transform([(2, 2), (5, 5)])
    MouseEvent("button_press_event", fig.canvas, *start, MouseButton.LEFT)._process()
    MouseEvent("button_release_event", fig.canvas, *stop, MouseButton.LEFT)._process()
    tb.home()
    seen.append(_limits(ax))
    _toggle_logs(ax, "log")
    return seen + [_limits(ax)]


def _draw(backend):
    fig = Figure(constrained_layout=True)
    if backend == "agg":
        FigureCanvasAgg(fig)
    else:
        module = __import__(f"matplotlib.backends.backend_{backend}", fromlist=["FigureCanvas"])
        module.FigureCanvas(fig)
    axes = fig.subplots(2, 2).ravel()
    before = [list(ax.get_position().bounds) for ax in axes]
    fig.canvas.draw()
    return [before, [list(ax.get_position().bounds) for ax in axes]]


def _pan_scene():
    fig = _canvas_figure()
    ax = fig.add_subplot()
    ax.plot(np.arange(10))
    ax.set_aspect("equal")
    return fig, ax


def _pan(key, start, end):
    fig, ax = _pan_scene()
    fig.canvas.draw()
    tb = NavigationToolbar2(fig.canvas)
    tb.pan()
    tb.press_pan(MouseEvent("button_press_event", fig.canvas, *start,
                            button=MouseButton.LEFT, key=key))
    tb.drag_pan(MouseEvent("motion_notify_event", fig.canvas, *end, button=MouseButton.LEFT,
                           buttons={MouseButton.LEFT}, key=key))
    tb.release_pan(MouseEvent("button_release_event", fig.canvas, *end,
                              button=MouseButton.LEFT, key=key))
    return _limits(ax)


def _events_scene(patch_visible, forward, twin_or_share):
    """The bottom and top axes and their twins, in that order."""
    fig = _canvas_figure()
    ax_b = fig.add_subplot()
    ax_t = fig.add_subplot(221, zorder=99)
    ax_t.set_forward_navigation_events(forward)
    ax_t.patch.set_visible(patch_visible)
    if twin_or_share == "share":
        ax_t_twin = fig.add_subplot(222)
        ax_t_twin.sharex(ax_t)
        ax_t_twin.sharey(ax_t)
        ax_b_twin = fig.add_subplot(223)
        ax_b_twin.sharex(ax_b)
        ax_b_twin.sharey(ax_b)
    else:
        ax_t_twin = ax_t.twinx()
        ax_b_twin = ax_b.twinx()
    for ax in [ax_t, ax_b]:
        ax.set_xlim(0, 10)
        ax.set_ylim(0, 10)
    return fig, (ax_b, ax_t, ax_b_twin, ax_t_twin)


def _events(tool, button, patch_visible, forward, twin_or_share, start, stop):
    fig, (ax_b, ax_t, ax_b_twin, ax_t_twin) = _events_scene(patch_visible, forward,
                                                            twin_or_share)
    fig.canvas.draw()
    button = MouseButton(button)
    tb = NavigationToolbar2(fig.canvas)
    getattr(tb, tool)()
    MouseEvent("button_press_event", fig.canvas, *start, button)._process()
    MouseEvent("motion_notify_event", fig.canvas, *stop, button, buttons={button})._process()
    MouseEvent("button_release_event", fig.canvas, *stop, button)._process()
    return [_limits(ax_t), _limits(ax_b), _limits(ax_t_twin), _limits(ax_b_twin)]


def _colorbar_scene(plot_func, orientation):
    fig = _canvas_figure()
    ax = fig.add_subplot()
    data = np.arange(12).reshape((4, 3))
    mappable = getattr(ax, plot_func)(data, vmin=0, vmax=10)
    colorbar = fig.colorbar(mappable, ax=ax, orientation=orientation)
    fig.canvas.draw()
    return fig, colorbar


def _colorbar(plot_func, orientation, tool, button, start, end):
    fig, colorbar = _colorbar_scene(plot_func, orientation)
    if not colorbar.ax.get_navigate():
        return [False]
    button = MouseButton(button)
    start_event = MouseEvent("button_press_event", fig.canvas, *start, button)
    drag_event = MouseEvent("motion_notify_event", fig.canvas, *end, button, buttons={button})
    stop_event = MouseEvent("button_release_event", fig.canvas, *end, button)
    tb = NavigationToolbar2(fig.canvas)
    if tool == "zoom":
        tb.zoom()
        tb.press_zoom(start_event)
        tb.drag_zoom(drag_event)
        tb.release_zoom(stop_event)
    else:
        tb.pan()
        tb.press_pan(start_event)
        tb.drag_pan(drag_event)
        tb.release_pan(stop_event)
    return [True, colorbar.vmin, colorbar.vmax]


def _width_height(figsize, dpi):
    fig = _canvas_figure(figsize=figsize, dpi=dpi)
    return [fig.bbox.height, list(fig.canvas.get_width_height())]


def test_uses_per_path():
    full = {"paths": 4, "transforms": 4, "offsets": 10, "facecolors": 2, "edgecolors": 2}
    variants = [{}, {"paths": 1}, {"paths": 0}, {"transforms": 1}, {"transforms": 0},
                *({"offsets": n} for n in range(10)), {"facecolors": 0}, {"edgecolors": 0},
                {"facecolors": 0, "edgecolors": 0}, {"facecolors": 1}]
    for variant in variants:
        args = {**full, **variant}
        probes.probe("RendererBase._iter_collection", _iter_collection, paths=args["paths"],
                     transforms=args["transforms"], offsets=args["offsets"],
                     facecolors=args["facecolors"], edgecolors=args["edgecolors"])


def test_canvas_helpers():
    probes.probe("FigureCanvasBase.get_default_filename", _default_filename, title="0:1/2<3")
    probes.probe("FigureCanvasBase.grab_mouse", _grab_mouse)
    for figsize in [[1, 2.03], [9.2, 5.0], [2.555, 1.0]]:
        probes.probe("FigureCanvasBase.get_width_height", _width_height, figsize=figsize,
                     dpi=100)


def test_location_event_position():
    for x, y in [(42, 24), (200, 100.01), (205.75, 2.0)]:
        probes.probe("LocationEvent", _location_event, x=x, y=y)
        # Floats: matplotlib formats Python integers with "%d", a distinction
        # of Python types that data coordinates from events never make.
        probes.probe("Axes.format_coord", _format_coord, x=float(x), y=float(y))
        probes.probe("Axes.format_coord", _format_coord, x=float(x), y=float(y), fmt="foo")


def test_location_event_position_twin():
    probes.probe("Axes.format_coord", _format_coord_twins, twins=["x", "y"])


def test_pick():
    probes.probe("pick_event", _pick, key="a")


def test_interactive_zoom():
    _, ax = _zoom_scene("logit")
    start, stop = _pixels(ax, (1e-6, 0.1)), _pixels(ax, (1 - 1e-5, 0.8))
    probes.probe("NavigationToolbar2.zoom", _zoom, scale="logit", start=start, stop=stop)


def test_widgetlock_zoompan():
    probes.probe("NavigationToolbar2.widgetlock", _widgetlock)


def test_interactive_colorbar():
    for plot_func in ["imshow", "contourf"]:
        for orientation in ["vertical", "horizontal"]:
            _, colorbar = _colorbar_scene(plot_func, orientation)
            ends = [(4, 0.5), (6, 0.5)]
            if orientation == "vertical":
                ends = [end[::-1] for end in ends]
            start, end = (_pixels(colorbar.ax, point) for point in ends)
            for tool, button in [("zoom", 1), ("zoom", 3), ("pan", 1), ("pan", 3)]:
                probes.probe("Colorbar.navigation", _colorbar, plot_func=plot_func,
                             orientation=orientation, tool=tool, button=button, start=start,
                             end=end)


def test_toolbar_home_restores_autoscale():
    probes.probe("NavigationToolbar2.home", _home)


def test_draw():
    for backend in ["agg", "svg", "ps", "pdf"]:
        probes.probe("FigureCanvasBase.draw", _draw, backend=backend)


PANS = [(None, (0.2, 0.2)), (None, (0.2, 0.5)), (None, (0.5, 0.2)), (None, (0.5, 0.5)),
        (None, (0.8, 0.25)), (None, (0.2, 0.25)), (None, (0.8, 0.85)), (None, (0.2, 0.85)),
        ("shift", (0.2, 0.4)), ("shift", (0.4, 0.2)), ("shift", (0.2, 0.25)),
        ("shift", (0.8, 0.25)), ("shift", (0.8, 0.9)), ("shift", (0.2, 0.85)),
        ("x", (0.2, 0.1)), ("y", (0.1, 0.2)), ("control", (0.2, 0.2)), ("control", (0.4, 0.2))]


def test_interactive_pan():
    for key, mouseend in PANS:
        _, ax = _pan_scene()
        probes.probe("NavigationToolbar2.pan", _pan, key=key, start=_pixels(ax, (0.5, 0.5)),
                     end=_pixels(ax, mouseend))


def test_interactive_pan_zoom_events():
    for tool in ["zoom", "pan"]:
        for button in [1, 3]:
            for patch_visible in [True, False]:
                for forward in [True, False, "auto"]:
                    for twin_or_share in ["twin", "share"]:
                        _, axes = _events_scene(patch_visible, forward, twin_or_share)
                        top = axes[1]
                        probes.probe("NavigationToolbar2.events", _events, tool=tool,
                                     button=button, patch_visible=patch_visible,
                                     forward=forward, twin_or_share=twin_or_share,
                                     start=_pixels(top, (1, 1)), stop=_pixels(top, (2, 2)))
