"""
The event loop of pltrs' own windows, behind the ``pltrs`` backend
(`.backend_pltrs`, and ``macosx`` on macOS).

pltrs (Rust) runs the windows (``_native.gui``, a ``WindowHost``): each
shows its figure's canvas above a toolbar of the backend's buttons, and
reports what happens as tuples, which this module hands to the canvas
of the window, the timer or the file chooser waiting for them. The host
is made when first needed, normally on the main thread, and is tied to
the thread that makes it (macOS runs windows on its main thread only);
from then on its framework counts as running (`is_connected`, which
`.cbook._get_running_interactive_framework` asks), as a Qt application
does once it exists.

The loop runs in slices of `SLICE` seconds, between which Python handles
signals (Ctrl+C interrupts `plt.show()`) and the calls other threads make
on the windows run (a worker drawing a shown figure has its window show
the new pixels); idle draws (``draw_idle``) happen before each slice, as
Tk's do when its loop is idle.
"""

import sys
import time
import traceback
import weakref

from .._native import gui as _gui
from .._native import run_queued_calls, serve_threads
from .registry import _PLTRS_FRAMEWORK

#: The interactive framework the windows are (`.FigureCanvasBase.
#: required_interactive_framework`): ``"macosx"`` on macOS, where they
#: stand for matplotlib's macOS backend, else ``"pltrs"``.
FRAMEWORK = _PLTRS_FRAMEWORK

#: The longest the loop waits for the window system at once, in seconds.
SLICE = 0.02

#: The host, once made.
_host = None
#: The canvases shown in the windows, by window number.
_canvases = weakref.WeakValueDictionary()
#: The running timers by number (the loop keeps them alive, as Tk's
#: ``after`` keeps its callbacks).
_timers = {}
#: The canvases waiting for an idle draw, in order.
_idle_draws = []
#: The files chosen in the file choosers of `choose_file`, by window.
_chosen = {}


def is_connected():
    """Whether the windows' host exists (the framework is running)."""
    return _host is not None


def host():
    """The host of the windows, connected to the window system when first
    asked for; `RuntimeError` when there is none."""
    global _host
    if _host is None:
        _host = _gui.connect()
    return _host


def connect_headless():
    """
    Use a host without a display from now on, for tests: windows open
    off screen. Returns the handle driving them (``_native.gui``'s
    ``inject_pointer``, ``inject_key``, ``inject_window``, ``capture``).
    """
    global _host
    _host, handle = _gui.connect_headless()
    return handle


def add_window(canvas, title, size, toolbar):
    """
    Describe a window titled *title* showing *canvas*, its canvas *size*
    ``(width, height)`` logical pixels, with a toolbar when *toolbar*;
    it opens when shown. Returns its number.
    """
    width, height = size
    window = _gui.add_window(host(), canvas.figure._native, title,
                             float(width), float(height), bool(toolbar))
    _canvases[window] = canvas
    return window


def close(window):
    """Close the window numbered *window* and forget it."""
    _canvases.pop(window, None)
    _chosen.pop(window, None)
    _gui.close(host(), window)


def windows_shown():
    """Whether any window is open on screen."""
    return any(_gui.is_shown(host(), window) for window in list(_canvases))


def start_timer(timer, interval, repeat):
    """Start firing *timer* (`.TimerBase._on_timer`) every *interval*
    seconds, or once unless *repeat*; returns the host's timer number."""
    number = _gui.start_timer(host(), interval, repeat)
    _timers[number] = timer
    return number


def stop_timer(number):
    """Stop the timer numbered *number*."""
    _timers.pop(number, None)
    if _host is not None:
        _gui.stop_timer(_host, number)


def draw_idle(canvas):
    """Draw *canvas* once the loop is next idle (once, however often it
    is asked meanwhile)."""
    if not any(pending() is canvas for pending in _idle_draws):
        _idle_draws.append(weakref.ref(canvas))


def _draw_pending():
    """Run the idle draws asked for."""
    while _idle_draws:
        canvas = _idle_draws.pop(0)()
        if canvas is None:
            continue
        try:
            canvas._draw_idle_now()
        except Exception:
            # As matplotlib's Qt backend: a failing draw does not stop the
            # event loop.
            traceback.print_exc()


def _dispatch(event):
    """Hand one event of the host to whom it is for."""
    kind = event[0]
    if kind == "timer":
        timer = _timers.get(event[1])
        if timer is not None:
            timer._fired(event[1])
        return
    if kind == "file":
        _chosen[event[1]] = event[2]
        return
    canvas = _canvases.get(event[1])
    if canvas is not None:
        canvas._handle_window_event(event)


def process_events(timeout=0):
    """
    Run one round of the loop: the idle draws, then whatever the window
    system has within *timeout* seconds (none: wait until something
    happens), then the calls other threads queued for the windows.
    """
    _draw_pending()
    for event in _gui.pump(host(), timeout):
        _dispatch(event)
    _draw_pending()
    run_queued_calls()


def run(keep_going, timeout=None):
    """
    Run the loop while *keep_going()* is true, for *timeout* seconds at
    most (none: no limit), serving the calls other threads make on the
    windows meanwhile.
    """
    deadline = None if timeout is None else time.monotonic() + timeout
    serve_threads(True)
    try:
        while keep_going():
            wait = SLICE
            if deadline is not None:
                wait = min(wait, max(deadline - time.monotonic(), 0))
            process_events(wait)
            if deadline is not None and time.monotonic() >= deadline:
                break
    finally:
        serve_threads(False)


def choose_file(window, filename, directory=None):
    """
    Ask where to save the figure of window *window* with the file chooser
    of pltrs' windows, in *directory* (else in :rc:`savefig.directory`, or
    the working directory), proposing *filename*: returns the path chosen,
    or None when cancelled. The window takes no input meanwhile, while the
    loop runs everything else.
    """
    _chosen.pop(window, None)
    _gui.choose_file(host(), window, directory, filename)
    run(lambda: window not in _chosen and window in _canvases)
    return _chosen.pop(window, None)


def _input_hook(context):
    """IPython's input hook for the windows: run the loop until the user
    types (``%gui pltrs``)."""
    while not context.input_is_ready():
        process_events(SLICE)


def _register_ipython_hook():
    """Let IPython's terminal run the windows while it waits for input,
    as it does Qt's and Tk's, when IPython is running."""
    pt_inputhooks = sys.modules.get("IPython.terminal.pt_inputhooks")
    if pt_inputhooks is not None and FRAMEWORK not in pt_inputhooks.backends:
        pt_inputhooks.register(FRAMEWORK, _input_hook)


_register_ipython_hook()
