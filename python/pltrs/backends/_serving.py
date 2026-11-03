"""
The calls other threads make on pltrs objects, run by a GUI toolkit's own
loop.

pltrs (Rust) values live on the thread that made them, so a call another
thread makes on them is queued for that thread, which runs it while it
serves (see `.FigureCanvasBase.start_event_loop`). The loops of GTK's
application and of wx (`.backend_gtk3`, `.backend_gtk4`, `.backend_wx`)
serve through `serving_threads`, as Tk's does (`._backend_tk`), so that a
figure can be changed, drawn or closed from another thread while it is
shown.
"""

from contextlib import contextmanager

from .._native import run_queued_calls, serve_threads

#: How often a toolkit's loop runs the calls other threads queue for it, in
#: milliseconds.
INTERVAL = 10


@contextmanager
def serving_threads(start_timer):
    """
    While the block runs a toolkit's loop, run the calls other threads
    queue for this thread every `INTERVAL` milliseconds.

    *start_timer* ``(interval, function)`` starts a repeating timer of the
    toolkit's calling *function* every *interval* milliseconds, and returns
    a function stopping it.
    """
    serve_threads(True)
    stop_timer = start_timer(INTERVAL, run_queued_calls)
    try:
        yield
    finally:
        serve_threads(False)
        stop_timer()
