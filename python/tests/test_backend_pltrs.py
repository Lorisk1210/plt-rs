"""
Smoke tests of the ``pltrs`` backend (`pltrs.backends.backend_pltrs`):
pltrs' own windows behind matplotlib's backend API.

Each test runs its body in a fresh interpreter, as matplotlib's
interactive backend tests (``test_backends_interactive.py``, whose bodies
the first ones follow) run theirs: on a headless host, whose windows open
off screen and take injected input, and, when a display server is
reachable, on the display. The expected values are matplotlib's: the
assertions of its interactive tests, and the limits, points and file
names real matplotlib 3.11 computes for the same pointer positions
(Agg canvas, the same synthetic events).
"""

import inspect
import os
import signal
import subprocess
import sys
import textwrap
import time

import pytest

from pltrs import _c_internal_utils

import screen

#: How long a body may run, in seconds.
TIMEOUT = 60

#: What a body runs first on a headless host: the host, and the handle
#: driving its display as ``_display``.
_HEADLESS = """\
from pltrs.backends import _pltrs_loop
from pltrs._native import gui as _gui
_display = _pltrs_loop.connect_headless()
"""

#: Why the bodies cannot run on the display, or None when they can (on
#: a Mac only when allowed, as the display is the user's screen).
_no_display = screen.REFUSED or (None if _c_internal_utils.display_is_valid()
                                 else "no display server ($DISPLAY, $WAYLAND_DISPLAY)")

#: Where the bodies run: always headless, on the display when there is one.
HOSTS = ["headless", pytest.param("display", marks=pytest.mark.skipif(
    _no_display is not None, reason=str(_no_display)))]


def _source(body, *args, host="headless"):
    """The program running *body* with *args* on *host*."""
    prelude = _HEADLESS if host == "headless" else ""
    call = f"\n{body.__name__}(*{args!r})\n"
    return prelude + textwrap.dedent(inspect.getsource(body)) + call


def _environment(**extra):
    """The environment of a body: matplotlib's look, the pltrs backend."""
    return {**os.environ, "PLTRS_LOOK": "matplotlib", "MPLBACKEND": "pltrs",
            **extra}


def _run(body, *args, host="headless", env=None):
    """Run *body* in a fresh interpreter; return what it printed."""
    proc = subprocess.run(
        [sys.executable, "-c", _source(body, *args, host=host)],
        env=_environment(**(env or {})), capture_output=True, text=True,
        timeout=TIMEOUT)
    assert proc.returncode == 0, proc.stderr
    return proc.stdout


def _interactive(toolbar):
    """matplotlib's ``_test_interactive_impl`` on the pltrs backend."""
    import importlib.util
    import io
    import os
    import tempfile
    import warnings

    from PIL import Image

    import pltrs as mpl
    import pltrs.pyplot as plt
    from pltrs.backend_bases import FigureCanvasBase, KeyEvent
    from pltrs.backend_tools import ToolToggleBase

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")  # toolmanager is experimental.
        mpl.rcParams["toolbar"] = toolbar
    # The framework runs once a figure was made: another one cannot load.
    fig = plt.figure()
    plt.close(fig)
    if importlib.util.find_spec("tkinter"):
        try:
            mpl.use("tkagg", force=True)
        except ImportError:
            pass
        else:
            raise AssertionError("TkAgg loaded while pltrs' windows run")
    mpl.use("svg", force=True)
    fig = plt.figure()
    assert type(fig.canvas).__module__ == "pltrs.backends.backend_svg"
    plt.close("all")
    mpl.use("pltrs", force=True)

    fig, ax = plt.subplots()
    assert type(fig.canvas).__module__ == "pltrs.backends.backend_pltrs"
    assert fig.canvas.manager.get_window_title() == "Figure 1"
    if toolbar == "toolmanager":
        # A toolbar icon of an image in LA mode (matplotlib's GH 25174).
        with tempfile.TemporaryDirectory() as tempdir:
            path = os.path.join(tempdir, "test_la_icon.png")
            Image.new("LA", (26, 26)).save(path)

            class CustomTool(ToolToggleBase):
                image = path
                description = ""

            fig.canvas.manager.toolmanager.add_tool("test", CustomTool)
            fig.canvas.manager.toolbar.add_tool("test", "group")
    ax.plot([0, 1], [2, 3])
    if fig.canvas.toolbar:  # i.e. toolbar2.
        fig.canvas.toolbar.draw_rubberband(None, 1., 1, 2., 2)

    timer = fig.canvas.new_timer(1.)  # Floats are cast to int.
    timer.add_callback(KeyEvent("key_press_event", fig.canvas, "q")._process)
    # Quit upon draw.
    fig.canvas.mpl_connect("draw_event", lambda event: timer.start())
    fig.canvas.mpl_connect("close_event", print)

    result = io.BytesIO()
    fig.savefig(result, format="png", dpi=100)
    plt.show()
    plt.pause(0.5)  # The window is really closed.
    # Closing the figure gave it a base canvas back; it still saves.
    assert type(fig.canvas) == FigureCanvasBase, str(fig.canvas)
    result_after = io.BytesIO()
    fig.savefig(result_after, format="png", dpi=100)
    assert result.getvalue() == result_after.getvalue()


