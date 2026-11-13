"""
Runs a scenario's program (`recorder`) in a real window and feeds it the
scenario's input through the X server (`xclient`), as a user's mouse and
keyboard would: the same input whichever library and backend show the
window, so that what the programs see can be compared event for event.
"""

import contextlib
import importlib.util
import json
import math
import os
import pathlib
import subprocess
import sys
import time

from . import wayland as wayland_driving
from . import xclient, xdisplay
from .activity import Processes
from .canvas import POLL, SHOWN_WITHIN, Canvas, wait_shown
from .errors import ProgramFailed
from .scenarios import SCENARIOS

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[2]

#: How long the log must stay unchanged, with the processes the input
#: passes through idle, for the program to count as done with the input
#: it was given (a timer it started meanwhile fires within it), and how
#: long to wait for that at most: a deadline, which only a program busy
#: on its own (an animation playing on) reaches.
QUIET, PATIENCE = 0.1, 5.0
#: How often to look at the log and the processes meanwhile, in seconds.
SAMPLE = 0.01
#: The time between the presses and releases of a double click.
DOUBLE_CLICK_STEP = 0.05
#: How long after a press of a button, once handled, the next press of
#: it comes unless it makes a double click: beyond the double-click time
#: of Qt's and pltrs' windows (400 ms), so that a press counts as a
#: click of its own however long the waits before it took.
CLICKS_APART = 0.45
#: How long a program may take to show its window, and to end once closed.
STARTUP, SHUTDOWN = 60.0, 20.0


def _toolkits_env():
    """The variables of ``python/tools/toolkits.sh env`` (the Qt
    bindings' X libraries), or none when the toolkits are not installed."""
    script = REPO / "python/tools/toolkits.sh"
    proc = subprocess.run(["sh", str(script), "env"], capture_output=True,
                          text=True)
    if proc.returncode != 0:
        return {}
    return dict(line.split("=", 1) for line in proc.stdout.splitlines()
                if "=" in line)


#: The variables of the libraries a program keeps: where their caches go.
KEPT_VARIABLES = ("MPLCONFIGDIR", "PLTRSCONFIGDIR")

#: Whether the tests run the source tree's pltrs (``python/`` on the path,
#: as `cargo xtask python test` runs them), which the programs then load
#: too, rather than one installed in this interpreter (a wheel, as
#: ``python/tools/check_dist.py`` tests it), which they load instead.
FROM_TREE = pathlib.Path(importlib.util.find_spec("pltrs").origin).resolve().is_relative_to(
    REPO / "python")


def program_env(library, backend, display, wayland=None):
    """The environment of the program of *library* with *backend* on X
    display *display*, or on the Wayland compositor *wayland* (a socket
    name), or with neither on the session's own display (macOS, Windows,
    a desktop)."""
    env = {key: value for key, value in os.environ.items()
           if key in KEPT_VARIABLES
           or not key.startswith(("MPL", "PLTRS", "QT_", "WAYLAND"))}
    toolkits = _toolkits_env()
    paths = [str(REPO / "python")] if library == "pltrs" and FROM_TREE else []
    paths += [toolkits.pop("PYTHONPATH")] if "PYTHONPATH" in toolkits else []
    env.update(toolkits)
    env.update(
        LANG="C.UTF-8", LC_ALL="C.UTF-8",
        PYTHONPATH=os.pathsep.join(paths), GUI_EVENTS_LIBRARY=library,
        GUI_EVENTS_BACKEND=backend, PLTRS_LOOK="matplotlib",
        PYTHONDONTWRITEBYTECODE="1")
    if wayland is not None:
        env.pop("DISPLAY", None)
        env.update(WAYLAND_DISPLAY=wayland, QT_QPA_PLATFORM="wayland",
                   XDG_RUNTIME_DIR=xdisplay.runtime_dir())
    elif display is not None:
        env.update(DISPLAY=display, QT_QPA_PLATFORM="xcb")
    return env


def python_of(library):
    """The interpreter of *library*'s venv: the reference venv's for
    matplotlib; for pltrs, the pltrs venv's (else this one) when the tests
    run the source tree's pltrs, this one when they run an installed one."""
    if library == "matplotlib":
        return REPO / ".venv" / "bin" / "python"
    venv = REPO / ".venv-pltrs" / "bin" / "python"
    return venv if FROM_TREE and venv.exists() else pathlib.Path(sys.executable)


