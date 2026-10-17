"""The JavaScript animation player (``matplotlib.animation``).

Area ``animation``. Each op animates an empty 1x1 inch figure at 20 dpi
with ``FuncAnimation(fig, lambda i: None, frames=frames,
interval=interval, repeat=repeat)`` and exports it.

Argument conventions
--------------------
* ``frames``: the number of frames; ``interval``: milliseconds;
  ``repeat``: bool.
* ``fps`` (int) and ``default_mode`` (``"once"``, ``"loop"`` or
  ``"reflect"``): optional ``to_jshtml`` arguments.

Ops
---
``animation.jshtml``  {frames, interval, repeat, fps?, default_mode?} -> str
    the ``to_jshtml`` page with each embedded frame's base64 payload
    replaced by ``DATA`` (``base64,DATA"``) and every run of 32 lowercase
    hex digits (the player's ``uuid4().hex`` id) by ``ID``.
"""

import re

from matplotlib.animation import FuncAnimation
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("animation")


def _jshtml(frames, interval, repeat, fps=None, default_mode=None):
    fig = Figure(figsize=(1, 1), dpi=20)
    animation = FuncAnimation(fig, lambda i: None, frames=frames, interval=interval, repeat=repeat)
    html = animation.to_jshtml(fps=fps, default_mode=default_mode)
    html = re.sub(r'base64,[^"]*"', 'base64,DATA"', html)
    return re.sub(r"[0-9a-f]{32}", "ID", html)


def test_jshtml():
    probes.probe("animation.jshtml", _jshtml, frames=3, interval=200, repeat=True)
    probes.probe("animation.jshtml", _jshtml, frames=1, interval=30, repeat=False)
    probes.probe("animation.jshtml", _jshtml, frames=2, interval=33, repeat=True)
    probes.probe("animation.jshtml", _jshtml, frames=2, interval=50, repeat=True, fps=24,
                 default_mode="reflect")
    probes.probe("animation.jshtml", _jshtml, frames=1, interval=100, repeat=True, default_mode="once")
    probes.probe("animation.jshtml", _jshtml, frames=1, interval=100, repeat=False, default_mode="loop")
