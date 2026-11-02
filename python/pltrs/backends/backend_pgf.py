"""
The PGF backend (matplotlib's ``backend_pgf``): `FigureCanvasPgf`, whose
``print_pgf`` writes the figure as a LaTeX PGF picture (and ``print_pdf``
and ``print_png`` that picture typeset by the TeX system), `LatexManager`,
which measures text with the TeX system, and `PdfPages`, a PDF document of
several figures typeset by the TeX system. pltrs (Rust) writes the pictures,
their preamble and the TeX source of their texts; the TeX system runs as
matplotlib's runs it (its code, from matplotlib 3.11.2, under the
Matplotlib License).
"""

import datetime
import functools
from io import BytesIO
import logging
import os
import pathlib
import shutil
import subprocess
from tempfile import TemporaryDirectory
import weakref

from .. import _api, _get_executable_info, cbook, ExecutableNotFoundError, rcParams
from .._native import texmanager as _texmanager, text_overrides as _text_overrides
from .._pylab_helpers import Gcf
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from ..figure import Figure
from ..font_manager import FontManager
from ._native_drawing import DocumentDrawingMixin
from .backend_mixed import MixedModeRenderer
from .backend_pdf import _create_pdf_info_dict, _datetime_to_pdf

_log = logging.getLogger(__name__)

_DOCUMENTCLASS = r"\documentclass{article}"

# Note: When formatting floating point values, it is important to use the
# %f/{:f} format rather than %s/{} to avoid triggering scientific notation,
# which is not recognized by TeX.

# It's better to use only one unit for all coordinates, since the
# arithmetic in latex seems to produce inaccurate conversions.
latex_pt_to_in = 1. / 72.27
latex_in_to_pt = 1. / latex_pt_to_in
mpl_pt_to_in = 1. / 72.
mpl_in_to_pt = 1. / mpl_pt_to_in


def make_pdf_to_png_converter():
    """Return a function that converts a pdf file to a png file."""
    try:
        _get_executable_info("pdftocairo")
    except ExecutableNotFoundError:
        pass
    else:
        return lambda pdffile, pngfile, dpi: subprocess.check_output(
            ["pdftocairo", "-singlefile", "-transp", "-png", "-r", "%d" % dpi,
             pdffile, os.path.splitext(pngfile)[0]],
            stderr=subprocess.STDOUT)
    try:
        gs_info = _get_executable_info("gs")
    except ExecutableNotFoundError:
        pass
    else:
        return lambda pdffile, pngfile, dpi: subprocess.check_output(
            [gs_info.executable,
             '-dQUIET', '-dSAFER', '-dBATCH', '-dNOPAUSE', '-dNOPROMPT',
             '-dUseCIEColor', '-dTextAlphaBits=4',
             '-dGraphicsAlphaBits=4', '-dDOINTERPOLATE',
             '-sDEVICE=pngalpha', '-sOutputFile=%s' % pngfile,
             '-r%d' % dpi, pdffile],
            stderr=subprocess.STDOUT)
    raise RuntimeError("No suitable pdf to png renderer found.")


class LatexError(Exception):
    def __init__(self, message, latex_output=""):
        super().__init__(message)
        self.latex_output = latex_output

    def __str__(self):
        s, = self.args
        if self.latex_output:
            s += "\n" + self.latex_output
        return s


