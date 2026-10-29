"""
The PostScript backend (matplotlib's ``backend_ps``): `FigureCanvasPS`,
whose ``print_ps`` and ``print_eps`` write the figure as PostScript, on
a page of :rc:`ps.papersize` or as Encapsulated PostScript. pltrs (Rust)
writes the documents.
"""

from enum import Enum
import functools

from .. import _api, rcParams
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase

#: The paper sizes, in inches (portrait), by matplotlib's names.
papersize = {'letter': (8.5, 11),
             'legal': (8.5, 14),
             'ledger': (11, 17),
             **{f'a{i}': ((841 / 2**(i / 2)) / 25.4,
                          (1189 / 2**(i / 2)) / 25.4)
                for i in range(11)},
             **{f'b{i}': ((1000 / 2**(i / 2)) / 25.4,
                          (1414 / 2**(i / 2)) / 25.4)
                for i in range(11)}}


class _Orientation(Enum):
    portrait, landscape = range(2)

    def swap_if_landscape(self, shape):
        return shape[::-1] if self.name == "landscape" else shape


class FigureCanvasPS(FigureCanvasBase):
    fixed_dpi = 72
    filetypes = {'ps': 'Postscript',
                 'eps': 'Encapsulated Postscript'}

    def get_default_filetype(self):
        return 'ps'

    def _print_ps(
            self, fmt, outfile, *,
            metadata=None, papertype=None, orientation='portrait',
            bbox_inches_restore=None, facecolor=None, edgecolor=None):
        """Write the figure as *fmt*, ``ps`` or ``eps``, after matplotlib's
        checks of the paper type and orientation."""
        if papertype is None:
            papertype = rcParams['ps.papersize']
        papertype = papertype.lower()
        _api.check_in_list(['figure', *papersize], papertype=papertype)
        orientation = _api.getitem_checked(
            _Orientation, orientation=orientation.lower())
        self._print_native(
            fmt, outfile, metadata=metadata,
            bbox_inches_restore=bbox_inches_restore,
            facecolor=facecolor, edgecolor=edgecolor,
            orientation=orientation.name, papertype=papertype)

    print_ps = functools.partialmethod(_print_ps, "ps")
    print_eps = functools.partialmethod(_print_ps, "eps")


FigureManagerPS = FigureManagerBase


@_Backend.export
class _BackendPS(_Backend):
    backend_version = 'Level II'
    FigureCanvas = FigureCanvasPS
