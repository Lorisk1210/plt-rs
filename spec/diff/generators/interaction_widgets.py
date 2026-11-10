"""The widgets of ``matplotlib.widgets`` driven by the mouse and the
keyboard, for ``interaction.py``: sliders dragged, buttons clicked, text
typed, and every selector drawn, moved and resized by each of its
handles, with ``useblit`` on and off.

Each program reads the widget's values and its artists after each
gesture; the widgets' callbacks print what they receive.
"""

from __future__ import annotations

from typing import Iterator

#: A figure with room for the control widgets under the plot.
CONTROLS = [
    "from matplotlib import widgets",
    "fig = plt.figure(figsize=(5, 4))",
    "ax = fig.add_axes((0.1, 0.45, 0.8, 0.5))",
    "ax.plot(x, y)",
    "low = fig.add_axes((0.2, 0.25, 0.6, 0.06))",
    "high = fig.add_axes((0.2, 0.1, 0.6, 0.06))",
    "side = fig.add_axes((0.92, 0.45, 0.04, 0.5))",
    "fig.canvas.draw()",
]
#: The Axes the selectors draw on, data from 0 to 200 on both axes.
SELECTING = [
    "from matplotlib import widgets",
    "fig, ax = plt.subplots(figsize=(4, 4))",
    "ax.plot([0, 200], [0, 200])",
    "fig.canvas.draw()",
]


def _slider() -> Iterator[str]:
    yield "s = widgets.Slider(low, 'freq', 0.1, 10.0, valinit=3)"
    yield "cid = s.on_changed(lambda v: show('changed', v))"
    yield "def slider_state(s):\n" \
          "    return (s.val, s.valtext.get_text(), s.drag_active, s._handle.get_xdata(), s._handle.get_ydata(), " \
          "s.poly.get_path().get_extents(s.poly.get_patch_transform()).bounds)"
    yield "slider_state(s)"
    for fx in (0.5, 0.8, 1.4, -0.3, 0.25):
        yield f"mouse('motion_notify_event', px(low, {fx}, 0.5), buttons={{MouseButton.LEFT}})" if fx != 0.5 else \
            "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "slider_state(s)"
    yield "mouse('button_release_event', px(low, 0.25, 0.5), MouseButton.LEFT)"
    yield "slider_state(s), fig.canvas.mouse_grabber"
    yield "mouse('motion_notify_event', px(low, 0.9, 0.5))"
    yield "slider_state(s)"
    yield "drag(px(low, 0.4, 0.5), px(low, 0.6, 0.5), MouseButton.RIGHT)"
    yield "slider_state(s)"
    yield "drag(px(ax, 0.4, 0.5), px(low, 0.6, 0.5))"
    yield "slider_state(s)"
    yield "stepped = widgets.Slider(high, 'step', 0, 10, valinit=5, valstep=2, valfmt='%1.2f', initcolor='none')"
    yield "stepped.on_changed(lambda v: show('stepped', v))"
    yield "drag(px(high, 0.5, 0.5), px(high, 0.77, 0.5), steps=3)"
    yield "slider_state(stepped)"
    yield "stepped.valstep = [0, 1, 2.5, 7]"
    yield "drag(px(high, 0.3, 0.5), px(high, 0.62, 0.5))"
    yield "slider_state(stepped)"
    yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
    yield "mouse('button_press_event', px(high, 0.5, 0.5), MouseButton.LEFT)"
    yield "mouse('button_release_event', px(high, 0.5, 0.5), MouseButton.LEFT)"
    yield "fig.canvas.mouse_grabber"
    yield "fig.canvas.release_mouse(low)"
    yield "s.slidermin = stepped"
    yield "drag(px(low, 0.5, 0.5), px(low, 0.05, 0.5))"
    yield "slider_state(s)"
    yield "s.slidermin = None"
    yield "s.slidermax = stepped"
    yield "drag(px(low, 0.5, 0.5), px(low, 0.95, 0.5))"
    yield "slider_state(s)"
    yield "s.slidermax = None"
    yield "vertical = widgets.Slider(side, 'v', -1, 1, valinit=0, orientation='vertical', dragging=False, " \
          "closedmin=False, closedmax=False)"
    yield "vertical.on_changed(lambda v: show('vertical', v))"
    yield "drag(px(side, 0.5, 0.5), px(side, 0.5, 0.9))"
    yield "slider_state(vertical)"
    yield "mouse('button_press_event', px(side, 0.5, 0.0), MouseButton.LEFT)"
    yield "mouse('button_release_event', px(side, 0.5, 0.0), MouseButton.LEFT)"
    yield "slider_state(vertical)"
    yield "mouse('button_press_event', px(side, 0.5, 1.0), MouseButton.LEFT)"
    yield "mouse('button_release_event', px(side, 0.5, 1.0), MouseButton.LEFT)"
    yield "slider_state(vertical)"
    yield "s.set_val(4.25)"
    yield "s.reset()"
    yield "slider_state(s)"
    yield "s.set_active(False)"
    yield "drag(px(low, 0.5, 0.5), px(low, 0.9, 0.5))"
    yield "slider_state(s), s.get_active()"
    yield "s.set_active(True)"
    yield "s.eventson = False"
    yield "drag(px(low, 0.5, 0.5), px(low, 0.9, 0.5))"
    yield "slider_state(s)"
    yield "s.eventson = True"
    yield "s.disconnect(cid)"
    yield "drag(px(low, 0.9, 0.5), px(low, 0.1, 0.5))"
    yield "slider_state(s)"
    yield "s.disconnect_events()"
    yield "drag(px(low, 0.1, 0.5), px(low, 0.6, 0.5))"
    yield "slider_state(s)"


