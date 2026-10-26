"""The public API the API audit found missing in plt-rs and added
(``target/agents/api-audit/coverage.md`` in the audit's worktree).

Area ``api_audit``. Every op works on a fresh ``Figure``, drawn with the
Agg canvas where it says so.

Ops
---
``Axes.has_data``  {artist} -> ``has_data()`` after adding one artist:
    ``"none"``, ``"text"``, ``"line"``, ``"patch"``, ``"scatter"``,
    ``"image"``, ``"table"``, or a line removed again (``"removed"``).
``Figure.artists``  {} -> the lengths of ``[artists, images, texts,
    legends]`` of a figure holding an axes, a text, a circle and a line
    added with ``add_artist``, a ``figimage`` and a figure legend.
``Axes.callbacks``  {scenario} -> how often ``xlim_changed`` and
    ``ylim_changed`` callbacks ran, and the limits they saw:
    ``"set_xlim"``, ``"set_xbound"``, ``"invert"``, ``"shared"`` (two axes
    sharing x, limits set on the first), ``"read"`` (autoscaling when the
    y limits are read after plotting, read twice), ``"draw"`` (autoscaling
    while drawing), ``"clear"`` (callbacks of a cleared axes),
    ``"disconnect"``, ``"chain"`` (a callback of the first of two axes
    setting the y limits of the second, whose callbacks record them).
``Axes.set_subplotspec``  {to} -> the position bounds of an axes made in
    cell ``[0, 0]`` of a 2 × 2 grid and moved to cell ``to``, and of its
    twin.
``YAxis.set_offset_position``  {side} -> where the y offset text is drawn
    relative to the axes box: ``[left edge, right edge]`` minus the axes'
    left edge, in pixels.
``PolyCollection.set_verts``  {verts} -> vertices and codes of the paths
    after ``set_verts`` replaced a triangle.
``Collection.set_paths``  {verts} -> vertices of the paths of a path
    collection after ``set_paths``.
``FigureImage.get_extent``  {xo, yo, rows, cols} -> the extent of a
    ``figimage``, and its shape after ``set_data`` with one more row and
    column.
``NonUniformImage.set_data``  {x, y} -> the extent after ``set_data``
    (centres rounded to single precision).
``ScalarMappable.colorbar``  {} -> whether an image's ``colorbar`` is
    None before a colorbar is made, the first colorbar after, and the
    second after another one.
``Colorbar.solids``  {hatched} -> whether the colorbar of an image (or
    of hatched filled contours) has ``solids``, and how many
    ``solids_patches``.
``Divider``  {} -> ``[len(get_horizontal()), len(get_vertical()),
    get_aspect()]`` of a divider after ``append_size("left", ...)`` and
    ``append_size("top", ...)``, and ``get_aspect()`` of the divider of an
    axes with and without an equal aspect.
``Line3D.set_data_3d``  {x, y, z} -> ``get_data_3d()`` after
    ``set_data_3d`` and the x limits before and after.
``Text3D.get_position_3d``  {x, y, z} -> the position of ``text(x, y, z)``.
``Axis3D.get_rotate_label``  {label, rotate} -> ``get_rotate_label`` of
    the z label ``label`` after ``set_rotate_label(rotate)`` (``None``
    keeps the default, which rotates labels longer than four characters).
``Axes3D.set_title``  {loc} -> the left, centre and right titles after
    ``set_title("t", loc=loc)``.

The 2-D and interaction API the audit left missing; the figures are 4 × 3
inches at 100 dpi with an Agg canvas, their axes at ``[0.25, 0.25, 0.5,
0.5]`` (pixels 100 to 300 across, 75 to 225 up) unless noted:

``Axis.pickradius``  {axis, radius} -> the default ``pickradius`` of the x
    or y axis, then ``get_pickradius()`` and the property after
    ``set_pickradius(radius)``.
``Axis.contains``  {axis, radius, points} -> per display point, whether
    ``axis.contains`` holds for a motion event there (on the whole pixel
    below, as events are), at that pick radius.
``Axis.pick``  {pickers, line, at} -> the picks a press at display point
    ``at`` fires (``"xaxis"``, ``"yaxis"``, ``"line"``), the axes over
    ``[0, 1]²`` with ``set_picker(True)`` on the axes named in ``pickers``
    and, when ``line``, an unclipped line from ``(0.5, -0.2)`` to
    ``(0.5, 0.5)`` with ``picker=5``.
``LockableBbox``  {lock} -> the points of ``LockableBbox(Bbox.unit(),
    **{lock: 2})`` and its lock, after the child moved by 10, after
    unlocking (points and lock), after locking at 3 (points and the four
    locks): the steps of matplotlib's ``test_lockable_bbox``.
``SpanSelector.new_axes``  {target} -> the selections reported, whether
    the selector's axes is the target, how many patches the target has
    and the extents, for a horizontal selector made on the left half of
    the figure (x from 0 to 10), moved to the right half (``"same"``, x
    from 100 to 200) or to the whole of another figure (``"other"``, x
    from 0 to 100), after a drag from 50 to 150 pixels in the first
    figure, then one in the target (240 to 300 pixels, or 80 to 160).
``Animation.pause``  {blit} -> the frames the frame function saw when the
    animation started and stepped once, and whether its line was animated
    then, after ``pause()`` and after ``resume()``.
``FigureManagerBase.resize``  {} -> the size in inches of a pyplot figure
    of the agg backend after ``manager.resize(300, 200)``.
``MouseButton``  {} -> the name of every ``MouseButton`` as the ``keymap.*``
    rcParams spell it (``"MouseButton.BACK"``; ``str(button)`` is the
    number since Python 3.11), and its value.
``ToolToggleBase``  {scenario} -> the enables and disables of toggles
    added to a ``ToolManager``, ``[name, enabled]`` in order, and then
    whether each is toggled: ``"radio"`` (``a`` and ``b`` in group ``g``,
    ``c`` in none; triggering ``a``, ``c``, ``b``, ``b``) or
    ``"default_toggled"`` (``a`` in ``g`` triggered on, then ``d`` added
    in ``g`` with ``default_toggled``).
``CallbackRegistry.blocked``  {signal} -> the callbacks of
    ``button_press_event``, ``key_press_event`` and ``pick_event`` (the
    axes has ``picker=True``) that ran for a press and a key in the axes
    inside ``canvas.callbacks.blocked(signal=signal)``, then after it.
``boxplot.bootstrap``  {data, bootstrap} -> for notches bootstrapped from
    numpy's global generator, whose values no golden can hold: whether
    ``min <= cilo <= cihi <= max``, whether the other statistics equal
    the unbootstrapped ones, whether the notches ``boxplot(notch=True)``
    draws lie in the data range, and how many points the box outline has.
"""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib import cbook
from matplotlib.animation import FuncAnimation
from matplotlib.backend_bases import KeyEvent, MouseButton, MouseEvent
from matplotlib.backend_managers import ToolManager
from matplotlib.backend_tools import ToolToggleBase
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.collections import PathCollection, PolyCollection
from matplotlib.figure import Figure
from matplotlib.lines import Line2D
from matplotlib.patches import Circle
from matplotlib.path import Path
from matplotlib.transforms import Bbox, LockableBbox
from matplotlib.widgets import SpanSelector
from mpl_toolkits.axes_grid1 import Divider, Size, make_axes_locatable

