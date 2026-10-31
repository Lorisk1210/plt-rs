"""
The Qt backend drawing with Agg (matplotlib's ``backends.backend_qtagg``):
importing it raises `ImportError`, as `.backend_qt` does.
"""

from . import backend_qt  # noqa: F401
