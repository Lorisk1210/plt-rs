"""
Implementation details for :mod:`.mathtext` (matplotlib's ``_mathtext``).

pltrs (Rust) parses and typesets mathtext (``pltrs_text::mathtext``, a port
of matplotlib's parser, font sets and TeX box model). This module keeps
matplotlib's names for what the Python side sees of it: the parse results,
the font sets (which select the fonts pltrs (Rust) typesets with), the
parser's symbol tables and `get_unicode_index`.
"""

from typing import NamedTuple

from ._native import mathtext as _mathtext


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


VectorParse.__module__ = "matplotlib.mathtext"


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


RasterParse.__module__ = "matplotlib.mathtext"


def get_unicode_index(symbol):
    r"""
    Return the integer index (from the Unicode table) of *symbol*.

    Parameters
    ----------
    symbol : str
        A single (Unicode) character, a TeX command (e.g. r'\pi') or a Type1
        symbol name (e.g. 'phi').
    """
    return _mathtext.get_unicode_index(symbol)


class Fonts:
    """
    An abstract base class for a system of fonts to use for mathtext.

    The class must be able to take symbol keys and font file names and
    return the character metrics.  It also delegates to a backend class
    to do the actual drawing.

    In pltrs the font set names the fonts pltrs (Rust) typesets with
    (``_fontset``, a value of :rc:`mathtext.fontset`).
    """

    #: The :rc:`mathtext.fontset` this font set stands for.
    _fontset = None

    def __init__(self, default_font_prop, load_glyph_flags):
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

    def get_underline_thickness(self, font, fontsize, dpi):
        """
        Get the line thickness that matches the given font.  Used as a
        base unit for drawing lines such as in a fraction or radical.
        """
        raise NotImplementedError()


class TruetypeFonts(Fonts):
    """
    A generic base class for all font setups that use Truetype fonts
    (through FT2Font).
    """

    def get_underline_thickness(self, font, fontsize, dpi):
        # This function used to grab underline thickness from the font
        # metrics, but that information is just too un-reliable, so it
        # is now hardcoded.
        return ((0.75 / 12.0) * fontsize * dpi) / 72.0


class BakomaFonts(TruetypeFonts):
    """
    Use the Bakoma TrueType fonts for rendering.

    Symbols are strewn about a number of font files, each of which has
    its own proprietary 8-bit encoding.
    """
    _fontset = 'cm'


class UnicodeFonts(TruetypeFonts):
    """
    An abstract base class for handling Unicode fonts.

    While some reasonably complete Unicode fonts (such as DejaVu) may
    work in some situations, the only Unicode font I'm aware of with a
    complete set of math symbols is STIX.

    This class will "fallback" on the Bakoma fonts when a required
    symbol cannot be found in the font.
    """
    _fontset = 'custom'


class DejaVuFonts(UnicodeFonts):
    """The DejaVu fonts, with STIX as fallback for the missing symbols."""


class DejaVuSerifFonts(DejaVuFonts):
    """
    A font handling class for the DejaVu Serif fonts

    If a glyph is not found it will fallback to Stix Serif
    """
    _fontset = 'dejavuserif'


class DejaVuSansFonts(DejaVuFonts):
    """
    A font handling class for the DejaVu Sans fonts

    If a glyph is not found it will fallback to Stix Sans
    """
    _fontset = 'dejavusans'


class StixFonts(UnicodeFonts):
    """
    A font handling class for the STIX fonts.

    In addition to what UnicodeFonts provides, this class:

    - supports "virtual fonts" which are complete alpha numeric
      character sets with different font styles at special Unicode
      code points, such as "Blackboard".

    - handles sized alternative characters for the STIXSizeX fonts.
    """
    _fontset = 'stix'


class StixSansFonts(StixFonts):
    """
    A font handling class for the STIX fonts (using sans-serif
    characters by default).
    """
    _fontset = 'stixsans'


class Parser:
    """
    A pyparsing-based parser for strings containing math expressions in
    matplotlib; here its symbol tables, pltrs (Rust) doing the parsing.
    """

    _accent_map = dict(_mathtext.accent_map())

    _wide_accents = set(_mathtext.wide_accents())