from spec_support import ProbeSet

probes = ProbeSet("api_audit")


def _has_data(artist):
    ax = Figure().add_subplot()
    if artist == "text":
        ax.text(0.5, 0.5, "note")
    elif artist == "line":
        ax.plot([0, 1], [0, 1])
    elif artist == "patch":
        ax.add_patch(Circle((0.5, 0.5), 0.1))
    elif artist == "scatter":
        ax.scatter([0, 1], [0, 1])
    elif artist == "image":
        ax.imshow([[0, 1], [1, 0]])
    elif artist == "table":
        ax.table(cellText=[["1", "2"]])
    elif artist == "removed":
        ax.plot([0, 1], [0, 1])[0].remove()
    return ax.has_data()


def _figure_artists():
    fig = Figure()
    ax = fig.add_subplot()
    (line,) = ax.plot([0, 1], [0, 1], label="data")
    fig.text(0.5, 0.5, "note")
    fig.add_artist(Circle((0.5, 0.5), 0.1))
    fig.add_artist(Line2D([0, 1], [0, 1]))
    fig.figimage(np.ones((2, 2)))
    fig.legend([line], ["data"])
    return [len(fig.artists), len(fig.images), len(fig.texts), len(fig.legends)]


class _Recorder:
    """Counts the limit callbacks of an axes and the limits they saw."""

    def __init__(self, ax):
        self.calls = {"x": [], "y": []}
        self.cids = [
            ax.callbacks.connect("xlim_changed", lambda a: self.calls["x"].append(list(a.get_xlim()))),
            ax.callbacks.connect("ylim_changed", lambda a: self.calls["y"].append(list(a.get_ylim()))),
        ]

    def counts(self):
        return [len(self.calls["x"]), len(self.calls["y"])]


