"""
The backends: the canvases figures render into and the managers of their
windows (matplotlib's ``backends``).

`backend_registry` knows every backend by name; each lives in the module
``backend_<name>`` of this package.

matplotlib's ``_macosx`` extension exists on macOS only, which programs
and tests check for (``importlib.util.find_spec``): pltrs' module of that
name, in ``_darwin``, joins the package there only.
"""

import os
import sys

from .registry import BackendFilter, backend_registry  # noqa: F401

if sys.platform == "darwin":
    __path__.append(os.path.join(os.path.dirname(__file__), "_darwin"))

# NOTE: plt.switch_backend() (called at import time) will add a "backend"
# attribute here for backcompat.
_QT_FORCE_QT5_BINDING = False
