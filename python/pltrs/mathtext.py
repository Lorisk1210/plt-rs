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
from ._native import mathtext as _native_mathtext, register as _register
from .font_manager import (
    FontManager, FontPath, FontProperties, _data_font_path, findfont, get_font)
from ._mathtext import (  # noqa: F401, reexported API
    RasterParse, VectorParse, get_unicode_index)

_log = logging.getLogger(__name__)


get_unicode_index.__module__ = __name__


def _rc(name):
    """The rcParam *name*."""
    from . import rcParams
    return rcParams._get(name)


#: matplotlib's font class of each fontset, which pltrs (Rust) typesets as.
_OWN_FONT_CLASSES = {
    'cm':          _mathtext.BakomaFonts,
    'dejavuserif': _mathtext.DejaVuSerifFonts,
    'dejavusans':  _mathtext.DejaVuSansFonts,
    'stix':        _mathtext.StixFonts,
    'stixsans':    _mathtext.StixSansFonts,
    'custom':      _mathtext.UnicodeFonts,
}


class _FontTypeMapping(dict):
    """
    The font class of each fontset (``MathTextParser._font_type_mapping``),
    which tells pltrs (Rust) whenever it changes whether a class of the
    program's serves a fontset: matplotlib makes the class for every
    parse and asks it for every glyph, so a typesetting run of such a
    fontset asks an instance of the program's class (see
    `_program_fontset`); runs of the fontsets matplotlib's own classes
    serve stay pltrs (Rust)'s alone.
    """

    def _changed(self):
        _native_mathtext.set_program_fonts(any(
            cls is not _OWN_FONT_CLASSES.get(name) for name, cls in self.items()))
        # A parse made with the class a fontset had is not served once
        # another one serves it.
        MathTextParser._parse_cached.cache_clear()

    def __setitem__(self, key, value):
        super().__setitem__(key, value)
        self._changed()

    def __delitem__(self, key):
        super().__delitem__(key)
        self._changed()

    def update(self, *args, **kwargs):
        super().update(*args, **kwargs)
        self._changed()

    def setdefault(self, key, default=None):
        value = super().setdefault(key, default)
        self._changed()
        return value

    def pop(self, *args):
        value = super().pop(*args)
        self._changed()
        return value

    def popitem(self):
        item = super().popitem()
        self._changed()
        return item

    def clear(self):
        super().clear()
        self._changed()

    def __ior__(self, other):
        super().__ior__(other)
        self._changed()
        return self


# Named as the type it extends, as matplotlib's mapping is a dict
# (``type(MathTextParser._font_type_mapping)`` reads ``<class 'dict'>``).
_FontTypeMapping.__name__ = _FontTypeMapping.__qualname__ = "dict"
_FontTypeMapping.__module__ = "builtins"


class MathTextParser:
    _font_type_mapping = _FontTypeMapping(_OWN_FONT_CLASSES)

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
        # The hinting raster output's glyphs take, as a key of the cache
        # (matplotlib's ``load_glyph_flags``).
        load_glyph_flags = _rc('text.hinting') if self._output_type == "raster" else None
        return self._parse_cached(s, dpi, prop, antialiased, load_glyph_flags)

    @functools.lru_cache(50)
    def _parse_cached(self, s, dpi, prop, antialiased, load_glyph_flags):
        if prop is None:
            prop = FontProperties()
        _api.getitem_checked(
            self._font_type_mapping, fontset=prop.get_math_fontfamily())
        if not (hasattr(dpi, "__float__") or hasattr(dpi, "__index__")):
            # matplotlib's parser sizes its fonts at the resolution, which
            # pybind11 refuses.
            from .ft2font import FT2Font
            FT2Font.set_size.check(get_font(findfont(prop)),
                                   prop.get_size_in_points(), dpi)
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


def _program_fontset(fontset, font, dpi):
    """
    An instance of the program's class serving *fontset* for a typesetting
    run, made as matplotlib's parse makes it (``fontset_class(prop,
    load_glyph_flags)``) with the run's text font *font* (``(families,
    style, weight, stretch, size)``); None while matplotlib's own class
    serves it.
    """
    cls = MathTextParser._font_type_mapping.get(fontset)
    if cls is None or cls is _OWN_FONT_CLASSES.get(fontset):
        return None
    families, style, weight, stretch, size = font
    prop = FontProperties(family=families, style=style, weight=weight,
                          stretch=stretch, size=size, math_fontfamily=fontset)
    return cls(prop, LoadFlags.NO_HINTING)


def _program_glyph(fonts, fontname, font_class, sym):
    """The glyph the program's font set *fonts* gives *sym* (its
    ``_get_glyph``), as pltrs (Rust) takes it: ``(font file, face index,
    character code, slanted)``."""
    font, num, slanted = fonts._get_glyph(fontname, font_class, sym)
    return font.fname, font.face_index, int(num), bool(slanted)


def _program_alternatives(fonts, fontname, sym):
    """The size variants the program's font set *fonts* gives *sym*."""
    return [(str(name), str(symbol))
            for name, symbol in fonts.get_sized_alternatives_for_symbol(fontname, sym)]


_register("program_fontset", _program_fontset)
_register("program_glyph", _program_glyph)
_register("program_alternatives", _program_alternatives)


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
