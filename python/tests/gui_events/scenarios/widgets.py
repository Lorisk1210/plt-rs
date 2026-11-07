"""
The widgets of ``matplotlib.widgets`` used by hand: buttons, check and
radio buttons, sliders, the text box and the cursors. Each reports what
it tells the program (its callbacks) and, when the window closes, its
state; the driver takes snapshots where the widget shows its state.
"""

from . import at, one_axes, scenario


def _widgets(plt):
    """The library's ``widgets`` module (pyplot's package's)."""
    import importlib
    return importlib.import_module(plt.__name__.rpartition(".")[0] + ".widgets")


def _on_close(fig, report):
    """Call *report* when the window closes."""
    fig.canvas.mpl_connect("close_event", lambda event: report())


def _figure(plt):
    return plt.figure(figsize=(4, 3), dpi=100)


@scenario("button")
def button(plt, log):
    widgets = _widgets(plt)
    fig = _figure(plt)
    # x 120 to 280, y 120 to 180.
    ax = fig.add_axes([0.3, 0.4, 0.4, 0.2])
    fig._button = widgets.Button(ax, "Press", color="0.85", hovercolor="0.6")
    fig._button.on_clicked(lambda event: log.note(
        "clicked", button=int(event.button), x=event.x, y=event.y))
    return fig


@button.steps
def _(d):
    d.move(60, 60)
    d.snapshot("away")
    d.move(200, 150)
    d.snapshot("hover")
    d.click(1)
    d.click(3)
    # Pressed inside, released outside: no click; and the other way.
    d.press(1)
    d.move(60, 60)
    d.release(1)
    d.press(1)
    d.move(200, 150)
    d.release(1)
    d.leave()
    d.snapshot("left")
    d.close()


@scenario("check-radio")
def check_radio(plt, log):
    widgets = _widgets(plt)
    fig = _figure(plt)
    # Boxes at (61, 210), (61, 150), (61, 90); labels from x 75.
    check = widgets.CheckButtons(fig.add_axes([0.1, 0.1, 0.35, 0.8]),
                                 ["one", "two", "three"], [True, False, True])
    check.on_clicked(lambda label: log.note(
        "checked", label=label, status=check.get_status()))
    # Buttons at (241, 210), (241, 150), (241, 90).
    radio = widgets.RadioButtons(fig.add_axes([0.55, 0.1, 0.35, 0.8]),
                                 ["red", "green", "blue"], active=1)
    radio.on_clicked(lambda label: log.note(
        "selected", label=label, value=radio.value_selected,
        index=radio.index_selected))
    _on_close(fig, lambda: log.note(
        "state", status=check.get_status(), checked=check.get_checked_labels(),
        radio=radio.value_selected))
    fig._widgets = check, radio
    return fig


@check_radio.steps
def _(d):
    d.snapshot("initial")
    for point in [(61, 150), (61, 210), (85, 90), (61, 120), (241, 210),
                  (241, 210), (270, 90), (241, 120), (300, 280)]:
        d.move(*point)
        d.click(1)
    d.move(61, 150)
    d.click(3)
    d.snapshot("clicked")
    d.close()


@scenario("sliders")
def sliders(plt, log):
    widgets = _widgets(plt)
    fig = _figure(plt)
    # Horizontal: x 60 to 300 for 0 to 10; vertical: y 30 to 270 for -1 to 1.
    horizontal = widgets.Slider(fig.add_axes([0.15, 0.45, 0.6, 0.1]), "h",
                                0, 10, valinit=3, valstep=0.5)
    vertical = widgets.Slider(fig.add_axes([0.85, 0.1, 0.05, 0.8]), "v", -1,
                              1, valinit=0, orientation="vertical")
    horizontal.on_changed(lambda val: log.note("h", val=val))
    vertical.on_changed(lambda val: log.note("v", val=val))
    _on_close(fig, lambda: log.note("state", h=horizontal.val,
                                    v=vertical.val))
    fig._widgets = horizontal, vertical
    return fig


