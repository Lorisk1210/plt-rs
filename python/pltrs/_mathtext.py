"""
Implementation details for :mod:`.mathtext` (matplotlib's ``_mathtext``).

pltrs (Rust) parses and typesets mathtext (``pltrs_text::mathtext``, a port
of matplotlib's parser, font sets and TeX box model). This module keeps
matplotlib's names for what the Python side sees of it:

* the parse results, `get_unicode_index` and matplotlib's font classes
  (`Fonts`, `TruetypeFonts`, `BakomaFonts`, `UnicodeFonts`, `DejaVuFonts`,
  `StixFonts` and their font constants), as they are: from matplotlib
  3.11.2, under the Matplotlib License (see ``LICENSES/matplotlib.txt``).
  pltrs (Rust) typesets with its own copy of what the classes do; they are
  what a program's subclass of them (put in
  ``MathTextParser._font_type_mapping``) calls with ``super()``: while a
  class of the program's serves a fontset, pltrs (Rust) makes one per
  typesetting run and asks it for every glyph (see :mod:`.mathtext`). The
  tables they read are pltrs' own (`._mathtext_data`);
* the box tree `Parser.parse` returns (the node classes and their reprs
  are matplotlib's, from matplotlib 3.11.2, under the Matplotlib License)
  and the parser's symbol tables.
"""

from __future__ import annotations

import abc
import functools
import logging
import os
import textwrap
import typing as T
import unicodedata
from typing import NamedTuple

from . import cbook
from ._mathtext_data import (
    latex_to_bakoma, stix_glyph_fixes, stix_virtual_fonts, tex2uni)
from ._native import mathtext as _mathtext, register as _register
from .font_manager import FontProperties, findfont, get_font
from .ft2font import FT2Font, Kerning, LoadFlags

if T.TYPE_CHECKING:
    from .ft2font import CharacterCodeType, Glyph, GlyphIndexType

mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
_log = logging.getLogger(__name__.partition('.')[0] + ".mathtext")


def log_mathtext_warning(message):
    """Log *message*, about a formula pltrs (Rust) typeset (a list packed to
    a size its glue cannot reach, "Underful Hlist: ...", a symbol no font
    has), as matplotlib's mathtext logs it."""
    _log.warning("%s", message)


# pltrs (Rust) notes the messages as it typesets formulas.
_register("log_mathtext_warning", log_mathtext_warning)


class VectorParse(NamedTuple):
    """
    The namedtuple type returned by ``MathTextParser("path").parse(...)``.

    Attributes
    ----------
    width, height, depth : float
        The global metrics.
    glyphs : list
        The glyphs including their positions.
    rect : list
        The list of rectangles.
    """
    width: float
    height: float
    depth: float
    glyphs: list
    rects: list


VectorParse.__module__ = __name__.rpartition(".")[0] + ".mathtext"


class RasterParse(NamedTuple):
    """
    The namedtuple type returned by ``MathTextParser("agg").parse(...)``.

    Attributes
    ----------
    ox, oy : float
        The offsets are always zero.
    width, height, depth : float
        The global metrics.
    image : 2D array of uint8
        A raster image.
    """
    ox: float
    oy: float
    width: float
    height: float
    depth: float
    image: object


RasterParse.__module__ = __name__.rpartition(".")[0] + ".mathtext"



def get_unicode_index(symbol: str) -> CharacterCodeType:  # Publicly exported.
    r"""
    Return the integer index (from the Unicode table) of *symbol*.

    Parameters
    ----------
    symbol : str
        A single (Unicode) character, a TeX command (e.g. r'\pi') or a Type1
        symbol name (e.g. 'phi').
    """
    try:  # This will succeed if symbol is a single Unicode char
        return ord(symbol)
    except TypeError:
        pass
    try:  # Is symbol a TeX symbol (i.e. \alpha)
        return tex2uni[symbol.strip("\\")]
    except KeyError as err:
        raise ValueError(
            f"{symbol!r} is not a valid Unicode character or TeX/Type1 symbol"
            ) from err


class FontMetrics(NamedTuple):
    """
    Metrics of a font.

    Attributes
    ----------
    advance : float
        The advance distance (in points) of the glyph.
    height : float
        The height of the glyph in points.
    width : float
        The width of the glyph in points.
    xmin, xmax, ymin, ymax : float
        The ink rectangle of the glyph.
    iceberg : float
        The distance from the baseline to the top of the glyph. (This corresponds to
        TeX's definition of "height".)
    slanted : bool
        Whether the glyph should be considered as "slanted" (currently used for kerning
        sub/superscripts).
    """
    advance: float
    height: float
    width: float
    xmin: float
    xmax: float
    ymin: float
    ymax: float
    iceberg: float
    slanted: bool


class FontInfo(NamedTuple):
    font: FT2Font
    fontsize: float
    postscript_name: str
    metrics: FontMetrics
    num: CharacterCodeType
    glyph_index: GlyphIndexType
    glyph: Glyph
    offset: float


