"""matplotlib's own tests of its widgets (``matplotlib/tests/test_widgets.py``):
selectors, check and radio buttons, text boxes, sliders and cursors, driven
by the synthetic mouse and key events of ``matplotlib.testing.widgets``.

Area ``mpl_widgets``. Every probe runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``), given as ``style``.
Unless noted, a widget works in the axes of
``matplotlib.testing.widgets.get_ax``: ``ax = Figure().add_subplot()`` with
an Agg canvas, ``ax.plot([0, 200], [0, 200])``, ``ax.set_aspect(1)`` and a
draw. Colours are recorded as RGBA lists (``to_rgba``).

Selector steps
--------------
The selector ops replay ``steps`` on ``tool``, in order:

* ``["press", [x, y], button]``, ``["move", [x, y], button]`` (``button``
  null for a motion without one) and ``["release", [x, y], button]``:
  ``MouseEvent._from_ax_coords`` at data point ``(x, y)`` of ``tool.ax``,
  processed.
* ``["drag", [x0, y0], [x1, y1], key]``: ``click_and_drag(tool, start, end,
  key)`` (``key`` null for none).
* ``["drag_handle", handle, [dx, dy], key]``: the same from where the handle
  ``"NE"``, ``"E"``, ``"W"`` or ``"SW"`` of the current ``extents`` is
  (``(x1, y1)``, ``(x1, ymid)``, ``(x0, ymid)``, ``(x0, y0)``), by ``(dx,
  dy)``, as the ``test_rectangle_resize*`` tests drag.
* ``["key", name]`` and ``["keyup", name]``: ``KeyEvent("key_press_event",
  canvas, name)`` (``"key_release_event"``), over no axes.
* ``["call", method, arg?]``: ``tool.method(arg)``, or ``tool.method(**arg)``
  for a dictionary; records ``null``, or ``"error"`` when it raises.
* ``["set", attribute, value]``: ``tool.attribute = value``; records ``null``
  or ``"error"``.
* ``["get", query]``: records the value of the query (listed with each op).

A selector op returns ``{"steps", "selects"}``: what the ``call``, ``set``
and ``get`` steps recorded, in order, and the arguments of every call of
``onselect``. The selector ``options`` are matplotlib's keyword arguments;
``useblit`` among them only changes how matplotlib draws.

Ops
---
``RectangleSelector`` / ``EllipseSelector``  {options, steps, aspect?} ->
    the selector made with ``options`` (``minspan_between: [[x0, y0], [x1,
    y1]]`` stands for ``minspanx, minspany = transData.transform((x1, y1)) -
    transData.transform((x0, y0))``) on axes whose aspect becomes ``aspect``
    when given; ``selects`` are ``[epress.xdata, epress.ydata,
    erelease.xdata, erelease.ydata]``. Queries: ``extents``, ``center``,
    ``corners``, ``edge_centers``, ``rotation``, ``geometry`` (its rows),
    ``geometry.shape``, ``geometry[:, 0]``, ``_state`` (sorted),
    ``_selection_completed``, ``get_visible``,
    ``_selection_artist.get_facecolor``,
    ``_handles_artists.get_markeredgecolor``, ``_handles_artists.get_alpha``,
    ``_corner_handles.artists.get_markerfacecolor`` and
    ``_corner_handles.artists.get_markeredgecolor`` (lists over the handle
    artists). (``test_rectangle_selector``, ``test_rectangle_minspan``,
    ``test_rectangle_drag``, ``test_rectangle_selector_set_props_handle_props``,
    ``test_rectangle_resize``, ``test_rectangle_add_state``,
    ``test_rectangle_resize_center``, ``test_rectangle_resize_square``,
    ``test_rectangle_resize_square_center``, ``test_rectangle_rotate``,
    ``test_rectangle_add_remove_set``,
    ``test_rectangle_resize_square_center_aspect``, ``test_ellipse``,
    ``test_rectangle_handles``, ``test_rectangle_selector_onselect``,
    ``test_rectangle_selector_ignore_outside``, ``test_selector_clear``,
    ``test_selector_clear_method``)
``SpanSelector``  {direction, options, steps, axes?, onmove?} -> the
    selector made with ``options`` on ``axes``: ``"get_ax"`` (default),
    ``"bound"`` (``Figure().subplots()`` with ``plot([10, 20], [10, 30])``,
    drawn), or ``"twin"``/``"inset"``: ``get_ax`` axes with ``set_aspect("auto")``
    and a ``twinx()``, the selector on those axes or on their
    ``inset_axes([0, 1, 1, 1], xlim=(0, 200), ylim=(0, 200))``. With
    ``onmove`` the selector gets an ``onmove_callback`` and the result a
    ``moves`` list. ``selects`` and ``moves`` are ``[vmin, vmax]``. Queries:
    ``extents``, ``direction``, ``_edge_handles.direction``,
    ``_edge_handles.positions``, ``snap_values``, ``_selection_completed``,
    ``get_visible``, ``_selection_artist.get_facecolor``,
    ``_handles_artists.get_color``, ``_handles_artists.get_alpha``,
    ``ax.get_xbound``, ``ax.get_ybound``. (``test_span_selector``,
    ``test_span_selector_onselect``, ``test_span_selector_ignore_outside``,
    ``test_span_selector_drag``, ``test_span_selector_direction``,
    ``test_span_selector_set_props_handle_props``, ``test_selector_clear``,
    ``test_selector_clear_method``, ``test_span_selector_add_state``,
    ``test_span_selector_bound``, ``test_span_selector_snap``,
    ``test_span_selector_extents``)
``SpanSelector._snap``  {values, snap_values} -> each value snapped to the
    nearest of ``snap_values`` (``test_snapping_values_span_selector``).
``LassoSelector``  {options, steps} -> ``selects`` are the vertex lists.
    Queries: ``_selection_artist.get_color``, ``_selection_artist.get_alpha``.
    (``test_lasso_selector``, ``test_lasso_selector_set_props``)
``Lasso``  {props?, set_color?, set_alpha?} -> ``{color, linestyle, lw,
    alpha}`` of ``Lasso(ax, (100, 100), onselect, props=props).line``, after
    ``set_color`` and ``set_alpha`` when given (``test_lasso_set_props``).
``PolygonSelector``  {options, steps, xlim?, ylim?} -> ``selects`` are the
    vertex lists, on axes whose limits become ``xlim`` and ``ylim`` when
    given. Queries: ``verts``, ``_box.extents``, ``_selection_completed``,
    ``get_visible``, ``_selection_artist.get_visible``,
    ``_selection_artist.get_xydata``, ``_selection_artist.get_color``,
    ``_selection_artist.get_alpha``, ``_handles_artists.get_color``,
    ``_handles_artists.get_alpha``, ``_handles_artists.get_marker``,
    ``_handles_artists.get_markerfacecolor``,
    ``_handles_artists.get_markeredgecolor``,
    ``_box._selection_artist.get_linestyle`` (by name),
    ``_box._selection_artist.get_facecolor``,
    ``_box._selection_artist.get_edgecolor``,
    ``_box._handles_artists.get_marker``,
    ``_box._handles_artists.get_markerfacecolor``,
    ``_box._handles_artists.get_markeredgecolor``. (``test_polygon_selector``,
    ``test_polygon_selector_set_props_handle_props``,
    ``test_polygon_selector_remove``,
    ``test_polygon_selector_remove_first_point``,
    ``test_polygon_selector_redraw``, ``test_polygon_selector_verts_setter``,
    ``test_polygon_selector_box``,
    ``test_polygon_selector_box_props_handle_props``,
    ``test_polygon_selector_clear_method``)
``CheckButtons``  {labels, actives} -> ``{status, after_set_active, checked,
    after_clear, checked_after_clear, disconnected}``: ``get_status()``
    before and after ``set_active(0)``, ``get_checked_labels()`` then, both
    again after ``clear()``, and whether ``disconnect`` of a callback id
    succeeds (``test_CheckButtons``).
``CheckButtons.set_active``  {labels, actives, index} -> ``get_status()``
    after ``set_active(index)``, or the error (``test_CheckButtons``).
``RadioButtons``  {labels} -> ``[[value_selected, index_selected] after
    set_active(1), the same after clear()]`` (``test_RadioButtons``).
``RadioButtons.activecolor``  {labels, activecolor, facecolor} -> the face
    colours of ``_buttons`` made with ``activecolor`` and ``radio_props=
    {"facecolor": facecolor}`` (``test_radio_button_active_conflict``).
``_Buttons.on_clicked``  {widget} -> how often ``on_clicked`` callbacks of a
    one-button ``RadioButtons`` or ``CheckButtons`` run after a press at its
    first button, axes point ``(0.15, 0.5)`` (``test__buttons_callbacks``).
``TextBox``  {toolbar} -> ``[text, text after set_val("x**2"), changes then,
    submits after begin_typing() and stop_typing(), changes after a click
    and the keys "+" and "5"]`` with rcParams ``toolbar`` set
    (``test_TextBox``).
``Slider``  {valmin, valmax, valinit, valstep?, orientation?, set_val?,
    reset?} -> ``{val, poly}``: the value of ``Slider(ax, "", ...)`` on
    ``Figure().subplots()`` after ``set_val`` and ``reset()`` when asked, and
    the bounds of ``poly`` in axes coordinates
    (``test_slider_valmin_valmax``, ``test_slider_valstep_snapping``,
    ``test_slider_horizontal_vertical``, ``test_slider_reset``).
``Slider.slidermin`` / ``Slider.slidermax``  {other, valinit} -> the value
    of a 0 to 24 slider made with ``valinit`` and the slider of value
    ``other`` as its ``slidermin`` (``slidermax``)
    (``test_slider_slidermin_slidermax``).
``RangeSlider``  {orientation, valinit, steps} -> after construction and
    after each step (``["set_val", [a, b]]`` or ``["reset"]``): ``{val,
    handles, poly}``, the handle positions along the slider and the corners
    of ``poly`` in axes coordinates (``test_range_slider``,
    ``test_range_slider_same_init_values``).
``MultiCursor``  {horizOn, vertOn, useblit} -> the line counts and, after a
    motion at ``(0.5, 0.25)`` of the first of two axes sharing x, after
    toggling both switches and moving there again, and after a motion over
    the other axes (not in the cursor): the x data of the vertical lines,
    the y data of the horizontal lines and how many of each are visible;
    the cursor spans the first axes and the axes of a second figure, and
    with ``useblit`` blits its animated lines (``test_MultiCursor``, which
    turns blitting off, and the same with blitting).
``Cursor.useblit``  {rects} -> ``[useblit, animated]`` of ``Cursor(ax,
    useblit=True)`` on the first of axes added at ``rects``: whether the
    cursor blits, which it does not over axes overlapping others, and
    whether its vertical line is animated
    (``test_cursor_overlapping_axes_blitting_warning``).

The image tests and the ``fig_test`` sides of the ``check_figures_equal``
tests are figure cases of the ``mpl_rendering`` group;
``test_save_blitted_widget_as_pdf`` and ``test_parent_axes_removal``, which
draw and save, are cases of ``mpl_tests_widgets``.

Left out: ``test_axeswidget_del_on_failed_init`` (Python's ``TypeError`` for
an unknown keyword and unraisable exceptions), ``test_tool_line_handle``
(plt-rs keeps ``ToolLineHandles`` inside the span selector, whose handles
the ``SpanSelector`` queries read), ``test_span_selector_animated_artists_callback``
(the Qt backend), ``test_slider_slidermin_slidermax_invalid`` (a
``slidermin`` that is not a slider cannot be written in Rust), the warning
of ``test_cursor_overlapping_axes_blitting_warning`` (plt-rs has no
warnings; the probe records that the cursor does not blit), the
``with_deprecated_canvas`` cases of ``test_MultiCursor``
(deprecation machinery), and of the tests ported, the ``TypeError`` of
``CheckButtons.set_active(1, state="invalid")`` and the ``ValueError`` of an
invalid ``Rectangle.rotation_point`` (both typed in plt-rs).
"""

