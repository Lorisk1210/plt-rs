r"""
A module for parsing a subset of the TeX math syntax and rendering it to a
Matplotlib backend (matplotlib's ``mathtext``).

For a tutorial of its usage, see :ref:`mathtext`.  pltrs (Rust) parses and
typesets the expressions (``pltrs_text::mathtext``); this module is
matplotlib's interface to it.

The Bakoma distribution of the TeX Computer Modern fonts, and STIX
fonts are supported.  There is experimental support for using
arbitrary fonts, but results may vary without proper tweaking and
metrics for those fonts.
"""

import functools
import logging

import numpy as np

from . import _api, _mathtext
from ._native import mathtext as _native_mathtext
from .font_manager import (
    FontManager, FontPath, FontProperties, _data_font_path, get_font)
from ._mathtext import (  # noqa: F401, reexported API
    RasterParse, VectorParse, get_unicode_index)

_log = logging.getLogger(__name__)


get_unicode_index.__module__ = __name__


def _rc(name):
    """The rcParam *name*."""
    from . import rcParams
    return rcParams[name]


class MathTextParser:
    _font_type_mapping = {
        'cm':          _mathtext.BakomaFonts,
        'dejavuserif': _mathtext.DejaVuSerifFonts,
        'dejavusans':  _mathtext.DejaVuSansFonts,
        'stix':        _mathtext.StixFonts,
        'stixsans':    _mathtext.StixSansFonts,
        'custom':      _mathtext.UnicodeFonts,
    }

    def __init__(self, output):
        """
        Create a MathTextParser for the given backend *output*.

        Parameters
        ----------
        output : {"path", "agg"}
            Whether to return a `VectorParse` ("path") or a
            `RasterParse` ("agg", or its synonym "macosx").
        """
        self._output_type = _api.getitem_checked(
            {"path": "vector", "agg": "raster", "macosx": "raster"},
            output=output.lower())

    def parse(self, s, dpi=72, prop=None, *, antialiased=None):
        """
        Parse the given math expression *s* at the given *dpi*.  If *prop* is
        provided, it is a `.FontProperties` object specifying the "default"
        font to use in the math expression, used for all non-math text.

        The results are cached, so multiple calls to `parse`
        with the same expression should be fast.

        Depending on the *output* type, this returns either a `VectorParse` or
        a `RasterParse`.
        """
        # lru_cache can't decorate parse() directly because prop is
        # mutable, so we key the cache using an internal copy; likewise,
        # we need to check the mutable state of the text.antialiased and
        # text.hinting rcParams (raster output hints its glyphs).
        if not isinstance(s, str):
            # matplotlib's pyparsing parser expands the tabs of what it
            # parses first, failing on what has no ``expandtabs``.
            s.expandtabs()
        prop = prop.copy() if prop is not None else None
        if antialiased is None:
            antialiased = _rc('text.antialiased')
        hinting = _rc('text.hinting') if self._output_type == "raster" else None
        return self._parse_cached(s, dpi, prop, antialiased, hinting)

    @functools.lru_cache(50)
    def _parse_cached(self, s, dpi, prop, antialiased, hinting):
        if prop is None:
            prop = FontProperties()
        _api.getitem_checked(
            self._font_type_mapping, fontset=prop.get_math_fontfamily())
        # matplotlib's parser (pyparsing) first expands the tabs of *s*.
        s = s.expandtabs()
        props = FontManager._props(prop)
        if self._output_type == "vector":
            (width, height, depth), glyphs, rects = _native_mathtext.parse_path(
                s, float(dpi), props)
            # The numpy floats of the boxes matplotlib's parser sets (and
            # of the glyphs' heights, which it moves them by).
            return VectorParse(
                np.float64(width), np.float64(height), np.float64(depth),
                [(get_font(FontPath(_data_font_path(file), face_index)),
                  size, code, index, ox, np.float64(oy))
                 for (file, face_index), size, (code, index), (ox, oy)
                 in glyphs],
                [(x, np.float64(y), width, height) for x, y, width, height in rects])
        (width, height, depth), image = _native_mathtext.parse_raster(
            s, float(dpi), props, bool(antialiased))
        return RasterParse(0, 0, width, height, depth, image)


def math_to_image(s, filename_or_obj, prop=None, dpi=None, format=None,
                  *, color=None):
    """
    Given a math expression, renders it in a closely-clipped bounding
    box to an image file.

    Parameters
    ----------
    s : str
        A math expression.  The math portion must be enclosed in dollar signs.
    filename_or_obj : str or path-like or file-like
        Where to write the image data.
    prop : `.FontProperties`, optional
        The size and style of the text.
    dpi : float, optional
        The output dpi.  If not set, the dpi is determined as for
        `.Figure.savefig`.
    format : str, optional
        The output format, e.g., 'svg', 'pdf', 'ps' or 'png'.  If not set, the
        format is determined as for `.Figure.savefig`.
    color : str, optional
        Foreground color, defaults to :rc:`text.color`.
    """
    from .figure import Figure

    parser = MathTextParser('path')
    width, height, depth, _, _ = parser.parse(s, dpi=72, prop=prop)

    fig = Figure(figsize=(width / 72.0, height / 72.0))
    fig.text(0, depth/height, s, fontproperties=prop, color=color)
    fig.savefig(filename_or_obj, dpi=dpi, format=format)

    return depth


# What matplotlib's module also has from its imports, which programs
# import from it.
from .ft2font import LoadFlags  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