class Fonts(abc.ABC):
    """
    An abstract base class for a system of fonts to use for mathtext.

    The class must be able to take symbol keys and font file names and
    return the character metrics.  It also delegates to a backend class
    to do the actual drawing.
    """

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        """
        Parameters
        ----------
        default_font_prop : `~.font_manager.FontProperties`
            The default non-math font, or the base font for Unicode (generic)
            font rendering.
        load_glyph_flags : `.ft2font.LoadFlags`
            Flags passed to the glyph loader (e.g. ``FT_Load_Glyph`` and
            ``FT_Load_Char`` for FreeType-based fonts).
        """
        self.default_font_prop = default_font_prop
        self.load_glyph_flags = load_glyph_flags

    def get_kern(self, font1: str, fontclass1: str, sym1: str, fontsize1: float,
                 font2: str, fontclass2: str, sym2: str, fontsize2: float,
                 dpi: float) -> float:
        """
        Get the kerning distance for font between *sym1* and *sym2*.

        See `~.Fonts.get_metrics` for a detailed description of the parameters.
        """
        return 0.

    def _get_font(self, font: str) -> FT2Font:
        raise NotImplementedError

    def _get_info(self, font: str, font_class: str, sym: str, fontsize: float,
                  dpi: float) -> FontInfo:
        raise NotImplementedError

    def get_metrics(self, font: str, font_class: str, sym: str, fontsize: float,
                    dpi: float) -> FontMetrics:
        r"""
        Parameters
        ----------
        font : str
            One of the TeX font names: "tt", "it", "rm", "cal", "sf", "bf",
            "default", "regular", "normal", "bb", "frak", "scr".  "default"
            and "regular" are synonyms and use the non-math font.
            "normal" denotes the normal math font.
        font_class : str
            One of the TeX font names (as for *font*), but **not** "bb",
            "frak", or "scr".  This is used to combine two font classes.  The
            only supported combination currently is ``get_metrics("frak", "bf",
            ...)``.
        sym : str
            A symbol in raw TeX form, e.g., "1", "x", or "\sigma".
        fontsize : float
            Font size in points.
        dpi : float
            Rendering dots-per-inch.

        Returns
        -------
        FontMetrics
        """
        info = self._get_info(font, font_class, sym, fontsize, dpi)
        return info.metrics

    def render_glyph(self, output: Output, ox: float, oy: float, font: str,
                     font_class: str, sym: str, fontsize: float, dpi: float) -> None:
        """
        At position (*ox*, *oy*), draw the glyph specified by the remaining
        parameters (see `get_metrics` for their detailed description).
        """
        info = self._get_info(font, font_class, sym, fontsize, dpi)
        output.glyphs.append((ox, oy, info))

    def render_rect_filled(self, output: Output,
                           x: float, y: float, w: float, h: float) -> None:
        """
        Draw a filled rectangle at (*x*, *y*) with size (*w*, *h*).
        """
        output.rects.append((x, y, w, h))

    def get_axis_height(self, font: str, fontsize: float, dpi: float) -> float:
        """
        Get the axis height for the given *font* and *fontsize*.
        """
        raise NotImplementedError()

    def get_quad(self, font: str, fontsize: float, dpi: float) -> float:
        """
        Get the size of a quad for the given *font* and *fontsize*.
        """
        raise NotImplementedError()

    def get_xheight(self, font: str, fontsize: float, dpi: float) -> float:
        """
        Get the xheight for the given *font* and *fontsize*.
        """
        raise NotImplementedError()

    def get_underline_thickness(self, font: str, fontsize: float, dpi: float) -> float:
        """
        Get the line thickness that matches the given font.  Used as a
        base unit for drawing lines such as in a fraction or radical.
        """
        raise NotImplementedError()

    def get_sized_alternatives_for_symbol(self, fontname: str,
                                          sym: str) -> list[tuple[str, str]]:
        """
        Override if your font provides multiple sizes of the same
        symbol.  Should return a list of symbols matching *sym* in
        various sizes.  The expression renderer will select the most
        appropriate size for a given situation from this list.
        """
        return [(fontname, sym)]

    def get_font_constants(self) -> type[FontConstantsBase]:
        return FontConstantsBase


