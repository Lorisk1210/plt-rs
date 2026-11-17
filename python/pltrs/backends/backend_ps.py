"""
The PostScript backend (matplotlib's ``backend_ps``): `FigureCanvasPS`,
whose ``print_ps`` and ``print_eps`` write the figure as PostScript, on
a page of :rc:`ps.papersize` or as Encapsulated PostScript. pltrs (Rust)
writes the documents.
"""

from enum import Enum
import functools
import logging
import math
import os
import pathlib
from tempfile import TemporaryDirectory

from .. import _api, _get_executable_info, cbook, rcParams
from .._native import (
    backends as _backends, font_manager as _font_manager, text_overrides as _text_overrides)
from ..backend_bases import (
    _Backend, FigureCanvasBase, FigureManagerBase, GraphicsContextBase)
from ..font_manager import FontManager
from ..text import _drawing_with
from ._backend_pdf_ps import RendererPDFPSBase
from ._native_drawing import DocumentDrawingMixin
from ._ps_arguments import CAPS, JOINS, PsArgumentsMixin, check_numbers
from ._source_date import source_date
from .backend_mixed import MixedModeRenderer

_log = logging.getLogger(__name__)

debugPS = False

#: The Adobe glyph names of the characters that have one, by code point
#: (matplotlib's ``_mathtext_data.uni2type1``).
uni2type1 = dict(_font_manager.uni2type1())


def _bbox_extents(bbox):
    """A bounding box ``(llx, lly, urx, ury)`` as floats, or None."""
    return None if bbox is None else tuple(map(float, bbox))


def _paper_options(eps, ptype, bbox, sep):
    """The paper matplotlib's distillers ask for, its options joined by
    *sep* (``-dEPSCrop``, the size of the figure read from *bbox*, or
    *ptype*), with their errors; and the paper pltrs (Rust) distills on,
    which *eps* crops to the bounding box."""
    if eps:
        return ["-dEPSCrop"], "letter"
    if ptype == "figure":
        # The bbox will have its lower-left corner at (0, 0), so upper-right
        # corner corresponds with paper size.
        return [f"-dDEVICEWIDTHPOINTS{sep}{bbox[2]}",
                f"-dDEVICEHEIGHTPOINTS{sep}{bbox[3]}"], ptype
    return [f"-sPAPERSIZE{sep}{ptype}"], str(ptype)


def gs_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    """
    Use ghostscript's pswrite or epswrite device to distill a file.
    This yields smaller files without illegal encapsulated postscript
    operators. The output is low-level, converting text to outlines.
    """
    paper_option, paper = _paper_options(eps, ptype, bbox, "=")
    psfile = tmpfile + '.ps'
    gs = _get_executable_info("gs").executable
    if not os.path.exists(tmpfile):
        # pltrs (Rust) distills a copy of the file; matplotlib hands
        # Ghostscript the file itself, whose failure it reports.
        cbook._check_and_log_subprocess(
            [gs, "-dBATCH", "-dNOPAUSE", "-dSAFER",
             "-r%d" % rcParams['ps.distiller.res'], "-sDEVICE=ps2write",
             *paper_option, f"-sOutputFile={psfile}", tmpfile],
            _log)
    if eps:
        # matplotlib's `pstoeps` of the distilled file.
        _check_eps_header(bbox, rotated)
    _backends.distill(os.fspath(tmpfile), "ghostscript", bool(eps), paper,
                      _bbox_extents(bbox) if eps or paper == "figure" else None,
                      bool(eps and rotated))


def xpdf_distill(tmpfile, eps=False, ptype='letter', bbox=None, rotated=False):
    """
    Use ghostscript's ps2pdf and xpdf's/poppler's pdftops to distill a file.
    This yields smaller files without illegal encapsulated postscript
    operators. This distiller is preferred, generating high-level postscript
    output that treats text as text.
    """
    _get_executable_info("gs")  # Effectively checks for ps2pdf.
    _get_executable_info("pdftops")
    paper_option, paper = _paper_options(eps, ptype, bbox, "#")
    if not (isinstance(tmpfile, (str, os.PathLike)) and os.path.exists(tmpfile)):
        # matplotlib hands ps2pdf the file itself, whose failure it reports.
        with TemporaryDirectory() as tmpdir:
            cbook._check_and_log_subprocess(
                ["ps2pdf",
                 "-dSAFER",
                 "-dAutoFilterColorImages#false",
                 "-dAutoFilterGrayImages#false",
                 "-sAutoRotatePages#None",
                 "-sGrayImageFilter#FlateEncode",
                 "-sColorImageFilter#FlateEncode",
                 *paper_option,
                 tmpfile, pathlib.Path(tmpdir, "tmp.pdf")],
                _log)
    # matplotlib's xpdf distillation leaves the bounding box and the
    # rotation of the file as pdftops writes them: *bbox* sizes a page the
    # size of the figure alone, and *rotated* is not used.
    _backends.distill(os.fspath(tmpfile), "xpdf", bool(eps), paper,
                      _bbox_extents(bbox) if paper == "figure" else None, False)