@sliders.steps
def _(d):
    d.snapshot("initial")
    d.move(228, 150)
    d.click(1)
    d.drag(1, [(228, 150), (240, 150), (251, 150), (400 - 1, 150)])
    d.drag(1, [(100, 150), (40, 150), (90, 152)])
    d.key("right")
    d.key("left")
    d.drag(1, [(350, 150), (350, 200), (350, 10), (350, 222)])
    d.drag(3, [(350, 150), (350, 100)])
    d.snapshot("moved")
    d.close()


@scenario("range-slider")
def range_slider(plt, log):
    widgets = _widgets(plt)
    fig = _figure(plt)
    # x 80 to 320 for 0 to 10.
    slider = widgets.RangeSlider(fig.add_axes([0.2, 0.45, 0.6, 0.1]), "range",
                                 0, 10, valinit=(2, 6), valstep=0.25)
    slider.on_changed(lambda val: log.note("range", val=list(val)))
    _on_close(fig, lambda: log.note("state", val=list(slider.val)))
    fig._widgets = slider
    return fig


@range_slider.steps
def _(d):
    d.snapshot("initial")
    d.drag(1, [(128, 150), (110, 150), (104, 150)])
    d.drag(1, [(224, 150), (260, 150), (300, 150)])
    d.drag(1, [(200, 150), (190, 150)])
    d.drag(1, [(300, 150), (90, 150), (60, 150)])
    d.snapshot("moved")
    d.close()


@scenario("textbox")
def textbox(plt, log):
    widgets = _widgets(plt)
    fig = _figure(plt)
    # x 80 to 320, y 135 to 165.
    box = widgets.TextBox(fig.add_axes([0.2, 0.45, 0.6, 0.1]), "label",
                          initial="abc")
    box.on_text_change(lambda text: log.note("change", text=text))
    box.on_submit(lambda text: log.note("submit", text=text))
    _on_close(fig, lambda: log.note("state", text=box.text,
                                    cursor=box.cursor_index))
    fig._widgets = box
    return fig


@textbox.steps
def _(d):
    d.move(200, 150)
    d.click(1)
    d.snapshot("typing")
    d.type("xy Z")
    d.key("backspace")
    d.key("left")
    d.key("left")
    d.type("Q")
    d.key("delete")
    d.key("home")
    d.type("1")
    d.key("end")
    d.key("escape")
    d.key("ctrl+a")
    d.snapshot("typed")
    d.key("enter")
    d.move(200, 50)
    d.click(1)
    d.snapshot("done")
    d.close()


# A key the driver binds to a spare keycode, which a nested weston does
# not see: X11 only.
@scenario("textbox-accents", x11_only=True)
def textbox_accents(plt, log):
    return textbox(plt, log)


@textbox_accents.steps
def _(d):
    d.move(90, 150)
    d.click(1)
    d.type("é")
    d.key("enter")
    d.close()


@scenario("cursor")
def cursor(plt, log):
    widgets = _widgets(plt)
    fig, ax = one_axes(plt)
    fig._cursor = widgets.Cursor(ax, useblit=True, color="red", linewidth=1)
    return fig


@cursor.steps
def _(d):
    d.move(*at(3, 4))
    d.snapshot("first")
    d.move(*at(7, 6))
    d.snapshot("moved")
    d.move(20, 20)
    d.snapshot("outside")
    d.move(*at(5, 5))
    d.leave()
    d.close()


@scenario("multicursor")
def multicursor(plt, log):
    widgets = _widgets(plt)
    fig, axes = plt.subplots(1, 2, figsize=(4, 3), dpi=100)
    for ax in axes:
        ax.set(xlim=(0, 10), ylim=(0, 10))
    fig._cursor = widgets.MultiCursor(None, axes, useblit=True, color="r",
                                      horizOn=True, linewidth=1)
    return fig


@multicursor.steps
def _(d):
    d.move(100, 150)
    d.snapshot("left")
    d.move(300, 100)
    d.snapshot("right")
    d.move(200, 150)
    d.snapshot("between")
    d.close()
