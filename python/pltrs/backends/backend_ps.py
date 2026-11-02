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

from .. import _api, _get_executable_info, rcParams
from .._native import (
    backends as _backends, font_manager as _font_manager, text_overrides as _text_overrides)
from ..backend_bases import _Backend, FigureCanvasBase, FigureManagerBase
from ..font_manager import FontManager
from ._backend_pdf_ps import RendererPDFPSBase
from ._native_drawing import DocumentDrawingMixin
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


class RendererPS(DocumentDrawingMixin, RendererPDFPSBase):
    """
    The renderer of a PostScript page of *width* × *height* inches, drawn
    in points, its images at *imagedpi*, written to *pswriter*.

    pltrs (Rust) writes the documents itself: this is the renderer the
    program's drawing receives while a figure is saved as PostScript
    (`FigureCanvasPS._program_renderer`), which draws into the document
    pltrs (Rust) writes.
    """

    _use_afm_rc_name = "ps.useafm"

    def __init__(self, width, height, pswriter, imagedpi=72):
        super().__init__(width, height)
        self._pswriter = pswriter
        self.imagedpi = imagedpi

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
        checks of the paper type and orientation."""
        if papertype is None:
            papertype = rcParams['ps.papersize']
        papertype = papertype.lower()
        _api.check_in_list(['figure', *papersize], papertype=papertype)
        orientation = _api.getitem_checked(
            _Orientation, orientation=orientation.lower())
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
