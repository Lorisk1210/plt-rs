"""
The PDF backend (matplotlib's ``backend_pdf``): `FigureCanvasPdf`, whose
``print_pdf`` writes the figure as a PDF document, and `PdfPages`, a
document of several figures. pltrs (Rust) writes the documents.

The document information helpers (`_create_pdf_info_dict`,
`_datetime_to_pdf`), which the PGF backend shares, are matplotlib's (from
matplotlib 3.11.2, under the Matplotlib License).
"""

import datetime
import os
import time

from .. import _api
from .._native import backends as _backends
from .._pylab_helpers import Gcf
from ..backend_bases import (
    _Backend, _write_bytes, DrawEvent, FigureCanvasBase, FigureManagerBase)
from ..figure import Figure


def _create_pdf_info_dict(backend, metadata):
    """
    Create a PDF infoDict based on user-supplied metadata.

    A default ``Creator``, ``Producer``, and ``CreationDate`` are added, though
    the user metadata may override it. The date may be the current time, or a
    time set by the ``SOURCE_DATE_EPOCH`` environment variable.

    Metadata is verified to have the correct keys and their expected types. Any
    unknown keys/types will raise a warning.

    Parameters
    ----------
    backend : str
        The name of the backend to use in the Producer value.

    metadata : dict[str, Union[str, datetime]]
        A dictionary of metadata supplied by the user with information
        following the PDF specification. If any value is *None*, then the key
        will be removed. This can be used to remove any pre-defined values.

    Returns
    -------
    dict[str, Union[str, datetime]]
        A validated dictionary of metadata.
    """
    source_date_epoch = os.getenv("SOURCE_DATE_EPOCH")
    if source_date_epoch:
        source_date = datetime.datetime.fromtimestamp(
            int(source_date_epoch), datetime.timezone.utc)
    else:
        source_date = datetime.datetime.today()

    creator = _backends.default_creator()
    info = {
        'Creator': f'{creator}, matplotlib {__version__} API',
        'Producer': f'{creator} {backend} backend',
        'CreationDate': source_date,
        **metadata
    }
    info = {k: v for (k, v) in info.items() if v is not None}
    _check_info(info)
    return info


def _is_string_like(x):
    return isinstance(x, str)


_is_string_like.text_for_warning = "an instance of str"


def _is_date(x):
    return isinstance(x, datetime)


_is_date.text_for_warning = "an instance of datetime.datetime"


def _check_trapped(x):
    return x in ('True', 'False', 'Unknown')


_check_trapped.text_for_warning = 'one of {"True", "False", "Unknown"}'

#: The checks of the keys of a document information dictionary.
_INFO_CHECKS = {
    'Title': _is_string_like,
    'Author': _is_string_like,
    'Subject': _is_string_like,
    'Keywords': _is_string_like,
    'Creator': _is_string_like,
    'Producer': _is_string_like,
    'CreationDate': _is_date,
    'ModDate': _is_date,
    'Trapped': _check_trapped,
}


def _check_info(info):
    """Warn about the unknown keys and the values of the wrong type of the
    information dictionary *info*; return the keys that pass."""
    valid = []
    for k in info:
        if k not in _INFO_CHECKS:
            _api.warn_external(f'Unknown infodict keyword: {k!r}. '
                               f'Must be one of {set(_INFO_CHECKS)!r}.')
        elif not _INFO_CHECKS[k](info[k]):
            _api.warn_external(f'Bad value for infodict keyword {k}. '
                               f'Got {info[k]!r} which is not '
                               f'{_INFO_CHECKS[k].text_for_warning}.')
        else:
            valid.append(k)
    return valid


def _native_info(metadata):
    """The document *metadata* of a PDF document for pltrs (Rust), checked
    as matplotlib checks it: the entries it warns about are left out, dates
    are seconds since the epoch and None removes an entry."""
    if metadata is None:
        return None
    given = {k: v for k, v in metadata.items() if v is not None}
    native = {k: None for k, v in metadata.items() if v is None}
    for key in _check_info(given):
        value = given[key]
        native[key] = (int(value.timestamp()) if isinstance(value, datetime)
                       else value)
    return native


def _datetime_to_pdf(d):
    """
    Convert a datetime to a PDF string representing it.

    Used for PDF and PGF.
    """
    r = d.strftime('D:%Y%m%d%H%M%S')
    z = d.utcoffset()
    if z is not None:
        z = z.seconds
    else:
        if time.daylight:
            z = time.altzone
        else:
            z = time.timezone
    if z == 0:
        r += 'Z'
    elif z < 0:
        r += "+%02d'%02d'" % ((-z) // 3600, (-z) % 3600)
    else:
        r += "-%02d'%02d'" % (z // 3600, z % 3600)
    return r


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

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()

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
            self._print_native("pdf", filename, metadata=_native_info(metadata),
                               **settings)


FigureManagerPdf = FigureManagerBase


@_Backend.export
class _BackendPdf(_Backend):
    FigureCanvas = FigureCanvasPdf