def _callbacks(scenario):
    fig = Figure()
    if scenario == "shared":
        first, second = fig.subplots(1, 2, sharex=True)
        records = [_Recorder(first), _Recorder(second)]
        first.set_xlim(0, 3)
        return [r.counts() for r in records] + [records[1].calls["x"]]
    if scenario == "chain":
        first, second = fig.subplots(1, 2)
        record = _Recorder(second)
        first.callbacks.connect("xlim_changed", lambda a: second.set_ylim(*a.get_xlim()))
        first.set_xlim(1, 2)
        return [record.counts(), record.calls["y"]]
    ax = fig.add_subplot()
    record = _Recorder(ax)
    if scenario == "set_xlim":
        ax.set_xlim(0, 2)
    elif scenario == "set_xbound":
        ax.set_xbound(1, 3)
    elif scenario == "invert":
        ax.invert_yaxis()
    elif scenario == "read":
        ax.plot([0, 1], [0, 4])
        before = record.counts()
        ax.get_ylim()
        ax.get_ylim()
        return [before, record.counts(), record.calls["y"]]
    elif scenario == "draw":
        ax.plot([0, 1], [0, 4])
        FigureCanvasAgg(fig).draw()
        return [record.counts(), record.calls["x"], record.calls["y"]]
    elif scenario == "clear":
        ax.clear()
        ax.set_xlim(0, 2)
    elif scenario == "disconnect":
        ax.callbacks.disconnect(record.cids[0])
        ax.set_xlim(0, 2)
        ax.set_ylim(0, 2)
    return [record.counts(), record.calls["x"], record.calls["y"]]


def _set_subplotspec(to):
    fig = Figure()
    grid = fig.add_gridspec(2, 2)
    ax = fig.add_subplot(grid[0, 0])
    twin = ax.twinx()
    ax.set_subplotspec(grid[to[0], to[1]])
    return [list(ax.get_position().bounds), list(twin.get_position().bounds)]


def _offset_position(side):
    fig = Figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot()
    ax.plot([0, 1], [1e6, 1e6 + 1])
    ax.yaxis.set_offset_position(side)
    FigureCanvasAgg(fig).draw()
    box = ax.yaxis.offsetText.get_window_extent()
    left = ax.get_window_extent().x0
    return [box.x0 - left, box.x1 - left]


def _set_verts(verts):
    ax = Figure().add_subplot()
    polys = PolyCollection([[(0, 0), (1, 0), (0, 1)]])
    ax.add_collection(polys)
    polys.set_verts(verts)
    return [[p.vertices.tolist(), p.codes.tolist()] for p in polys.get_paths()]


def _set_paths(verts):
    ax = Figure().add_subplot()
    paths = PathCollection([Path([(0, 0), (1, 1)])])
    ax.add_collection(paths)
    paths.set_paths([Path(v) for v in verts])
    return [p.vertices.tolist() for p in paths.get_paths()]


def _figure_image(xo, yo, rows, cols):
    fig = Figure()
    image = fig.figimage(np.ones((rows, cols)), xo=xo, yo=yo)
    extent = list(image.get_extent())
    image.set_data(np.ones((rows + 1, cols + 1)))
    return [extent, list(image.get_array().shape)]


def _nonuniform_set_data(x, y):
    from matplotlib.image import NonUniformImage
    ax = Figure().add_subplot()
    image = NonUniformImage(ax)
    image.set_data([0, 1], [0, 1], np.ones((2, 2)))
    ax.add_image(image)
    image.set_data(x, y, np.ones((len(y), len(x))))
    return [float(v) for v in image.get_extent()]


def _mappable_colorbar():
    fig = Figure()
    ax = fig.add_subplot()
    image = ax.imshow([[0, 1], [1, 0]])
    before = image.colorbar is None
    first = fig.colorbar(image)
    after_first = image.colorbar is first
    second = fig.colorbar(image)
    return [before, after_first, image.colorbar is second]


def _colorbar_solids(hatched):
    fig = Figure()
    ax = fig.add_subplot()
    if hatched:
        z = [[0, 1, 2], [1, 2, 3], [2, 3, 4]]
        colorbar = fig.colorbar(ax.contourf(z, levels=[0, 2, 4], hatches=["/", "x"]))
    else:
        colorbar = fig.colorbar(ax.imshow([[0, 1], [1, 0]]))
    return [colorbar.solids is not None, len(colorbar.solids_patches)]