def _range_slider() -> Iterator[str]:
    yield "r = widgets.RangeSlider(low, 'range', 0, 10, valinit=(2, 6))"
    yield "r.on_changed(lambda v: show('range', *v))"
    yield "def range_state(r):\n" \
          "    return (tuple(r.val), r.valtext.get_text(), r.drag_active, [h.get_xdata() for h in r._handles], " \
          "[h.get_ydata() for h in r._handles], r.poly.get_path().get_extents(r.poly.get_patch_transform()).bounds)"
    yield "range_state(r)"
    for start, end in (((0.2, 0.5), (0.1, 0.5)), ((0.62, 0.5), (0.9, 0.5)), ((0.45, 0.5), (0.3, 0.5)),
                       ((0.3, 0.5), (1.5, 0.5)), ((0.95, 0.5), (-0.4, 0.5)), ((0.0, 0.5), (0.05, 0.5))):
        yield f"drag(px(low, {start[0]}, {start[1]}), px(low, {end[0]}, {end[1]}), steps=3)"
        yield "range_state(r)"
    yield "r.set_min(3)"
    yield "r.set_max(1)"
    yield "range_state(r)"
    yield "r.set_val((7, 2))"
    yield "range_state(r)"
    yield "r.set_val((1, 9))"
    yield "r.reset()"
    yield "range_state(r)"
    yield "stepped = widgets.RangeSlider(high, 'stepped', 0, 1, valinit=(0.2, 0.8), valstep=0.25, valfmt='%.2f')"
    yield "stepped.on_changed(lambda v: show('stepped', *v))"
    yield "drag(px(high, 0.2, 0.5), px(high, 0.4, 0.5))"
    yield "drag(px(high, 0.8, 0.5), px(high, 0.55, 0.5))"
    yield "range_state(stepped)"
    yield "same = widgets.RangeSlider(side, 'same', 0, 1, valinit=(0.5, 0.5), orientation='vertical')"
    yield "drag(px(side, 0.5, 0.5), px(side, 0.5, 0.8))"
    yield "range_state(same)"
    yield "drag(px(side, 0.5, 0.5), px(side, 0.5, 0.1))"
    yield "range_state(same)"


