"""
What a program that opens a toolkit's window (the ``gui`` list, `gui.py`)
runs with, in the traced process: each toolkit's main loop ends once the
windows drew and went quiet, and the figures the windows hold are saved
then, so that they compare as any program's figures do.

The loops are those of GTK 3 (``Gtk.main``), GTK 4 and Gio applications
(``Gio.Application.run``), Qt (``QApplication.exec`` and its siblings, in
every binding), Tk (``Misc.mainloop`` and ``tkinter.mainloop``) and wx
(``App.MainLoop``). None is imported here: each is patched when the
program imports its toolkit (`_Hooks`), so that a program sees the
modules it imports and no others.

"Quiet" is `gui_events`' rule (`python/tests/gui_events/activity.py`): no
thread of the program (but the watcher's own) nor of its X server
runnable at two samples in a row, and no figure drawn, for `QUIET`
seconds, every window's canvas having drawn once. A watcher thread
samples; a timer of the loop's own polls what it found and ends the loop
from the loop's thread, as the toolkits want. `PATIENCE` after the loop
started, it ends whatever the program does (an animation's timer): the
figures are those of that moment.
"""

import importlib.abc
import importlib.util
import os
import pathlib
import sys
import threading
import time
import weakref

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "python" / "tests"))
from gui_events import activity  # noqa: E402

del sys.path[0]

#: Seconds without a draw or a busy thread that count as quiet.
QUIET = 0.1
#: Seconds between two samples.
SAMPLE = 0.01
#: Milliseconds between two polls of the loop's timer.
POLL_MS = 20
#: Seconds after the loop started within which it ends in any case.
PATIENCE = 20.0
#: The variable naming the X server's process id (`gui.py` sets it).
SERVER_PID = "DIFF_GUI_SERVER_PID"


class _State:
    """What the watcher knows: the canvases of windows, their draws, and
    whether the loop may end."""

    def __init__(self):
        self.canvases = []
        self.draws = 0
        self.loop_started = None
        self.ready = threading.Event()
        self.save_figures = None


_STATE = _State()


def _interactive(canvas):
    """Whether *canvas* draws into a window of a toolkit."""
    return getattr(type(canvas), "required_interactive_framework", None) is not None


def _track_canvases():
    """Count the draws of every canvas made from now on, keeping those of
    windows (weakly)."""
    from matplotlib import backend_bases

    original = backend_bases.FigureCanvasBase.__init__

    def __init__(self, *args, **kwargs):
        original(self, *args, **kwargs)
        if _interactive(self):
            _STATE.canvases.append(weakref.ref(self))
            self.mpl_connect("draw_event", _drew)
            self._gui_shim_draws = 0

    backend_bases.FigureCanvasBase.__init__ = __init__


def _drew(event):
    _STATE.draws += 1
    canvas = event.canvas
    canvas._gui_shim_draws = getattr(canvas, "_gui_shim_draws", 0) + 1


def _windows():
    """The canvases of windows alive, in the order they were made."""
    return [canvas for canvas in (ref() for ref in _STATE.canvases) if canvas is not None]


def _watch():
    """Set `_State.ready` once a loop ran, every window's canvas drew, and
    the program and its X server were quiet for `QUIET` seconds; again for
    each loop the program runs (a notebook's cells)."""
    me = threading.get_native_id()
    server = os.environ.get(SERVER_PID)
    pids = [os.getpid()] + ([int(server)] if server else [])
    with activity.Processes(pids) as processes:
        busy, draws, since = set(), -1, time.monotonic()
        while True:
            time.sleep(SAMPLE)
            started = _STATE.loop_started
            if started is None or _STATE.ready.is_set():
                busy, since = set(), time.monotonic()
                continue
            if time.monotonic() - started > PATIENCE:
                _STATE.ready.set()
                continue
            now = {thread for thread in processes.busy()
                   if not (thread[0] == pids[0] and int(thread[1]) == me)}
            drawn = all(getattr(c, "_gui_shim_draws", 0) for c in _windows())
            if _STATE.draws != draws or busy & now or not drawn:
                draws, since = _STATE.draws, time.monotonic()
            elif time.monotonic() - since >= QUIET:
                _STATE.ready.set()
            busy = now


def _save_windows():
    """Save the figures of the windows not saved yet (each at the end of
    the first loop that showed it; pyplot's the tracer saves itself)."""
    fresh = [canvas for canvas in _windows() if not getattr(canvas, "_gui_shim_saved", False)]
    for canvas in fresh:
        canvas._gui_shim_saved = True
    if _STATE.save_figures is not None and fresh:
        _STATE.save_figures([canvas.figure for canvas in fresh])


