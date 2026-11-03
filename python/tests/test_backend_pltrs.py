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

#: How long a body may run, in seconds.
TIMEOUT = 60

#: What a body runs first on a headless host: the host, and the handle
#: driving its display as ``_display``.
_HEADLESS = """\
from pltrs.backends import _pltrs_loop
from pltrs._native import gui as _gui
_display = _pltrs_loop.connect_headless()
"""

_has_display = _c_internal_utils.display_is_valid()

#: Where the bodies run: always headless, on the display when there is one.
HOSTS = ["headless", pytest.param("display", marks=pytest.mark.skipif(
    not _has_display, reason="no display server ($DISPLAY, $WAYLAND_DISPLAY)"))]


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
    _gui.inject_window(_display, window, "leave")
    plt.pause(0.05)
    assert seen == [
        ("button_press_event", 100, 430, MouseButton.LEFT, False,
         ["ctrl"], 0),
        ("button_release_event", 110, 420, MouseButton.LEFT, False, [], 0),
        ("button_press_event", 110, 420, MouseButton.RIGHT, True, [], 0),
        ("scroll_event", 110, 420, "down", False, [], -1.0),
        ("key_press_event", 110, 420, "a"),
        ("key_release_event", 110, 420, "a"),
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


def _subplot_tool():
    """The subplot tool opens in a window of its own, once."""
    import pltrs.pyplot as plt

    fig, ax = plt.subplots()
    plt.show(block=False)
    plt.pause(0.05)
    tool = fig.canvas.toolbar.configure_subplots()
    assert tool is not None
    assert tool == fig.canvas.toolbar.configure_subplots()
    manager = tool.figure.canvas.manager
    assert manager.get_window_title() == "Subplot configuration tool"
    assert manager.toolbar is None
    plt.pause(0.05)
    assert len(_gui.windows(_display)) == 2
    plt.close(fig)
    plt.pause(0.05)
    assert _gui.windows(_display) == [], "the tool closes with its figure"


def test_subplot_tool():
    _run(_subplot_tool)


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
    """With a display, pyplot picks pltrs' windows by itself."""
    import pltrs
    import pltrs.pyplot as plt

    assert not isinstance(pltrs.rcParams._get("backend"), str)
    plt.figure()
    assert plt.get_backend() == "pltrs"


@pytest.mark.skipif(not _has_display, reason="no display server")
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
    import pltrs.pyplot as plt

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