class TruetypeFonts(Fonts, metaclass=abc.ABCMeta):
    """
    A generic base class for all font setups that use Truetype fonts
    (through FT2Font).
    """

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        super().__init__(default_font_prop, load_glyph_flags)
        # Per-instance cache.
        self._get_info = functools.cache(self._get_info)  # type: ignore[method-assign]
        self._fonts = {}
        self.fontmap: dict[str, str] = {}

        filename = findfont(self.default_font_prop)
        default_font = get_font(filename)
        self._fonts['default'] = default_font
        self._fonts['regular'] = default_font

    def _get_font(self, font: str) -> FT2Font:
        basename = self.fontmap.get(font, font)
        cached_font = self._fonts.get(basename)
        if cached_font is None and os.path.exists(basename):
            cached_font = get_font(basename)
            self._fonts[basename] = cached_font
            self._fonts[cached_font.postscript_name] = cached_font
            self._fonts[cached_font.postscript_name.lower()] = cached_font
        return T.cast(FT2Font, cached_font)  # FIXME: Not sure this is guaranteed.

    def _get_offset(self, font: FT2Font, glyph: Glyph, fontsize: float,
                    dpi: float) -> float:
        if font.postscript_name == 'Cmex10':
            return (glyph.height / 64 / 2) + (fontsize/3 * dpi/72)
        return 0.

    def _get_glyph(self, fontname: str, font_class: str,
                   sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        raise NotImplementedError

    # The return value of _get_info is cached per-instance.
    def _get_info(self, fontname: str, font_class: str, sym: str, fontsize: float,
                  dpi: float) -> FontInfo:
        font, num, slanted = self._get_glyph(fontname, font_class, sym)
        font.set_size(fontsize, dpi)
        glyph_index = font.get_char_index(num)
        glyph = font.load_glyph(glyph_index, flags=self.load_glyph_flags)

        xmin, ymin, xmax, ymax = (val / 64 for val in glyph.bbox)
        offset = self._get_offset(font, glyph, fontsize, dpi)
        metrics = FontMetrics(
            advance=glyph.linearHoriAdvance / 65536,
            height=glyph.height / 64,
            width=glyph.width / 64,
            xmin=xmin,
            xmax=xmax,
            ymin=ymin + offset,
            ymax=ymax + offset,
            # iceberg is the equivalent of TeX's "height"
            iceberg=glyph.horiBearingY / 64 + offset,
            slanted=slanted
        )

        return FontInfo(
            font=font,
            fontsize=fontsize,
            postscript_name=font.postscript_name,
            metrics=metrics,
            num=num,
            glyph_index=glyph_index,
            glyph=glyph,
            offset=offset
        )

    def get_axis_height(self, fontname: str, fontsize: float, dpi: float) -> float:
        consts = self.get_font_constants()
        if consts.axis_height is not None:
            return consts.axis_height * fontsize * dpi / 72
        else:
            # The fraction line (if present) must be aligned with the minus sign.
            # Therefore, the height of the latter from the baseline is the axis height.
            metrics = self.get_metrics(
                fontname, mpl.rcParams['mathtext.default'], '\u2212', fontsize, dpi)
            return (metrics.ymax + metrics.ymin) / 2

    def get_quad(self, fontname: str, fontsize: float, dpi: float) -> float:
        consts = self.get_font_constants()
        if consts.quad is not None:
            return consts.quad * fontsize * dpi / 72
        else:
            # With no other option, we measure the size of an 'm'.
            metrics = self.get_metrics(
                fontname, mpl.rcParams['mathtext.default'], 'm', fontsize, dpi)
            return metrics.advance

    def get_xheight(self, fontname: str, fontsize: float, dpi: float) -> float:
        consts = self.get_font_constants()
        if consts.x_height is not None:
            return consts.x_height * fontsize * dpi / 72
        else:
            # Some fonts report the wrong x-height, while some don't store it, so
            # we do a poor man's x-height.
            metrics = self.get_metrics(
                fontname, mpl.rcParams['mathtext.default'], 'x', fontsize, dpi)
            return metrics.iceberg

    def get_underline_thickness(self, font: str, fontsize: float, dpi: float) -> float:
        # This function used to grab underline thickness from the font
        # metrics, but that information is just too un-reliable, so it
        # is now hardcoded.
        return ((0.75 / 12) * fontsize * dpi) / 72

    def get_kern(self, font1: str, fontclass1: str, sym1: str, fontsize1: float,
                 font2: str, fontclass2: str, sym2: str, fontsize2: float,
                 dpi: float) -> float:
        if font1 == font2 and fontsize1 == fontsize2:
            info1 = self._get_info(font1, fontclass1, sym1, fontsize1, dpi)
            info2 = self._get_info(font2, fontclass2, sym2, fontsize2, dpi)
            font = info1.font
            return font.get_kerning(info1.glyph_index, info2.glyph_index,
                                    Kerning.DEFAULT) / 64
        return super().get_kern(font1, fontclass1, sym1, fontsize1,
                                font2, fontclass2, sym2, fontsize2, dpi)


class BakomaFonts(TruetypeFonts):
    """
    Use the Bakoma TrueType fonts for rendering.

    Symbols are strewn about a number of font files, each of which has
    its own proprietary 8-bit encoding.
    """
    _fontmap = {
        'normal': 'cmmi10',
        'cal': 'cmsy10',
        'rm':  'cmr10',
        'tt':  'cmtt10',
        'it':  'cmti10',
        'bf':  'cmb10',
        'sf':  'cmss10',
        'ex':  'cmex10',
    }

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self._stix_fallback = StixFonts(default_font_prop, load_glyph_flags)

        super().__init__(default_font_prop, load_glyph_flags)
        for key, val in self._fontmap.items():
            fullpath = findfont(val)
            self.fontmap[key] = fullpath
            self.fontmap[val] = fullpath

    _slanted_symbols = set(r"\int \oint".split())

    def _get_glyph(self, fontname: str, font_class: str,
                   sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        font = None

        if fontname in self.fontmap and sym in latex_to_bakoma:
            basename, num = latex_to_bakoma[sym]
            slanted = (basename in ("cmmi10", "cmti10")) or sym in self._slanted_symbols
            font = self._get_font(basename)
        elif len(sym) == 1:
            slanted = (fontname in ("it", "normal"))
            if fontname == "normal" and sym.isdigit():
                # use digits from cmr (roman alphabet) instead of cmm (math alphabet),
                # same as LaTeX does.
                fontname = "rm"
                slanted = False
            font = self._get_font(fontname)
            if font is not None:
                num = ord(sym)
        if font is not None and font.get_char_index(num) != 0:
            return font, num, slanted
        else:
            return self._stix_fallback._get_glyph(fontname, font_class, sym)

    # The Bakoma fonts contain many pre-sized alternatives for the delimiters. The
    # Auto(Height|Width)Char classes will use these alternatives and select the best
    # (closest sized) glyph.
    _latex_sizes = ('big', 'Big', 'bigg', 'Bigg')
    _size_alternatives = {
        '(': [('rm', '('), *[('ex', fr'\__parenleft{s}__') for s in _latex_sizes]],
        ')': [('rm', ')'), *[('ex', fr'\__parenright{s}__') for s in _latex_sizes]],
        '{': [('ex', fr'\__braceleft{s}__') for s in _latex_sizes],
        '}': [('ex', fr'\__braceright{s}__') for s in _latex_sizes],
        '[': [('rm', '['), *[('ex', fr'\__bracketleft{s}__') for s in _latex_sizes]],
        ']': [('rm', ']'), *[('ex', fr'\__bracketright{s}__') for s in _latex_sizes]],
        '<': [('cal', r'\__angbracketleft__'),
              *[('ex', fr'\__angbracketleft{s}__') for s in _latex_sizes]],
        '>': [('cal', r'\__angbracketright__'),
              *[('ex', fr'\__angbracketright{s}__') for s in _latex_sizes]],
        r'\lfloor': [('ex', fr'\__floorleft{s}__') for s in _latex_sizes],
        r'\rfloor': [('ex', fr'\__floorright{s}__') for s in _latex_sizes],
        r'\lceil': [('ex', fr'\__ceilingleft{s}__') for s in _latex_sizes],
        r'\rceil': [('ex', fr'\__ceilingright{s}__') for s in _latex_sizes],
        r'\__sqrt__': [('ex', fr'\__radical{s}__') for s in _latex_sizes],
        r'\backslash': [('ex', fr'\__backslash{s}__') for s in _latex_sizes],
        r'/': [('rm', '/'), *[('ex', fr'\__slash{s}__') for s in _latex_sizes]],
        r'\widehat': [('rm', '\x5e'), ('ex', r'\__hatwide__'), ('ex', r'\__hatwider__'),
                      ('ex', r'\__hatwidest__')],
        r'\widetilde': [('rm', '\x7e'), ('ex', r'\__tildewide__'),
                        ('ex', r'\__tildewider__'), ('ex', r'\__tildewidest__')],
    }

    for alias, target in [(r'\leftparen', '('), (r'\rightparen', ')'),
                          (r'\leftbrace', '{'), (r'\rightbrace', '}'),
                          (r'\leftbracket', '['), (r'\rightbracket', ']'),
                          (r'\langle', '<'), (r'\rangle', '>'),
                          (r'\{', '{'), (r'\}', '}'),
                          (r'\[', '['), (r'\]', ']')]:
        _size_alternatives[alias] = _size_alternatives[target]

    def get_sized_alternatives_for_symbol(self, fontname: str,
                                          sym: str) -> list[tuple[str, str]]:
        return self._size_alternatives.get(sym, [(fontname, sym)])

    def get_font_constants(self) -> type[FontConstantsBase]:
        return ComputerModernFontConstants


class UnicodeFonts(TruetypeFonts):
    """
    An abstract base class for handling Unicode fonts.

    While some reasonably complete Unicode fonts (such as DejaVu) may
    work in some situations, the only Unicode font I'm aware of with a
    complete set of math symbols is STIX.

    This class will "fallback" on the Bakoma fonts when a required
    symbol cannot be found in the font.
    """

    # Some glyphs are not present in the `cmr10` font, and must be brought in
    # from `cmsy10`. Map the Unicode indices of those glyphs to the indices at
    # which they are found in `cmsy10`.
    _cmr10_substitutions: dict[CharacterCodeType, CharacterCodeType] = {
        0x00D7: 0x00A3,  # Multiplication sign.
        0x2212: 0x00A1,  # Minus sign.
    }

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        # This must come first so the backend's owner is set correctly
        fallback_rc = mpl.rcParams['mathtext.fallback']
        font_cls: type[TruetypeFonts] | None = {
            'stix': StixFonts,
            'stixsans': StixSansFonts,
            'cm': BakomaFonts
        }.get(fallback_rc)
        self._fallback_font = (font_cls(default_font_prop, load_glyph_flags)
                               if font_cls else None)

        super().__init__(default_font_prop, load_glyph_flags)
        for texfont in "cal rm tt it bf sf bfit".split():
            prop = mpl.rcParams['mathtext.' + texfont]  # type: ignore[index]
            font = findfont(prop)
            self.fontmap[texfont] = font
        prop = FontProperties('cmex10')
        font = findfont(prop)
        self.fontmap['ex'] = font

        # include STIX sized alternatives for glyphs if fallback is STIX
        if isinstance(self._fallback_font, StixFonts):
            stixsizedaltfonts = {
                '0': 'STIXGeneral',
                '1': 'STIXSizeOneSym',
                '2': 'STIXSizeTwoSym',
                '3': 'STIXSizeThreeSym',
                '4': 'STIXSizeFourSym',
                '5': 'STIXSizeFiveSym',
            }

            for size, name in stixsizedaltfonts.items():
                fullpath = findfont(name)
                self.fontmap[size] = fullpath
                self.fontmap[name] = fullpath

    _slanted_symbols = set(r"\int \oint".split())

    def _map_virtual_font(self, fontname: str, font_class: str,
                          uniindex: CharacterCodeType) -> tuple[str, CharacterCodeType]:
        return fontname, uniindex

    def _get_glyph(self, fontname: str, font_class: str,
                   sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        try:
            uniindex = get_unicode_index(sym)
            found_symbol = True
        except ValueError:
            uniindex = ord('?')
            found_symbol = False
            _log.warning("No TeX to Unicode mapping for %a.", sym)

        fontname, uniindex = self._map_virtual_font(fontname, font_class, uniindex)

        new_fontname = fontname

        # Only characters in the "Letter" class should be italicized in 'it'
        # mode.  Greek capital letters should be Roman.
        if found_symbol:
            if fontname == 'normal' and uniindex < 0x10000:
                # normal mathematics font
                char = chr(uniindex)
                if (unicodedata.category(char)[0] != "L"
                        or unicodedata.name(char).startswith("GREEK CAPITAL")):
                    new_fontname = 'rm'
                else:
                    new_fontname = 'it'

            slanted = (new_fontname == 'it') or sym in self._slanted_symbols
            found_symbol = False
            font = self._get_font(new_fontname)
            if font is not None:
                if (uniindex in self._cmr10_substitutions
                        and font.family_name == "cmr10"):
                    font = get_font(
                        cbook._get_data_path("fonts/ttf/cmsy10.ttf"))
                    uniindex = self._cmr10_substitutions[uniindex]
                glyphindex = font.get_char_index(uniindex)
                if glyphindex != 0:
                    found_symbol = True

        if not found_symbol:
            if self._fallback_font:
                if (fontname in ('it', 'regular', 'normal')
                        and isinstance(self._fallback_font, StixFonts)):
                    fontname = 'rm'

                g = self._fallback_font._get_glyph(fontname, font_class, sym)
                family = g[0].family_name
                if family in BakomaFonts._fontmap.values():
                    family = "Computer Modern"
                _log.info("Substituting symbol %s from %s", sym, family)
                return g

            else:
                if (fontname in ('it', 'regular', 'normal')
                        and isinstance(self, StixFonts)):
                    return self._get_glyph('rm', font_class, sym)
                _log.warning("Font %r does not have a glyph for %a [U+%x], "
                             "substituting with a dummy symbol.",
                             new_fontname, sym, uniindex)
                font = self._get_font('rm')
                uniindex = 0xA4  # currency char, for lack of anything better
                slanted = False

        return font, uniindex, slanted

    def get_sized_alternatives_for_symbol(self, fontname: str,
                                          sym: str) -> list[tuple[str, str]]:
        if self._fallback_font:
            return self._fallback_font.get_sized_alternatives_for_symbol(fontname, sym)
        return [(fontname, sym)]


class DejaVuFonts(UnicodeFonts, metaclass=abc.ABCMeta):
    _fontmap: dict[str, str] = {}

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        # This must come first so the backend's owner is set correctly
        if isinstance(self, DejaVuSerifFonts):
            self._fallback_font = StixFonts(default_font_prop, load_glyph_flags)
        else:
            self._fallback_font = StixSansFonts(default_font_prop, load_glyph_flags)
        self.bakoma = BakomaFonts(default_font_prop, load_glyph_flags)
        TruetypeFonts.__init__(self, default_font_prop, load_glyph_flags)
        # Include Stix sized alternatives for glyphs
        self._fontmap.update({
            '1': 'STIXSizeOneSym',
            '2': 'STIXSizeTwoSym',
            '3': 'STIXSizeThreeSym',
            '4': 'STIXSizeFourSym',
            '5': 'STIXSizeFiveSym',
        })
        for key, name in self._fontmap.items():
            fullpath = findfont(name)
            self.fontmap[key] = fullpath
            self.fontmap[name] = fullpath

    def _get_glyph(self, fontname: str, font_class: str,
                   sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        # Override prime symbol to use Bakoma.
        if sym == r'\prime':
            return self.bakoma._get_glyph(fontname, font_class, sym)
        else:
            # check whether the glyph is available in the display font
            uniindex = get_unicode_index(sym)
            font = self._get_font('ex')
            if font is not None:
                glyphindex = font.get_char_index(uniindex)
                if glyphindex != 0:
                    return super()._get_glyph('ex', font_class, sym)
            # otherwise return regular glyph
            return super()._get_glyph(fontname, font_class, sym)


class DejaVuSerifFonts(DejaVuFonts):
    """
    A font handling class for the DejaVu Serif fonts

    If a glyph is not found it will fallback to Stix Serif
    """
    _fontmap = {
        'rm': 'DejaVu Serif',
        'it': 'DejaVu Serif:italic',
        'bf': 'DejaVu Serif:weight=bold',
        'bfit': 'DejaVu Serif:italic:bold',
        'sf': 'DejaVu Sans',
        'tt': 'DejaVu Sans Mono',
        'ex': 'DejaVu Serif Display',
        '0': 'DejaVu Serif',
    }

    def get_font_constants(self) -> type[FontConstantsBase]:
        return DejaVuSerifFontConstants


class DejaVuSansFonts(DejaVuFonts):
    """
    A font handling class for the DejaVu Sans fonts

    If a glyph is not found it will fallback to Stix Sans
    """
    _fontmap = {
        'rm': 'DejaVu Sans',
        'it': 'DejaVu Sans:italic',
        'bf': 'DejaVu Sans:weight=bold',
        'bfit': 'DejaVu Sans:italic:bold',
        'sf': 'DejaVu Sans',
        'tt': 'DejaVu Sans Mono',
        'ex': 'DejaVu Sans Display',
        '0': 'DejaVu Sans',
    }

    def get_font_constants(self) -> type[FontConstantsBase]:
        return DejaVuSansFontConstants


class StixFonts(UnicodeFonts):
    """
    A font handling class for the STIX fonts.

    In addition to what UnicodeFonts provides, this class:

    - supports "virtual fonts" which are complete alpha numeric
      character sets with different font styles at special Unicode
      code points, such as "Blackboard".

    - handles sized alternative characters for the STIXSizeX fonts.
    """
    _fontmap = {
        'rm': 'STIXGeneral',
        'it': 'STIXGeneral:italic',
        'bf': 'STIXGeneral:weight=bold',
        'bfit': 'STIXGeneral:italic:bold',
        'nonunirm': 'STIXNonUnicode',
        'nonuniit': 'STIXNonUnicode:italic',
        'nonunibf': 'STIXNonUnicode:weight=bold',
        '0': 'STIXGeneral',
        '1': 'STIXSizeOneSym',
        '2': 'STIXSizeTwoSym',
        '3': 'STIXSizeThreeSym',
        '4': 'STIXSizeFourSym',
        '5': 'STIXSizeFiveSym',
    }
    _fallback_font = None
    _sans = False

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        TruetypeFonts.__init__(self, default_font_prop, load_glyph_flags)
        for key, name in self._fontmap.items():
            fullpath = findfont(name)
            self.fontmap[key] = fullpath
            self.fontmap[name] = fullpath

    def _map_virtual_font(self, fontname: str, font_class: str,
                          uniindex: CharacterCodeType) -> tuple[str, CharacterCodeType]:
        # Handle these "fonts" that are actually embedded in
        # other fonts.
        font_mapping = stix_virtual_fonts.get(fontname)
        if (self._sans and font_mapping is None
                and fontname not in ('regular', 'default')):
            font_mapping = stix_virtual_fonts['sf']
            doing_sans_conversion = True
        else:
            doing_sans_conversion = False

        if isinstance(font_mapping, dict):
            try:
                mapping = font_mapping[font_class]
            except KeyError:
                mapping = font_mapping['rm']
        elif isinstance(font_mapping, list):
            mapping = font_mapping
        else:
            mapping = None

        if mapping is not None:
            # Binary search for the source glyph
            lo = 0
            hi = len(mapping)
            while lo < hi:
                mid = (lo+hi)//2
                range = mapping[mid]
                if uniindex < range[0]:
                    hi = mid
                elif uniindex <= range[1]:
                    break
                else:
                    lo = mid + 1

            if range[0] <= uniindex <= range[1]:
                uniindex = uniindex - range[0] + range[3]
                fontname = range[2]
            elif not doing_sans_conversion:
                # This will generate a dummy character
                uniindex = 0x1
                fontname = mpl.rcParams['mathtext.default']

        # Fix some incorrect glyphs.
        if fontname in ('rm', 'it', 'normal'):
            uniindex = stix_glyph_fixes.get(uniindex, uniindex)

        # Handle private use area glyphs
        if fontname in ('it', 'rm', 'bf', 'bfit') and 0xe000 <= uniindex <= 0xf8ff:
            fontname = 'nonuni' + fontname

        return fontname, uniindex

    @functools.cache
    def get_sized_alternatives_for_symbol(self, fontname: str,
                                          sym: str) -> list[tuple[str, str]]:
        fixes = {
            '\\{': '{', '\\}': '}', '\\[': '[', '\\]': ']',
            '<': '\N{MATHEMATICAL LEFT ANGLE BRACKET}',
            '>': '\N{MATHEMATICAL RIGHT ANGLE BRACKET}',
        }
        sym = fixes.get(sym, sym)
        try:
            uniindex = get_unicode_index(sym)
        except ValueError:
            return [(fontname, sym)]
        alternatives = [(str(i), chr(uniindex)) for i in range(6)
                        if self._get_font(str(i)).get_char_index(uniindex) != 0]
        # The largest size of the radical symbol in STIX has incorrect
        # metrics that cause it to be disconnected from the stem.
        if sym == r'\__sqrt__':
            alternatives = alternatives[:-1]
        return alternatives

    def get_font_constants(self) -> type[FontConstantsBase]:
        if self._sans:
            return STIXSansFontConstants
        else:
            return STIXFontConstants


class StixSansFonts(StixFonts):
    """
    A font handling class for the STIX fonts (that uses sans-serif
    characters by default).
    """
    _sans = True


##############################################################################
# TeX-LIKE BOX MODEL

# The following is based directly on the document 'woven' from the
# TeX82 source code.  This information is also available in printed
# form:
#
#    Knuth, Donald E.. 1986.  Computers and Typesetting, Volume B:
#    TeX: The Program.  Addison-Wesley Professional.
#
# The most relevant "chapters" are:
#    Data structures for boxes and their friends
#    Shipping pages out (ship())
#    Packaging (hpack() and vpack())
#    Data structures for math mode
#    Subroutines for math mode
#    Typesetting math formulas
#
# Many of the docstrings below refer to a numbered "node" in that
# book, e.g., node123
#
# Note that (as TeX) y increases downward, unlike many other parts of
# matplotlib.

# How much text shrinks when going to the next-smallest level.
SHRINK_FACTOR   = 0.7
# The number of different sizes of chars to use, beyond which they will not
# get any smaller
NUM_SIZE_LEVELS = 6


class FontConstantsBase:
    """
    A set of constants that controls how certain things, such as sub-
    and superscripts are laid out.  These are all metrics that can't
    be reliably retrieved from the font metrics in the font itself.
    """
    # Percentage of x-height of additional horiz. space after sub/superscripts
    script_space: T.ClassVar[float] = 0.05

    # Percentage of x-height that superscripts drop below the top of large box
    supdrop: T.ClassVar[float] = 0.4

    # Percentage of x-height that subscripts drop below the bottom of large box
    subdrop: T.ClassVar[float] = 0.4

    # Percentage of x-height that superscripts are raised from the baseline
    sup1: T.ClassVar[float] = 0.7

    # Percentage of x-height that subscripts drop below the baseline
    sub1: T.ClassVar[float] = 0.3

    # Percentage of x-height that subscripts drop below the baseline when a
    # superscript is present
    sub2: T.ClassVar[float] = 0.5

    # Percentage of x-height that sub/superscripts are offset relative to the
    # nucleus edge for non-slanted nuclei
    delta: T.ClassVar[float] = 0.025

    # Additional percentage of last character height above 2/3 of the
    # x-height that superscripts are offset relative to the subscript
    # for slanted nuclei
    delta_slanted: T.ClassVar[float] = 0.2

    # Percentage of x-height that superscripts and subscripts are offset for
    # integrals
    delta_integral: T.ClassVar[float] = 0.1

    # Percentage of x-height the numerator is shifted up in display style.
    num1: T.ClassVar[float] = 1.4

    # Percentage of x-height the numerator is shifted up in text, script and
    # scriptscript styles if there is a fraction line.
    num2: T.ClassVar[float] = 1.5

    # Percentage of x-height the numerator is shifted up in text, script and
    # scriptscript styles if there is no fraction line.
    num3: T.ClassVar[float] = 1.3

    # Percentage of x-height the denominator is shifted down in display style.
    denom1: T.ClassVar[float] = 1.3

    # Percentage of x-height the denominator is shifted down in text, script
    # and scriptscript styles.
    denom2: T.ClassVar[float] = 1.1

    # The height of a horizontal reference line used for positioning elements in a
    # formula, similar to a baseline, as a multiple of design size.
    axis_height: T.ClassVar[float | None] = None

    # The size of a quad space in LaTeX, as a multiple of design size.
    quad: T.ClassVar[float | None] = None

    # The size of x-height in font design units (i.e., divided by units-per-em). If not
    # provided, then this will be measured from the font itself.
    x_height: T.ClassVar[float | None] = None


class ComputerModernFontConstants(FontConstantsBase):
    # Previously, the x-height of Computer Modern was obtained from the font
    # table. However, that x-height was greater than the actual (rendered)
    # x-height by a factor of 1.771484375 (at font size 12, DPI 100 and hinting
    # type 32). Now that we're using the rendered x-height, some font constants
    # have been increased by the same factor to compensate.
    script_space = 0.132861328125
    delta = 0.132861328125
    delta_slanted = 0.3
    delta_integral = 0.3
    _x_height = 451470
    # These all come from the cmsy10.tfm metrics, divided by the design xheight from
    # there, since we multiply these values by the scaled xheight later.
    supdrop = 404864 / _x_height
    subdrop = 52429 / _x_height
    sup1 = 432949 / _x_height
    sub1 = 157286 / _x_height
    sub2 = 259226 / _x_height
    num1 = 709370 / _x_height
    num2 = 412858 / _x_height
    num3 = 465286 / _x_height
    denom1 = 719272 / _x_height
    denom2 = 361592 / _x_height
    # These come from the cmsy10.tfm metrics, scaled so they are in multiples of design
    # size.
    axis_height = 262144 / 2**20
    quad = 1048579 / 2**20
    x_height = _x_height / 2**20


class STIXFontConstants(FontConstantsBase):
    script_space = 0.1
    delta = 0.05
    delta_slanted = 0.3
    delta_integral = 0.3
    _x_height = 450
    x_height = _x_height / 1000
    # These values are extracted from the TeX table of STIXGeneral.ttf using FontForge,
    # and then divided by design xheight, since we multiply these values by the scaled
    # xheight later.
    supdrop = 386 / _x_height
    subdrop = 50.0002 / _x_height
    sup1 = 413 / _x_height
    sub1 = 150 / _x_height
    sub2 = 309 / _x_height
    num1 = 747 / _x_height
    num2 = 424 / _x_height
    num3 = 474 / _x_height
    denom1 = 756 / _x_height
    denom2 = 375 / _x_height
    # These come from the same TeX table, scaled by Em size so they are in multiples of
    # design size.
    axis_height = 250 / 1000
    quad = 1000 / 1000


class STIXSansFontConstants(STIXFontConstants):
    script_space = 0.05
    delta_slanted = 0.6
    delta_integral = 0.3


class DejaVuSerifFontConstants(FontConstantsBase):
    _x_height = 1063
    x_height = _x_height / 2048
    # These values are extracted from the TeX table of DejaVuSerif.ttf using FontForge,
    # and then divided by design xheight, since we multiply these values by the scaled
    # xheight later.
    supdrop = 790.527 / _x_height
    subdrop = 102.4 / _x_height
    sup1 = 845.824 / _x_height
    sub1 = 307.199 / _x_height
    sub2 = 632.832 / _x_height
    num1 = 1529.86 / _x_height
    num2 = 868.352 / _x_height
    num3 = 970.752 / _x_height
    denom1 = 1548.29 / _x_height
    denom2 = 768 / _x_height
    # These come from the same TeX table, scaled by Em size so they are in multiples of
    # design size.
    axis_height = 512 / 2048


class DejaVuSansFontConstants(FontConstantsBase):
    _x_height = 1120
    x_height = _x_height / 2048
    # These values are extracted from the TeX table of DejaVuSans.ttf using FontForge,
    # and then divided by design xheight, since we multiply these values by the scaled
    # xheight later.
    supdrop = 790.527 / _x_height
    subdrop = 102.4 / _x_height
    sup1 = 845.824 / _x_height
    sub1 = 307.199 / _x_height
    sub2 = 632.832 / _x_height
    num1 = 1529.86 / _x_height
    num2 = 868.352 / _x_height
    num3 = 970.752 / _x_height
    denom1 = 1548.29 / _x_height
    denom2 = 768 / _x_height
    # These come from the same TeX table, scaled by Em size so they are in multiples of
    # design size.
    axis_height = 512 / 2048


# Each of matplotlib's font classes names the :rc:`mathtext.fontset` it
# stands for (``_fontset``), whose fonts pltrs (Rust) typesets with.
Fonts._fontset = None
BakomaFonts._fontset = 'cm'
UnicodeFonts._fontset = 'custom'
DejaVuSerifFonts._fontset = 'dejavuserif'
DejaVuSansFonts._fontset = 'dejavusans'
StixFonts._fontset = 'stix'
StixSansFonts._fontset = 'stixsans'


class Parser:
    """
    A pyparsing-based parser for strings containing math expressions in
    matplotlib; here its symbol tables, pltrs (Rust) doing the parsing.
    """

    _accent_map = dict(_mathtext.accent_map())

    _wide_accents = set(_mathtext.wide_accents())

    def parse(self, s, fonts_object, fontsize, dpi):
        """
        Parse expression *s* using the given *fonts_object* for
        output, at the given *fontsize* and *dpi*.

        Returns the parse tree of `Node` instances.
        """
        from .font_manager import FontManager
        prop = fonts_object.default_font_prop.copy()
        prop.set_size(fontsize)
        if fonts_object._fontset is not None:
            prop.set_math_fontfamily(fonts_object._fontset)
        return _node(_mathtext.parse_box_tree(
            s, float(dpi), FontManager._props(prop)))


def _node(native):
    """The `Node` of the box tree node *native* (``(class name, payload)``,
    see ``_native.mathtext.parse_box_tree``)."""
    name, payload = native
    cls = _NODE_CLASSES[name]
    if issubclass(cls, List):
        width, height, depth, shift_amount, children = payload
        return cls(width, height, depth, shift_amount,
                   [_node(child) for child in children])
    if cls is Char:
        return Char(payload)
    if cls is Kern:
        return Kern(payload)
    return cls()


class Node:
    """A node in the TeX box model."""

    def __repr__(self):
        return type(self).__name__


class Box(Node):
    """A node with a physical location."""


class Vbox(Box):
    """A box with only height (zero width)."""


class Hbox(Box):
    """A box with only width (zero height and depth)."""


class Char(Node):
    """A single character."""

    def __init__(self, c):
        self.c = c

    def __repr__(self):
        return '`%s`' % self.c


class List(Box):
    """A list of nodes (either horizontal or vertical)."""

    def __init__(self, width, height, depth, shift_amount, children):
        self.width = width
        self.height = height
        self.depth = depth
        self.shift_amount = shift_amount
        self.children = children

    def __repr__(self):
        return "{}<w={:.02f} h={:.02f} d={:.02f} s={:.02f}>[{}]".format(
            super().__repr__(),
            self.width, self.height,
            self.depth, self.shift_amount,
            "\n" + textwrap.indent(
                "\n".join(map("{!r},".format, self.children)),
                "  ") + "\n"
            if self.children else ""
        )


class Hlist(List):
    """A horizontal list of boxes."""


class Vlist(List):
    """A vertical list of boxes."""


class Rule(Box):
    """A solid black rectangle."""


class Hrule(Rule):
    """Convenience class to create a horizontal rule."""


class Glue(Node):
    """Most of the information in this object is stored in the underlying
    ``_GlueSpec`` class, which is shared between multiple glue objects."""


class HCentered(Hlist):
    """A convenience class to create an `Hlist` whose contents are
    centered within its enclosing box."""


class Kern(Node):
    """A `Kern` node has a width field to specify a (normally negative)
    amount of spacing."""

    def __init__(self, width):
        self.width = width

    def __repr__(self):
        return "k%.02f" % self.width


class AutoHeightChar(Hlist):
    """A character as close to the given height and depth as possible."""


class AutoWidthChar(Hlist):
    """A character as close to the given width as possible."""


_NODE_CLASSES = {cls.__name__: cls for cls in [
    Box, Hbox, Vbox, Char, Kern, Glue, Hrule,
    Hlist, Vlist, HCentered, AutoHeightChar, AutoWidthChar]}