@pytest.mark.parametrize("host", HOSTS)
@pytest.mark.parametrize("toolbar", ["toolbar2", "toolmanager"])
def test_interactive_backend(host, toolbar):
    stdout = _run(_interactive, toolbar, host=host)
    assert stdout.count("CloseEvent") == 1


def _timers():
    """matplotlib's ``_impl_test_interactive_timers``."""
    from unittest.mock import Mock

    import pltrs.pyplot as plt

    pause_time = 0.5
    fig = plt.figure()
    plt.pause(pause_time)
    timer = fig.canvas.new_timer(0.1)
    mock = Mock()
    timer.add_callback(mock)
    timer.start()
    plt.pause(pause_time)
    timer.stop()
    assert mock.call_count > 1

    # A single shot fires once.
    mock.reset_mock()
    timer.single_shot = True
    timer.start()
    plt.pause(pause_time)
    assert mock.call_count == 1

    # It can be started a second time.
    timer.start()
    plt.pause(pause_time)
    assert mock.call_count == 2
    plt.close("all")


def _timer_intervals():
    import pltrs.pyplot as plt

    fig = plt.figure()
    timer = fig.canvas.new_timer(50)
    timer.add_callback(lambda: None)
    timer.start()
    running = timer._timer
    # An animation sets its interval again at every frame: the timer goes
    # on, its ticks due every 50 ms from its start, as a QTimer's.
    timer.interval = 50
    assert timer._timer == running
    timer.interval = 60
    assert timer._timer not in (None, running)
    timer.stop()
    print("KEPT", flush=True)


def test_setting_the_same_interval_keeps_the_timer_running():
    assert "KEPT" in _run(_timer_intervals)


@pytest.mark.parametrize("host", HOSTS)
def test_interactive_timers(host):
    _run(_timers, host=host)


def _thread_safety():
    """matplotlib's ``_test_thread_impl``: artists made and the figure
    drawn from worker threads, the window showing what they drew (its
    calls run by the main thread's loop)."""
    from concurrent.futures import ThreadPoolExecutor

    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    plt.pause(0.5)
    future = ThreadPoolExecutor().submit(ax.plot, [1, 3, 6])
    future.result()
    fig.canvas.mpl_connect("close_event", print)
    future = ThreadPoolExecutor().submit(fig.canvas.draw)
    plt.pause(0.5)
    future.result()
    plt.close()


@pytest.mark.parametrize("host", HOSTS)
def test_interactive_thread_safety(host):
    assert _run(_thread_safety, host=host).count("CloseEvent") == 1


def _number_of_draws():
    """matplotlib's ``_test_number_of_draws_script``: blitting draws
    nothing more."""
    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    ln, = ax.plot([0, 1], [1, 2], animated=True)
    plt.show(block=False)
    plt.pause(0.3)
    fig.canvas.mpl_connect("draw_event", print)
    bg = fig.canvas.copy_from_bbox(fig.bbox)
    ax.draw_artist(ln)
    fig.canvas.blit(fig.bbox)
    for _ in range(10):
        fig.canvas.restore_region(bg)
        ln, = ax.plot([0, 1], [1, 2])
        ax.draw_artist(ln)
        fig.canvas.blit(fig.bbox)
        fig.canvas.flush_events()
    plt.pause(0.1)


@pytest.mark.parametrize("host", HOSTS)
def test_blitting_events(host):
    draws = _run(_number_of_draws, host=host).count("DrawEvent")
    assert 0 < draws < 5