import copy
import warnings

import matplotlib as mpl
import matplotlib.colors as mcolors
import numpy as np
from matplotlib import widgets
from matplotlib.backend_bases import KeyEvent, MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.testing.widgets import click_and_drag

from spec_support import ProbeSet

probes = ProbeSet("mpl_widgets")

CLASSIC = ["classic", "_classic_test_patch"]


def probe(op, fn, tol=None, **args):
    def styled(style, **kw):
        # The widgets change the property dictionaries they are given.
        with mpl.style.context(style):
            return fn(**copy.deepcopy(kw))

    return probes.probe(op, styled, tol=tol, style=CLASSIC, **args)


def _figure():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig


def _get_ax():
    """``matplotlib.testing.widgets.get_ax`` without pyplot."""
    ax = _figure().add_subplot()
    ax.plot([0, 200], [0, 200])
    ax.set_aspect(1.0)
    ax.figure.canvas.draw()
    return ax


LINESTYLE_NAMES = {"-": "solid", "--": "dashed", "-.": "dashdot", ":": "dotted"}


def _plain(path, value):
    """``value`` of the query ``path`` as the spec records it."""
    if path.endswith("color"):
        if isinstance(value, (list, tuple)) and value and not isinstance(value[0], float):
            return [list(mcolors.to_rgba(c)) for c in value]
        return list(mcolors.to_rgba(value))
    if path.endswith("get_linestyle"):
        return LINESTYLE_NAMES.get(value, "none")
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, set):
        return sorted(value)
    return value