def _button() -> Iterator[str]:
    for useblit in (True, False):
        yield f"b = widgets.Button(low, 'Go', color='0.85', hovercolor='0.95', useblit={useblit})"
        yield "cid = b.on_clicked(lambda e: show('clicked', e.name, e.button, e.inaxes is b.ax))"
        yield "def button_state(b):\n" \
              "    return (b.ax.get_facecolor(), b.label.get_text(), b.color, b.hovercolor, fig.canvas.mouse_grabber)"
        yield "button_state(b)"
        yield "mouse('motion_notify_event', px(low, 0.5, 0.5))"
        yield "button_state(b)"
        yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "button_state(b)"
        yield "mouse('button_release_event', px(low, 0.6, 0.4), MouseButton.LEFT)"
        yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.RIGHT)"
        yield "mouse('button_release_event', px(low, 0.5, 0.5), MouseButton.RIGHT)"
        yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "mouse('button_release_event', px(ax, 0.5, 0.5), MouseButton.LEFT)"
        yield "mouse('motion_notify_event', px(ax, 0.5, 0.5))"
        yield "button_state(b)"
        yield "b.set_active(False)"
        yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "mouse('button_release_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "b.set_active(True)"
        yield "b.disconnect(cid)"
        yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "mouse('button_release_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
        yield "button_state(b)"
        yield "b.disconnect_events()"
        yield "low.clear()"
    yield "image = widgets.Button(high, '', image=z)"
    yield "image.ax.images[0].get_array().shape, image.label.get_text()"


def _buttons(kind: str) -> Iterator[str]:
    if kind == "check":
        make = "widgets.CheckButtons(side_ax, ['alpha', 'beta', 'gamma'], [True, False, True], useblit={})"
        state = "w.get_status(), w.get_checked_labels()"
    else:
        make = "widgets.RadioButtons(side_ax, ('one', 'two', 'three'), active=1, activecolor='r', useblit={})"
        state = "w.value_selected, w.index_selected"
    yield "side_ax = fig.add_axes((0.1, 0.05, 0.3, 0.3))"
    for useblit in (True, False):
        yield f"w = {make.format(useblit)}"
        yield "cid = w.on_clicked(lambda label: print('clicked', label))"
        yield state
        yield "spots = w._frames.get_offsets() if hasattr(w, '_frames') else w._buttons.get_offsets()"
        yield "spots"
        for index in (1, 0, 2, 1):
            yield f"mouse('button_press_event', px(side_ax, *spots[{index}]), MouseButton.LEFT)"
            yield f"mouse('button_release_event', px(side_ax, *spots[{index}]), MouseButton.LEFT)"
            yield state
        for index in (2, 0):
            yield f"label_at = w.labels[{index}].get_window_extent()"
            yield "mouse('button_press_event', (round(label_at.x1 - 2), round((label_at.y0 + label_at.y1) / 2)), " \
                  "MouseButton.LEFT)"
            yield state
        yield "mouse('button_press_event', px(side_ax, 0.95, 0.02), MouseButton.LEFT)"
        yield "mouse('button_press_event', px(side_ax, *spots[1]), MouseButton.RIGHT)"
        yield state
        yield "w.set_active(0)"
        yield state
        yield "w.set_active(5)"
        yield "w.eventson = False"
        yield "w.set_active(2)"
        yield state
        yield "w.eventson = True"
        yield "w.disconnect(cid)"
        yield "w.set_active(1)"
        yield state
        yield "w.clear()" if kind == "check" else "w.set_active(0)"
        yield state
        yield "[t.get_text() for t in w.labels], [t.get_color() for t in w.labels]"
        yield "w.set_label_props({'color': ['r', 'g', 'b'], 'fontsize': [8, 10, 12]})"
        yield "[t.get_color() for t in w.labels], [t.get_fontsize() for t in w.labels]"
        if kind == "check":
            yield "w.set_frame_props({'edgecolor': 'm', 's': [40, 60, 80]})"
            yield "w.set_check_props({'facecolor': 'c'})"
            yield "w._frames.get_edgecolor(), w._frames.get_sizes(), w._buttons.get_facecolor()"
        else:
            yield "w.set_radio_props({'facecolor': 'c', 'edgecolor': 'k'})"
            yield "w.activecolor = 'g'"
            yield "w._buttons.get_facecolor(), w.activecolor"
        yield "w.disconnect_events()"
        yield "side_ax.clear()"
    yield "grid_ax = fig.add_axes((0.5, 0.05, 0.45, 0.3))"
    layout = "(2, 2)"
    yield f"w = {make.format(True).replace('side_ax', 'grid_ax').replace('useblit=True', 'layout=' + layout)}"
    yield "spots = w._frames.get_offsets() if hasattr(w, '_frames') else w._buttons.get_offsets()"
    yield "spots, [t.get_position() for t in w.labels]"
    yield "mouse('button_press_event', px(grid_ax, *spots[2]), MouseButton.LEFT)"
    yield state


