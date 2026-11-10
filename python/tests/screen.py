"""
Whether the smoke tests may open windows on this machine's screen.

On Linux they reach a display only where one is given to them
(``$DISPLAY``, ``$WAYLAND_DISPLAY``: an Xvfb, a nested weston, the
desktop session they run in). A Mac always has one, the user's, even for
a process started over ssh, so there the tests showing windows (Tk's,
pltrs' own) run only when asked to with ``PLTRS_MACOS_LIVE=1``, as
pltrs-platform's AppKit tests do; Qt's draw on its offscreen platform
instead.
"""

import os
import sys

#: The variable allowing tests to open windows on a Mac's screen.
LIVE_VARIABLE = "PLTRS_MACOS_LIVE"

#: Why the tests must not open windows on this machine's screen, or None.
REFUSED = (f"windows would open on the Mac's screen (set {LIVE_VARIABLE}=1 to allow it)"
           if sys.platform == "darwin" and os.environ.get(LIVE_VARIABLE) != "1" else None)
