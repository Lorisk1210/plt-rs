"""
Private display servers for the harness: an Xvfb of its own, so that the
synthetic input never reaches the desktop of whoever runs the tests, and
a weston nested in it, for Wayland.
"""

import contextlib
import os
import pathlib
import shutil
import subprocess
import time

#: The display numbers tried, away from those desktops and the other
#: tests use.
NUMBERS = range(160, 260)


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
        proc = subprocess.Popen(
            [xvfb, f":{number}", "-screen", "0", "1280x800x24", "-dpi", "96",
             "-nolisten", "tcp", "-noreset"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        socket = f"/tmp/.X11-unix/X{number}"
        deadline = time.monotonic() + 10
        while proc.poll() is None and not os.path.exists(socket):
            if time.monotonic() > deadline:
                proc.kill()
                break
            time.sleep(0.02)
        if proc.poll() is not None:
            # Another server took the number meanwhile: try the next one.
            continue
        try:
            yield f":{number}"
        finally:
            proc.terminate()
            proc.wait()
        return
    raise Unavailable("no free display number")


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
                      f"background-color=0xff2e3440\n"
                      f"[output]\nname=X1\nmode=1024x768\n"
                      # As Xvfb's faked keys, which do not repeat.
                      f"[keyboard]\nrepeat-rate=0\n")
    socket = f"wayland-gui-events-{os.getpid()}-{display.lstrip(':')}"
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
    path = pathlib.Path(runtime_dir()) / socket
    deadline = time.monotonic() + 10
    while proc.poll() is None and not path.exists():
        if time.monotonic() > deadline:
            proc.kill()
            break
        time.sleep(0.02)
    if proc.poll() is not None:
        raise Unavailable("weston did not start")
    # The shell's background and the output's window come up meanwhile.
    time.sleep(0.5)
    try:
        yield socket
    finally:
        proc.terminate()
        proc.wait()