def _textbox() -> Iterator[str]:
    yield "box = widgets.TextBox(low, 'Eval ', initial='x')"
    yield "box.on_submit(lambda text: print('submit', repr(text)))"
    yield "change = box.on_text_change(lambda text: print('change', repr(text)))"
    yield "def box_state():\n" \
          "    return (box.text, box.cursor_index, box.capturekeystrokes, box.text_disp.get_text(), " \
          "box.cursor.get_visible(), box.cursor.get_segments(), box.ax.get_facecolor())"
    yield "box_state()"
    yield "mouse('motion_notify_event', px(low, 0.5, 0.5))"
    yield "box_state()"
    yield "mouse('button_press_event', px(low, 0.5, 0.5), MouseButton.LEFT)"
    yield "box_state()"
    yield "ax.xaxis.majorTicks[0].gridline.get_visible()"
    for key in ("*", "*", "2", "left", "left", "backspace", "home", "delete", "end", "g", "right", " ", "escape",
                "shift", "+", "1", "enter"):
        yield f"press({key!r}, px(low, 0.5, 0.5))"
        yield "box_state()"
    yield "ax.xaxis.majorTicks[0].gridline.get_visible()"
    yield "mouse('button_press_event', px(ax, 0.5, 0.5), MouseButton.LEFT)"
    yield "box_state()"
    yield "press('g', px(ax, 0.5, 0.5))"
    yield "ax.xaxis.majorTicks[0].gridline.get_visible()"
    yield "box.set_val('x**2')"
    yield "box_state()"
    yield "box.begin_typing()"
    yield "box.stop_typing()"
    yield "box_state()"
    yield "box.disconnect(change)"
    yield "box.begin_typing()"
    yield "press('z', px(low, 0.5, 0.5))"
    yield "box.stop_typing()"
    yield "box_state()"
    yield "ResizeEvent('resize_event', fig.canvas)._process()"
    yield "centered = widgets.TextBox(high, 'c', initial='middle', textalignment='center', label_pad=0.05)"
    yield "mouse('button_press_event', px(high, 0.9, 0.5), MouseButton.LEFT)"
    yield "press('!', px(high, 0.9, 0.5))"
    yield "centered.text, centered.cursor_index, centered.cursor.get_segments()"
    yield "centered.set_active(False)"
    yield "press('?', px(high, 0.9, 0.5))"
    yield "centered.text"
    yield "centered.stop_typing()"


#: Programs on the control widgets' figure.
CONTROL_PROGRAMS = {
    "widgets_slider": _slider,
    "widgets_range_slider": _range_slider,
    "widgets_button": _button,
    "widgets_check_buttons": lambda: _buttons("check"),
    "widgets_radio_buttons": lambda: _buttons("radio"),
    "widgets_textbox": _textbox,
}


