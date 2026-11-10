"""
Core functions and attributes for the matplotlib style library:

``use``
    Select style sheet to override the current matplotlib settings.
``context``
    Context manager to use a style sheet temporarily.
``available``
    List available style sheets.
``library``
    A dictionary of style names and matplotlib settings.

Deprecated since matplotlib 3.11, which moved these to `matplotlib.style`
(from matplotlib 3.11.2, under the Matplotlib License).
"""

from os.path import join as _join

from .. import _api, get_data_path as _get_data_path
from . import (
    use, context, available, library, reload_library, USER_LIBRARY_PATHS,
    _STYLE_EXTENSION as STYLE_EXTENSION,
    _STYLE_BLACKLIST as STYLE_BLACKLIST,
)

# The bundled style sheets, as files: those pltrs (Rust) writes into the data
# directory (matplotlib's ``style._BASE_LIBRARY_PATH``; the style library
# reads them from pltrs (Rust) itself).
BASE_LIBRARY_PATH = _join(_get_data_path(), 'stylelib')

__all__ = [
    "use", "context", "available", "library", "reload_library",
    "USER_LIBRARY_PATHS", "BASE_LIBRARY_PATH", "STYLE_EXTENSION", "STYLE_BLACKLIST",
]

_api.warn_deprecated("3.11", name=__name__, obj_type="module")
