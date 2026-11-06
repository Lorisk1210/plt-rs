"""
The PostScript backend (matplotlib's ``backend_ps``): `FigureCanvasPS`,
whose ``print_ps`` and ``print_eps`` write the figure as PostScript, on
a page of :rc:`ps.papersize` or as Encapsulated PostScript. pltrs (Rust)
writes the documents.
"""

from enum import Enum
import functools
import os
import pathlib

from .. import _api, _get_executable_info, cbook, rcParams
from .._native import (
    backends as _backends, font_manager as _font_manager, text_overrides as _text_overrides)
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase
from ..font_manager import FontManager
from ..text import _measuring_with
from ._backend_pdf_ps import RendererPDFPSBase
from ._native_drawing import DocumentDrawingMixin, PdfPsDrawingMixin
from .backend_mixed import MixedModeRenderer

debugPS = False

#: The Adobe glyph names of the characters that have one, by code point
#: (matplotlib's ``_mathtext_data.uni2type1``).
uni2type1 = dict(_font_manager.uni2type1())


def _bbox_extents(bbox):
    """A bounding box ``(llx, lly, urx, ury)`` as floats, or None."""
    return None if bbox is None else tuple(map(float, bbox))


def gs_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    """
    Use ghostscript's pswrite or epswrite device to distill a file.
    This yields smaller files without illegal encapsulated postscript
    operators. The output is low-level, converting text to outlines.
    """
    _get_executable_info("gs")
    _backends.distill(os.fspath(tmpfile), "ghostscript", bool(eps), ptype,
                      _bbox_extents(bbox), bool(rotated))


def xpdf_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    """
    Use ghostscript's ps2pdf and xpdf's/poppler's pdftops to distill a file.
    This yields smaller files without illegal encapsulated postscript
    operators. This distiller is preferred, generating high-level postscript
    output that treats text as text.
    """
    _get_executable_info("gs")  # Effectively checks for ps2pdf.
    _get_executable_info("pdftops")
    _backends.distill(os.fspath(tmpfile), "xpdf", bool(eps), ptype,
                      _bbox_extents(bbox), bool(rotated))


def pstoeps(tmpfile, bbox=None, rotated=False):
    """
    Convert the postscript to encapsulated postscript.  The bbox of
    the eps file will be replaced with the given *bbox* argument. If
    None, original bbox will be used.
    """
    _backends.pstoeps(os.fspath(tmpfile), _bbox_extents(bbox), bool(rotated))

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


def _dsc_metadata(outfile, metadata):
    """
    The document metadata of a PostScript file written to *outfile* as
    matplotlib writes its DSC comments: the file's name as title and the
    creator of *metadata*, whose other entries are not written.
    """
    dsc_comments = {}
    if isinstance(outfile, (str, os.PathLike)):
        dsc_comments["Title"] = pathlib.Path(outfile).name.encode(
            "ascii", "replace").decode("ascii")
    if metadata is not None and "Creator" in metadata:
        dsc_comments["Creator"] = metadata["Creator"]
    return dsc_comments