class Driver:
    """
    Input to one program's window, in the figure's pixels from its
    bottom-left corner. Every method waits until the program has handled
    what it was given (see `settle`); *processes* (an
    `activity.Processes`) are the program and the display servers the
    input passes through.
    """

    def __init__(self, display, window, canvas, log_path, processes,
                 closer=None):
        self.x = display
        self.window = window
        self._log_path = log_path
        self._processes = processes
        self._closer = closer
        #: The last button pressed, and when its press was handled.
        self._last_press = (None, 0.0)
        x, y, _, _ = display.geometry(window)
        cx, cy, self.width, self.height = canvas
        self._origin = (x + cx, y + cy)
        #: The snapshots taken, by name: ``(width, height, rgb)``.
        self.snapshots = {}

    # Waiting.

    def settle(self):
        """
        Wait until the program is done with the input it was given and
        with what it asked for in turn (an idle draw, a blit, a timer
        within `QUIET`): its log has not grown for `QUIET` seconds, and
        meanwhile no thread of it or of the display servers was busy
        (`activity`) at two samples in a row. A program waiting for a
        CPU, however long, is busy, not done.
        """
        deadline = time.monotonic() + PATIENCE
        size, since, busy = -1, time.monotonic(), set()
        while time.monotonic() < deadline:
            now_size = self._log_path.stat().st_size
            now_busy = self._processes.busy()
            if now_size != size or busy & now_busy:
                size, since = now_size, time.monotonic()
            elif time.monotonic() - since >= QUIET:
                return
            busy = now_busy
            time.sleep(SAMPLE)

    def await_record(self, wanted, after=0):
        """Wait for a record of the log after its first *after* ones for
        which *wanted* holds (`ProgramFailed` after `PATIENCE` seconds)."""
        deadline = time.monotonic() + PATIENCE
        while time.monotonic() < deadline:
            if any(map(wanted, read_log(self._log_path)[after:])):
                return
            time.sleep(SAMPLE)
        raise ProgramFailed(f"no record as awaited within {PATIENCE} s")

    def wait(self, seconds):
        """Let *seconds* pass: to watch for what should not come (frames
        after an animation's last)."""
        time.sleep(seconds)

    # The pointer.

    def screen(self, x, y):
        """The screen position of the figure's pixel *(x, y)* (halves
        rounded up, the same way whatever the canvas' place)."""
        ox, oy = self._origin
        return (math.floor(ox + x + 0.5),
                math.floor(oy + self.height - y + 0.5))

    def move(self, x, y):
        """Move the pointer to the figure's pixel *(x, y)*."""
        self.x.motion(*self.screen(x, y))
        self.settle()

    def leave(self):
        """Move the pointer off the canvas, 40 pixels beyond its right
        edge (off the window, which has nothing to its right)."""
        self.x.motion(*self.screen(self.width + 40, self.height // 2))
        self.settle()

    def _clear_of_last_press(self, button):
        """Wait until a press of *button* cannot make a double click with
        the last press (`CLICKS_APART`)."""
        last, handled = self._last_press
        if last == button:
            time.sleep(max(handled + CLICKS_APART - time.monotonic(), 0))

    def press(self, button):
        """Press mouse *button* (1 left, 2 middle, 3 right, 8 back, 9
        forward): a click of its own, never the second of a double
        click."""
        self._clear_of_last_press(button)
        self.x.button(button, True)
        self.settle()
        self._last_press = (button, time.monotonic())

    def release(self, button):
        """Release mouse *button*."""
        self.x.button(button, False)
        self.settle()

    def click(self, button=1):
        """Press and release *button*."""
        self.press(button)
        self.release(button)

    def double_click(self, button=1):
        """Two clicks of *button* as a hand makes them: 50 ms apart from
        each press to the next release, within the double-click time."""
        self._clear_of_last_press(button)
        for pressed in (True, False, True, False):
            self.x.button(button, pressed)
            time.sleep(DOUBLE_CLICK_STEP)
        self.settle()
        self._last_press = (button, time.monotonic())

    def drag(self, button, points):
        """Press *button* at the first of *points*, move through the
        others, release at the last."""
        self.move(*points[0])
        self.press(button)
        for point in points[1:]:
            self.move(*point)
        self.release(button)

    def scroll(self, steps):
        """Turn the wheel *steps* notches, up when positive (X buttons 4
        and 5; 6 and 7 for ``"left"`` and ``"right"``)."""
        if steps in ("left", "right"):
            buttons = [6 if steps == "left" else 7]
        else:
            buttons = [4 if steps > 0 else 5] * abs(steps)
        for button in buttons:
            self.x.button(button, True)
            self.x.button(button, False)
        self.settle()

    # The keyboard.

    def _keys_of(self, spec):
        """The keycodes to hold for the key *spec*: modifiers then the key,
        ``"ctrl+alt+A"``, ``"shift+left"``, ``"f5"``, ``"dead_acute"``."""
        *mods, name = spec.split("+") if spec != "+" else ["+"]
        names = {"ctrl": "Control_L", "alt": "Alt_L", "shift": "Shift_L",
                 "super": "Super_L", "control": "Control_L",
                 "enter": "Return", "escape": "Escape", "tab": "Tab",
                 "backspace": "BackSpace", "delete": "Delete",
                 "insert": "Insert", "home": "Home", "end": "End",
                 "pageup": "Prior", "pagedown": "Next", "left": "Left",
                 "right": "Right", "up": "Up", "down": "Down",
                 "space": "space", "pause": "Pause", "menu": "Menu",
                 "caps_lock": "Caps_Lock", "altgr": "ISO_Level3_Shift"}
        codes = [self.x.keycode(xclient.keysym_of(names.get(m, m)))[0]
                 for m in mods]
        if len(name) > 1 and name[0] == "f" and name[1:].isdigit():
            name = name.upper()
        code, shifted = self.x.keycode(xclient.keysym_of(names.get(name, name)))
        shift = self.x.keycode(xclient.KEYSYMS["Shift_L"])[0]
        if shifted and shift not in codes:
            codes.append(shift)
        return codes + [code]

    def key(self, spec):
        """Press and release the key *spec* with its modifiers, as a user
        types it (modifiers down first, up last)."""
        codes = self._keys_of(spec)
        for code in codes:
            self.x.key(code, True)
            self.settle()
        for code in reversed(codes):
            self.x.key(code, False)
            self.settle()

    def type(self, text):
        """Type the characters of *text*."""
        for char in text:
            self.key("space" if char == " " else char)

    @contextlib.contextmanager
    def hold(self, spec):
        """Hold the keys of *spec* down meanwhile."""
        codes = self._keys_of(spec)
        for code in codes:
            self.x.key(code, True)
            self.settle()
        try:
            yield
        finally:
            for code in reversed(codes):
                self.x.key(code, False)
                self.settle()

    # The window.

    def resize(self, width, height):
        """Resize the window's canvas to *width* × *height* pixels (the
        window by as much), as a window manager does."""
        _, _, window_width, window_height = self.x.geometry(self.window)
        known = len(read_log(self._log_path))
        self.x.configure(self.window,
                         width=window_width + width - self.width,
                         height=window_height + height - self.height)
        self.width, self.height = width, height
        self.await_record(lambda record: record["name"] == "resize_event"
                                 and record["size"] == [width, height],
                          known)
        self.settle()

    def close(self):
        """Ask the window to close, as its close button does."""
        if self._closer is not None:
            self._closer(self)
        else:
            self.x.delete(self.window)

    def snapshot(self, name):
        """Keep the canvas' pixels as snapshot *name*, once the program is
        done (`settle`), and with it the display servers: a blit or a
        paint that follows the last record is done by then too."""
        self.settle()
        x, y = self.screen(0, self.height)
        wx, wy, _, _ = self.x.geometry(self.window)
        self.snapshots[name] = (self.width, self.height, self.x.image(
            self.window, x - wx, y - wy, self.width, self.height))


def read_log(path):
    """The records of a program's log so far."""
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    return [json.loads(line) for line in text.splitlines()
            if line.endswith("}")]


class Run:
    """What one program saw: its log's records and the snapshots."""

    def __init__(self, records, snapshots):
        self.records = records
        self.snapshots = snapshots


def run(name, library, backend, display, workdir, wayland=False):
    """
    Run scenario *name* under *library* with *backend* on X display
    *display*, or with *wayland* on a weston nested in it (started for
    the run, so that the window opens where the first did), feeding it
    the scenario's input (a `Run`). *workdir* holds the log and the
    program's output.
    """
    if not wayland:
        return _run(name, library, backend, display, workdir, None)
    with xdisplay.private_wayland(display, pathlib.Path(workdir) /
                                  f"weston-{library}") as weston:
        return _run(name, library, backend, display, workdir, weston)


def _run(name, library, backend, display, workdir, weston):
    """`run` on X display *display*, or on *weston* (an
    `xdisplay.Weston` nested in it)."""
    socket = None if weston is None else weston.socket
    scenario = SCENARIOS[name]
    workdir = pathlib.Path(workdir)
    workdir.mkdir(parents=True, exist_ok=True)
    log_path = workdir / f"{name}.{library}.jsonl"
    canvas_path = log_path.with_suffix(".canvas")
    log_path.unlink(missing_ok=True)
    _park_pointer(display)
    env = program_env(library, backend, display, socket)
    env["GUI_EVENTS_CANVAS"] = str(canvas_path)
    with open(workdir / f"{name}.{library}.out", "w") as out:
        proc = subprocess.Popen(
            [str(python_of(library)), str(HERE / "recorder.py"), name,
             str(log_path)],
            env=env, cwd=workdir, stdout=out, stderr=subprocess.STDOUT)
    try:
        ready = _wait_for(log_path, proc, "ready", STARTUP)
        x, y, width, height = ready["canvas"]
        shown = Canvas.read(canvas_path, (width, height))
        keep = workdir / f"{name}.{library}"
        conn = xclient.Display(display)
        try:
            if socket is None:
                window = _find_window(conn, ready["title"])
                canvas = wait_shown(conn, window, shown, (x, y), keep)
                closer = None
            else:
                window = _find_window(conn, wayland_driving.OUTPUT_TITLE)
                canvas = wait_shown(conn, window, shown, keep=keep)
                closer = wayland_driving.close
            conn.focus(window)
            with Processes([proc.pid, xdisplay.server_pid(display),
                            None if weston is None else weston.pid
                            ]) as processes:
                driver = Driver(conn, window, canvas, log_path, processes,
                                closer)
                driver.settle()
                scenario.steps(driver)
            _wait_for(log_path, proc, "end", SHUTDOWN)
        finally:
            conn.close()
        proc.wait(SHUTDOWN)
    except BaseException:
        proc.kill()
        proc.wait()
        raise
    return Run(read_log(log_path), driver.snapshots)


def _park_pointer(display):
    """Put the pointer near the screen's bottom-left corner, away from
    where windows open (also on a nested weston's output), so that each
    program sees it come in."""
    conn = xclient.Display(display)
    try:
        conn.motion(2, conn.height - 40)
    finally:
        conn.close()


def _wait_for(log_path, proc, name, timeout):
    """The first record *name* of the log, waiting for it."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for record in read_log(log_path):
            if record["name"] == name:
                return record
        if proc.poll() is not None:
            # It may have written the record just before ending.
            for record in read_log(log_path):
                if record["name"] == name:
                    return record
            raise ProgramFailed(f"the program ended ({proc.returncode}) "
                                f"before {name!r}")
        time.sleep(0.02)
    raise ProgramFailed(f"no {name!r} within {timeout} s")


def _find_window(display, title):
    deadline = time.monotonic() + SHOWN_WITHIN
    while time.monotonic() < deadline:
        window = display.find_window(title)
        if window is not None:
            return window
        time.sleep(POLL)
    raise ProgramFailed(f"no window titled {title!r}")


def run_both(name, displays, workdir, reference=None, wayland=False):
    """Run scenario *name* under matplotlib with backend *reference* (by
    default the scenario's) and under pltrs, at once, on the two X
    *displays* (or, with *wayland*, on westons nested in them):
    ``(reference run, pltrs run)``."""
    from concurrent.futures import ThreadPoolExecutor
    reference = reference or SCENARIOS[name].reference
    if reference is None:
        return None, run(name, "pltrs", "pltrs", displays[-1], workdir,
                         wayland)
    with ThreadPoolExecutor(2) as pool:
        ref = pool.submit(run, name, "matplotlib", reference, displays[0],
                          workdir, wayland)
        got = pool.submit(run, name, "pltrs", "pltrs", displays[-1], workdir,
                          wayland)
        return ref.result(), got.result()


def differences_of(name, ref, got, save_to=None, wayland=False):
    """The lines describing what is wrong with pltrs' run *got* of
    scenario *name*: how it differs from matplotlib's run *ref* (records
    and snapshots, those that differ written to *save_to*), if any, and
    what the scenario's check finds. On *wayland*, where the windows of
    the two are elsewhere, the snapshots are left out, and so are the
    places of the pointer's entering and leaving (see
    `compare.without_crossings`)."""
    from .compare import compare, compare_snapshots, without_crossings
    scenario = SCENARIOS[name]
    lines = [] if scenario.check is None else scenario.check(got.records)
    if ref is not None:
        summarize = scenario.summarize or (lambda records: records)
        if wayland:
            summarize = without_crossings(summarize)
        lines += compare(summarize(ref.records), summarize(got.records))
        if not wayland:
            lines += compare_snapshots(ref.snapshots, got.snapshots, save_to)
    return lines
