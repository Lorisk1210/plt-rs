"""
What the GUI toolkit backends do only on Windows: the Tk backend makes its
process per-monitor DPI aware and hooks its windows so that a change of
DPI (``WM_DPICHANGED``, as a move to another screen sends it) rescales Tk
and the canvas, as matplotlib's ``_tkagg`` helper does. The expected values
are real matplotlib 3.11's, on Windows 11 with Tk 9.0.
"""

import importlib.util
import inspect
import os
import subprocess
import sys
import textwrap

import pytest

import screen

pytestmark = [
    pytest.mark.skipif(sys.platform != "win32", reason="Windows only"),
    pytest.mark.skipif(importlib.util.find_spec("tkinter") is None,
                       reason="no tkinter"),
    pytest.mark.skipif(screen.REFUSED is not None, reason=str(screen.REFUSED)),
]


def _run(body):
    """Run the function *body* in a fresh interpreter, in matplotlib's
    look; its standard output."""
    source = textwrap.dedent(inspect.getsource(body)) + f"\n{body.__name__}()\n"
    proc = subprocess.run([sys.executable, "-c", source],
                          env={**os.environ, "PLTRS_LOOK": "matplotlib"},
                          capture_output=True, text=True, timeout=60)
    assert proc.returncode == 0, proc.stderr
    return proc.stdout


def _tk_window_follows_its_dpi():
    import ctypes
    import threading
    import time
    from ctypes import wintypes

    import pltrs
    pltrs.use("tkagg")
    import pltrs.pyplot as plt

    user32 = ctypes.WinDLL("user32")
    user32.SendMessageW.argtypes = [wintypes.HWND, wintypes.UINT,
                                    wintypes.WPARAM, ctypes.c_void_p]
    user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
    fig = plt.figure(figsize=(4, 3), dpi=100)
    manager = fig.canvas.manager
    window = manager.window
    print("hooked", bool(manager._window_dpi_cbname))
    manager.show()
    window.update()
    frame = int(window.wm_frame(), 16)
    for dpi in [144, 120]:
        # Windows sends WM_DPICHANGED (which cannot be posted) as the window
        # moves to another screen; another thread's send arrives as Tk's
        # loop reads its messages, as that does.
        rect = wintypes.RECT()
        user32.GetWindowRect(frame, ctypes.byref(rect))
        sender = threading.Thread(target=user32.SendMessageW, args=(
            frame, 0x02E0, dpi | dpi << 16, ctypes.addressof(rect)))
        sender.start()
        while sender.is_alive():
            window.update()
            time.sleep(0.01)
        window.update()
        print(f"{window.tk.call('tk', 'scaling'):.4f}",
              fig.canvas.device_pixel_ratio, fig.dpi)
    print(window.winfo_screenwidth())
    plt.close(fig)


def _tk_scaling(screen_width, points_per_pixel):
    """What ``tk scaling`` reads after it was set to *points_per_pixel* on
    a screen *screen_width* pixels wide: Tk keeps the setting as the
    screen's width in whole millimetres (``ScalingCmd`` of
    ``tkCmds.c``), so what it reads depends on the screen's width."""
    millimetres = int(0.5 + screen_width * (25.4 / 72) / points_per_pixel)
    return 25.4 / 72 * screen_width / millimetres


def test_tk_scaling_follows_tks_whole_millimetres():
    # As matplotlib 3.11 read them on Windows 11 with Tk 9.0, on a screen
    # 2560 pixels wide.
    assert [f"{_tk_scaling(2560, dpi / 72):.4f}" for dpi in [144, 120]] == [
        "1.9980", "1.6663"]


def test_tk_window_follows_its_dpi():
    lines = _run(_tk_window_follows_its_dpi).split("\n")
    width = int(lines[3])
    assert lines[:3] == [
        "hooked True",
        f"{_tk_scaling(width, 2.0):.4f} 1.5 150.0",
        f"{_tk_scaling(width, 120 / 72):.4f} 1.25 125.0"]
