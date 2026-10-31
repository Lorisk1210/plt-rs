"""
The Qt backend restricted to Qt 5 bindings (matplotlib's
``backends.backend_qt5``): `.backend_qt` with PyQt5 or PySide2, so that
importing it raises `ImportError` naming only those bindings when none is
installed.
"""

from .. import backends

backends._QT_FORCE_QT5_BINDING = True

from . import backend_qt  # noqa: E402, F401
