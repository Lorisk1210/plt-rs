"""
The PGF backend (matplotlib's ``backend_pgf``): `FigureCanvasPgf`, whose
``print_pgf`` writes the figure as a LaTeX PGF picture. pltrs (Rust)
writes the pictures.
"""

from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase


class FigureCanvasPgf(FigureCanvasBase):
    filetypes = {"pgf": "LaTeX PGF picture"}

    def get_default_filetype(self):
        return 'pgf'

    def print_pgf(self, fname_or_fh, *, bbox_inches_restore=None,
                  facecolor=None, edgecolor=None):
        """
        Output pgf macros for drawing the figure so it can be included and
        rendered in latex documents.
        """
        self._print_native("pgf", fname_or_fh,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)


FigureManagerPgf = FigureManagerBase


@_Backend.export
class _BackendPgf(_Backend):
    FigureCanvas = FigureCanvasPgf
