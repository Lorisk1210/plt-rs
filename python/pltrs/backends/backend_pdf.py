"""
The PDF backend (matplotlib's ``backend_pdf``): `FigureCanvasPdf`, whose
``print_pdf`` writes the figure as a PDF document, and `PdfPages`, a
document of several figures. pltrs (Rust) writes the documents.
"""

from .. import _api
from .._native import backends as _backends
from .._pylab_helpers import Gcf
from ..backend_bases import (
    _Backend, _write_bytes, DrawEvent, FigureCanvasBase, FigureManagerBase)
from ..figure import Figure


class PdfPages:
    """
    A multi-page PDF file.

    Examples
    --------
    >>> import matplotlib.pyplot as plt
    >>> # Initialize:
    >>> with PdfPages('foo.pdf') as pdf:
    ...     # As many times as you like, create a figure fig and save it:
    ...     fig = plt.figure()
    ...     pdf.savefig(fig)
    ...     # When no figure is specified the current figure is saved
    ...     pdf.savefig()
    """

    @_api.delete_parameter("3.10", "keep_empty",
                           addendum="This parameter does nothing.")
    def __init__(self, filename, keep_empty=None, metadata=None):
        """
        Create a new PdfPages object.

        Parameters
        ----------
        filename : str or path-like or file-like
            Plots using `PdfPages.savefig` will be written to a file at this location.
            The file is opened when a figure is saved for the first time (overwriting
            any older file with the same name).
        metadata : dict, optional
            Information dictionary object (see PDF reference section 10.2.1
            'Document Information Dictionary').
        """
        if metadata is not None:
            raise NotImplementedError("pltrs: PdfPages(metadata=...)")
        self._filename = filename
        self._file = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def _ensure_file(self):
        """The native document, started on first use."""
        if self._file is None:
            self._file = _backends.pdf_pages_new()
        return self._file

    def close(self):
        """
        Finalize this object, making the underlying file a complete
        PDF file.
        """
        if self._file is not None:
            data = _backends.pdf_pages_finish(self._file)
            self._file = None
            _write_bytes(data, self._filename)

    def savefig(self, figure=None, **kwargs):
        """
        Save a `.Figure` to this file as a new page.

        Any other keyword arguments are passed to `~.Figure.savefig`.

        Parameters
        ----------
        figure : `.Figure` or int, default: the active figure
            The figure, or index of the figure, that is saved to the file.
        """
        if not isinstance(figure, Figure):
            if figure is None:
                manager = Gcf.get_active()
            else:
                manager = Gcf.get_fig_manager(figure)
            if manager is None:
                raise ValueError(f"No figure {figure}")
            figure = manager.canvas.figure
        # Force use of pdf backend, as PdfPages is tightly coupled with it.
        figure.savefig(self, format="pdf", backend="pdf", **kwargs)

    def get_pagecount(self):
        """Return the current number of pages in the multipage pdf file."""
        return _backends.pdf_pages_count(self._ensure_file())

    def attach_note(self, text, positionRect=[-100, -100, 0, 0]):
        """
        Add a new text note to the page to be saved next. The note is
        placed outside the page, so that it is invisible on printouts.
        """
        if list(positionRect) != [-100, -100, 0, 0]:
            raise NotImplementedError("pltrs: attach_note(positionRect=...)")
        _backends.pdf_pages_attach_note(self._ensure_file(), str(text))


class FigureCanvasPdf(FigureCanvasBase):
    # docstring inherited

    fixed_dpi = 72
    filetypes = {'pdf': 'Portable Document Format'}

    def get_default_filetype(self):
        return 'pdf'

    def print_pdf(self, filename, *, bbox_inches_restore=None, metadata=None,
                  facecolor=None, edgecolor=None):
        """Write the figure as a PDF document to *filename*, with the
        document *metadata*, or as a new page of the `PdfPages` *filename*
        (whose document has its own metadata)."""
        settings = dict(bbox_inches_restore=bbox_inches_restore,
                        facecolor=facecolor, edgecolor=edgecolor)
        if isinstance(filename, PdfPages):
            _backends.pdf_pages_savefig(
                filename._ensure_file(), self.figure._native,
                self._print_settings(**settings))
            DrawEvent("draw_event", self, None)._process()
        else:
            self._print_native("pdf", filename, metadata=metadata, **settings)


FigureManagerPdf = FigureManagerBase


@_Backend.export
class _BackendPdf(_Backend):
    FigureCanvas = FigureCanvasPdf