def _divider():
    fig = Figure()
    divider = Divider(fig, (0.1, 0.1, 0.8, 0.8), [Size.Fixed(1)], [Size.Fixed(1)], aspect=False)
    divider.append_size("left", Size.Fixed(0.5))
    divider.append_size("top", Size.Fixed(0.25))
    plain = make_axes_locatable(fig.add_subplot(1, 2, 1))
    equal_ax = fig.add_subplot(1, 2, 2)
    equal_ax.set_aspect("equal")
    equal = make_axes_locatable(equal_ax)
    return [len(divider.get_horizontal()), len(divider.get_vertical()), divider.get_aspect(),
            plain.get_aspect(), equal.get_aspect()]


def _line_data_3d(x, y, z):
    fig = Figure()
    ax = fig.add_subplot(projection="3d")
    (line,) = ax.plot([0, 1], [0, 1], [0, 1])
    FigureCanvasAgg(fig).draw()
    before = list(ax.get_xlim())
    line.set_data_3d(x, y, z)
    data = [np.asarray(c, float).tolist() for c in line.get_data_3d()]
    return [data, before, list(ax.get_xlim())]


def _text_position_3d(x, y, z):
    ax = Figure().add_subplot(projection="3d")
    return [float(v) for v in ax.text(x, y, z, "here").get_position_3d()]


def _rotate_label(label, rotate):
    ax = Figure().add_subplot(projection="3d")
    ax.set_zlabel(label)
    if rotate is not None:
        ax.zaxis.set_rotate_label(rotate)
    return ax.zaxis.get_rotate_label(ax.zaxis.label.get_text())


def _title_3d(loc):
    ax = Figure().add_subplot(projection="3d")
    ax.set_title("t", loc=loc)
    return [ax.get_title(loc="left"), ax.get_title(), ax.get_title(loc="right")]


def test_has_data():
    for artist in ["none", "text", "line", "patch", "scatter", "image", "table", "removed"]:
        probes.probe("Axes.has_data", _has_data, artist=artist)


def test_figure_artists():
    probes.probe("Figure.artists", _figure_artists)


def test_limit_callbacks():
    for scenario in ["set_xlim", "set_xbound", "invert", "shared", "read", "draw", "clear",
                     "disconnect", "chain"]:
        probes.probe("Axes.callbacks", _callbacks, scenario=scenario)


def test_set_subplotspec():
    for to in [[1, 1], [0, 1]]:
        probes.probe("Axes.set_subplotspec", _set_subplotspec, to=to)


def test_offset_position():
    for side in ["left", "right"]:
        probes.probe("YAxis.set_offset_position", _offset_position, side=side)


def test_collection_paths():
    probes.probe("PolyCollection.set_verts", _set_verts,
                 verts=[[[0, 0], [2, 0], [2, 2]], [[1, 1], [3, 1], [3, 3], [1, 3]]])
    probes.probe("Collection.set_paths", _set_paths, verts=[[[1, 1], [2, 2], [3, 1]]])


def test_figure_image():
    probes.probe("FigureImage.get_extent", _figure_image, xo=10, yo=20, rows=2, cols=3)


def test_nonuniform_set_data():
    probes.probe("NonUniformImage.set_data", _nonuniform_set_data, x=[0.1, 1, 4], y=[0, 2.3])


def test_mappable_colorbar():
    probes.probe("ScalarMappable.colorbar", _mappable_colorbar)


def test_colorbar_solids():
    for hatched in [False, True]:
        probes.probe("Colorbar.solids", _colorbar_solids, hatched=hatched)


def test_divider():
    probes.probe("Divider", _divider)


def test_axes3d_updates():
    probes.probe("Line3D.set_data_3d", _line_data_3d, x=[1, 2, 3], y=[0, 0, 0], z=[3, 2, 1])
    probes.probe("Text3D.get_position_3d", _text_position_3d, x=1, y=2, z=3)
    for label in ["z", "label"]:
        for rotate in [None, True, False]:
            probes.probe("Axis3D.get_rotate_label", _rotate_label, label=label, rotate=rotate)
    for loc in ["left", "center", "right"]:
        probes.probe("Axes3D.set_title", _title_3d, loc=loc)


