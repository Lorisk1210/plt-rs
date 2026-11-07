"""
The toolbar's tools from their keyboard shortcuts (``keymap.*``): pan
and zoom with the mouse, the view history, grids, log scales, full
screen and quit; and the wheel's zoom.
"""

from . import at, axes_state, one_axes, scenario


def _navigated(plt, log):
    fig, ax = one_axes(plt)
    ax.plot([1, 9], [1, 9])
    report = lambda event: log.note(  # noqa: E731
        "view", after=event.name, key=getattr(event, "key", None),
        mode=str(fig.canvas.toolbar.mode), **axes_state(ax))
    fig.canvas.mpl_connect("key_release_event", report)
    fig.canvas.mpl_connect("button_release_event", report)
    fig.canvas.mpl_connect("scroll_event", report)
    return fig


@scenario("navigation", toolbar=True, keymaps=True)
def navigation(plt, log):
    return _navigated(plt, log)


@navigation.steps
def _(d):
    d.move(*at(5, 5))
    # Pan: a drag with the left button moves, with the right one zooms.
    d.key("p")
    d.drag(1, [at(5, 5), at(4, 5), at(3, 4)])
    d.drag(3, [at(5, 5), at(6, 6)])
    with d.hold("x"):
        d.drag(1, [at(5, 5), at(4, 4)])
    d.key("p")
    # Zoom: a rectangle with the left button zooms in, the right one out.
    d.key("o")
    d.drag(1, [at(2, 2), at(4, 3), at(6, 6)])
    d.drag(3, [at(2, 2), at(6, 6)])
    with d.hold("y"):
        d.drag(1, [at(2, 2), at(6, 6)])
    d.key("o")
    # The view history: back, back, forward, home, and the mouse's
    # buttons.
    for key in ("c", "left", "v", "right", "backspace", "h"):
        d.key(key)
    d.click(8)
    d.click(9)
    d.key("r")
    # Grids, log scales.
    for key in ("g", "g", "G", "G", "g", "l", "l", "k", "L"):
        d.key(key)
    d.move(10, 10)
    d.key("g")
    d.move(*at(5, 5))
    # The wheel zooms with ctrl held (and x or y), recording the view.
    with d.hold("ctrl"):
        d.scroll(1)
    with d.hold("x"):
        d.scroll(-1)
    d.key("h")
    d.key("q")


# Without a window manager, a window cannot go full screen: the keys only
# ask for it.
@scenario("fullscreen-keys", toolbar=True, keymaps=True, x11_only=True)
def fullscreen_keys(plt, log):
    return _navigated(plt, log)


@fullscreen_keys.steps
def _(d):
    d.move(*at(5, 5))
    d.key("f")
    d.key("ctrl+f")
    d.key("q")


@scenario("quit-keys", toolbar=True, keymaps=True)
def quit_keys(plt, log):
    return _navigated(plt, log)


@quit_keys.steps
def _(d):
    d.move(*at(5, 5))
    d.key("ctrl+w")