def _canvas_input():
    """Injected input reaches the callbacks as matplotlib's events, in
    matplotlib's coordinates (from the figure's bottom left)."""
    import pltrs.pyplot as plt
    from pltrs.backend_bases import MouseButton

    fig, ax = plt.subplots()
    seen = []

    def record(event):
        seen.append((event.name, event.x, event.y, event.button,
                     event.dblclick, sorted(event.modifiers), event.step))

    def record_key(event):
        seen.append((event.name, event.x, event.y, event.key))

    for name in ["button_press_event", "button_release_event",
                 "scroll_event"]:
        fig.canvas.mpl_connect(name, record)
    for name in ["key_press_event", "key_release_event"]:
        fig.canvas.mpl_connect(name, record_key)
    fig.canvas.mpl_connect(
        "figure_leave_event", lambda event: seen.append((event.name,)))
    plt.show(block=False)
    plt.pause(0.05)
    window = _gui.window_system_id(_pltrs_loop.host(), fig.canvas._window)
    _gui.inject_pointer(_display, window, "press", (100.0, 50.0), ["ctrl"],
                        (1, 1, 0.0))
    _gui.inject_pointer(_display, window, "release", (110.0, 60.0), [],
                        (1, 1, 0.0))
    _gui.inject_pointer(_display, window, "press", (110.0, 60.0), [],
                        (3, 2, 0.0))
    _gui.inject_pointer(_display, window, "scroll", (110.0, 60.0), [],
                        (0, 1, -1.0))
    _gui.inject_key(_display, window, "a", True, [])
    _gui.inject_key(_display, window, "a", False, [])
    # As in Qt, the pointer leaving with a button held leaves once the
    # button is released (outside).
    _gui.inject_window(_display, window, "leave")
    plt.pause(0.05)
    seen.append("released:")
    _gui.inject_pointer(_display, window, "release", (-10.0, 60.0), [],
                        (3, 1, 0.0))
    plt.pause(0.05)
    assert seen == [
        ("button_press_event", 100, 430, MouseButton.LEFT, False,
         ["ctrl"], 0),
        ("button_release_event", 110, 420, MouseButton.LEFT, False, [], 0),
        ("button_press_event", 110, 420, MouseButton.RIGHT, True, [], 0),
        ("scroll_event", 110, 420, "down", False, [], -1.0),
        ("key_press_event", 110, 420, "a"),
        ("key_release_event", 110, 420, "a"),
        "released:",
        ("button_release_event", -10, 420, MouseButton.RIGHT, False, [], 0),
        ("figure_leave_event",),
    ], seen


def test_canvas_input():
    _run(_canvas_input)


def _navigation():
    """The toolbar's pan and zoom follow injected drags, as matplotlib's
    do: the limits are those matplotlib computes for the same drags."""
    import pytest

    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    plt.show(block=False)
    plt.pause(0.05)
    host = _pltrs_loop.host()
    window = _gui.window_system_id(host, fig.canvas._window)

    def drag(start, end):
        _gui.inject_pointer(_display, window, "press", start, [], (1, 1, 0.0))
        _gui.inject_pointer(_display, window, "move", end, [], (1, 1, 0.0))
        _gui.inject_pointer(_display, window, "release", end, [], (1, 1, 0.0))
        plt.pause(0.05)

    toolbar = fig.canvas.toolbar
    toolbar._clicked("Pan")
    assert _gui.tool_checked(host, fig.canvas._window, "Pan")
    drag((320.0, 240.0), (220.0, 240.0))
    assert ax.get_xlim() == pytest.approx((2.0161290322580645,
                                           12.016129032258064))
    assert ax.get_ylim() == pytest.approx((0, 10))
    assert _gui.message(host, fig.canvas._window).startswith("(x, y) = ")
    toolbar._clicked("Home")
    assert ax.get_xlim() == (0, 10)
    toolbar._clicked("Zoom")
    assert not _gui.tool_checked(host, fig.canvas._window, "Pan")
    drag((200.0, 100.0), (400.0, 300.0))
    assert ax.get_xlim() == pytest.approx((2.4193548387096775,
                                           6.451612903225806))
    assert ax.get_ylim() == pytest.approx((3.441558441558442,
                                           8.852813852813854))
    toolbar._clicked("Back")
    assert ax.get_xlim() == (0, 10)


