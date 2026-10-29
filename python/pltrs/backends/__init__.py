"""
The backends: the canvases figures render into and the managers of their
windows (matplotlib's ``backends``).

`backend_registry` knows every backend by name; each lives in the module
``backend_<name>`` of this package.
"""

from .registry import BackendFilter, backend_registry  # noqa: F401

# NOTE: plt.switch_backend() (called at import time) will add a "backend"
# attribute here for backcompat.
_QT_FORCE_QT5_BINDING = False