def _resolve(obj, path):
    """The value at the dotted ``path`` of ``obj``: getters are called and
    lists along the path are mapped over."""
    value = obj
    for name in path.split("."):
        if isinstance(value, (list, tuple)):
            value = [_resolve(item, name) for item in value]
            continue
        value = getattr(value, name)
        if name.startswith("get_"):
            value = value()
    return value


QUERIES = {
    "geometry.shape": lambda tool: list(tool.geometry.shape),
    "geometry[:, 0]": lambda tool: tool.geometry[:, 0].tolist(),
}


def _query(tool, path):
    if path in QUERIES:
        return QUERIES[path](tool)
    value = _resolve(tool, path)
    if isinstance(value, list) and path.split(".")[-1].endswith("color"):
        return [_plain(path, v) for v in value]
    return _plain(path, value)


def _attempt(fn):
    try:
        fn()
    except Exception:  # noqa: BLE001 - the error is what is recorded
        return "error"
    return None


def _handle_at(extents, name):
    """Where the handle ``name`` of a rectangle of ``extents`` is, as the
    ``test_rectangle_resize*`` tests compute it."""
    x0, x1, y0, y1 = extents
    ymid = y0 + (y1 - y0) / 2
    return {"NE": (x1, y1), "E": (x1, ymid), "W": (x0, ymid), "SW": (x0, y0)}[name]


def _replay(tool, steps):
    ax = tool.ax
    canvas = ax.figure.canvas
    recorded = []
    for kind, *rest in steps:
        if kind in ("press", "move", "release"):
            name = {"press": "button_press_event", "move": "motion_notify_event",
                    "release": "button_release_event"}[kind]
            xy, button = rest
            MouseEvent._from_ax_coords(name, ax, xy, button)._process()
        elif kind == "drag":
            start, end, key = rest
            click_and_drag(tool, start=start, end=end, key=key)
        elif kind == "drag_handle":
            name, (dx, dy), key = rest
            start = _handle_at(tool.extents, name)
            click_and_drag(tool, start=start, end=(start[0] + dx, start[1] + dy), key=key)
        elif kind in ("key", "keyup"):
            name = "key_press_event" if kind == "key" else "key_release_event"
            KeyEvent(name, canvas, rest[0])._process()
        elif kind == "call":
            method, *args = rest
            if args and isinstance(args[0], dict):
                recorded.append(_attempt(lambda: getattr(tool, method)(**args[0])))
            else:
                recorded.append(_attempt(lambda: getattr(tool, method)(*args)))
        elif kind == "set":
            attribute, value = rest
            recorded.append(_attempt(lambda: setattr(tool, attribute, value)))
        elif kind == "get":
            recorded.append(_query(tool, rest[0]))
        else:
            raise ValueError(f"unknown step {kind!r}")
    return recorded


def _region(kind, options, steps, aspect=None):
    ax = _get_ax()
    if aspect is not None:
        ax.set_aspect(aspect)
    options = dict(options)
    if "minspan_between" in options:
        start, end = options.pop("minspan_between")
        options["minspanx"], options["minspany"] = (
            ax.transData.transform(end) - ax.transData.transform(start)).tolist()
    selects = []

    def onselect(press, release):
        selects.append([press.xdata, press.ydata, release.xdata, release.ydata])

    selector = widgets.RectangleSelector if kind == "rectangle" else widgets.EllipseSelector
    tool = selector(ax, onselect=onselect, **options)
    return {"steps": _replay(tool, steps), "selects": selects}


def _rectangle(options, steps, aspect=None):
    return _region("rectangle", options, steps, aspect)


def _ellipse(options, steps, aspect=None):
    return _region("ellipse", options, steps, aspect)


def _span_axes(axes):
    if axes == "bound":
        fig = _figure()
        ax = fig.subplots()
        ax.plot([10, 20], [10, 30])
        fig.canvas.draw()
        return ax
    ax = _get_ax()
    if axes in ("twin", "inset"):
        ax.set_aspect("auto")
        ax.twinx()
        child = ax.inset_axes([0, 1, 1, 1], xlim=(0, 200), ylim=(0, 200))
        return child if axes == "inset" else ax
    return ax


def _span(direction, options, steps, axes="get_ax", onmove=False):
    ax = _span_axes(axes)
    selects, moves = [], []
    if "snap_values" in options:
        options["snap_values"] = np.array(options["snap_values"])
    if onmove:
        options["onmove_callback"] = lambda vmin, vmax: moves.append([vmin, vmax])
    tool = widgets.SpanSelector(ax, lambda vmin, vmax: selects.append([vmin, vmax]),
                                direction, **options)
    result = {"steps": _replay(tool, steps), "selects": selects}
    if onmove:
        result["moves"] = moves
    return result


def _snap(values, snap_values):
    return widgets.SpanSelector._snap(np.array(values), np.array(snap_values))


def _lasso_selector(options, steps):
    ax = _get_ax()
    selects = []
    tool = widgets.LassoSelector(ax, onselect=lambda verts: selects.append(verts), **options)
    return {"steps": _replay(tool, steps), "selects": selects}