def _span(useblit: bool, options: str, direction: str) -> Iterator[str]:
    yield f"span = widgets.SpanSelector(ax, lambda vmin, vmax: show('select', vmin, vmax), {direction!r}, " \
          f"useblit={useblit}, onmove_callback=lambda vmin, vmax: show('move', vmin, vmax), " \
          f"props=dict(facecolor='tab:red', alpha=0.3){options})"
    along = "0" if direction == "horizontal" else "1"
    yield "def span_state():\n" \
          "    return (span.extents, span.get_visible(), span.get_active(), [a.get_visible() for a in span.artists], " \
          "span._selection_completed)"
    yield "def spanned(start, end, key=None):\n" \
          f"    pin = {150 if direction == 'horizontal' else 40}\n" \
          f"    drag_data(ax, (start, pin) if {along} == 0 else (pin, start), (end, pin) if {along} == 0 " \
          "else (pin, end), key=key)\n" \
          "    return span_state()"
    yield "span_state()"
    yield "spanned(40, 120)"
    yield "spanned(120, 160)"
    yield "spanned(80, 90)"
    yield "spanned(span.extents[0], span.extents[0] - 25)"
    yield "spanned(span.extents[1], span.extents[1] + 30)"
    yield "spanned(sum(span.extents) / 2, sum(span.extents) / 2 + 15)"
    yield "spanned(60, 61)"
    yield "spanned(190, 230)"
    yield "spanned(-40, 50)"
    yield "press('escape', px(ax, 0.5, 0.5))"
    yield "span_state()"
    yield "spanned(30, 100)"
    yield "spanned(50, 70, key=' ')"
    yield "span.extents = (10, 20)"
    yield "span_state()"
    yield "span.set_props(facecolor='g')"
    yield "span.set_handle_props(color='k', linewidth=3)" if "interactive=True" in options else "span.artists"
    yield "span.direction = 'vertical' if span.direction == 'horizontal' else 'horizontal'"
    yield "span.direction, span_state()"
    yield "span.direction = 'horizontal' if span.direction == 'vertical' else 'vertical'"
    yield "span.set_active(False)"
    yield "spanned(10, 90)"
    yield "span.set_active(True)"
    yield "span.add_state('move')"
    yield "span.add_state('square')"
    yield "span.clear()"
    yield "span_state()"
    yield "spanned(15, 45)"
    yield "span.disconnect_events()"
    yield "spanned(100, 150)"


#: SpanSelector programs: (useblit, keywords, direction).
SPANS = {
    "plain": (False, "", "horizontal"),
    "interactive": (True, ", interactive=True, minspan=5, grab_range=8", "horizontal"),
    "anywhere": (False, ", interactive=True, drag_from_anywhere=True, ignore_event_outside=True", "horizontal"),
    "vertical": (True, ", interactive=True, button=[1, 3], snap_values=np.arange(0, 201, 25)", "vertical"),
}


