"""
Private display servers for the harness: an Xvfb of its own, so that the
synthetic input never reaches the desktop of whoever runs the tests, and
a weston nested in it, for Wayland.
"""

import contextlib
import itertools
import os
import pathlib
import select
import shutil
import subprocess
import time

from . import xclient
from .canvas import POLL, SHOWN_WITHIN
from .errors import ProgramFailed
from .wayland import BACKGROUND, OUTPUT_TITLE

#: The display numbers tried, away from those desktops and the other
#: tests use.
NUMBERS = range(160, 260)
#: How long a display server may take to accept clients, in seconds: a
#: deadline, the waits end when it does.
STARTED_WITHIN = 30
#: Numbers the westons' sockets, one of its own for each.
_WESTONS = itertools.count()


class Unavailable(Exception):
    """No private X server can be started here."""


def _free(number):
    return not (os.path.exists(f"/tmp/.X11-unix/X{number}")
                or os.path.exists(f"/tmp/.X{number}-lock"))


@contextlib.contextmanager
def private_display():
    """Start an Xvfb (1280 × 800, 96 dpi, no TCP) on a free display and
    give its name (``":N"``); it stops when the block ends."""
    xvfb = shutil.which("Xvfb")
    if xvfb is None:
        raise Unavailable("no Xvfb on PATH")
    for number in NUMBERS:
        if not _free(number):
            continue
        proc = _start_xvfb(xvfb, number)
        if proc is None:
            # Another server took the number meanwhile: try the next one.
            continue
        try:
            yield f":{number}"
        finally:
            proc.terminate()
            proc.wait()
        return
    raise Unavailable("no free display number")


def _start_xvfb(xvfb, number):
    """Xvfb on display *number* (1280 × 800, 96 dpi, no TCP, kept up
    between clients, keys not repeated however long the driver holds
    them on a busy machine) once it accepts clients, as it says through
    ``-displayfd``; None when it ended instead."""
    readable, writable = os.pipe()
    try:
        proc = subprocess.Popen(
            [xvfb, f":{number}", "-screen", "0", "1280x800x24", "-dpi", "96",
             "-nolisten", "tcp", "-noreset", "-r", "-displayfd", str(writable)],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            pass_fds=(writable,))
        os.close(writable)
        writable = None
        # The display's number and a newline once ready; nothing (the end
        # of the pipe) when it ends first.
        said = b""
        deadline = time.monotonic() + STARTED_WITHIN
        while not said.endswith(b"\n"):
            left = deadline - time.monotonic()
            if left <= 0 or not select.select([readable], [], [], left)[0]:
                proc.kill()
                proc.wait()
                raise Unavailable(f"Xvfb :{number} did not start within "
                                  f"{STARTED_WITHIN} s")
            chunk = os.read(readable, 64)
            if not chunk:
                proc.wait()
                return None
            said += chunk
        return proc
    finally:
        os.close(readable)
        if writable is not None:
            os.close(writable)


#: Where ``crates/pltrs-platform/tools/weston.sh install`` puts weston.
WESTON_PREFIX = pathlib.Path(os.environ.get(
    "WESTON_PREFIX", pathlib.Path.home() / ".local/weston"))
#: Where ``python/tools/toolkits.sh install`` puts the X libraries the
#: Qt bindings need, which weston's X11 backend needs too.
TOOLKITS_PREFIX = pathlib.Path(os.environ.get(
    "TOOLKITS_PREFIX", pathlib.Path.home() / ".local/gtk"))


def runtime_dir():
    """Where Wayland compositors put their sockets."""
    return os.environ.get("XDG_RUNTIME_DIR") or f"/run/user/{os.getuid()}"