class LatexManager:
    """
    The LatexManager opens an instance of the LaTeX application for
    determining the metrics of text elements. The LaTeX environment can be
    modified by setting fonts and/or a custom preamble in `.rcParams`.
    """

    @staticmethod
    def _build_latex_header():
        latex_header = [
            _DOCUMENTCLASS,
            # Include TeX program name as a comment for cache invalidation.
            # TeX does not allow this to be the first line.
            rf"% !TeX program = {rcParams['pgf.texsystem']}",
            # Test whether \includegraphics supports interpolate option.
            r"\usepackage{graphicx}",
            _texmanager.pgf_preamble(),
            r"\begin{document}",
            r"\typeout{pgf_backend_query_start}",
        ]
        return "\n".join(latex_header)

    @classmethod
    def _get_cached_or_new(cls):
        """
        Return the previous LatexManager if the header and tex system did not
        change, or a new instance otherwise.
        """
        return cls._get_cached_or_new_impl(cls._build_latex_header())

    @classmethod
    @functools.lru_cache(1)
    def _get_cached_or_new_impl(cls, header):  # Helper for _get_cached_or_new.
        return cls()

    def _stdin_writeln(self, s):
        if self.latex is None:
            self._setup_latex_process()
        self.latex.stdin.write(s)
        self.latex.stdin.write("\n")
        self.latex.stdin.flush()

    def _expect(self, s):
        s = list(s)
        chars = []
        while True:
            c = self.latex.stdout.read(1)
            chars.append(c)
            if chars[-len(s):] == s:
                break
            if not c:
                self.latex.kill()
                self.latex = None
                raise LatexError("LaTeX process halted", "".join(chars))
        return "".join(chars)

    def _expect_prompt(self):
        return self._expect("\n*")

    def __init__(self):
        # create a tmp directory for running latex, register it for deletion
        self._tmpdir = TemporaryDirectory()
        self.tmpdir = self._tmpdir.name
        self._finalize_tmpdir = weakref.finalize(self, self._tmpdir.cleanup)

        # test the LaTeX setup to ensure a clean startup of the subprocess
        self._setup_latex_process(expect_reply=False)
        stdout, stderr = self.latex.communicate("\n\\makeatletter\\@@end\n")
        if self.latex.returncode != 0:
            raise LatexError(
                f"LaTeX errored (probably missing font or error in preamble) "
                f"while processing the following input:\n"
                f"{self._build_latex_header()}",
                stdout)
        self.latex = None  # Will be set up on first use.
        # Per-instance cache.
        self._get_box_metrics = functools.lru_cache(self._get_box_metrics)

    def _setup_latex_process(self, *, expect_reply=True):
        # Open LaTeX process for real work; register it for deletion.  On
        # Windows, we must ensure that the subprocess has quit before being
        # able to delete the tmpdir in which it runs; in order to do so, we
        # must first `kill()` it, and then `communicate()` with or `wait()` on
        # it.
        try:
            self.latex = subprocess.Popen(
                [rcParams["pgf.texsystem"], "-halt-on-error", "-no-shell-escape"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                encoding="utf-8", cwd=self.tmpdir)
        except FileNotFoundError as err:
            raise RuntimeError(
                f"{rcParams['pgf.texsystem']!r} not found; install it or change "
                f"rcParams['pgf.texsystem'] to an available TeX implementation"
            ) from err
        except OSError as err:
            raise RuntimeError(
                f"Error starting {rcParams['pgf.texsystem']!r}") from err

        def finalize_latex(latex):
            latex.kill()
            try:
                latex.communicate()
            except RuntimeError:
                latex.wait()

        self._finalize_latex = weakref.finalize(
            self, finalize_latex, self.latex)
        # write header with 'pgf_backend_query_start' token
        self._stdin_writeln(self._build_latex_header())
        if expect_reply:  # read until 'pgf_backend_query_start' token appears
            self._expect("*pgf_backend_query_start")
            self._expect_prompt()

    def get_width_height_descent(self, text, prop):
        """
        Get the width, total height, and descent (in TeX points) for a text
        typeset by the current LaTeX environment.
        """
        return self._get_box_metrics(
            _texmanager.pgf_text(text, FontManager._props(prop)))

    def _get_box_metrics(self, tex):
        """
        Get the width, total height and descent (in TeX points) for a TeX
        command's output in the current LaTeX environment.
        """
        # This method gets wrapped in __init__ for per-instance caching.
        self._stdin_writeln(  # Send textbox to TeX & request metrics typeout.
            # \sbox doesn't handle catcode assignments inside its argument,
            # so repeat the assignment of the catcode of "^" and "%" outside.
            r"{\catcode`\^=\active\catcode`\%%=\active\sbox0{%s}"
            r"\typeout{\the\wd0,\the\ht0,\the\dp0}}"
            % tex)
        try:
            answer = self._expect_prompt()
        except LatexError as err:
            # Here and below, use '{}' instead of {!r} to avoid doubling all
            # backslashes.
            raise ValueError("Error measuring {}\nLaTeX Output:\n{}"
                             .format(tex, err.latex_output)) from err
        try:
            # Parse metrics from the answer string.  Last line is prompt, and
            # next-to-last-line is blank line from \typeout.
            width, height, offset = answer.splitlines()[-3].split(",")
        except Exception as err:
            raise ValueError("Error measuring {}\nLaTeX Output:\n{}"
                             .format(tex, answer)) from err
        w, h, o = float(width[:-2]), float(height[:-2]), float(offset[:-2])
        # The height returned from LaTeX goes from base to top;
        # the height Matplotlib expects goes from bottom to top.
        return w, h + o, o


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


class RendererPgf(DocumentDrawingMixin, RendererBase):
    """
    The renderer of a PGF picture of *figure*, written to *fh*, in the
    figure's pixels.

    pltrs (Rust) writes the pictures itself: this is the renderer the
    program's drawing receives while a figure is saved as PGF
    (`FigureCanvasPgf._program_renderer`), which draws into the picture
    pltrs (Rust) writes.
    """

    def __init__(self, figure, fh):
        super().__init__()
        self.dpi = figure.dpi
        self.fh = fh
        self.figure = figure
        self.image_counter = 0

    def option_scale_image(self):
        # docstring inherited
        return True

    def option_image_nocomposite(self):
        # docstring inherited
        return not rcParams['image.composite_image']

    def flipy(self):
        # docstring inherited
        return False

    def get_canvas_width_height(self):
        # docstring inherited
        return (self.figure.get_figwidth() * self.dpi,
                self.figure.get_figheight() * self.dpi)

    def points_to_pixels(self, points):
        # docstring inherited
        return points * mpl_pt_to_in * self.dpi

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure(s, FontManager._props(prop), ismath,
                                       float(self.dpi))


class FigureCanvasPgf(FigureCanvasBase):
    filetypes = {"pgf": "LaTeX PGF picture",
                 "pdf": "LaTeX compiled PGF picture",
                 "png": "Portable Network Graphics", }

    def get_default_filetype(self):
        return 'pgf'

    def _program_renderer(self, dpi, bbox_inches_restore):
        """The renderer the program's drawing receives while the figure is
        saved at *dpi* (matplotlib's ``print_pgf`` draws the figure with
        it): a `MixedModeRenderer` around a `RendererPgf`."""
        width, height = self.figure.get_size_inches()
        return MixedModeRenderer(
            self.figure, width, height, dpi, RendererPgf(self.figure, None),
            bbox_inches_restore=bbox_inches_restore)

    def print_pgf(self, fname_or_fh, *, bbox_inches_restore=None,
                  facecolor=None, edgecolor=None):
        """
        Output pgf macros for drawing the figure so it can be included and
        rendered in latex documents.
        """
        self._print_native("pgf", fname_or_fh,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    def print_pdf(self, fname_or_fh, *, metadata=None, **kwargs):
        """Use LaTeX to compile a pgf generated figure to pdf."""
        w, h = self.figure.get_size_inches()

        info_dict = _create_pdf_info_dict('pgf', metadata or {})
        pdfinfo = ','.join(
            _metadata_to_str(k, v) for k, v in info_dict.items())

        # print figure to pgf and compile it with latex
        with TemporaryDirectory() as tmpdir:
            tmppath = pathlib.Path(tmpdir)
            self.print_pgf(tmppath / "figure.pgf", **kwargs)
            (tmppath / "figure.tex").write_text(
                "\n".join([
                    _DOCUMENTCLASS,
                    r"\usepackage[pdfinfo={%s}]{hyperref}" % pdfinfo,
                    r"\usepackage[papersize={%fin,%fin}, margin=0in]{geometry}"
                    % (w, h),
                    r"\usepackage{pgf}",
                    _texmanager.pgf_preamble(),
                    r"\begin{document}",
                    r"\centering",
                    r"\input{figure.pgf}",
                    r"\end{document}",
                ]), encoding="utf-8")
            texcommand = rcParams["pgf.texsystem"]
            cbook._check_and_log_subprocess(
                [texcommand, "-interaction=nonstopmode", "-halt-on-error",
                 "-no-shell-escape", "figure.tex"], _log, cwd=tmpdir)
            with ((tmppath / "figure.pdf").open("rb") as orig,
                  cbook.open_file_cm(fname_or_fh, "wb") as dest):
                shutil.copyfileobj(orig, dest)  # copy file contents to target

    def print_png(self, fname_or_fh, **kwargs):
        """Use LaTeX to compile a pgf figure to pdf and convert it to png."""
        converter = make_pdf_to_png_converter()
        with TemporaryDirectory() as tmpdir:
            tmppath = pathlib.Path(tmpdir)
            pdf_path = tmppath / "figure.pdf"
            png_path = tmppath / "figure.png"
            self.print_pdf(pdf_path, **kwargs)
            converter(pdf_path, png_path, dpi=self.figure.dpi)
            with (png_path.open("rb") as orig,
                  cbook.open_file_cm(fname_or_fh, "wb") as dest):
                shutil.copyfileobj(orig, dest)  # copy file contents to target


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
