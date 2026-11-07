"""The mouse, the wheel, the keyboard and the window itself."""

from . import WIDTH, scenario


def two_axes(plt):
    """A 400 × 300 figure with two axes side by side, limits 0 to 10 (the
    left one spans x 50 to 191, the right one 219 to 360)."""
    fig, axes = plt.subplots(1, 2, figsize=(4, 3), dpi=100)
    for ax in axes:
        ax.set(xlim=(0, 10), ylim=(0, 10))
    return fig


@scenario("pointer")
def pointer(plt, log):
    return two_axes(plt)


@pointer.steps
def _(d):
    d.move(100, 150)
    d.move(110, 160)
    for button in (1, 2, 3, 8, 9):
        d.click(button)
    # Between the axes, then into the right one with the left button.
    d.move(200, 150)
    d.press(1)
    d.move(310, 140)
    d.move(320, 130)
    d.release(1)
    # The right button dragged out of the axes and back into the left one.
    d.press(3)
    d.move(200, 150)
    d.move(100, 100)
    d.release(3)
    # Two buttons at once.
    d.press(1)
    d.press(3)
    d.move(105, 105)
    d.release(1)
    d.move(110, 110)
    d.release(3)
    d.double_click(1)
    d.double_click(3)
    d.wait(0.6)
    d.click(1)
    # Out of the window with a button held, released outside, back in.
    d.press(1)
    d.leave()
    d.release(1)
    d.move(50, 50)
    d.move(0, 0)
    d.move(WIDTH - 1, 299)
    d.close()


@scenario("wheel")
def wheel(plt, log):
    return two_axes(plt)


@wheel.steps
def _(d):
    d.move(100, 150)
    d.scroll(1)
    d.scroll(-1)
    d.scroll(3)
    for keys in ("ctrl", "shift", "alt", "ctrl+shift"):
        with d.hold(keys):
            d.scroll(1)
            d.scroll(-1)
    d.scroll("left")
    d.scroll("right")
    d.move(200, 20)
    d.scroll(1)
    d.close()


#: Keys typed in the keyboard scenario, as the driver names them.
TYPED = [
    "a", "z", "A", "Z", "1", "!", "@", "-", "+", ",", ".", "/", "\\", "#",
    "space", "ctrl+a", "ctrl+A", "ctrl+1", "alt+b", "alt+B", "ctrl+alt+c",
    "super+d", "ctrl+shift+left", "shift+left", "ctrl+left", "alt+left",
    "enter", "escape", "tab", "backspace", "delete", "insert",
    "home", "end", "pageup", "pagedown", "left", "right", "up", "down",
    "pause", "menu", "caps_lock", "a", "caps_lock", "shift", "control",
    "alt", "super",
    "altgr", *(f"f{n}" for n in range(1, 13)), "ctrl+f5", "shift+f5",
]


@scenario("keys")
def keys(plt, log):
    return two_axes(plt)


@keys.steps
def _(d):
    d.move(100, 150)
    for key in TYPED:
        d.key(key)
    d.close()


# Where pltrs means to differ from QtAgg: Qt's canvas composes nothing and
# reports no dead key, and keeps Shift+Tab for moving the focus. pltrs
# names dead keys as matplotlib's GTK and Tk backends do, names a key that
# completes a compose sequence by the character it typed (so that text
# boxes take accented letters), and gives the canvas Shift+Tab (Ctrl+Tab
# moves the focus to the toolbar). X11 only: the driver binds the dead
# keys to spare keycodes, which a nested weston, keeping the keyboard
# layout it started with, does not see.
@scenario("pltrs-keys", reference=None, x11_only=True)
def pltrs_keys(plt, log):
    return two_axes(plt)


#: The keys of the pltrs-keys scenario, and the names of their presses.
COMPOSED = [
    (("dead_acute", "e"), ["dead_acute", "é"]),
    (("dead_circumflex", "o"), ["dead_circumflex", "ô"]),
    (("dead_acute", "x"), ["dead_acute", "x"]),
    (("dead_diaeresis", "A"), ["dead_diaeresis", "shift", "Ä"]),
    (("shift+tab",), ["shift", "shift+tab"]),
]


@pltrs_keys.steps
def _(d):
    d.move(100, 150)
    for keys, _ in COMPOSED:
        for key in keys:
            d.key(key)
    d.close()


@pltrs_keys.check
def _(records):
    pressed = [r["key"] for r in records if r["name"] == "key_press_event"]
    expected = [name for _, names in COMPOSED for name in names]
    return [] if pressed == expected else [f"keys {pressed} != {expected}"]


@scenario("held-keys")
def held_keys(plt, log):
    return two_axes(plt)


@held_keys.steps
def _(d):
    d.move(100, 150)
    with d.hold("shift"):
        d.move(110, 150)
        d.click(1)
    with d.hold("ctrl"):
        d.move(120, 150)
        d.drag(1, [(120, 150), (130, 160)])
    with d.hold("x"):
        d.move(140, 150)
        d.click(1)
    # A key typed while the pointer is off the window (it keeps the
    # focus), and the pointer coming back with a key held.
    d.leave()
    d.key("a")
    with d.hold("shift"):
        d.move(100, 100)
    d.close()


@scenario("resize", x11_only=True)
def resize(plt, log):
    return two_axes(plt)


@resize.steps
def _(d):
    d.move(100, 150)
    d.resize(500, 400)
    d.move(100, 150)
    d.click(1)
    d.resize(300, 200)
    d.move(250, 150)
    d.click(1)
    d.close()
