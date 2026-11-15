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

The loop (`run`) blocks until something happens, as Qt's does, so that
idle windows cost nothing: input, the next timer, a call another thread
makes on the windows (queued to the loop's thread, which runs it: a
worker drawing a shown figure has its window show the new pixels), an
idle draw or a stop asked for on another thread (`wake`), or a signal.
For signals, Python's handler writes to a pipe whose writes wake the
loop (``signal.set_wakeup_fd``, around the loop on the main thread, as
matplotlib's Qt backend does), so that Ctrl+C interrupts `plt.show()`
at once; the program's own wakeup descriptor (asyncio's, a kernel's) is
put back afterwards, with the signals that came meanwhile written to it.
While it waits, the loop does without the GIL, so the program's other
threads run. Idle draws (``draw_idle``) happen before each wait, as Tk's
do when its loop is idle, or when the window is repainted before then
(after it was exposed or resized, before the input that came after
that), as Qt's canvas draws when it paints.
"""

import atexit
import contextlib
import os
import select
import signal
import socket
import sys
import threading
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

#: How often IPython's input hook looks whether the user typed on
#: Windows, in seconds (`_input_hook`).
SLICE = 0.02

#: The host, once made.
_host = None
#: The thread the host was made on, which runs its loop (`threading`'s
#: identifier).
_host_thread = None
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
    global _host, _host_thread
    if _host is None:
        _host = _gui.connect()
        _host_thread = threading.get_ident()
    return _host


def connect_headless():
    """
    Use a host without a display from now on, for tests: windows open
    off screen. Returns the handle driving them (``_native.gui``'s
    ``inject_pointer``, ``inject_key``, ``inject_window``, ``capture``).
    """
    global _host, _host_thread
    _host, handle = _gui.connect_headless()
    _host_thread = threading.get_ident()
    return handle


def wake():
    """Have the loop look at what changed, when called on another thread
    than the loop's (an idle draw asked for, a loop asked to stop): the
    loop's own thread looks before it waits again."""
    if _host is not None and threading.get_ident() != _host_thread:
        _gui.wake()


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


@atexit.register
def _stop_timers():
    """Stop the timers still running when Python exits, while this module
    and the backend are whole. The loop keeps running timers alive, so
    one left running (its window closed during an animation) would
    otherwise go only when the modules are torn down, and its `__del__`
    would find the backend's globals gone (as matplotlib's Qt timers
    check that their ``QTimer`` still exists)."""
    for timer in list(_timers.values()):
        timer.stop()


def draw_idle(canvas):
    """Draw *canvas* once the loop is next idle (once, however often it
    is asked meanwhile)."""
    if not any(pending() is canvas for pending in _idle_draws):
        _idle_draws.append(weakref.ref(canvas))
        wake()


def _draw_pending():
    """Run the idle draws asked for; those asked for meanwhile wait for
    the next round, as Qt's single-shot timers do."""
    pending = _idle_draws[:]
    del _idle_draws[:]
    for ref in pending:
        canvas = ref()
        if canvas is not None:
            _draw(canvas)


def _draw_exposed(canvas):
    """Run the idle draw of *canvas* asked for, if any, now that its
    window is repainted (matplotlib's Qt canvas draws in its
    ``paintEvent``)."""
    for index, ref in enumerate(_idle_draws):
        if ref() is canvas:
            del _idle_draws[index]
            _draw(canvas)
            return


def _draw(canvas):
    """Run the idle draw of *canvas*."""
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
    if canvas is None:
        return
    if kind == "expose":
        _draw_exposed(canvas)
    else:
        canvas._handle_window_event(event)


def process_events(timeout=0):
    """
    Run one round of the loop: whatever the window system has within
    *timeout* seconds (none: wait until something happens; no wait while
    idle draws are due), then the idle draws, then the calls other threads
    queued for the windows. A window shown reports its size before it is
    first drawn, as Qt's widgets do.
    """
    for event in _gui.pump(host(), 0 if _idle_draws else timeout):
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
        with _woken_by_signals():
            while keep_going():
                wait = (None if deadline is None
                        else max(deadline - time.monotonic(), 0))
                process_events(wait)
                if deadline is not None and time.monotonic() >= deadline:
                    break
    finally:
        serve_threads(False)


@contextlib.contextmanager
def _woken_by_signals():
    """
    Have signals wake the loop meanwhile, so that Python runs their
    handlers at once (Ctrl+C raises `KeyboardInterrupt` in the loop): on
    the main thread, the only one Python handles signals and lets set the
    wakeup descriptor on, by having Python's handler write to the host's
    signal pipe. The descriptor the program had (asyncio's, an IPython
    kernel's) is put back afterwards, as matplotlib's Qt backend does, and
    the signals' numbers that came meanwhile are written to it, for its
    reader. A loop inside a loop keeps the pipe.
    """
    if threading.current_thread() is not threading.main_thread():
        yield
        return
    fd = _gui.signal_fd()
    try:
        outer = signal.set_wakeup_fd(fd)
    except ValueError:
        # Not the main interpreter: its signals are not Python's to see.
        yield
        return
    if outer != fd:
        # What came between loops was for nobody.
        _gui.take_signals()
    try:
        yield
    finally:
        signal.set_wakeup_fd(outer)
        if outer != fd:
            came = _gui.take_signals()
            if outer != -1 and came:
                _forward_signals(outer, came)


def _forward_signals(fd, numbers):
    """Write the signals' *numbers* to the program's wakeup descriptor
    *fd* (a socket handle on Windows), as Python's handler would have:
    what does not fit is dropped, as Python drops it."""
    try:
        if sys.platform == "win32":
            sock = socket.socket(fileno=fd)
            try:
                sock.send(numbers)
            finally:
                sock.detach()
        else:
            os.write(fd, numbers)
    except OSError:
        pass


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


def _until_readable(fd):
    """Run the loop until descriptor *fd* (a socket on Windows) is
    readable: a thread waits for it without the GIL and wakes the loop,
    as Qt's socket notifiers do."""
    ready = threading.Event()

    def watch():
        try:
            select.select([fd], [], [])
        finally:
            ready.set()
            _gui.wake()
    threading.Thread(target=watch, name="pltrs descriptor watch",
                     daemon=True).start()
    while not ready.is_set():
        process_events(None)


def _input_hook(context):
    """
    IPython's input hook for the windows (``%gui pltrs``): run the loop
    until the terminal needs IPython (the user types, or prompt_toolkit's
    own loop has something to do), which `context.fileno()` becoming
    readable tells, as IPython's Qt hook has a socket notifier tell it. On
    Windows, where only sockets can be waited for so, the loop looks every
    `SLICE` seconds, as IPython's Qt hook polls there.
    """
    if sys.platform == "win32":
        while not context.input_is_ready():
            process_events(SLICE)
        return
    _until_readable(context.fileno())


def _kernel_loop(kernel):
    """ipykernel's event loop for the windows (``%gui pltrs`` and
    ``%matplotlib pltrs`` in a Jupyter kernel): run them until a message
    comes for the kernel, as its loops of the other toolkits do, waiting
    for the shell stream's ZMQ descriptor as its Qt loop does (readable
    once ZMQ has something to look at, a message or not)."""
    import zmq
    from ipykernel.eventloops import get_shell_stream
    stream = get_shell_stream(kernel)
    _serve_kernel(stream, stream.socket.getsockopt(zmq.FD),
                  lambda: kernel.shell.exit_now)


def _serve_kernel(stream, fd, exiting):
    """Run the loop until *stream* (a ZMQ stream) handles a message
    (``flush``), looking whenever its descriptor *fd* is readable, or
    until *exiting()*."""
    while not exiting():
        if stream.flush(limit=1):
            return
        _until_readable(fd)


def _register_ipython_hook():
    """Let IPython's terminal run the windows while it waits for input,
    as it does Qt's and Tk's, when IPython is running, and a Jupyter
    kernel between its messages."""
    pt_inputhooks = sys.modules.get("IPython.terminal.pt_inputhooks")
    if pt_inputhooks is not None and FRAMEWORK not in pt_inputhooks.backends:
        pt_inputhooks.register(FRAMEWORK, _input_hook)
    if "ipykernel" in sys.modules:
        from ipykernel import eventloops
        if FRAMEWORK not in eventloops.loop_map:
            eventloops.register_integration(FRAMEWORK)(_kernel_loop)


_register_ipython_hook()
