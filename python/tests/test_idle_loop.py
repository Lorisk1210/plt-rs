"""
pltrs' event loop blocks until something happens, as Qt's does: an idle
window costs nothing, and what used to come with the loop's 20 ms slices
still comes at once: Ctrl+C in every blocking call, an idle draw and a
close asked for on another thread, other Python threads running while the
window idles. Each check runs a program (`gui_events.loop_probe`) on the
private X server of `gui_events`, and on a weston nested in it. The bounds
hold on a machine busy with other work; the slices' loop failed the
wake-up bound by an order of magnitude (40 a second).
"""

import sys

import pytest

from gui_events import idling
from gui_events.xdisplay import Unavailable, private_display, private_wayland

pytestmark = pytest.mark.skipif(not sys.platform.startswith("linux"),
                                reason="X11 and Wayland only")

#: The most wake-ups a second an idle window may make (QtAgg's makes
#: none; a few may come from the window system itself).
IDLE_WAKEUPS = 4
#: The longest a signal may take to interrupt a blocking call, and an idle
#: draw or a close asked for on another thread to happen, in seconds.
PROMPT = 2.0
#: The least share of its own speed a worker thread keeps while the window
#: idles (the main thread waits without the GIL).
WORKER_SHARE = 0.5


@pytest.fixture(scope="module")
def display():
    """A private X server for the module's tests."""
    try:
        with private_display() as name:
            yield name
    except Unavailable as error:
        pytest.skip(str(error))


@pytest.fixture(params=["x11", "wayland"])
def place(request, display, tmp_path):
    """``(display, Wayland socket or None, workdir)`` for each platform."""
    if request.param == "x11":
        yield display, None, tmp_path
        return
    try:
        with private_wayland(display, tmp_path / "weston") as weston:
            yield display, weston.socket, tmp_path
    except Unavailable as error:
        pytest.skip(str(error))


def test_an_idle_window_does_not_wake(place):
    display, wayland, workdir = place
    measured = idling.idle("pltrs", "pltrs", display, workdir, wayland)
    assert measured["wakeups"] < IDLE_WAKEUPS, measured
    assert measured["interrupt"] < PROMPT, measured


@pytest.mark.parametrize("action", [a for a in idling.BLOCKING
                                    if a != "show"])
def test_a_signal_interrupts_each_blocking_call(place, action):
    display, wayland, workdir = place
    assert idling.interrupt_latency("pltrs", "pltrs", action, display,
                                    workdir, wayland) < PROMPT


def test_calls_from_another_thread_wake_the_loop(place):
    display, wayland, workdir = place
    measured = idling.queued("pltrs", "pltrs", display, workdir, wayland)
    assert measured["redraw"] < PROMPT, measured
    assert measured["close"] < PROMPT, measured


def test_other_threads_run_while_the_window_idles(place):
    display, wayland, workdir = place
    assert idling.worker_progress("pltrs", "pltrs", display, workdir,
                                  wayland) > WORKER_SHARE
