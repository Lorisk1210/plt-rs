"""
What matplotlib's macOS backend asks of its C extension ``_macosx``,
answered by pltrs' own windows (`._pltrs_loop`), which pltrs (Rust) opens
with AppKit: the save panel and whether the event loop runs.

Only macOS has it, as only macOS has matplotlib's extension: the
package's ``__init__`` adds its directory to the package's path there.
"""

from .._pylab_helpers import Gcf
from . import _pltrs_loop


def choose_save_file(title, directory, filename):
    """
    Ask where to save the figure of the active window, in *directory*,
    proposing *filename*; return the path chosen, or None when cancelled.
    pltrs' file chooser has its own title: *title* is matplotlib's
    argument.
    """
    manager = Gcf.get_active()
    if manager is None or getattr(manager, "_window", None) is None:
        return None
    return _pltrs_loop.choose_file(manager._window, filename, directory)


def event_loop_is_running():
    """Whether the windows' event loop is set up."""
    return _pltrs_loop.is_connected()