class RendererPS(PdfPsDrawingMixin, DocumentDrawingMixin, RendererPDFPSBase):
    """
    The renderer of a PostScript page of *width* × *height* inches, drawn
    in points, its images at *imagedpi*, written to *pswriter*.

    pltrs (Rust) writes the documents itself: this is the renderer the
    program's drawing receives while a figure is saved as PostScript
    (`FigureCanvasPS._program_renderer`), which draws into the document
    pltrs (Rust) writes. One the program makes around a *pswriter* of its
    own writes the PostScript of what it draws there, as it draws, with
    the graphics state it keeps (``set_color``, ...).
    """

    _afm_font_dir = cbook._get_data_path("fonts/afm")
    _use_afm_rc_name = "ps.useafm"

    def __init__(self, width, height, pswriter, imagedpi=72):
        super().__init__(width, height)
        self._pswriter = pswriter
        self.imagedpi = imagedpi
        if pswriter is not None:
            self._file = pswriter
            self._page = _backends.ps_page_new(float(width), float(height),
                                               float(imagedpi))

    def _state(self, change, *args):
        """Change the page's graphics state with ``_native.backends``'
        *change*, writing what that writes."""
        self._write((change(self._document_page(), *args), ()))

    def set_color(self, r, g, b, store=True):
        """Set the colour of the page, kept as its colour when *store*."""
        self._state(_backends.ps_set_color, (float(r), float(g), float(b)),
                    bool(store))

    def set_linewidth(self, linewidth, store=True):
        """Set the line width of the page in points."""
        self._state(_backends.ps_set_linewidth, float(linewidth), bool(store))

    def set_linejoin(self, linejoin, store=True):
        """Set the line join of the page (a name, or matplotlib's code)."""
        self._state(_backends.ps_set_linejoin, linejoin, bool(store))

    def set_linecap(self, linecap, store=True):
        """Set the line cap of the page (a name, or matplotlib's code)."""
        self._state(_backends.ps_set_linecap, linecap, bool(store))

    def set_linedash(self, offset, seq, store=True):
        """Set the dashes of the page: *offset* into *seq* (None or empty
        for solid lines)."""
        self._state(_backends.ps_set_linedash, float(offset),
                    None if seq is None else [float(s) for s in seq],
                    bool(store))

    def set_font(self, fontname, fontsize, store=True):
        """Select the font *fontname* at *fontsize* points."""
        self._state(_backends.ps_set_font, str(fontname), float(fontsize),
                    bool(store))

    def create_hatch(self, hatch, linewidth):
        """Return the name of the pattern of *hatch*, defining it with lines
        *linewidth* points wide on first use."""
        name, written = _backends.ps_create_hatch(
            self._document_page(), str(hatch), float(linewidth))
        self._write((written, ()))
        return name

    def get_image_magnification(self):
        # docstring inherited
        return self.imagedpi / 72.0

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure_ps(s, FontManager._props(prop), ismath,
                                          72.0)


class FigureCanvasPS(FigureCanvasBase):
    fixed_dpi = 72
    filetypes = {'ps': 'Postscript',
                 'eps': 'Encapsulated Postscript'}

    def get_default_filetype(self):
        return 'ps'

    def _program_renderer(self, dpi, bbox_inches_restore):
        """The renderer the program's drawing receives while the figure is
        saved at *dpi* (matplotlib's ``print_ps`` draws the figure with it):
        a `MixedModeRenderer` around a `RendererPS`, the figure drawn at 72
        dpi meanwhile."""
        width, height = self.figure.get_size_inches()
        return MixedModeRenderer(
            self.figure, width, height, dpi,
            RendererPS(width, height, None, imagedpi=dpi),
            bbox_inches_restore=bbox_inches_restore)

    def draw(self):
        self.figure.draw_without_rendering()
        return super().draw()

    def _print_ps(
            self, fmt, outfile, *,
            metadata=None, papertype=None, orientation='portrait',
            bbox_inches_restore=None, facecolor=None, edgecolor=None):
        """Write the figure as *fmt*, ``ps`` or ``eps``, after matplotlib's
        checks of the paper type and orientation, its texts measured
        through the metrics cache of a renderer of the print's own, as
        matplotlib's ``print_ps`` draws with a new `RendererPS`."""
        if papertype is None:
            papertype = rcParams['ps.papersize']
        papertype = papertype.lower()
        _api.check_in_list(['figure', *papersize], papertype=papertype)
        orientation = _api.getitem_checked(
            _Orientation, orientation=orientation.lower())
        width, height = self.figure.get_size_inches()
        with _measuring_with(RendererPS(width, height, None, imagedpi=self.figure.dpi)):
            self._print_native(
                fmt, outfile, metadata=_dsc_metadata(outfile, metadata),
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
