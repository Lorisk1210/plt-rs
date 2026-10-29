"""
Common functionality between the PDF and PS backends (matplotlib's
``backends._backend_pdf_ps``): the subsetting of the TrueType fonts they
embed.

pltrs (Rust) writes the documents and subsets the fonts itself
(pltrs-font's ``subset``, which keeps the glyphs matplotlib's fontTools
subsetting keeps); this module gives its subsetting matplotlib's
functions.
"""

from contextlib import contextmanager
from io import BytesIO
import typing

from .._native import ft2font as _ft2font


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
