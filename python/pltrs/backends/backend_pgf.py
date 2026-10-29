"""
The PGF backend (matplotlib's ``backend_pgf``): `FigureCanvasPgf`, whose
``print_pgf`` writes the figure as a LaTeX PGF picture, and `PdfPages`, a
PDF document of several figures typeset by the TeX system. pltrs (Rust)
writes the pictures and their preamble; `PdfPages` runs the TeX system as
matplotlib's does (its code, from matplotlib 3.11.2, under the Matplotlib
License).
"""

import datetime
from io import BytesIO
import logging
import pathlib
import shutil
from tempfile import TemporaryDirectory

from .. import cbook, rcParams
from .._native import texmanager as _texmanager
from .._pylab_helpers import Gcf
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase
from ..figure import Figure
from .backend_pdf import _create_pdf_info_dict, _datetime_to_pdf

_log = logging.getLogger(__name__)

_DOCUMENTCLASS = r"\documentclass{article}"


def _metadata_to_str(key, value):
    """Convert metadata key/value to a form that hyperref accepts."""
    if isinstance(value, datetime.datetime):
        value = _datetime_to_pdf(value)
    elif key == 'Trapped':
        value = (value.name.decode('ascii') if hasattr(value, 'name')
                 else str(value))
    else:
        value = str(value)

    # ensure that metadata does not contain special TeX chars because we
    # insert the metadata as raw text into the TeX source
    invalid_chars = r"\{}[]()"
    if any(c in value + key for c in invalid_chars):
        raise ValueError(
            f"Invalid metadata value for {key!r}: {value!r}. "
            f"The value must not contain the chars {invalid_chars}.")
    return f"{key}={{{value}}}"


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


class PdfPages:
    """
    A multi-page PDF file using the pgf backend

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

    def __init__(self, filename, *, metadata=None):
        """
        Create a new PdfPages object.

        Parameters
        ----------
        filename : str or path-like
            Plots using `PdfPages.savefig` will be written to a file at this
            location. Any older file with the same name is overwritten.

        metadata : dict, optional
            Information dictionary object (see PDF reference section 10.2.1
            'Document Information Dictionary'), e.g.:
            ``{'Creator': 'My software', 'Author': 'Me', 'Title': 'Awesome'}``.

            The standard keys are 'Title', 'Author', 'Subject', 'Keywords',
            'Creator', 'Producer', 'CreationDate', 'ModDate', and
            'Trapped'. Values have been predefined for 'Creator', 'Producer'
            and 'CreationDate'. They can be removed by setting them to `None`.

            Note that some versions of LaTeX engines may ignore the 'Producer'
            key and set it to themselves.
        """
        self._output_name = filename
        self._n_figures = 0
        self._metadata = (metadata or {}).copy()
        self._info_dict = _create_pdf_info_dict('pgf', self._metadata)
        self._file = BytesIO()

    def _write_header(self, width_inches, height_inches):
        pdfinfo = ','.join(
            _metadata_to_str(k, v) for k, v in self._info_dict.items())
        latex_header = "\n".join([
            _DOCUMENTCLASS,
            r"\usepackage[pdfinfo={%s}]{hyperref}" % pdfinfo,
            r"\usepackage[papersize={%fin,%fin}, margin=0in]{geometry}"
            % (width_inches, height_inches),
            r"\usepackage{pgf}",
            _texmanager.pgf_preamble(),
            r"\setlength{\parindent}{0pt}",
            r"\begin{document}%",
        ])
        self._file.write(latex_header.encode('utf-8'))

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def close(self):
        """
        Finalize this object, running LaTeX in a temporary directory
        and moving the final pdf file to *filename*.
        """
        self._file.write(rb'\end{document}\n')
        if self._n_figures > 0:
            self._run_latex()
        self._file.close()

    def _run_latex(self):
        texcommand = rcParams["pgf.texsystem"]
        with TemporaryDirectory() as tmpdir:
            tex_source = pathlib.Path(tmpdir, "pdf_pages.tex")
            tex_source.write_bytes(self._file.getvalue())
            cbook._check_and_log_subprocess(
                [texcommand, "-interaction=nonstopmode", "-halt-on-error",
                 "-no-shell-escape", tex_source],
                _log, cwd=tmpdir)
            shutil.move(tex_source.with_suffix(".pdf"), self._output_name)

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

        width, height = figure.get_size_inches()
        if self._n_figures == 0:
            self._write_header(width, height)
        else:
            # \pdfpagewidth and \pdfpageheight exist on pdftex, xetex, and
            # luatex<0.85; they were renamed to \pagewidth and \pageheight
            # on luatex>=0.85.
            self._file.write(
                rb'\newpage'
                rb'\ifdefined\pdfpagewidth\pdfpagewidth\else\pagewidth\fi=%fin'
                rb'\ifdefined\pdfpageheight\pdfpageheight\else\pageheight\fi=%fin'
                b'%%\n' % (width, height)
            )
        figure.savefig(self._file, format="pgf", backend="pgf", **kwargs)
        self._n_figures += 1

    def get_pagecount(self):
        """Return the current number of pages in the multipage pdf file."""
        return self._n_figures