def _framed():
    """A 4 × 3 inch figure on an Agg canvas and its axes, 100 to 300 pixels
    across and 75 to 225 up."""
    fig = Figure(figsize=(4, 3), dpi=100)
    canvas = FigureCanvasAgg(fig)
    return fig, canvas, fig.add_axes([0.25, 0.25, 0.5, 0.5])


def _pickradius(axis, radius):
    ax = Figure().add_subplot()
    target = getattr(ax, axis + "axis")
    default = target.get_pickradius()
    target.set_pickradius(radius)
    return [default, target.get_pickradius(), target.pickradius]


def _axis_contains(axis, radius, points):
    fig, canvas, ax = _framed()
    target = getattr(ax, axis + "axis")
    target.set_pickradius(radius)
    return [bool(target.contains(MouseEvent("motion_notify_event", canvas, x, y))[0])
            for x, y in points]


def _axis_pick(pickers, line, at):
    fig, canvas, ax = _framed()
    ax.set(xlim=(0, 1), ylim=(0, 1))
    for name in pickers:
        getattr(ax, name + "axis").set_picker(True)
    names = {id(ax.xaxis): "xaxis", id(ax.yaxis): "yaxis"}
    if line:
        (drawn,) = ax.plot([0.5, 0.5], [-0.2, 0.5], clip_on=False, picker=5)
        names[id(drawn)] = "line"
    canvas.draw()
    picks = []
    canvas.mpl_connect("pick_event", lambda event: picks.append(names[id(event.artist)]))
    MouseEvent("button_press_event", canvas, *at, button=1)._process()
    return picks


def _lockable_bbox(lock):
    child = Bbox.unit()
    locked = LockableBbox(child, **{lock: 2})
    steps = [locked.get_points().tolist(), getattr(locked, "locked_" + lock)]
    child.set_points(child.get_points() + 10)
    steps.append(locked.get_points().tolist())
    setattr(locked, "locked_" + lock, None)
    steps += [locked.get_points().tolist(), getattr(locked, "locked_" + lock)]
    setattr(locked, "locked_" + lock, 3)
    steps += [locked.get_points().tolist(),
              [locked.locked_x0, locked.locked_y0, locked.locked_x1, locked.locked_y1]]
    return steps


def _drag(canvas, x0, x1, y):
    MouseEvent("button_press_event", canvas, x0, y, button=1)._process()
    MouseEvent("motion_notify_event", canvas, x1, y, buttons={MouseButton.LEFT})._process()
    MouseEvent("button_release_event", canvas, x1, y, button=1)._process()


def _span_new_axes(target):
    fig = Figure(figsize=(4, 3), dpi=100)
    canvas = FigureCanvasAgg(fig)
    left = fig.add_axes([0, 0, 0.5, 1], xlim=(0, 10))
    selections = []
    span = SpanSelector(left, lambda vmin, vmax: selections.append([vmin, vmax]), "horizontal")
    if target == "same":
        target_canvas = canvas
        target_ax = fig.add_axes([0.5, 0, 0.5, 1], xlim=(100, 200))
        drag = (240, 300)
    else:
        other = Figure(figsize=(4, 3), dpi=100)
        target_canvas = FigureCanvasAgg(other)
        target_ax = other.add_axes([0, 0, 1, 1], xlim=(0, 100))
        drag = (80, 160)
    span.new_axes(target_ax)
    _drag(canvas, 50, 150, 150)
    _drag(target_canvas, *drag, 150)
    return [selections, span.ax is target_ax, len(target_ax.patches), list(span.extents)]


def _animation_pause(blit):
    fig = Figure(figsize=(2, 2), dpi=50)
    canvas = FigureCanvasAgg(fig)
    (line,) = fig.add_subplot().plot([0, 1], [0, 0])
    frames = []

    def update(i):
        frames.append(i)
        line.set_ydata([0, i])
        return (line,)

    animation = FuncAnimation(fig, update, frames=5, blit=blit, interval=10)
    canvas.draw()
    animation._step()
    states = [line.get_animated()]
    animation.pause()
    states.append(line.get_animated())
    animation.resume()
    states.append(line.get_animated())
    return [frames, states]


def _manager_resize():
    fig = plt.figure()
    fig.canvas.manager.resize(300, 200)
    size = fig.get_size_inches().tolist()
    plt.close(fig)
    return size


def _mouse_buttons():
    return [[f"MouseButton.{button.name}", button.value] for button in MouseButton]


