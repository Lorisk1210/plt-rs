"""
The Qt backend drawing with Agg, restricted to Qt 5 bindings (matplotlib's
``backends.backend_qt5agg``): importing it raises `ImportError`, as
`.backend_qt5` does.
"""

from . import backend_qt5  # noqa: F401