def test_navigation():
    _run(_navigation)


def _ginput_and_waitforbuttonpress():
    """ginput collects injected clicks and waitforbuttonpress sees an
    injected key while their event loops run."""
    import numpy as np

    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    plt.show(block=False)
    plt.pause(0.05)
    window = _gui.window_system_id(_pltrs_loop.host(), fig.canvas._window)

    def clicks():
        for at in [(200.0, 200.0), (300.0, 250.0)]:
            for kind in ["press", "release"]:
                _gui.inject_pointer(_display, window, kind, at, [], (1, 1, 0.0))

    timer = fig.canvas.new_timer(50)
    timer.single_shot = True
    timer.add_callback(clicks)
    timer.start()
    points = fig.ginput(2, timeout=5)
    np.testing.assert_allclose(points, [(2.4193548387096775, 6.147186147186148),
                                        (4.435483870967742, 4.794372294372295)])

    key = fig.canvas.new_timer(50)
    key.single_shot = True
    key.add_callback(
        lambda: _gui.inject_key(_display, window, "x", True, []))
    key.start()
    assert fig.waitforbuttonpress(timeout=5) is True
    assert fig.waitforbuttonpress(timeout=0.1) is None


def test_ginput_and_waitforbuttonpress():
    _run(_ginput_and_waitforbuttonpress)


def _manager():
    """The manager renames, resizes and closes its window."""
    import pltrs.pyplot as plt

    fig = plt.figure()
    manager = fig.canvas.manager
    manager.set_window_title("Before")
    plt.show(block=False)
    plt.pause(0.05)
    window = _gui.window_system_id(_pltrs_loop.host(), fig.canvas._window)
    assert _gui.shown_title(_display, window) == "Before"
    manager.set_window_title("After")
    assert manager.get_window_title() == "After"
    assert _gui.shown_title(_display, window) == "After"
    sizes = []
    fig.canvas.mpl_connect("resize_event", lambda event: sizes.append(
        (event.width, event.height)))
    manager.resize(500, 400)
    plt.pause(0.05)
    assert sizes == [(500, 400)], sizes
    assert tuple(fig.get_size_inches()) == (5, 4)
    manager.full_screen_toggle()
    assert _gui.is_fullscreen(_pltrs_loop.host(), fig.canvas._window)
    manager.full_screen_toggle()
    closed = []
    fig.canvas.mpl_connect("close_event", closed.append)
    _gui.inject_window(_display, window, "close")
    plt.pause(0.05)
    assert len(closed) == 1
    assert plt.get_fignums() == []
    assert window not in _gui.windows(_display)


def test_manager():
    _run(_manager)


def _save_figure(directory):
    """The toolbar's save asks with pltrs' file chooser, saves, and
    remembers the directory; cancelling saves nothing."""
    import os

    import pltrs as mpl
    import pltrs.pyplot as plt

    mpl.rcParams["savefig.directory"] = directory
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    plt.show(block=False)
    plt.pause(0.05)
    assert fig.canvas.get_default_filename() == "Figure_1.png"

    def press(key):
        def answer():
            chooser = _gui.windows(_display)[-1]
            _gui.inject_key(_display, chooser, key, True, [])
        timer = fig.canvas.new_timer(50)
        timer.single_shot = True
        timer.add_callback(answer)
        timer.start()

    press("enter")
    fname = fig.canvas.toolbar.save_figure()
    assert fname == os.path.join(directory, "Figure_1.png")
    assert os.path.getsize(fname) > 0
    assert mpl.rcParams["savefig.directory"] == directory
    press("escape")
    assert fig.canvas.toolbar.save_figure() is None


def test_save_figure(tmp_path):
    _run(_save_figure, str(tmp_path))


def _screen_density():
    """
    On a screen of twice the density, the canvas has twice the pixels
    and the figure twice the dpi, at the same size in inches, as Qt's
    canvas takes ``devicePixelRatio``; resize callbacks learn the size in
    logical pixels (`.FigureCanvasBase.get_width_height`), and the window
    shows the figure at the screen's density.
    """
    import pltrs.pyplot as plt

    _gui.set_scale_factor(_display, 2.0)
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    sizes = []
    fig.canvas.mpl_connect("resize_event", lambda event: sizes.append(
        (event.width, event.height)))
    plt.show(block=False)
    plt.pause(0.1)
    assert fig.canvas.device_pixel_ratio == 2
    assert fig.dpi == 200
    assert tuple(fig.get_size_inches()) == (6.4, 4.8)
    assert sizes == [(640, 480)], sizes
    window = _gui.window_system_id(_pltrs_loop.host(), fig.canvas._window)
    assert _gui.capture(_display, window).shape[1] == 1280