def _lasso(props=None, set_color=None, set_alpha=None):
    ax = _get_ax()
    line = widgets.Lasso(ax, (100, 100), lambda verts: None, props=props).line
    if set_color is not None:
        line.set_color(set_color)
    if set_alpha is not None:
        line.set_alpha(set_alpha)
    return {"color": list(mcolors.to_rgba(line.get_color())),
            "linestyle": LINESTYLE_NAMES[line.get_linestyle()], "lw": line.get_lw(),
            "alpha": line.get_alpha()}


def _polygon(options, steps, xlim=None, ylim=None):
    ax = _get_ax()
    if xlim is not None:
        ax.set(xlim=xlim, ylim=ylim)
    selects = []
    tool = widgets.PolygonSelector(ax, onselect=lambda verts: selects.append(verts), **options)
    return {"steps": _replay(tool, steps), "selects": selects}


def press(xy, button=1):
    return ["press", xy, button]


def move(xy, button=None):
    return ["move", xy, button]


def release(xy, button=1):
    return ["release", xy, button]


def drag(start, end, key=None):
    return ["drag", start, end, key]


def get(*queries):
    return [["get", query] for query in queries]


def place(xy):
    """``polygon_place_vertex``."""
    return [move(xy), press(xy), release(xy)]


def unplace(xy):
    """``polygon_remove_vertex``."""
    return [move(xy), press(xy, 3), release(xy, 3)]


# --- rectangle and ellipse selectors ---------------------------------------


def test_rectangle_selector():
    for options in [{}, {"useblit": True, "button": 1},
                    {"minspanx": 10, "minspany": 10, "spancoords": "pixels"},
                    {"props": {"fill": True}}]:
        probe("RectangleSelector", _rectangle, options=options,
              steps=[press([100, 100]), move([199, 199], 1), release([250, 250]),
                     *get("geometry")])


def test_rectangle_minspan():
    for spancoords in ["data", "pixels"]:
        for minspanx, x1 in [[0, 10], [1, 10.5], [1, 11]]:
            for minspany, y1 in [[0, 10], [1, 10.5], [1, 11]]:
                x0, y0 = 10, 10
                options = {"interactive": True, "spancoords": spancoords}
                if spancoords == "pixels":
                    options["minspan_between"] = [[x0, y0], [x1, y1]]
                else:
                    options.update(minspanx=minspanx, minspany=minspany)
                probe("RectangleSelector", _rectangle, options=options, steps=[
                    drag([x0, x1], [y0, y1]), *get("_selection_completed"),
                    drag([20, 20], [30, 30]), *get("_selection_completed"),
                    drag([x0, y0], [x1, y1]), *get("_selection_completed")])


def test_rectangle_drag():
    for drag_from_anywhere in [True, False]:
        probe("RectangleSelector", _rectangle,
              options={"interactive": True, "drag_from_anywhere": drag_from_anywhere},
              steps=[drag([10, 10], [90, 120]), *get("center"),
                     drag([25, 15], [35, 25]), *get("center"),
                     drag([175, 185], [185, 195]), *get("center")])


def test_rectangle_selector_set_props_handle_props():
    probe("RectangleSelector", _rectangle,
          options={"interactive": True, "props": {"facecolor": "b", "alpha": 0.2},
                   "handle_props": {"alpha": 0.5}},
          steps=[drag([0, 10], [100, 120]),
                 *get("_selection_artist.get_facecolor"),
                 ["call", "set_props", {"facecolor": "r", "alpha": 0.3}],
                 *get("_selection_artist.get_facecolor",
                      "_handles_artists.get_markeredgecolor", "_handles_artists.get_alpha"),
                 ["call", "set_handle_props", {"markeredgecolor": "r", "alpha": 0.3}],
                 *get("_handles_artists.get_markeredgecolor", "_handles_artists.get_alpha")])


def handle(name, dx, dy, key=None):
    """A drag of the handle ``name`` by ``(dx, dy)``, then the extents."""
    return [["drag_handle", name, [dx, dy], key], *get("extents")]


#: The handle drags of ``test_rectangle_resize``.
RESIZES = [("NE", 10, 5), ("E", 10, 0), ("W", 15, 0), ("SW", 20, 25)]
#: The handle drags of the ``test_rectangle_resize_*`` tests that follow.
RESIZES_BOTH_WAYS = [("NE", 10, 5), ("E", 10, 0), ("E", -20, 0), ("W", 15, 0), ("W", -25, 0),
                     ("SW", 20, 25)]


def test_rectangle_resize():
    probe("RectangleSelector", _rectangle, options={"interactive": True}, steps=[
        drag([10, 10], [100, 120]), *get("extents"),
        *[step for name, dx, dy in RESIZES for step in handle(name, dx, dy)]])


def test_rectangle_add_state():
    probe("RectangleSelector", _rectangle, options={"interactive": True}, steps=[
        drag([70, 65], [125, 130]),
        ["call", "add_state", "unsupported_state"], ["call", "add_state", "clear"],
        ["call", "add_state", "move"], ["call", "add_state", "square"],
        ["call", "add_state", "center"], *get("_state")])


def _resize_with(state, key, end, moves=RESIZES_BOTH_WAYS):
    """``test_rectangle_resize_<state>``: a rectangle from ``(70, 65)`` to
    ``end``, then ``moves`` with ``state`` added, or with ``key`` held."""
    state_steps = [["call", "add_state", state]] if key is None else []
    return [drag([70, 65], end), *get("extents"), *state_steps,
            *[step for name, dx, dy in moves for step in handle(name, dx, dy, key)]]


def test_rectangle_resize_center():
    for add_state in [True, False]:
        probe("RectangleSelector", _rectangle, options={"interactive": True},
              steps=_resize_with("center", None if add_state else "control", [125, 130]))


def test_rectangle_resize_square():
    for add_state in [True, False]:
        probe("RectangleSelector", _rectangle, options={"interactive": True},
              steps=_resize_with("square", None if add_state else "shift", [120, 115]))


def test_rectangle_resize_square_center():
    moves = [*RESIZES_BOTH_WAYS[:3], ("W", 5, 0), *RESIZES_BOTH_WAYS[4:]]
    probe("RectangleSelector", _rectangle, options={"interactive": True}, tol=1e-9, steps=[
        drag([70, 65], [120, 115]), ["call", "add_state", "square"],
        ["call", "add_state", "center"], *get("extents"),
        *[step for name, dx, dy in moves for step in handle(name, dx, dy)]])