def _rectangle(kind: str, useblit: bool, options: str) -> Iterator[str]:
    yield "def onselect(press, release):\n" \
          "    show('select', press.xdata, press.ydata, release.xdata, release.ydata, press.button)"
    yield f"rs = widgets.{kind}(ax, onselect, useblit={useblit}, props=dict(facecolor='tab:green', alpha=0.4)" \
          f"{options})"
    yield "def rect_state():\n" \
          "    return (rs.extents, rs.center, rs.corners, rs.edge_centers, rs.rotation, rs.get_visible(), " \
          "rs.geometry.shape, [a.get_visible() for a in rs.artists])"
    yield "rect_state()"
    yield "drag_data(ax, (50, 60), (120, 140))"
    yield "rect_state()"
    for handle in ("corners", "edge_centers"):
        for index in range(4):
            yield f"xs, ys = rs.{handle}"
            yield f"drag_data(ax, (xs[{index}], ys[{index}]), (xs[{index}] + {6 + 3 * index}, ys[{index}] - " \
                  f"{4 + index}))"
            yield "rect_state()"
    yield "drag_data(ax, rs.center, (rs.center[0] + 20, rs.center[1] - 10))"
    yield "rect_state()"
    yield "drag_data(ax, (rs.extents[0] + 3, rs.extents[2] + 5), (rs.extents[0] + 13, rs.extents[2] + 25))"
    yield "rect_state()"
    yield "drag_data(ax, (150, 20), (180, 60))"
    yield "rect_state()"
    yield "drag_data(ax, (100, 100), (101, 101))"
    yield "rect_state()"
    for key in ("shift", "ctrl", "ctrl+shift"):
        yield f"drag_data(ax, (40, 40), (90, 70), key={key!r})"
        yield "rect_state()"
    yield "KeyEvent._from_ax_coords('key_press_event', ax, (30, 30), 'escape')._process()"
    yield "rect_state()"
    yield "at(ax, 'motion_notify_event', (30, 30))"
    yield "at(ax, 'button_press_event', (30, 30), 1)"
    yield "at(ax, 'motion_notify_event', (60, 50), 1)"
    yield "KeyEvent._from_ax_coords('key_press_event', ax, (60, 50), ' ')._process()"
    yield "at(ax, 'motion_notify_event', (90, 80), 1)"
    yield "KeyEvent._from_ax_coords('key_release_event', ax, (90, 80), ' ')._process()"
    yield "at(ax, 'motion_notify_event', (110, 90), 1)"
    yield "at(ax, 'button_release_event', (110, 90), 1)"
    yield "rect_state()"
    yield "drag_data(ax, (-30, 50), (60, 80))"
    yield "rect_state()"
    yield "drag_data(ax, (60, 60), (100, 120), button=3)"
    yield "rect_state()"
    yield "rs.add_state('square')"
    yield "rs.add_state('center')"
    yield "drag_data(ax, (100, 100), (130, 120))"
    yield "rect_state()"
    yield "rs.remove_state('square')"
    yield "rs.remove_state('center')"
    yield "rs.add_state('rotate')"
    yield "xs, ys = rs.corners"
    yield "drag_data(ax, (xs[2], ys[2]), (xs[2] + 10, ys[2] + 25))"
    yield "rect_state()"
    yield "rs.remove_state('rotate')"
    yield "rs.add_state('wobble')"
    yield "rs.add_state('clear')"
    yield "rs.extents = (20, 80, 30, 90)"
    yield "rect_state()"
    yield "rs.rotation = 30"
    yield "rect_state()"
    yield "rs.set_props(facecolor='r', edgecolor='b')"
    yield "rs.set_handle_props(markerfacecolor='y', markersize=4)"
    yield "[type(a).__name__ for a in rs.artists]"
    yield "rs.set_active(False)"
    yield "drag_data(ax, (10, 10), (50, 50))"
    yield "rect_state()"
    yield "rs.set_active(True)"
    yield "rs.clear()"
    yield "rect_state()"
    yield "mouse('scroll_event', px(ax, 0.5, 0.5), 'up', step=1)"
    yield "rs.disconnect_events()"
    yield "drag_data(ax, (10, 10), (50, 50))"
    yield "rect_state()"


#: RectangleSelector and EllipseSelector programs: (class, useblit,
#: keywords).
RECTANGLES = {
    "rectangle_plain": ("RectangleSelector", False, ""),
    "rectangle_interactive": ("RectangleSelector", True, ", interactive=True, minspanx=4, minspany=4"),
    "rectangle_anywhere": ("RectangleSelector", False, ", interactive=True, drag_from_anywhere=True, "
                                                       "ignore_event_outside=True, button=[1]"),
    "rectangle_pixels": ("RectangleSelector", True, ", interactive=True, spancoords='pixels', minspanx=15, "
                                                    "minspany=15, use_data_coordinates=True, grab_range=5"),
    "ellipse_plain": ("EllipseSelector", True, ""),
    "ellipse_interactive": ("EllipseSelector", False, ", interactive=True, drag_from_anywhere=True"),
}


