"""
The program the harness drives: shows one scenario's figure in a window
of the library and backend named by the environment, and writes what the
program sees to a log, one JSON object per line, as it happens.

    GUI_EVENTS_LIBRARY=matplotlib|pltrs  the library (``import ... as mpl``)
    GUI_EVENTS_BACKEND=qtagg|tkagg|pltrs  its backend
    python recorder.py SCENARIO LOG

Every event a program can connect to with ``mpl_connect`` is recorded
with its attributes (`describe`), and so is what the scenario's widgets
and animations report (`Log.note`). The one ``ready`` record comes
after the window's first draw: it gives the window's title and the
canvas' place in it (and the canvas' pixels go to the file
``GUI_EVENTS_CANVAS`` names), so that the driver can wait until the
window shows it and aim at the figure. When the
window has closed the program writes ``end`` and exits.
"""

import importlib
import json
import os
import sys
import time

LIBRARY = os.environ["GUI_EVENTS_LIBRARY"]
BACKEND = os.environ["GUI_EVENTS_BACKEND"]

mpl = importlib.import_module(LIBRARY)
testing = importlib.import_module(f"{LIBRARY}.testing")
backend_bases = importlib.import_module(f"{LIBRARY}.backend_bases")

import scenarios  # noqa: E402 (it imports neither library)

#: Every event `mpl_connect` takes.
EVENTS = [
    "button_press_event", "button_release_event", "motion_notify_event",
    "scroll_event", "key_press_event", "key_release_event", "resize_event",
    "figure_enter_event", "figure_leave_event", "axes_enter_event",
    "axes_leave_event", "close_event", "draw_event", "pick_event",
]


def plain(value, digits=6):
    """*value* as JSON takes it: numbers rounded to *digits* significant
    digits, arrays and tuples as lists, enums by value."""
    if hasattr(value, "tolist"):
        value = value.tolist()
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return int(value)
    if isinstance(value, float):
        return float(f"{value:.{digits}g}")
    if isinstance(value, (list, tuple)):
        return [plain(item, digits) for item in value]
    if isinstance(value, dict):
        return {key: plain(item, digits) for key, item in value.items()}
    return repr(value)


class Log:
    """The log file, written a line at a time, each line with the time
    since the program started (``t``)."""

    def __init__(self, path):
        self._file = open(path, "w", encoding="utf-8")
        self._start = time.monotonic()

    def write(self, record):
        """Write *record* (a dict of plain values)."""
        record = {**record, "t": round(time.monotonic() - self._start, 4)}
        self._file.write(json.dumps(record) + "\n")
        self._file.flush()

    def note(self, name, **fields):
        """Write what a widget or an animation reports, as record *name*."""
        self.write({"name": name, **plain(fields)})


def _axes_index(fig, ax):
    if ax is None:
        return None
    axes = fig.get_axes()
    return axes.index(ax) if ax in axes else "other"


def describe(event, signal):
    """The attributes of *event* a program connected to *signal* sees, as
    plain values (``axes_enter_event`` passes the motion event entering
    the axes, whose name is kept as ``event``)."""
    fig = event.canvas.figure
    record = {"name": signal, "gui": event.guiEvent is not None}
    if event.name != signal:
        record["event"] = event.name
    if isinstance(event, backend_bases.LocationEvent):
        record.update(
            x=event.x, y=event.y, inaxes=_axes_index(fig, event.inaxes),
            xdata=plain(event.xdata), ydata=plain(event.ydata),
            modifiers=sorted(event.modifiers))
    if isinstance(event, backend_bases.MouseEvent):
        record.update(
            button=plain(event.button), key=event.key, step=event.step,
            dblclick=event.dblclick,
            buttons=(None if event.buttons is None
                     else sorted(map(int, event.buttons))))
    if isinstance(event, backend_bases.KeyEvent):
        record["key"] = event.key
    if event.name == "pick_event":
        artist = event.artist
        record.update(
            artist=artist.get_label() or type(artist).__name__,
            mouse=event.mouseevent.name,
            button=plain(event.mouseevent.button),
            ind=plain(getattr(event, "ind", None)))
    if event.name == "resize_event":
        record.update(size=list(event.canvas.get_width_height()),
                      inches=plain(list(fig.get_size_inches())))
    return record


def _canvas_place(canvas):
    """``(x, y, width, height)`` of the canvas in its window, in logical
    pixels."""
    width, height = canvas.get_width_height()
    if BACKEND == "qtagg":
        origin = canvas.mapTo(canvas.window(), canvas.rect().topLeft())
        return origin.x(), origin.y(), width, height
    if BACKEND == "tkagg":
        widget = canvas.get_tk_widget()
        top = widget.winfo_toplevel()
        return (widget.winfo_rootx() - top.winfo_rootx(),
                widget.winfo_rooty() - top.winfo_rooty(), width, height)
    # pltrs' windows show the canvas in their top-left corner.
    return 0, 0, width, height


def report_ready(canvas, log, title):
    """Write the canvas' pixels where the driver looks for them (raw RGB
    rows, all at once: to a file of another name first), so that it can
    wait until they show and find them on screen where it cannot ask,
    then the ``ready`` record."""
    path = os.environ.get("GUI_EVENTS_CANVAS")
    if path:
        import numpy as np
        rgba = np.asarray(canvas.buffer_rgba())
        partial = f"{path}.partial"
        with open(partial, "wb") as file:
            file.write(np.ascontiguousarray(rgba[:, :, :3]).tobytes())
        os.replace(partial, path)
    log.write({"name": "ready", "title": title,
               "canvas": list(_canvas_place(canvas)),
               "ratio": canvas.device_pixel_ratio})


def main(name, log_path):
    # The spec's reference configuration, which selects Agg, then the
    # backend.
    testing.setup()
    mpl.rcParams["text.hinting"] = "none"
    mpl.use(BACKEND)
    scenario = scenarios.SCENARIOS[name]
    mpl.rcParams["toolbar"] = "toolbar2" if scenario.toolbar else "None"
    if not scenario.keymaps:
        for key in mpl.rcParams:
            if key.startswith("keymap."):
                mpl.rcParams[key] = []
    mpl.rcParams.update(scenario.rc)
    plt = importlib.import_module(f"{LIBRARY}.pyplot")
    log = Log(log_path)
    fig = scenario.setup(plt, log)
    canvas = fig.canvas
    for signal in EVENTS:
        canvas.mpl_connect(signal, lambda event, signal=signal:
                           log.write(describe(event, signal)))
    title = f"gui-events {name}"
    canvas.manager.set_window_title(title)

    def ready(_event):
        # Once: a draw_event callback drawing again (a selector saving its
        # background) nests a second first draw_event in the first.
        if hasattr(canvas, "_gui_events_ready"):
            return
        canvas.mpl_disconnect(first_draw)
        # Once the loop runs on, after the draw (the driver waits until
        # the window shows it).
        timer = canvas.new_timer(interval=0)
        timer.single_shot = True
        timer.add_callback(lambda: report_ready(canvas, log, title))
        timer.start()
        canvas._gui_events_ready = timer

    first_draw = canvas.mpl_connect("draw_event", ready)
    plt.show()
    log.write({"name": "end"})


if __name__ == "__main__":
    main(*sys.argv[1:])