def test_rectangle_rotate():
    for op, fn in [("RectangleSelector", _rectangle), ("EllipseSelector", _ellipse)]:
        probe(op, fn, options={"interactive": True}, tol=1e-6, steps=[
            drag([100, 100], [130, 140]), *get("extents", "_state"),
            ["key", "r"], *get("_state"),
            drag([130, 140], [120, 145]), ["key", "r"], *get("_state", "extents", "rotation"),
            ["set", "rotation", 45], *get("rotation", "corners"),
            drag([110, 145], [110, 160]), *get("extents")])


def test_rectangle_add_remove_set():
    steps = [drag([100, 100], [130, 140]), *get("extents", "_state")]
    for state in ["rotate", "square", "center"]:
        steps += [["call", "add_state", state], *get("_state"),
                  ["call", "remove_state", state], *get("_state")]
    probe("RectangleSelector", _rectangle, options={"interactive": True}, steps=steps)


def test_rectangle_resize_square_center_aspect():
    for use_data_coordinates in [False, True]:
        probe("RectangleSelector", _rectangle, aspect=0.8, tol=1e-9,
              options={"interactive": True, "use_data_coordinates": use_data_coordinates},
              steps=[drag([70, 65], [120, 115]), *get("extents"),
                     ["call", "add_state", "square"], ["call", "add_state", "center"],
                     *handle("NE", 10, 0)])


def test_ellipse():
    probe("EllipseSelector", _ellipse, options={"grab_range": 10, "interactive": True}, steps=[
        ["set", "extents", [100, 150, 100, 150]],
        drag([125, 125], [145, 145]), *get("extents"),
        drag([100, 100], [125, 125], "control"), *get("extents"),
        drag([10, 10], [35, 30], "shift"), *get("extents"),
        drag([100, 100], [125, 130], "ctrl+shift"), *get("extents"),
        *get("geometry.shape", "geometry[:, 0]")])


def test_rectangle_handles():
    probe("RectangleSelector", _rectangle,
          options={"grab_range": 10, "interactive": True,
                   "handle_props": {"markerfacecolor": "r", "markeredgecolor": "b"}},
          steps=[["set", "extents", [100, 150, 100, 150]],
                 *get("corners", "extents", "edge_centers", "extents"),
                 drag([100, 100], [120, 120]), *get("extents"),
                 drag([132, 132], [120, 120]), *get("extents"),
                 drag([10, 10], [100, 100]), *get("extents"),
                 *get("_corner_handles.artists.get_markerfacecolor",
                      "_corner_handles.artists.get_markeredgecolor")])


def test_rectangle_selector_onselect():
    for interactive in [True, False]:
        probe("RectangleSelector", _rectangle, options={"interactive": interactive},
              steps=[drag([100, 110], [150, 120]), *get("extents"),
                     drag([10, 100], [10, 100])])


def test_rectangle_selector_ignore_outside():
    for ignore_event_outside in [True, False]:
        probe("RectangleSelector", _rectangle,
              options={"ignore_event_outside": ignore_event_outside},
              steps=[drag([100, 110], [150, 120]), *get("extents"),
                     drag([150, 150], [160, 160]), *get("extents")])


def test_selector_clear():
    for ignore_event_outside in [False, True]:
        steps = [drag([10, 10], [100, 120]), drag([130, 130], [130, 130]),
                 *get("_selection_completed")]
        if ignore_event_outside:
            steps += [["key", "escape"], *get("_selection_completed")]
        options = {"interactive": True, "ignore_event_outside": ignore_event_outside}
        probe("SpanSelector", _span, direction="horizontal", options=options, steps=steps)
        probe("RectangleSelector", _rectangle, options=options, steps=steps)


def test_selector_clear_method():
    steps = [drag([10, 10], [100, 120]),
             *get("_selection_completed", "get_visible", "extents"),
             ["call", "clear"], *get("_selection_completed", "get_visible"),
             drag([10, 10], [50, 120]), *get("_selection_completed", "get_visible", "extents")]
    probe("SpanSelector", _span, direction="horizontal",
          options={"interactive": True, "ignore_event_outside": True}, steps=steps)
    probe("RectangleSelector", _rectangle, options={"interactive": True}, steps=steps)


# --- span selectors ----------------------------------------------------------


def test_span_selector():
    for direction, onmove, options in [
            ("horizontal", False, {"minspan": 10, "useblit": True}),
            ("vertical", True, {"button": 1}),
            ("horizontal", False, {"props": {"fill": True}}),
            ("horizontal", False, {"interactive": True})]:
        for axes in ["twin", "inset"]:
            probe("SpanSelector", _span, direction=direction, options=options, axes=axes,
                  onmove=onmove,
                  steps=[press([100, 100]), move([199, 199], 1), release([250, 250])])


def test_span_selector_onselect():
    for interactive in [True, False]:
        probe("SpanSelector", _span, direction="horizontal",
              options={"interactive": interactive},
              steps=[drag([100, 100], [150, 100]), *get("extents"),
                     drag([10, 100], [10, 100])])


def test_span_selector_ignore_outside():
    for ignore_event_outside in [True, False]:
        probe("SpanSelector", _span, direction="horizontal", onmove=True,
              options={"ignore_event_outside": ignore_event_outside},
              steps=[drag([100, 100], [125, 125]), *get("extents"),
                     drag([150, 150], [160, 160]), *get("extents")])


def test_span_selector_drag():
    for drag_from_anywhere in [True, False]:
        probe("SpanSelector", _span, direction="horizontal",
              options={"interactive": True, "drag_from_anywhere": drag_from_anywhere},
              steps=[drag([10, 10], [100, 120]), *get("extents"),
                     drag([25, 15], [35, 25]), *get("extents"),
                     drag([175, 185], [185, 195]), *get("extents")])


def test_span_selector_direction():
    probe("SpanSelector", _span, direction="horizontal", options={"interactive": True},
          steps=[*get("direction", "_edge_handles.direction"),
                 ["set", "direction", "vertical"],
                 *get("direction", "_edge_handles.direction"),
                 ["set", "direction", "invalid_string"]])
    probe("SpanSelector", _span, direction="invalid_direction", options={}, steps=[])


