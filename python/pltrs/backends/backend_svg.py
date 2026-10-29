"""
The SVG backend (matplotlib's ``backend_svg``): `FigureCanvasSVG`, whose
``print_svg`` and ``print_svgz`` write the figure as an SVG document,
plain or gzip-compressed. pltrs (Rust) writes the documents.
"""

from .. import __version__
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase


class FigureCanvasSVG(FigureCanvasBase):
    filetypes = {'svg': 'Scalable Vector Graphics',
                 'svgz': 'Scalable Vector Graphics'}

    fixed_dpi = 72

    def print_svg(self, filename, *, bbox_inches_restore=None, metadata=None,
                  facecolor=None, edgecolor=None):
        """
        Write the figure as an SVG document.

        Parameters
        ----------
        filename : str or path-like or file-like
            Output target; if a string, a file will be opened for writing.
        metadata : dict[str, Any], optional
            Metadata in the SVG file defined as key-value pairs of strings,
            datetimes, or lists of strings.
        """
        self._print_svg("svg", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def print_svgz(self, filename, *, bbox_inches_restore=None, metadata=None,
                   facecolor=None, edgecolor=None):
        """Write the figure as a gzip-compressed SVG document."""
        self._print_svg("svgz", filename, bbox_inches_restore, metadata,
                        facecolor, edgecolor)

    def _print_svg(self, fmt, filename, bbox_inches_restore, metadata,
                   facecolor, edgecolor):
        """Write the figure as *fmt*, ``svg`` or ``svgz``."""
        if metadata is not None:
            raise NotImplementedError(f"pltrs: print_{fmt}(metadata=...)")
        self._print_native(fmt, filename,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    def get_default_filetype(self):
        return 'svg'


FigureManagerSVG = FigureManagerBase


@_Backend.export
class _BackendSVG(_Backend):
    backend_version = __version__
    FigureCanvas = FigureCanvasSVG
