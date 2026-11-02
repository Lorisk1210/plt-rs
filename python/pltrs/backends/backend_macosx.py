"""
matplotlib's macOS backend (``macosx``), in pltrs: pltrs' own windows
(`.backend_pltrs`), which pltrs (Rust) opens with AppKit, under
matplotlib's names. Its toolbar saves through ``_macosx.choose_save_file``
as matplotlib's does, so that programs and tests replacing it keep
working.

Only macOS has it: importing it elsewhere raises `ImportError`, as
matplotlib's does without its extension.
"""

import os

from .. import rcParams
from . import _macosx
from .backend_pltrs import (
    FigureCanvasPltrs, FigureManagerPltrs, NavigationToolbar2Pltrs, TimerPltrs,
    _BackendPltrs)


class TimerMac(TimerPltrs):
    """`.TimerBase` firing in the windows' event loop."""


class FigureCanvasMac(FigureCanvasPltrs):
    # docstring inherited
    _timer_cls = TimerMac


class NavigationToolbar2Mac(NavigationToolbar2Pltrs):

    def save_figure(self, *args):
        directory = os.path.expanduser(rcParams['savefig.directory'])
        filename = _macosx.choose_save_file('Save the figure',
                                            directory,
                                            self.canvas.get_default_filename())
        if filename is None:  # Cancel
            return
        # Save dir for next time, unless empty str (which means use cwd).
        if rcParams['savefig.directory']:
            rcParams['savefig.directory'] = os.path.dirname(filename)
        self.canvas.figure.savefig(filename)
        return filename


class FigureManagerMac(FigureManagerPltrs):
    _toolbar2_class = NavigationToolbar2Mac


FigureCanvasMac.manager_class = FigureManagerMac


@_BackendPltrs.export
class _BackendMac(_BackendPltrs):
    FigureCanvas = FigureCanvasMac
    FigureManager = FigureManagerMac
    mainloop = FigureManagerMac.start_main_loop