def test_span_selector_set_props_handle_props():
    probe("SpanSelector", _span, direction="horizontal",
          options={"interactive": True, "props": {"facecolor": "b", "alpha": 0.2},
                   "handle_props": {"alpha": 0.5}},
          steps=[drag([0, 10], [100, 120]),
                 *get("_selection_artist.get_facecolor"),
                 ["call", "set_props", {"facecolor": "r", "alpha": 0.3}],
                 *get("_selection_artist.get_facecolor", "_handles_artists.get_color",
                      "_handles_artists.get_alpha"),
                 ["call", "set_handle_props", {"color": "r", "alpha": 0.3}],
                 *get("_handles_artists.get_color", "_handles_artists.get_alpha")])


def test_span_selector_add_state():
    probe("SpanSelector", _span, direction="horizontal", options={"interactive": True},
          steps=[["call", "add_state", "unsupported_state"], ["call", "add_state", "center"],
                 ["call", "add_state", "square"], ["call", "add_state", "move"]])


def test_span_selector_bound():
    for direction in ["horizontal", "vertical"]:
        probe("SpanSelector", _span, direction=direction, options={"interactive": True},
              axes="bound",
              steps=[*get("ax.get_xbound", "ax.get_ybound", "_edge_handles.positions"),
                     drag([10.5, 11.5], [11, 13]),
                     *get("ax.get_xbound", "ax.get_ybound", "_edge_handles.positions")])


def test_snapping_values_span_selector():
    probe("SpanSelector._snap", _snap,
          values=[-0.1, 0.1, 0.2, 0.5, 0.6, 0.7, 0.9, 4.76, 5.0, 5.5],
          snap_values=np.linspace(0, 5, 11).tolist())


def test_span_selector_snap():
    probe("SpanSelector", _span, direction="horizontal",
          options={"snap_values": (np.arange(50) * 4).tolist()},
          steps=[["set", "extents", [17, 35]], *get("extents"),
                 ["set", "snap_values", None], *get("snap_values"),
                 ["set", "extents", [17, 35]], *get("extents")])


def test_span_selector_extents():
    probe("SpanSelector", _span, direction="horizontal",
          options={"ignore_event_outside": True},
          steps=[["set", "extents", [5, 10]], *get("extents", "_selection_completed"),
                 drag([12, 14], [20, 14]), *get("extents")])


# --- lasso selectors ---------------------------------------------------------


def test_lasso_selector():
    for options in [{}, {"useblit": False, "props": {"color": "red"}},
                    {"useblit": True, "button": 1}]:
        probe("LassoSelector", _lasso_selector, options=options,
              steps=[press([100, 100]), move([125, 125], 1), release([150, 150])])


def test_lasso_selector_set_props():
    probe("LassoSelector", _lasso_selector, options={"props": {"color": "b", "alpha": 0.2}},
          steps=[*get("_selection_artist.get_color", "_selection_artist.get_alpha"),
                 ["call", "set_props", {"color": "r", "alpha": 0.3}],
                 *get("_selection_artist.get_color", "_selection_artist.get_alpha")])


def test_lasso_set_props():
    probe("Lasso", _lasso)
    probe("Lasso", _lasso, props={"linestyle": "-", "color": "darkblue", "alpha": 0.2, "lw": 1},
          set_color="r", set_alpha=0.3)


# --- polygon selectors -------------------------------------------------------


TRIANGLE = [[50, 50], [150, 50], [50, 150]]


def _closed(verts):
    return [step for xy in [*verts, verts[0]] for step in place(xy)]


def _polygon_sequences():
    """The event sequences of ``test_polygon_selector``."""
    return [
        _closed(TRIANGLE),
        [*place([50, 50]), *place([150, 50]), ["key", "control"], move([50, 50]),
         press([50, 50]), move([75, 50]), release([75, 50]), ["keyup", "control"],
         *place([50, 150]), *place([75, 50])],
        [*place([50, 50]), *place([150, 50]), ["key", "shift"], move([100, 100]),
         press([100, 100]), move([100, 125]), release([100, 125]), ["keyup", "shift"],
         *place([50, 150]), *place([50, 75])],
        [*place([60, 50]), *place([150, 50]), *place([50, 150]), *place([60, 50]),
         move([60, 50]), press([60, 50]), move([85, 50]), release([85, 50])],
        [*_closed(TRIANGLE), ["key", "shift"], move([100, 100]), press([100, 100]),
         move([125, 125]), release([125, 125]), ["keyup", "shift"]],
        [["key", "control"], move([100, 100]), press([100, 100]), move([125, 125]),
         release([125, 125]), ["keyup", "control"], ["key", "shift"], move([100, 100]),
         press([100, 100]), move([125, 125]), release([125, 125]), ["keyup", "shift"],
         *_closed(TRIANGLE)],
        [*place([50, 50]), *place([250, 50]), ["key", "escape"], ["keyup", "escape"],
         *_closed(TRIANGLE)],
    ]


def test_polygon_selector():
    for draw_bounding_box in [False, True]:
        for steps in _polygon_sequences():
            probe("PolygonSelector", _polygon,
                  options={"draw_bounding_box": draw_bounding_box}, steps=steps)


def test_polygon_selector_set_props_handle_props():
    for draw_bounding_box in [False, True]:
        probe("PolygonSelector", _polygon,
              options={"props": {"color": "b", "alpha": 0.2}, "handle_props": {"alpha": 0.5},
                       "draw_bounding_box": draw_bounding_box},
              steps=[*_closed(TRIANGLE),
                     *get("_selection_artist.get_color", "_selection_artist.get_alpha"),
                     ["call", "set_props", {"color": "r", "alpha": 0.3}],
                     *get("_selection_artist.get_color", "_selection_artist.get_alpha",
                          "_handles_artists.get_color", "_handles_artists.get_alpha"),
                     ["call", "set_handle_props", {"color": "r", "alpha": 0.3}],
                     *get("_handles_artists.get_color", "_handles_artists.get_alpha")])