def _loop_starts(schedule, end):
    """Note that a loop starts, and have *schedule(callback)* run a poll on
    the loop's thread every `POLL_MS` ms, which saves the windows' figures
    and calls *end()* once the watcher says so; *schedule* returns
    nothing, the poll returns whether it is to run again."""
    _STATE.loop_started = time.monotonic()
    _STATE.ready.clear()

    def poll():
        if not _STATE.ready.is_set():
            return True
        _save_windows()
        end()
        return False

    schedule(poll)


def _patch_tk(tkinter):
    misc_mainloop = tkinter.Misc.mainloop

    def mainloop(self, n=0):
        def schedule(poll):
            def again():
                if poll():
                    self.after(POLL_MS, again)
            self.after(POLL_MS, again)
        _loop_starts(schedule, self.quit)
        return misc_mainloop(self, n)

    tkinter.Misc.mainloop = mainloop

    def module_mainloop(n=0):
        root = tkinter._get_default_root("run the main loop")
        return root.mainloop(n)

    tkinter.mainloop = module_mainloop


def _patch_wx(wx):
    original = wx.App.MainLoop

    def MainLoop(self):  # noqa: N802 - wx's name
        def schedule(poll):
            timer = wx.Timer()
            timer.Bind(wx.EVT_TIMER, lambda event: poll() or timer.Stop())
            timer.Start(POLL_MS)
            self._gui_shim_timer = timer
        _loop_starts(schedule, self.ExitMainLoop)
        return original(self)

    wx.App.MainLoop = MainLoop


def _patch_qt(qt_widgets, qt_core):
    application = qt_widgets.QApplication

    def patched(name):
        original = getattr(application, name)

        def run(*_args):
            # Static in every binding, whether called on the class or on
            # the application.
            app = qt_widgets.QApplication.instance()

            def schedule(poll):
                timer = qt_core.QTimer()
                timer.timeout.connect(lambda: poll() or timer.stop())
                timer.start(POLL_MS)
                app._gui_shim_timer = timer
            _loop_starts(schedule, app.quit)
            return original()
        return run

    for name in ("exec", "exec_"):
        if hasattr(application, name):
            setattr(application, name, patched(name))


def _patch_gtk(gtk):
    from gi.repository import Gio, GLib

    def schedule(poll):
        GLib.timeout_add(POLL_MS, poll)

    if hasattr(gtk, "main"):
        main = gtk.main

        def gtk_main(*args, **kwargs):
            _loop_starts(schedule, gtk.main_quit)
            return main(*args, **kwargs)

        gtk.main = gtk_main
    run = Gio.Application.run
    if getattr(run, "_gui_shim", False):
        return

    def application_run(self, *args, **kwargs):
        _loop_starts(schedule, self.quit)
        return run(self, *args, **kwargs)

    application_run._gui_shim = True
    Gio.Application.run = application_run


def _patch_qt_binding(binding):
    def patch(_widgets):
        qt_widgets = sys.modules[f"{binding}.QtWidgets"]
        qt_core = sys.modules.get(f"{binding}.QtCore") or importlib.import_module(f"{binding}.QtCore")
        _patch_qt(qt_widgets, qt_core)
    return patch


#: What to patch once a module is imported, by module name.
_PATCHES = {
    "tkinter": _patch_tk,
    "wx": _patch_wx,
    "gi.repository.Gtk": _patch_gtk,
    **{f"{binding}.QtWidgets": _patch_qt_binding(binding)
       for binding in ("PyQt6", "PySide6", "PyQt5", "PySide2")},
}


class _Loader(importlib.abc.Loader):
    """The loader of a toolkit's module, patching it once loaded."""

    def __init__(self, loader, patch):
        self._loader, self._patch = loader, patch

    def create_module(self, spec):
        return self._loader.create_module(spec)

    def exec_module(self, module):
        self._loader.exec_module(module)
        self._patch(module)


class _Hooks(importlib.abc.MetaPathFinder):
    """Finds the toolkits' modules through the other finders, giving them
    a loader that patches them once loaded."""

    def find_spec(self, name, path=None, target=None):
        patch = _PATCHES.get(name)
        if patch is None:
            return None
        index = sys.meta_path.index(self)
        for finder in sys.meta_path[index + 1:]:
            find = getattr(finder, "find_spec", None)
            spec = find(name, path, target) if find else None
            if spec is not None and spec.loader is not None:
                spec.loader = _Loader(spec.loader, patch)
                return spec
        return None


def install(save_figures):
    """Patch the toolkits as the program imports them, watch for quiet,
    and save the windows' figures with *save_figures(figures)* when a loop
    ends (or at `finish`)."""
    _STATE.save_figures = save_figures
    _track_canvases()
    for name, patch in _PATCHES.items():
        module = sys.modules.get(name)
        if module is not None:
            patch(module)
    sys.meta_path.insert(0, _Hooks())
    threading.Thread(target=_watch, name="gui-shim-watcher", daemon=True).start()


def finish():
    """Save the windows' figures if no loop ended (a program that never
    ran one)."""
    _save_windows()