def test_screen_density():
    _run(_screen_density)


def _subplot_tool():
    """
    The subplot tool is pltrs' dialog (as Qt's is its own), opened once
    and closed with its figure. Driven from the keyboard, it behaves as
    matplotlib's Qt tool: the arrow keys step a value by 0.005, "Tight
    layout" calls `tight_layout` and "Reset" puts back the parameters the
    tool opened with. The values are those real matplotlib 3.11 takes
    (``SubplotToolQt``'s spin box stepped once, `tight_layout` of the same
    figure).
    """
    import warnings

    import pytest

    import pltrs.pyplot as plt
    from pltrs.backends.backend_pltrs import SubplotToolPltrs
    from pltrs.layout_engine import PlaceHolderLayoutEngine

    fig, ax = plt.subplots()
    ax.plot([1, 2])
    plt.show(block=False)
    plt.pause(0.05)
    host = _pltrs_loop.host()
    main = _gui.window_system_id(host, fig.canvas._window)
    tool = fig.canvas.toolbar.configure_subplots()
    assert isinstance(tool, SubplotToolPltrs)
    assert tool == fig.canvas.toolbar.configure_subplots()
    plt.pause(0.05)
    windows = _gui.windows(_display)
    assert len(windows) == 2
    dialog = next(window for window in windows if window != main)
    assert _gui.shown_title(_display, dialog) == "Subplot configuration tool"

    def press(key, modifiers=()):
        _gui.inject_key(_display, dialog, key, True, list(modifiers))
        _gui.inject_key(_display, dialog, key, False, list(modifiers))
        plt.pause(0.05)

    params = ["left", "bottom", "right", "top", "wspace", "hspace"]
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        press("right")  # The first control: the top border's slider.
        assert fig.subplotpars.top == pytest.approx(0.885)
        # Shift+Tab from it: Close, Reset, then Tight layout.
        for _ in range(3):
            press("tab", ["shift"])
        press(" ")
    expected = [0.07300347222222223, 0.08043981481481481, 0.9765625,
                0.96875, 0.2, 0.2]
    assert [getattr(fig.subplotpars, name) for name in params] == (
        pytest.approx(expected))
    assert isinstance(fig.get_layout_engine(), PlaceHolderLayoutEngine)
    press("tab")  # Reset
    press(" ")
    assert [getattr(fig.subplotpars, name) for name in params] == (
        pytest.approx([0.125, 0.11, 0.9, 0.88, 0.2, 0.2]))
    plt.close(fig)
    plt.pause(0.05)
    assert _gui.windows(_display) == [], "the tool closes with its figure"


def test_subplot_tool():
    _run(_subplot_tool)