def test_polygon_selector_remove():
    for idx in [1, 2, 3]:
        for draw_bounding_box in [False, True]:
            sequence = [place(xy) for xy in [*TRIANGLE, TRIANGLE[0]]]
            sequence.insert(idx, place([200, 200]))
            sequence.append(unplace([200, 200]))
            probe("PolygonSelector", _polygon, options={"draw_bounding_box": draw_bounding_box},
                  steps=[step for events in sequence for step in events])


def test_polygon_selector_remove_first_point():
    for draw_bounding_box in [False, True]:
        probe("PolygonSelector", _polygon, options={"draw_bounding_box": draw_bounding_box},
              steps=[*_closed(TRIANGLE), *unplace(TRIANGLE[0])])


def test_polygon_selector_redraw():
    for draw_bounding_box in [False, True]:
        probe("PolygonSelector", _polygon, options={"draw_bounding_box": draw_bounding_box},
              steps=[*_closed(TRIANGLE), *unplace(TRIANGLE[1]), *unplace(TRIANGLE[2]),
                     *place(TRIANGLE[1]), *get("verts")])


def test_polygon_selector_verts_setter():
    for draw_bounding_box in [False, True]:
        probe("PolygonSelector", _polygon, options={"draw_bounding_box": draw_bounding_box},
              steps=[["set", "verts", [[0.1, 0.4], [0.5, 0.9], [0.3, 0.2]]], *get("verts")])


def test_polygon_selector_box():
    probe("PolygonSelector", _polygon, options={"draw_bounding_box": True}, xlim=[-10, 50],
          ylim=[-10, 50], tol=1e-9, steps=[
              *_closed([[20, 0], [0, 20], [20, 40], [40, 20]]),
              press([40, 40]), move([20, 20]), release([20, 20]), *get("verts"),
              press([10, 10]), move([30, 30]), release([30, 30]),
              *get("verts", "_box.extents"),
              press([30, 20], 3), release([30, 20], 3), *get("verts", "_box.extents")])


def test_polygon_selector_box_props_handle_props():
    probe("PolygonSelector", _polygon, options={
        "draw_bounding_box": True, "props": {"color": "r"},
        "handle_props": {"marker": "o", "markerfacecolor": "w", "markeredgecolor": "r"},
        "box_props": {"linestyle": ":", "facecolor": "c", "edgecolor": "b"},
        "box_handle_props": {"markeredgecolor": "y"}},
        steps=[*_closed(TRIANGLE),
               *get("_selection_artist.get_color", "_handles_artists.get_marker",
                    "_handles_artists.get_markerfacecolor",
                    "_handles_artists.get_markeredgecolor",
                    "_box._selection_artist.get_linestyle",
                    "_box._selection_artist.get_facecolor",
                    "_box._selection_artist.get_edgecolor",
                    "_box._handles_artists.get_marker",
                    "_box._handles_artists.get_markerfacecolor",
                    "_box._handles_artists.get_markeredgecolor")])


def test_polygon_selector_clear_method():
    steps = []
    for verts in ([[50, 50], [150, 50], [50, 150], [50, 50]],
                  [[50, 50], [100, 50], [50, 150], [50, 50]]):
        steps += [step for xy in verts for step in place(xy)]
        steps += [*get("_selection_completed", "get_visible", "_selection_artist.get_visible",
                       "_selection_artist.get_xydata"),
                  ["call", "clear"], *get("_selection_completed", "_selection_artist.get_xydata")]
    probe("PolygonSelector", _polygon, options={}, steps=steps)


# --- check and radio buttons, text boxes -------------------------------------


def _check_buttons(labels, actives):
    check = widgets.CheckButtons(_get_ax(), labels, actives)
    status = check.get_status()
    check.set_active(0)
    after, checked = check.get_status(), check.get_checked_labels()
    check.clear()
    cid = check.on_clicked(lambda label: None)
    return {"status": status, "after_set_active": after, "checked": checked,
            "after_clear": check.get_status(), "checked_after_clear": check.get_checked_labels(),
            "disconnected": check.disconnect(cid) is None}


def _check_set_active(labels, actives, index):
    check = widgets.CheckButtons(_get_ax(), labels, actives)
    check.set_active(index=index)
    return check.get_status()


def test_CheckButtons():
    labels, actives = ["a", "b", "c"], [True, False, True]
    probe("CheckButtons", _check_buttons, labels=labels, actives=actives)
    for index in [-1, 3, 8, 1]:
        probe("CheckButtons.set_active", _check_set_active, labels=labels, actives=actives,
              index=index)


def _radio_buttons(labels):
    radio = widgets.RadioButtons(_get_ax(), labels)
    radio.set_active(1)
    selected = [radio.value_selected, radio.index_selected]
    radio.clear()
    return [selected, [radio.value_selected, radio.index_selected]]


def test_RadioButtons():
    probe("RadioButtons", _radio_buttons, labels=["Radio 1", "Radio 2", "Radio 3"])


def _radio_activecolor(labels, activecolor, facecolor):
    rb = widgets.RadioButtons(_get_ax(), labels, activecolor=activecolor,
                              radio_props={"facecolor": facecolor})
    return rb._buttons.get_facecolor().tolist()


def test_radio_button_active_conflict():
    probe("RadioButtons.activecolor", _radio_activecolor, labels=["tea", "coffee"],
          activecolor="red", facecolor="green")


def _buttons_callbacks(widget):
    ax = _get_ax()
    button = getattr(widgets, widget)(ax, ["Test Button"])
    clicks = []
    button.on_clicked(clicks.append)
    xy = ax.transData.inverted().transform(ax.transAxes.transform((0.15, 0.5)))
    MouseEvent._from_ax_coords("button_press_event", ax, xy, 1)._process()
    return len(clicks)


def test__buttons_callbacks():
    for widget in ["RadioButtons", "CheckButtons"]:
        probe("_Buttons.on_clicked", _buttons_callbacks, widget=widget)


def _textbox(toolbar):
    with mpl.rc_context({"toolbar": toolbar}):
        ax = _get_ax()
        changes, submits = [], []
        tool = widgets.TextBox(ax, "")
        tool.on_submit(submits.append)
        tool.on_text_change(changes.append)
        text = tool.text
        MouseEvent._from_ax_coords("button_press_event", ax, (.5, .5), 1)._process()
        tool.set_val("x**2")
        after = [tool.text, len(changes)]
        tool.begin_typing()
        tool.stop_typing()
        submitted = len(submits)
        MouseEvent._from_ax_coords("button_press_event", ax, (.5, .5), 1)._process()
        KeyEvent("key_press_event", ax.figure.canvas, "+")._process()
        KeyEvent("key_press_event", ax.figure.canvas, "5")._process()
        return [text, *after, submitted, len(changes)]