def _get_bbox_header(lbrt):
    """Return a PostScript header string for bounding box *lbrt*=(l, b, r, t)."""
    l, b, r, t = lbrt
    return (f"%%BoundingBox: {int(l)} {int(b)} {math.ceil(r)} {math.ceil(t)}\n"
            f"%%HiResBoundingBox: {l:.6f} {b:.6f} {r:.6f} {t:.6f}")


def _get_rotate_command(lbrt):
    """Return a PostScript 90° rotation command for bounding box *lbrt*=(l, b, r, t)."""
    l, b, r, t = lbrt
    return f"{l+r:.2f} {0:.2f} translate\n90 rotate"


def _check_eps_header(bbox, rotated):
    """Write the header of an EPS file declaring *bbox* and turned by
    *rotated* as matplotlib's `pstoeps` writes it, for its errors."""
    if bbox:
        _get_bbox_header(bbox)
    if rotated:
        _get_rotate_command(bbox)


def pstoeps(tmpfile, bbox=None, rotated=False):
    """
    Convert the postscript to encapsulated postscript.  The bbox of
    the eps file will be replaced with the given *bbox* argument. If
    None, original bbox will be used.
    """
    epsfile = tmpfile + '.eps'
    # matplotlib writes the EPS file beside the file, reading the file.
    with open(epsfile, 'wb'), open(tmpfile, 'rb'):
        pass
    os.remove(epsfile)
    _check_eps_header(bbox, rotated)
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
    matplotlib writes its DSC comments: the file's name as title, the
    creator of *metadata*, whose other entries are not written, and the
    date of ``SOURCE_DATE_EPOCH`` (in whole seconds) whatever *metadata*
    says.
    """
    dsc_comments = {}
    if isinstance(outfile, (str, os.PathLike)):
        dsc_comments["Title"] = pathlib.Path(outfile).name.encode(
            "ascii", "replace").decode("ascii")
    if metadata is not None and "Creator" in metadata:
        dsc_comments["Creator"] = metadata["Creator"]
    date = source_date()
    if date is not None:
        dsc_comments["CreationDate"] = int(date.timestamp())
    return dsc_comments


class RendererPS(PsArgumentsMixin, DocumentDrawingMixin, RendererPDFPSBase):
    """
    The renderer handles all the drawing primitives using a graphics
    context instance that controls the colors/styles.
    """
    #: How the renderer measures the text pltrs (Rust) lays out with it:
    #: as matplotlib's PostScript backend (see `._text_metrics`).
    _text_metrics = "ps"

    # pltrs (Rust) writes the documents itself: this is the renderer the
    # program's drawing receives while a figure is saved as PostScript
    # (`FigureCanvasPS._program_renderer`), which draws into the document
    # pltrs (Rust) writes. One the program makes around a *pswriter* of its
    # own writes the PostScript of what it draws there, as it draws, with the
    # graphics state it keeps (``set_color``, ...).

    _afm_font_dir = cbook._get_data_path("fonts/afm")
    _use_afm_rc_name = "ps.useafm"

    def __init__(self, width, height, pswriter, imagedpi=72):
        super().__init__(width, height)
        self._pswriter = pswriter
        if rcParams['text.usetex']:
            self.textcnt = 0
            self.psfrag = []
        self.imagedpi = imagedpi
        self._hatches = {}
        self._logwarn_once = functools.cache(_log.warning)
        self.image_magnification = imagedpi / 72
        if pswriter is not None:
            self._file = pswriter

    def _new_page(self):
        return _backends.ps_page_new(float(self.width), float(self.height),
                                     float(self.imagedpi))

    def _state(self, change, *args):
        """Change the page's graphics state with ``_native.backends``'
        *change*, writing what that writes."""
        self._write((change(self._document_page(), *args), ()))

    # The graphics state's setters check their arguments as matplotlib's
    # format them (`._ps_arguments`) before pltrs (Rust) writes them.

    def set_color(self, r, g, b, store=True):
        self._check_color(r, g, b)
        self._state(_backends.ps_set_color, (float(r), float(g), float(b)),
                    bool(store))

    def set_linewidth(self, linewidth, store=True):
        self._state(_backends.ps_set_linewidth, float(linewidth), bool(store))

    def set_linejoin(self, linejoin, store=True):
        JOINS[linejoin]  # noqa: B018
        self._state(_backends.ps_set_linejoin, linejoin, bool(store))

    def set_linecap(self, linecap, store=True):
        CAPS[linecap]  # noqa: B018
        self._state(_backends.ps_set_linecap, linecap, bool(store))

    def set_linedash(self, offset, seq, store=True):
        if seq is not None and len(seq):
            check_numbers(*seq)
            check_numbers(offset)
        self._state(_backends.ps_set_linedash, float(offset),
                    None if seq is None else [float(s) for s in seq],
                    bool(store))

    def set_font(self, fontname, fontsize, store=True):
        f"{fontsize:1.3f}"  # noqa: B018
        self._state(_backends.ps_set_font, str(fontname), float(fontsize),
                    bool(store))

    def create_hatch(self, hatch, linewidth):
        if hatch in self._hatches:
            return self._hatches[hatch]
        f"{linewidth:g}"  # noqa: B018  (matplotlib writes it into the pattern)
        name, written = _backends.ps_create_hatch(
            self._document_page(), str(hatch), float(linewidth))
        self._write((written, ()))
        self._hatches[hatch] = name
        return name

    def get_image_magnification(self):
        # docstring inherited
        """
        Get the factor by which to magnify images passed to draw_image.
        Allows a backend to have images at a different resolution to other
        artists.
        """
        return self.image_magnification

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure_ps(s, FontManager._props(prop), ismath,
                                          72.0)

    # The drawing methods of `._native_drawing.DocumentDrawingMixin`, which
    # pltrs (Rust) draws, as the renderer's own (matplotlib's renderers each
    # define them).

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        self._path_arguments(gc, path, transform, rgbFace)
        super().draw_path(gc, path, transform, rgbFace)

    def draw_markers(self, gc, marker_path, marker_trans, path, trans,
                     rgbFace=None):
        # docstring inherited
        if self._markers_arguments(gc, marker_path, marker_trans, path, trans, rgbFace):
            super().draw_markers(gc, marker_path, marker_trans, path, trans, rgbFace)

    def draw_path_collection(self, gc, master_transform, paths, all_transforms,
                             offsets, offset_trans, facecolors, edgecolors,
                             linewidths, linestyles, antialiaseds, urls,
                             offset_position, *, hatchcolors=None):
        # Is the optimization worth it? Rough calculation:
        # cost of emitting a path in-line is
        #     (len_path + 2) * uses_per_path
        # cost of definition+use is
        #     (len_path + 3) + 3 * uses_per_path
        len_path, uses_per_path = self._collection_uses(
            paths, all_transforms, offsets, facecolors, edgecolors)
        should_do_optimization = \
            len_path + 3 * uses_per_path + 3 < (len_path + 2) * uses_per_path
        self._draw_collection(
            should_do_optimization, gc, master_transform, paths, all_transforms,
            offsets, offset_trans, facecolors, edgecolors, linewidths, linestyles,
            antialiaseds, urls, offset_position, hatchcolors)

    def draw_gouraud_triangles(self, gc, points, colors, trans):
        # docstring inherited
        if self._gouraud_arguments(gc, points, colors, trans):
            # matplotlib's shading reads nothing of *gc*, which may be any
            # object.
            if not isinstance(gc, GraphicsContextBase):
                gc = self.new_gc()
            super().draw_gouraud_triangles(gc, points, colors, trans)

    def draw_image(self, gc, x, y, im, transform=None):
        # docstring inherited
        self._image_arguments(gc, x, y, im, transform)
        # matplotlib writes the image's bytes as they are, never casting
        # its values (no numpy warning for those that are no bytes).
        with np.errstate(invalid='ignore'):
            super().draw_image(gc, x, y, im, transform)

    def draw_mathtext(self, gc, x, y, s, prop, angle):
        """Draw the math text using matplotlib.mathtext."""
        self._mathtext_arguments(gc, x, y, s, prop, angle)
        self._draw_line(gc, x, y, s, prop, angle, True)

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        # docstring inherited
        if self._is_transparent(gc.get_rgb()):
            return  # Special handling for fully transparent.
        if not hasattr(self, "psfrag"):
            if self._pswriter is not None:
                # Drawing into a document of the program's (pltrs (Rust)
                # logs this once per document it prints).
                self._logwarn_once(
                    "The PS backend determines usetex status solely based on "
                    "rcParams['text.usetex'] and does not support having "
                    "usetex=True only for some elements; this element will "
                    "thus be rendered as if usetex=False.")
            self.draw_text(gc, x, y, s, prop, angle, False, mtext)
            return
        self._tex_arguments(gc, x, y, s, prop, angle)
        super().draw_tex(gc, x, y, s, prop, angle, mtext=mtext)

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited
        if ismath == 'TeX':
            if not self._is_transparent(gc.get_rgb()):
                self.draw_tex(gc, x, y, s, prop, angle)
        elif self._text_arguments(gc, x, y, s, prop, angle, ismath, mtext):
            # Any true *ismath* typesets mathtext, as matplotlib's does.
            super().draw_text(gc, x, y, s, prop, angle, bool(ismath), mtext)


class FigureCanvasPS(FigureCanvasBase):
    fixed_dpi = 72
    filetypes = {'ps': 'Postscript',
                 'eps': 'Encapsulated Postscript'}

    def get_default_filetype(self):
        return 'ps'

    def _print_renderer(self):
        """The renderer the figure is laid out with for its PostScript
        document (the one matplotlib's ``print_ps`` measures with)."""
        width, height = self.figure.get_size_inches()
        return RendererPS(width, height, None, imagedpi=self.figure.dpi)

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
            bbox_inches_restore=None):
        """Write the figure as *fmt*, ``ps`` or ``eps``, after matplotlib's
        checks of the paper type and orientation, its texts measured
        through the metrics cache of a renderer of the print's own, as
        matplotlib's ``print_ps`` draws with a new `RendererPS`."""
        dsc_comments = _dsc_metadata(outfile, metadata)
        if papertype is None:
            papertype = rcParams['ps.papersize']
        papertype = papertype.lower()
        _api.check_in_list(['figure', *papersize], papertype=papertype)
        orientation = _api.getitem_checked(
            _Orientation, orientation=orientation.lower())
        width, height = self.figure.get_size_inches()
        with _drawing_with(self.figure, RendererPS(width, height, None,
                                                   imagedpi=self.figure.dpi)):
            self._print_native(
                fmt, outfile, metadata=dsc_comments,
                bbox_inches_restore=bbox_inches_restore,
                orientation=orientation.name, papertype=papertype)

    print_ps = functools.partialmethod(_print_ps, "ps")
    print_eps = functools.partialmethod(_print_ps, "eps")


FigureManagerPS = FigureManagerBase


@_Backend.export
class _BackendPS(_Backend):
    backend_version = 'Level II'
    FigureCanvas = FigureCanvasPS


# What matplotlib's module also has from its imports, which programs
# import from it.
import bisect  # noqa: E402, F401
import codecs  # noqa: E402, F401
import datetime  # noqa: E402, F401
import itertools  # noqa: E402, F401
import numpy as np  # noqa: E402, F401
import shutil  # noqa: E402, F401
import struct  # noqa: E402, F401
import textwrap  # noqa: E402, F401
import time  # noqa: E402, F401
from io import StringIO  # noqa: E402, F401
from ..backend_bases import RendererBase  # noqa: E402, F401
from ..cbook import file_requires_unicode, is_writable_file_like  # noqa: E402, F401
from ..font_manager import get_font  # noqa: E402, F401
from ..ft2font import LoadFlags  # noqa: E402, F401
from ..path import Path  # noqa: E402, F401
from ..texmanager import TexManager  # noqa: E402, F401
from ..transforms import Affine2D  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
from .. import _lazy_imports  # noqa: E402
__getattr__ = _lazy_imports.getter(__name__, fontTools=("fontTools",))
