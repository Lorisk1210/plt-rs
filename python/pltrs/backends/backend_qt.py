"""
The Qt backend (matplotlib's ``backends.backend_qt``).

pltrs draws no Qt windows, so importing this module raises `ImportError`,
as matplotlib's does without a usable toolkit: `.qt_compat` raises
matplotlib's error when no Qt binding can be imported, and with one the
module says that pltrs has no Qt canvas.
"""

from .qt_compat import QT_API

raise ImportError(
    f"pltrs has no Qt backend (the Qt binding {QT_API} is installed, but "
    f"pltrs draws no Qt windows)")