def _lasso(useblit: bool) -> Iterator[str]:
    yield "def on_lasso(verts):\n    v = np.asarray(verts)\n    show('lasso', len(v), *v[0], *v[-1])"
    yield f"lasso = widgets.LassoSelector(ax, on_lasso, useblit={useblit}, props=dict(color='m'))"
    yield "lasso.verts"
    yield "at(ax, 'button_press_event', (50, 50), 1)"
    yield "lasso.verts"
    for point in ((80, 60), (120, 100), (90, 150), (60, 120)):
        yield f"at(ax, 'motion_notify_event', {point}, 1)"
    yield "lasso.verts, lasso._selection_artist.get_xydata().tolist()"
    yield "at(ax, 'button_release_event', (55, 60), 1)"
    yield "lasso.verts, lasso._selection_artist.get_visible()"
    yield "drag_data(ax, (10, 10), (30, 40), button=3)"
    yield "lasso.set_props(color='c', linewidth=4)"
    yield "lasso._selection_artist.get_color(), lasso._selection_artist.get_linewidth()"
    yield "lasso.set_active(False)"
    yield "drag_data(ax, (10, 10), (30, 40))"
    yield "lasso.set_active(True)"
    yield "drag_data(ax, (100, 100), (150, 120), steps=4)"
    yield "bare = widgets.Lasso(ax, (20, 30), lambda verts: show('bare lasso', len(verts)), " \
          f"useblit={useblit})"
    yield "bare.verts, bare.line.get_xydata().tolist()"
    yield "at(ax, 'motion_notify_event', (40, 60), 1)"
    yield "at(ax, 'motion_notify_event', (70, 50), 1)"
    yield "bare.verts"
    yield "at(ax, 'button_release_event', (70, 50), 1)"
    yield "bare.verts, bare.line"


def _polygon(useblit: bool, box: bool) -> Iterator[str]:
    yield "def place(xy):\n" \
          "    at(ax, 'motion_notify_event', xy)\n" \
          "    at(ax, 'button_press_event', xy, 1)\n" \
          "    at(ax, 'button_release_event', xy, 1)"
    yield "on_poly = lambda verts: show('polygon', len(verts), *np.ravel(verts))"
    yield f"ps = widgets.PolygonSelector(ax, on_poly, useblit={useblit}, draw_bounding_box={box}, grab_range=10)"
    yield "def poly_state():\n" \
          "    return (ps.verts, ps._selection_completed, ps.get_visible(), [a.get_visible() for a in ps.artists])"
    for point in ((50, 50), (150, 50), (100, 150)):
        yield f"place({point})"
        yield "poly_state()"
    yield "place((50, 50))"
    yield "poly_state()"
    yield "drag_data(ax, (150, 50), (170, 40))"
    yield "poly_state()"
    yield "drag_data(ax, (100, 100), (110, 120), key='shift')"
    yield "poly_state()"
    yield "at(ax, 'motion_notify_event', (110, 170))"
    yield "at(ax, 'button_press_event', (110, 170), 3)"
    yield "at(ax, 'button_release_event', (110, 170), 3)"
    yield "poly_state()"
    yield "press('escape', px(ax, 0.5, 0.5))"
    yield "poly_state()"
    for point in ((30, 30), (130, 40), (160, 120), (60, 140)):
        yield f"place({point})"
    yield "poly_state()"
    yield "drag_data(ax, (130, 40), (140, 60), key='control')"
    yield "poly_state()"
    yield "place((30, 30))"
    yield "poly_state()"
    yield "at(ax, 'motion_notify_event', (60, 140))"
    yield "at(ax, 'button_press_event', (60, 140), 3)"
    yield "at(ax, 'button_release_event', (60, 140), 3)"
    yield "poly_state()"
    yield "ps.verts = [(10, 10), (60, 20), (40, 90)]"
    yield "poly_state()"
    if box:
        yield "ps._box.extents"
        yield "drag_data(ax, (60, 90), (80, 110))"
        yield "poly_state(), ps._box.extents"
    yield "ps.set_props(color='r')"
    yield "ps.set_handle_props(markerfacecolor='g')"
    yield "ps.clear()"
    yield "poly_state()"
    yield "place((300, 300))"
    yield "poly_state()"