def _subplot_tool_constrained():
    """
    Under constrained layout (pltrs' modern look), the subplot tool adjusts
    constrained layout's spacing, and its switch hands the subplots over
    to the subplot parameters, starting from the tight layout of the
    subplots where those place them; nothing warns, as matplotlib's own
    tool does at every move under constrained layout.
    """
    import warnings

    import pytest

    import pltrs.pyplot as plt
    from pltrs.layout_engine import (
        ConstrainedLayoutEngine, PlaceHolderLayoutEngine)

    def subplots():
        fig, axs = plt.subplots(2, 2)
        for ax in axs.flat:
            ax.set_title("title")
            ax.set_xlabel("x")
        return fig

    fig = subplots()
    assert isinstance(fig.get_layout_engine(), ConstrainedLayoutEngine)
    plt.show(block=False)
    plt.pause(0.05)
    host = _pltrs_loop.host()
    main = _gui.window_system_id(host, fig.canvas._window)
    tool = fig.canvas.toolbar.configure_subplots()
    plt.pause(0.05)
    dialog = next(w for w in _gui.windows(_display) if w != main)

    def press(key, modifiers=()):
        _gui.inject_key(_display, dialog, key, True, list(modifiers))
        _gui.inject_key(_display, dialog, key, False, list(modifiers))
        plt.pause(0.05)

    params = ["left", "bottom", "right", "top", "wspace", "hspace"]
    w_pad = fig.get_layout_engine().get()["w_pad"]
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        # The switch, its h_pad slider, its field, then w_pad's slider.
        for _ in range(3):
            press("tab")
        press("right")
        # To the next multiple of the step, 0.005.
        assert fig.get_layout_engine().get()["w_pad"] == pytest.approx(
            (w_pad // 0.005 + 1) * 0.005)
        # Back to the switch: the subplot parameters.
        for _ in range(3):
            press("tab", ["shift"])
        press("right")
    assert isinstance(fig.get_layout_engine(), PlaceHolderLayoutEngine)
    plain = subplots()
    plain.set_layout_engine("none")
    plain.tight_layout()
    assert [getattr(fig.subplotpars, name) for name in params] == (
        pytest.approx([getattr(plain.subplotpars, name) for name in params]))
    tool._apply("reset", ())
    assert isinstance(fig.get_layout_engine(), ConstrainedLayoutEngine)
    assert fig.get_layout_engine().get()["w_pad"] == w_pad


def _subplot_tool_before_show():
    """``plt.subplot_tool()`` before the figure is shown shows it with
    the tool, as matplotlib's backends open their tools then."""
    import pltrs.pyplot as plt
    from pltrs.backends.backend_pltrs import SubplotToolPltrs

    fig, ax = plt.subplots()
    tool = plt.subplot_tool()
    assert isinstance(tool, SubplotToolPltrs)
    assert len(_gui.windows(_display)) == 2


def test_subplot_tool_before_show():
    _run(_subplot_tool_before_show)


def test_subplot_tool_constrained():
    _run(_subplot_tool_constrained, env={"PLTRS_LOOK": "modern"})


def _headless():
    """matplotlib's ``_lazy_headless``: without a display, pyplot picks
    Agg, and the pltrs backend cannot load."""
    import pltrs.pyplot as plt

    assert plt.get_backend() == "agg"
    try:
        plt.switch_backend("pltrs")
    except ImportError:
        pass
    else:
        raise AssertionError("the pltrs backend loaded without a display")


def test_lazy_linux_headless():
    if sys.platform != "linux":
        pytest.skip("Linux needs a display server")
    env = {**_environment(DISPLAY="", WAYLAND_DISPLAY="")}
    del env["MPLBACKEND"]
    proc = subprocess.run([sys.executable, "-c", _source(_headless, host="")],
                          env=env, capture_output=True, text=True,
                          timeout=TIMEOUT)
    assert proc.returncode == 0, proc.stderr


def _auto_backend():
    """With a display, pyplot picks pltrs' windows by itself (named as
    matplotlib's ``macosx`` backend on macOS)."""
    import sys

    import pltrs
    import pltrs.pyplot as plt

    assert not isinstance(pltrs.rcParams._get("backend"), str)
    plt.figure()
    assert plt.get_backend() == ("macosx" if sys.platform == "darwin" else "pltrs")


@pytest.mark.skipif(_no_display is not None, reason=str(_no_display))
def test_pltrs_is_the_default_interactive_backend():
    env = _environment()
    del env["MPLBACKEND"]
    proc = subprocess.run(
        [sys.executable, "-c", _source(_auto_backend, host="display")],
        env=env, capture_output=True, text=True, timeout=TIMEOUT)
    assert proc.returncode == 0, proc.stderr


def _sigint(target):
    """matplotlib's ``_test_sigint_impl``: Ctrl+C interrupts show and
    pause."""
    import signal

    import pltrs.pyplot as plt

    # A shell starting the tests in the background (`cmd &` without job
    # control) has them ignore SIGINT, and Python then leaves it ignored.
    signal.signal(signal.SIGINT, signal.default_int_handler)

    fig = plt.figure()
    fig.canvas.mpl_connect("draw_event",
                           lambda *args: print("DRAW", flush=True))
    try:
        if target == "show":
            plt.show(block=True)
        else:
            plt.pause(10)
    except KeyboardInterrupt:
        print("SUCCESS", flush=True)


@pytest.mark.skipif(sys.platform == "win32", reason="no SIGINT to send")
@pytest.mark.parametrize("target", ["show", "pause"])
def test_sigint(target):
    proc = subprocess.Popen(
        [sys.executable, "-c", _source(_sigint, target)],
        env=_environment(), stdout=subprocess.PIPE, text=True)
    try:
        assert proc.stdout.readline().strip() == "DRAW"
        time.sleep(0.2)
        proc.send_signal(signal.SIGINT)
        stdout, _ = proc.communicate(timeout=TIMEOUT)
    finally:
        if proc.poll() is None:
            proc.kill()
    assert "SUCCESS" in stdout


def _signal_from_a_thread():
    import signal
    import threading
    import time

    import pltrs.pyplot as plt

    signal.signal(signal.SIGINT, signal.default_int_handler)
    plt.figure()
    sent = []

    def interrupt():
        time.sleep(0.5)
        sent.append(time.monotonic())
        signal.raise_signal(signal.SIGINT)
    threading.Thread(target=interrupt).start()
    try:
        plt.pause(30)
    except KeyboardInterrupt:
        took = time.monotonic() - sent[0]
        # Python's handler wrote to the loop's wake pipe (a socket on
        # Windows), which woke the blocked loop.
        assert took < 5, took
        print("INTERRUPTED", flush=True)


def test_a_signal_wakes_the_blocked_loop():
    assert "INTERRUPTED" in _run(_signal_from_a_thread)


def _input_hook():
    import socket
    import threading
    import time

    from pltrs.backends import _pltrs_loop

    # What IPython's prompt_toolkit hands the hook: a descriptor readable
    # once the terminal needs IPython.
    reader, writer = socket.socketpair()

    class Context:
        def fileno(self):
            return reader.fileno()

        def input_is_ready(self):
            return bool(reader.recv(1, socket.MSG_PEEK | socket.MSG_DONTWAIT))

    # The clock read first: the timer counts from its start, which a busy
    # machine may delay past the next line.
    start = time.monotonic()
    threading.Timer(0.3, writer.send, (b"x",)).start()
    _pltrs_loop._input_hook(Context())
    took = time.monotonic() - start
    assert 0.3 <= took < 30, took
    print("RETURNED", flush=True)


@pytest.mark.skipif(sys.platform == "win32",
                    reason="the hook polls on Windows")
def test_ipython_input_hook_waits_for_the_terminal():
    assert "RETURNED" in _run(_input_hook)


def _kernel_loop():
    import socket
    import threading
    import time

    from pltrs.backends import _pltrs_loop

    # What a kernel's shell stream does: its descriptor becomes readable
    # when ZMQ has something to look at, a message or not (the first
    # write), and flushing handles a message if one came (the second).
    reader, writer = socket.socketpair()
    reader.setblocking(False)
    messages = []

    class Stream:
        flushes = 0

        def flush(self, limit):
            assert limit == 1
            self.flushes += 1
            while True:
                try:
                    reader.recv(1)
                except BlockingIOError:
                    break
            return messages.pop() if messages else 0

    stream = Stream()

    def kernel_side():
        time.sleep(0.2)
        writer.send(b"x")
        time.sleep(0.2)
        messages.append(1)
        writer.send(b"x")
    # The clock read first, as in the input hook's test.
    start = time.monotonic()
    threading.Thread(target=kernel_side).start()
    _pltrs_loop._serve_kernel(stream, reader.fileno(), lambda: False)
    took = time.monotonic() - start
    assert 0.4 <= took < 30, took
    # Once at first, once per readable descriptor: no polling meanwhile.
    assert stream.flushes == 3, stream.flushes
    print("SERVED", flush=True)


def test_kernel_loop_waits_for_the_shell_stream():
    assert "SERVED" in _run(_kernel_loop)


def test_registry_knows_pltrs_beside_matplotlibs_backends():
    from pltrs.backends import BackendFilter, backend_registry

    assert backend_registry.is_valid_backend("pltrs")
    assert backend_registry.is_valid_backend("PLTRS")
    expected = "macosx" if sys.platform == "darwin" else "pltrs"
    assert backend_registry.resolve_backend("pltrs") == ("pltrs", expected)
    assert backend_registry.resolve_gui_or_backend("pltrs") == (
        "pltrs", "pltrs")
    assert "pltrs" in backend_registry.list_all()
    # matplotlib's lists of its own backends and frameworks stay as they are.
    assert "pltrs" not in backend_registry.list_builtin()
    assert "pltrs" not in backend_registry.list_builtin(
        BackendFilter.INTERACTIVE)
    assert "pltrs" not in backend_registry.list_gui_frameworks()
