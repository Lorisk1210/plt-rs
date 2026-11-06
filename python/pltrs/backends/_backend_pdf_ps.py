"""
Common functionality between the PDF and PS backends (matplotlib's
``backends._backend_pdf_ps``): the subsetting of the TrueType fonts they
embed, and the base class of their renderers.

pltrs (Rust) writes the documents and subsets the fonts itself
(pltrs-font's ``subset``, which keeps the glyphs matplotlib's fontTools
subsetting keeps); this module gives its subsetting matplotlib's
functions.
"""

from contextlib import contextmanager
import functools
from io import BytesIO
import typing

from .. import font_manager, rcParams
from .._afm import AFM
from .._native import ft2font as _ft2font
from ..backend_bases import RendererBase


@functools.lru_cache(50)
def _cached_get_afm_from_fname(fname):
    """The `AFM` metrics of the file *fname*, read once."""
    with open(fname, "rb") as fh:
        return AFM(fh)


class _SubsetFont:
    """
    A subset font: the TrueType font file pltrs (Rust) made, where
    matplotlib has a fontTools ``TTFont``.
    """

    def __init__(self, data):
        self._data = data

    def save(self, file, reorderTables=False):
        """Write the font file to the binary file object *file*."""
        file.write(self._data)


class SubsetResults(typing.NamedTuple):
    """
    The results of a subsetting operation on a font.

    Attributes
    ----------
    font
        An open font object representing the subset.
    glyph_index_map : dict
        Mapping of requested glyph indices to actual subset glyph indices.
    """

    font: _SubsetFont
    glyph_index_map: dict

    @contextmanager
    def _as_cm(self):
        yield self


def get_glyphs_subset(fontfile, glyphs):
    """
    Subset a TTF font.

    Reads the named fontfile and restricts the font to the glyphs.

    Parameters
    ----------
    fontfile : FontPath
        Path to the font file
    glyphs : set[GlyphIndexType]
        Set of glyph indices to include in subset.

    Returns
    -------
    SubsetResults
        The font and new glyph index mapping.
    """
    with open(fontfile, 'rb') as file:
        data = file.read()
    subset, kept = _ft2font.subset_font(
        data, fontfile.face_index, sorted(int(glyph) for glyph in glyphs))
    return SubsetResults(_SubsetFont(subset), dict(kept))._as_cm()


# The base of the renderers of the PDF and PostScript backends, for a page of
# *width* × *height* inches drawn in points. Subclasses define
# ``_afm_font_dir``, the directory of their AFM fonts, and
# ``_use_afm_rc_name``, the rcParam choosing them, and measure text as their
# backend does. No docstring, as matplotlib's has none.
class RendererPDFPSBase(RendererBase):

    def __init__(self, width, height):
        super().__init__()
        self.width = width
        self.height = height

    def flipy(self):
        # docstring inherited
        return False  # y increases from bottom to top.

    def option_scale_image(self):
        # docstring inherited
        return True  # PDF and PS support arbitrary image scaling.

    def option_image_nocomposite(self):
        # docstring inherited
        # Decide whether to composite image based on rcParam value.
        return not rcParams["image.composite_image"]

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width * 72.0, self.height * 72.0

    def _get_font_afm(self, prop):
        """The `AFM` metrics of the font of the backend's AFM fonts that
        *prop* finds, those it draws text in under ``_use_afm_rc_name``."""
        fname = font_manager.findfont(
            prop, fontext="afm", directory=self._afm_font_dir)
        return _cached_get_afm_from_fname(fname)


def font_as_file(font):
    """
    Convert a subset font object into a file-like object.

    Parameters
    ----------
    font
        A font object

    Returns
    -------
    BytesIO
        A file object with the font saved into it
    """
    fh = BytesIO()
    font.save(fh, reorderTables=False)
    fh.seek(0)
    return fh