def _cursors(useblit: bool) -> Iterator[str]:
    yield "from matplotlib import widgets"
    yield "fig, (ax, ax2) = plt.subplots(2, 1, figsize=(4, 4))"
    yield "ax.plot(x, y)"
    yield "ax2.plot(x, values)"
    yield "fig.canvas.draw()"
    yield f"cursor = widgets.Cursor(ax, useblit={useblit}, color='r', linewidth=1)"
    yield f"multi = widgets.MultiCursor(None, (ax, ax2), useblit={useblit}, horizOn=True, color='b')"
    yield "def cursor_state():\n" \
          "    return (cursor.linev.get_xdata(), cursor.lineh.get_ydata(), cursor.linev.get_visible(), " \
          "cursor.lineh.get_visible(), [l.get_xdata() for l in multi.vlines], [l.get_ydata() for l in multi.hlines], " \
          "[l.get_visible() for l in multi.vlines + multi.hlines])"
    yield "cursor_state()"
    for a, point in (("ax", (0.3, 0.4)), ("ax", (0.7, 0.2)), ("ax2", (0.5, 0.5)), ("ax", (1.2, 0.5)),
                     ("ax", (0.6, 0.6))):
        yield f"mouse('motion_notify_event', px({a}, {point[0]}, {point[1]}))"
        yield "cursor_state()"
    yield "cursor.set_active(False)"
    yield "mouse('motion_notify_event', px(ax, 0.1, 0.1))"
    yield "cursor_state()"
    yield "cursor.set_active(True)"
    yield "cursor.visible = False"
    yield "mouse('motion_notify_event', px(ax, 0.2, 0.2))"
    yield "cursor_state()"
    yield "cursor.visible = True"
    yield "fig.canvas.draw()"
    yield "mouse('motion_notify_event', px(ax2, 0.2, 0.8))"
    yield "cursor_state()"
    yield "multi.disconnect()"
    yield "mouse('motion_notify_event', px(ax, 0.9, 0.9))"
    yield "cursor_state()"
    yield "cursor.disconnect_events()"
    yield "mouse('motion_notify_event', px(ax, 0.4, 0.4))"
    yield "cursor_state()"


def programs() -> Iterator[tuple[str, list[str]]]:
    """The widget and selector programs, without the header."""
    for name, statements in CONTROL_PROGRAMS.items():
        yield name, CONTROLS + list(statements())
    for name, (useblit, options, direction) in SPANS.items():
        yield f"selectors_span_{name}", SELECTING + list(_span(useblit, options, direction))
    for name, (kind, useblit, options) in RECTANGLES.items():
        yield f"selectors_{name}", SELECTING + list(_rectangle(kind, useblit, options))
    for useblit in (True, False):
        yield f"selectors_lasso_{'blit' if useblit else 'noblit'}", SELECTING + list(_lasso(useblit))
    for useblit, box in ((False, False), (True, True)):
        yield f"selectors_polygon_{'box' if box else 'plain'}", SELECTING + list(_polygon(useblit, box))
    for useblit in (True, False):
        yield f"widgets_cursors_{'blit' if useblit else 'noblit'}", list(_cursors(useblit))