def test_TextBox():
    for toolbar in ["None", "toolbar2", "toolmanager"]:
        probe("TextBox", _textbox, toolbar=toolbar)


# --- sliders -----------------------------------------------------------------


def _slider(valmin, valmax, valinit, valstep=None, orientation="horizontal", set_val=None,
            reset=False):
    ax = _figure().subplots()
    slider = widgets.Slider(ax=ax, label="", valmin=valmin, valmax=valmax, valinit=valinit,
                            valstep=valstep, orientation=orientation)
    if set_val is not None:
        slider.set_val(set_val)
    if reset:
        slider.reset()
    box = slider.poly.get_extents().transformed(ax.transAxes.inverted())
    return {"val": slider.val, "poly": list(box.bounds)}


def test_slider_valmin_valmax():
    for valinit in [-10.0, 25.0]:
        probe("Slider", _slider, valmin=0.0, valmax=24.0, valinit=valinit)


def test_slider_valstep_snapping():
    probe("Slider", _slider, valmin=0.0, valmax=24.0, valinit=11.4, valstep=1)
    probe("Slider", _slider, valmin=0.0, valmax=24.0, valinit=11.4, valstep=[0, 1, 5.5, 19.7])


def test_slider_horizontal_vertical():
    for orientation in ["horizontal", "vertical"]:
        probe("Slider", _slider, valmin=0, valmax=24, valinit=12, orientation=orientation,
              set_val=10, tol=1e-9)


def test_slider_reset():
    probe("Slider", _slider, valmin=0, valmax=1, valinit=0.5, set_val=0.75, reset=True)


def _slider_bound(which, other, valinit):
    ax = _figure().subplots()
    bound = widgets.Slider(ax=ax, label="", valmin=0.0, valmax=24.0, valinit=other)
    slider = widgets.Slider(ax=ax, label="", valmin=0.0, valmax=24.0, valinit=valinit,
                            **{which: bound})
    return slider.val


def test_slider_slidermin_slidermax():
    probe("Slider.slidermin", _slider_bound, which="slidermin", other=5.0, valinit=1.0)
    probe("Slider.slidermax", _slider_bound, which="slidermax", other=5.0, valinit=10.0)


def _range_slider(orientation, valinit, steps):
    ax = _figure().subplots()
    slider = widgets.RangeSlider(ax=ax, label="", valmin=0.0, valmax=1.0,
                                 orientation=orientation, valinit=valinit)

    def state():
        box = slider.poly.get_extents().transformed(ax.transAxes.inverted())
        handles = [(h.get_ydata() if orientation == "vertical" else h.get_xdata())[0]
                   for h in slider._handles]
        return {"val": list(slider.val), "handles": handles,
                "poly": box.get_points().flatten().tolist()}

    states = [state()]
    for name, *args in steps:
        getattr(slider, name)(*args)
        states.append(state())
    return states


def test_range_slider():
    for orientation in ["horizontal", "vertical"]:
        probe("RangeSlider", _range_slider, orientation=orientation, valinit=[0.1, 0.34],
              tol=1e-9,
              steps=[["set_val", [0.4, 0.6]], ["set_val", [0.2, 0.1]], ["set_val", [-1, 10]],
                     ["reset"]])


def test_range_slider_same_init_values():
    for orientation in ["horizontal", "vertical"]:
        probe("RangeSlider", _range_slider, orientation=orientation, valinit=[0, 0], steps=[],
              tol=1e-9)


# --- cursors -----------------------------------------------------------------


def _multicursor(horizOn, vertOn, useblit):
    fig = _figure()
    ax1, ax3 = fig.subplots(2, sharex=True)
    ax2 = _figure().subplots()
    multi = widgets.MultiCursor((ax1, ax2), useblit=useblit, horizOn=horizOn, vertOn=vertOn)

    def lines():
        return {"vx": [list(line.get_xdata()) for line in multi.vlines],
                "hy": [list(line.get_ydata()) for line in multi.hlines],
                "vvisible": sum(line.get_visible() for line in multi.vlines),
                "hvisible": sum(line.get_visible() for line in multi.hlines)}

    counts = [len(multi.vlines), len(multi.hlines),
              sum(line.get_animated() for line in multi.vlines + multi.hlines)]
    MouseEvent._from_ax_coords("motion_notify_event", ax1, (.5, .25))._process()
    fig.canvas.draw()
    moved = lines()
    multi.horizOn = not multi.horizOn
    multi.vertOn = not multi.vertOn
    MouseEvent._from_ax_coords("motion_notify_event", ax1, (.5, .25))._process()
    toggled = lines()
    MouseEvent._from_ax_coords("motion_notify_event", ax3, (.75, .75))._process()
    return {"counts": counts, "moved": moved, "toggled": toggled, "elsewhere": lines()}


def test_MultiCursor():
    for horizOn in [False, True]:
        for vertOn in [False, True]:
            for useblit in [False, True]:
                probe("MultiCursor", _multicursor, horizOn=horizOn, vertOn=vertOn,
                      useblit=useblit)


def _cursor_useblit(rects):
    fig = _figure()
    axes = [fig.add_axes(rect) for rect in rects]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        cursor = widgets.Cursor(axes[0], useblit=True)
    return [cursor.useblit, cursor.linev.get_animated()]


def test_cursor_overlapping_axes_blitting_warning():
    assert probe("Cursor.useblit", _cursor_useblit,
                 rects=[[0.1, 0.1, 0.8, 0.8], [0.2, 0.2, 0.6, 0.6]]) == [False, False]
    probe("Cursor.useblit", _cursor_useblit, rects=[[0, 0, 0.5, 1], [0.5, 0, 0.5, 1]])
    probe("Cursor.useblit", _cursor_useblit, rects=[[0.1, 0.1, 0.3, 0.3], [0.6, 0.6, 0.3, 0.3]])
    probe("Cursor.useblit", _cursor_useblit, rects=[[0.1, 0.1, 0.8, 0.8]])
