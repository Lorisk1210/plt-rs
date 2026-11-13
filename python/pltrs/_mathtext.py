"""
Implementation details for :mod:`.mathtext` (matplotlib's ``_mathtext``).

pltrs (Rust) parses and typesets mathtext (``pltrs_text::mathtext``, a port
of matplotlib's parser, font sets and TeX box model). This module keeps
matplotlib's names for what the Python side sees of it: the parse results,
the font sets (which select the fonts pltrs (Rust) typesets with), the box
tree `Parser.parse` returns (the node classes and their reprs are
matplotlib's, from matplotlib 3.11.2, under the Matplotlib License), the
parser's symbol tables and `get_unicode_index`.
"""

import abc
import logging
import textwrap
from typing import NamedTuple

from ._native import mathtext as _mathtext, register as _register

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


def get_unicode_index(symbol):
    r"""
    Return the integer index (from the Unicode table) of *symbol*.

    Parameters
    ----------
    symbol : str
        A single (Unicode) character, a TeX command (e.g. r'\pi') or a Type1
        symbol name (e.g. 'phi').
    """
    if not isinstance(symbol, str):
        # matplotlib takes it as a character, else as the name of a
        # symbol, failing for what is neither.
        try:
            return ord(symbol)
        except TypeError:
            symbol.strip("\\")
    return _mathtext.get_unicode_index(symbol)


class Fonts(abc.ABC):
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


class TruetypeFonts(Fonts, metaclass=abc.ABCMeta):
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