@contextlib.contextmanager
def private_wayland(display, workdir):
    """
    Start weston nested in X display *display* (its X11 backend), so
    that input faked on that server reaches its Wayland clients through
    its seat, as a user's would; give the name of its socket. Its
    configuration goes to *workdir*. `Unavailable` without weston (see
    ``crates/pltrs-platform/tools/weston.sh install``) or the X libraries
    of its X11 backend (``python/tools/toolkits.sh install``).
    """
    lib = WESTON_PREFIX / "root/usr/lib/x86_64-linux-gnu"
    weston = WESTON_PREFIX / "root/usr/bin/weston"
    backend = lib / "libweston-14/x11-backend.so"
    toolkit_libs = TOOLKITS_PREFIX / "root/usr/lib/x86_64-linux-gnu"
    if not (weston.exists() and backend.exists()
            and (toolkit_libs / "libxcb-xkb.so.1").exists()):
        raise Unavailable("no weston with its X11 backend")
    shell = WESTON_PREFIX / "root/usr/libexec/weston-desktop-shell"
    config = pathlib.Path(workdir) / "weston.ini"
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(f"[core]\nidle-time=0\n[input-method]\npath=\n"
                      f"[shell]\nclient={shell}\npanel-position=none\n"
                      f"background-color=0xff{BACKGROUND.hex()}\n"
                      f"[output]\nname=X1\nmode=1024x768\n"
                      # As Xvfb's faked keys, which do not repeat.
                      f"[keyboard]\nrepeat-rate=0\n")
    socket = (f"wayland-gui-events-{os.getpid()}-{display.lstrip(':')}-"
              f"{next(_WESTONS)}")
    modules = ";".join([
        f"x11-backend.so={backend}",
        f"desktop-shell.so={lib / 'weston/desktop-shell.so'}",
        f"weston-desktop-shell={shell}"])
    env = {**os.environ, "DISPLAY": display,
           "XDG_RUNTIME_DIR": runtime_dir(), "WESTON_MODULE_MAP": modules,
           "LD_LIBRARY_PATH": os.pathsep.join(map(str, [
               lib, lib / "weston", lib / "libweston-14", toolkit_libs]))}
    proc = subprocess.Popen(
        [str(weston), "--backend=x11", f"--socket={socket}",
         "--renderer=pixman", f"--config={config}", "--width=1024",
         "--height=768"],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        _wait_for_weston(proc, display, pathlib.Path(runtime_dir()) / socket)
        yield socket
    finally:
        proc.terminate()
        proc.wait()
        _wait_until_gone(display)


def _wait_for_weston(proc, display, socket):
    """Wait until weston (*proc*) listens on *socket* and its output's
    window on X display *display* shows the shell's background: the
    output is there for the clients' windows, and the pointer can enter
    it."""
    deadline = time.monotonic() + STARTED_WITHIN
    while not socket.exists():
        if proc.poll() is not None:
            raise Unavailable("weston did not start")
        if time.monotonic() > deadline:
            raise ProgramFailed(f"weston did not listen within "
                                f"{STARTED_WITHIN} s")
        time.sleep(POLL)
    conn = xclient.Display(display)
    try:
        deadline = time.monotonic() + SHOWN_WITHIN
        while not _shows_background(conn):
            if proc.poll() is not None:
                raise ProgramFailed(f"weston ended ({proc.returncode})")
            if time.monotonic() > deadline:
                raise ProgramFailed(f"weston's output did not show the "
                                    f"shell within {SHOWN_WITHIN} s")
            time.sleep(POLL)
    finally:
        conn.close()


def _wait_until_gone(display):
    """Wait until the X server has taken the window of an ended weston's
    output away, so that the next weston's is the only one."""
    conn = xclient.Display(display)
    try:
        deadline = time.monotonic() + SHOWN_WITHIN
        while conn.find_window(OUTPUT_TITLE) is not None:
            if time.monotonic() > deadline:
                raise ProgramFailed("the output of an ended weston stays")
            time.sleep(POLL)
    finally:
        conn.close()


def _shows_background(conn):
    """Whether weston's output window is there and shows the shell's
    background in its top-right corner, where no window opens first."""
    window = conn.find_window(OUTPUT_TITLE)
    if window is None:
        return False
    _, _, width, _ = conn.geometry(window)
    return conn.image(window, width - 1, 0, 1, 1) == BACKGROUND