def _toggle_tools(scenario):
    fig = Figure()
    FigureCanvasAgg(fig)
    manager = ToolManager(fig)
    log = []

    def toggle(name, group=None, start=False):
        class Tool(ToolToggleBase):
            radio_group = group
            default_toggled = start

            def enable(self, event=None):
                log.append([self.name, True])

            def disable(self, event=None):
                log.append([self.name, False])

        manager.add_tool(name, Tool)

    if scenario == "radio":
        names = ["a", "b", "c"]
        toggle("a", "g")
        toggle("b", "g")
        toggle("c")
        for name in ["a", "c", "b", "b"]:
            manager.trigger_tool(name)
    else:
        names = ["a", "d"]
        toggle("a", "g")
        manager.trigger_tool("a")
        toggle("d", "g", start=True)
    return [log, [manager.get_tool(name).toggled for name in names]]


def _callbacks_blocked(signal):
    fig, canvas, ax = _framed()
    ax.set_picker(True)
    log = []
    for name in ["button_press_event", "key_press_event", "pick_event"]:
        canvas.mpl_connect(name, lambda event, name=name: log.append(name))

    def send():
        MouseEvent("button_press_event", canvas, 200, 150, button=1)._process()
        KeyEvent("key_press_event", canvas, "a", 200, 150)._process()

    with canvas.callbacks.blocked(signal=signal):
        send()
    blocked = list(log)
    log.clear()
    send()
    return [blocked, log]


def _boxplot_bootstrap(data, bootstrap):
    stats = cbook.boxplot_stats([data], bootstrap=bootstrap)[0]
    plain = cbook.boxplot_stats([data])[0]
    low, high = min(data), max(data)
    parts = Figure().add_subplot().boxplot([data], notch=True, bootstrap=bootstrap)
    outline = parts["boxes"][0].get_ydata()
    return [bool(low <= stats["cilo"] <= stats["cihi"] <= high),
            [bool(stats[key] == plain[key]) for key in ("q1", "med", "q3", "whislo", "whishi")],
            bool(low <= outline[2] <= outline[4] <= high),
            len(outline)]


def test_axis_pickradius():
    for axis in ["x", "y"]:
        for radius in [5, 0, -1]:
            probes.probe("Axis.pickradius", _pickradius, axis=axis, radius=radius)
    probes.probe("Axis.contains", _axis_contains, axis="x", radius=15,
                 points=[[200, 70], [200, 60.5], [200, 59], [200, 75], [200, 230], [200, 239],
                         [200, 241], [100, 70], [300, 70], [99, 70], [301, 70], [200, 150]])
    probes.probe("Axis.contains", _axis_contains, axis="y", radius=4,
                 points=[[97, 150], [96.5, 150], [95, 150], [100, 150], [303, 150], [305, 150],
                         [97, 75], [97, 225], [97, 74], [97, 226], [200, 150]])
    for pickers, line, at in [(["x"], False, [200, 70]), (["x", "y"], False, [90, 150]),
                              (["x", "y"], True, [200, 70]), (["y"], False, [200, 70]),
                              (["x", "y"], False, [90, 70])]:
        probes.probe("Axis.pick", _axis_pick, pickers=pickers, line=line, at=at)


def test_lockable_bbox():
    for lock in ["x0", "y0", "x1", "y1"]:
        probes.probe("LockableBbox", _lockable_bbox, lock=lock)


def test_span_selector_new_axes():
    for target in ["same", "other"]:
        probes.probe("SpanSelector.new_axes", _span_new_axes, target=target)


def test_animation_pause():
    for blit in [False, True]:
        probes.probe("Animation.pause", _animation_pause, blit=blit)


def test_manager_resize():
    probes.probe("FigureManagerBase.resize", _manager_resize)


def test_mouse_buttons():
    probes.probe("MouseButton", _mouse_buttons)


def test_toggle_tools():
    for scenario in ["radio", "default_toggled"]:
        probes.probe("ToolToggleBase", _toggle_tools, scenario=scenario)


def test_callbacks_blocked():
    for signal in [None, "button_press_event", "key_press_event", "pick_event"]:
        probes.probe("CallbackRegistry.blocked", _callbacks_blocked, signal=signal)


def test_boxplot_bootstrap():
    data = [1, 2, 4, 8, 9, 12, 20, 3, 5]
    for bootstrap in [500, 1, 0]:
        probes.probe("boxplot.bootstrap", _boxplot_bootstrap, data=data, bootstrap=bootstrap)
    probes.probe("boxplot.bootstrap", _boxplot_bootstrap, data=[3, 3, 3], bootstrap=20)
