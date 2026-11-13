# The type stub of matplotlib._mathtext: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from __future__ import annotations
import abc
import copy
import enum
import functools
import itertools
import logging
import math
import os
import re
import types
import unicodedata
import string
import textwrap
import typing as T
from typing import NamedTuple
import numpy as np
from numpy.typing import NDArray
from pyparsing import Empty, Forward, Literal, Group, NotAny, OneOrMore, Optional, ParseBaseException, ParseException, ParseExpression, ParseFatalException, ParserElement, ParseResults, QuotedString, Regex, StringEnd, Token, ZeroOrMore, pyparsing_common, nested_expr, one_of
import matplotlib as mpl
from . import cbook
from ._mathtext_data import latex_to_bakoma, stix_glyph_fixes, stix_virtual_fonts, tex2uni
from .font_manager import FontProperties, findfont, get_font
from .ft2font import FT2Font, Kerning, LoadFlags
if T.TYPE_CHECKING:
    from collections.abc import Iterable
    from .ft2font import CharacterCodeType, Glyph, GlyphIndexType
ParserElement.enable_packrat()
_log = logging.getLogger('matplotlib.mathtext')

def get_unicode_index(symbol: str) -> CharacterCodeType:
    ...

class VectorParse(NamedTuple):
    width: float
    height: float
    depth: float
    glyphs: list[tuple[FT2Font, float, CharacterCodeType, GlyphIndexType, float, float]]
    rects: list[tuple[float, float, float, float]]
VectorParse.__module__ = 'matplotlib.mathtext'

class RasterParse(NamedTuple):
    ox: float
    oy: float
    width: float
    height: float
    depth: float
    image: NDArray[np.uint8]
RasterParse.__module__ = 'matplotlib.mathtext'

class Output:

    def __init__(self, box: Box):
        self.box = box
        self.glyphs: list[tuple[float, float, FontInfo]] = []
        self.rects: list[tuple[float, float, float, float]] = []

    def to_vector(self) -> VectorParse:
        ...

    def to_raster(self, *, antialiased: bool) -> RasterParse:
        ...

class FontMetrics(NamedTuple):
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

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self.default_font_prop = default_font_prop
        self.load_glyph_flags = load_glyph_flags

    def get_kern(self, font1: str, fontclass1: str, sym1: str, fontsize1: float, font2: str, fontclass2: str, sym2: str, fontsize2: float, dpi: float) -> float:
        ...

    def _get_font(self, font: str) -> FT2Font:
        ...

    def _get_info(self, font: str, font_class: str, sym: str, fontsize: float, dpi: float) -> FontInfo:
        ...

    def get_metrics(self, font: str, font_class: str, sym: str, fontsize: float, dpi: float) -> FontMetrics:
        ...

    def render_glyph(self, output: Output, ox: float, oy: float, font: str, font_class: str, sym: str, fontsize: float, dpi: float) -> None:
        ...

    def render_rect_filled(self, output: Output, x: float, y: float, w: float, h: float) -> None:
        ...

    def get_axis_height(self, font: str, fontsize: float, dpi: float) -> float:
        ...

    def get_quad(self, font: str, fontsize: float, dpi: float) -> float:
        ...

    def get_xheight(self, font: str, fontsize: float, dpi: float) -> float:
        ...

    def get_underline_thickness(self, font: str, fontsize: float, dpi: float) -> float:
        ...

    def get_sized_alternatives_for_symbol(self, fontname: str, sym: str) -> list[tuple[str, str]]:
        ...

    def get_font_constants(self) -> type[FontConstantsBase]:
        ...

class TruetypeFonts(Fonts, metaclass=abc.ABCMeta):

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self._get_info = functools.cache(self._get_info)
        self._fonts = {}
        self.fontmap: dict[str, str] = {}
        self._fonts['default'] = default_font
        self._fonts['regular'] = default_font

    def _get_font(self, font: str) -> FT2Font:
        self._fonts[basename] = cached_font
        self._fonts[cached_font.postscript_name] = cached_font
        self._fonts[cached_font.postscript_name.lower()] = cached_font

    def _get_offset(self, font: FT2Font, glyph: Glyph, fontsize: float, dpi: float) -> float:
        ...

    def _get_glyph(self, fontname: str, font_class: str, sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        ...

    def _get_info(self, fontname: str, font_class: str, sym: str, fontsize: float, dpi: float) -> FontInfo:
        ...

    def get_axis_height(self, fontname: str, fontsize: float, dpi: float) -> float:
        ...

    def get_quad(self, fontname: str, fontsize: float, dpi: float) -> float:
        ...

    def get_xheight(self, fontname: str, fontsize: float, dpi: float) -> float:
        ...

    def get_underline_thickness(self, font: str, fontsize: float, dpi: float) -> float:
        ...

    def get_kern(self, font1: str, fontclass1: str, sym1: str, fontsize1: float, font2: str, fontclass2: str, sym2: str, fontsize2: float, dpi: float) -> float:
        ...

class BakomaFonts(TruetypeFonts):
    _fontmap = {'normal': 'cmmi10', 'cal': 'cmsy10', 'rm': 'cmr10', 'tt': 'cmtt10', 'it': 'cmti10', 'bf': 'cmb10', 'sf': 'cmss10', 'ex': 'cmex10'}

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self._stix_fallback = StixFonts(default_font_prop, load_glyph_flags)
        self.fontmap[key] = fullpath
        self.fontmap[val] = fullpath
    _slanted_symbols = set('\\int \\oint'.split())

    def _get_glyph(self, fontname: str, font_class: str, sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        ...
    _latex_sizes = ('big', 'Big', 'bigg', 'Bigg')
    _size_alternatives = {'(': [('rm', '('), *[('ex', f'\\__parenleft{s}__') for s in _latex_sizes]], ')': [('rm', ')'), *[('ex', f'\\__parenright{s}__') for s in _latex_sizes]], '{': [('ex', f'\\__braceleft{s}__') for s in _latex_sizes], '}': [('ex', f'\\__braceright{s}__') for s in _latex_sizes], '[': [('rm', '['), *[('ex', f'\\__bracketleft{s}__') for s in _latex_sizes]], ']': [('rm', ']'), *[('ex', f'\\__bracketright{s}__') for s in _latex_sizes]], '<': [('cal', '\\__angbracketleft__'), *[('ex', f'\\__angbracketleft{s}__') for s in _latex_sizes]], '>': [('cal', '\\__angbracketright__'), *[('ex', f'\\__angbracketright{s}__') for s in _latex_sizes]], '\\lfloor': [('ex', f'\\__floorleft{s}__') for s in _latex_sizes], '\\rfloor': [('ex', f'\\__floorright{s}__') for s in _latex_sizes], '\\lceil': [('ex', f'\\__ceilingleft{s}__') for s in _latex_sizes], '\\rceil': [('ex', f'\\__ceilingright{s}__') for s in _latex_sizes], '\\__sqrt__': [('ex', f'\\__radical{s}__') for s in _latex_sizes], '\\backslash': [('ex', f'\\__backslash{s}__') for s in _latex_sizes], '/': [('rm', '/'), *[('ex', f'\\__slash{s}__') for s in _latex_sizes]], '\\widehat': [('rm', '^'), ('ex', '\\__hatwide__'), ('ex', '\\__hatwider__'), ('ex', '\\__hatwidest__')], '\\widetilde': [('rm', '~'), ('ex', '\\__tildewide__'), ('ex', '\\__tildewider__'), ('ex', '\\__tildewidest__')]}
    for alias, target in [('\\leftparen', '('), ('\\rightparen', ')'), ('\\leftbrace', '{'), ('\\rightbrace', '}'), ('\\leftbracket', '['), ('\\rightbracket', ']'), ('\\langle', '<'), ('\\rangle', '>'), ('\\{', '{'), ('\\}', '}'), ('\\[', '['), ('\\]', ']')]:
        _size_alternatives[alias] = _size_alternatives[target]

    def get_sized_alternatives_for_symbol(self, fontname: str, sym: str) -> list[tuple[str, str]]:
        ...

    def get_font_constants(self) -> type[FontConstantsBase]:
        ...

class UnicodeFonts(TruetypeFonts):
    _cmr10_substitutions: dict[CharacterCodeType, CharacterCodeType] = {215: 163, 8722: 161}

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self._fallback_font = font_cls(default_font_prop, load_glyph_flags) if font_cls else None
        self.fontmap['ex'] = font
        self.fontmap[texfont] = font
        self.fontmap[size] = fullpath
        self.fontmap[name] = fullpath
    _slanted_symbols = set('\\int \\oint'.split())

    def _map_virtual_font(self, fontname: str, font_class: str, uniindex: CharacterCodeType) -> tuple[str, CharacterCodeType]:
        ...

    def _get_glyph(self, fontname: str, font_class: str, sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        ...

    def get_sized_alternatives_for_symbol(self, fontname: str, sym: str) -> list[tuple[str, str]]:
        ...

class DejaVuFonts(UnicodeFonts, metaclass=abc.ABCMeta):
    _fontmap: dict[str, str] = {}

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self.bakoma = BakomaFonts(default_font_prop, load_glyph_flags)
        self._fallback_font = StixFonts(default_font_prop, load_glyph_flags)
        self._fallback_font = StixSansFonts(default_font_prop, load_glyph_flags)
        self.fontmap[key] = fullpath
        self.fontmap[name] = fullpath

    def _get_glyph(self, fontname: str, font_class: str, sym: str) -> tuple[FT2Font, CharacterCodeType, bool]:
        ...

class DejaVuSerifFonts(DejaVuFonts):
    _fontmap = {'rm': 'DejaVu Serif', 'it': 'DejaVu Serif:italic', 'bf': 'DejaVu Serif:weight=bold', 'bfit': 'DejaVu Serif:italic:bold', 'sf': 'DejaVu Sans', 'tt': 'DejaVu Sans Mono', 'ex': 'DejaVu Serif Display', '0': 'DejaVu Serif'}

    def get_font_constants(self) -> type[FontConstantsBase]:
        ...

class DejaVuSansFonts(DejaVuFonts):
    _fontmap = {'rm': 'DejaVu Sans', 'it': 'DejaVu Sans:italic', 'bf': 'DejaVu Sans:weight=bold', 'bfit': 'DejaVu Sans:italic:bold', 'sf': 'DejaVu Sans', 'tt': 'DejaVu Sans Mono', 'ex': 'DejaVu Sans Display', '0': 'DejaVu Sans'}

    def get_font_constants(self) -> type[FontConstantsBase]:
        ...

class StixFonts(UnicodeFonts):
    _fontmap = {'rm': 'STIXGeneral', 'it': 'STIXGeneral:italic', 'bf': 'STIXGeneral:weight=bold', 'bfit': 'STIXGeneral:italic:bold', 'nonunirm': 'STIXNonUnicode', 'nonuniit': 'STIXNonUnicode:italic', 'nonunibf': 'STIXNonUnicode:weight=bold', '0': 'STIXGeneral', '1': 'STIXSizeOneSym', '2': 'STIXSizeTwoSym', '3': 'STIXSizeThreeSym', '4': 'STIXSizeFourSym', '5': 'STIXSizeFiveSym'}
    _fallback_font = None
    _sans = False

    def __init__(self, default_font_prop: FontProperties, load_glyph_flags: LoadFlags):
        self.fontmap[key] = fullpath
        self.fontmap[name] = fullpath

    def _map_virtual_font(self, fontname: str, font_class: str, uniindex: CharacterCodeType) -> tuple[str, CharacterCodeType]:
        ...

    @functools.cache
    def get_sized_alternatives_for_symbol(self, fontname: str, sym: str) -> list[tuple[str, str]]:
        ...

    def get_font_constants(self) -> type[FontConstantsBase]:
        ...

class StixSansFonts(StixFonts):
    _sans = True
SHRINK_FACTOR = 0.7
NUM_SIZE_LEVELS = 6

class FontConstantsBase:
    script_space: T.ClassVar[float] = 0.05
    supdrop: T.ClassVar[float] = 0.4
    subdrop: T.ClassVar[float] = 0.4
    sup1: T.ClassVar[float] = 0.7
    sub1: T.ClassVar[float] = 0.3
    sub2: T.ClassVar[float] = 0.5
    delta: T.ClassVar[float] = 0.025
    delta_slanted: T.ClassVar[float] = 0.2
    delta_integral: T.ClassVar[float] = 0.1
    num1: T.ClassVar[float] = 1.4
    num2: T.ClassVar[float] = 1.5
    num3: T.ClassVar[float] = 1.3
    denom1: T.ClassVar[float] = 1.3
    denom2: T.ClassVar[float] = 1.1
    axis_height: T.ClassVar[float | None] = None
    quad: T.ClassVar[float | None] = None
    x_height: T.ClassVar[float | None] = None

class ComputerModernFontConstants(FontConstantsBase):
    script_space = 0.132861328125
    delta = 0.132861328125
    delta_slanted = 0.3
    delta_integral = 0.3
    _x_height = 451470
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
    axis_height = 262144 / 2 ** 20
    quad = 1048579 / 2 ** 20
    x_height = _x_height / 2 ** 20

class STIXFontConstants(FontConstantsBase):
    script_space = 0.1
    delta = 0.05
    delta_slanted = 0.3
    delta_integral = 0.3
    _x_height = 450
    x_height = _x_height / 1000
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
    axis_height = 250 / 1000
    quad = 1000 / 1000

class STIXSansFontConstants(STIXFontConstants):
    script_space = 0.05
    delta_slanted = 0.6
    delta_integral = 0.3

class DejaVuSerifFontConstants(FontConstantsBase):
    _x_height = 1063
    x_height = _x_height / 2048
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
    axis_height = 512 / 2048

class DejaVuSansFontConstants(FontConstantsBase):
    _x_height = 1120
    x_height = _x_height / 2048
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
    axis_height = 512 / 2048

class Node:

    def __init__(self) -> None:
        self.size = 0

    def __repr__(self) -> str:
        ...

    def get_kerning(self, next: Node | None) -> float:
        ...

    def shrink(self) -> None:
        ...

    def render(self, output: Output, x: float, y: float) -> None:
        ...

    def is_char_node(self) -> bool:
        ...

class Box(Node):

    def __init__(self, width: float, height: float, depth: float) -> None:
        self.width = width
        self.height = height
        self.depth = depth

    def shrink(self) -> None:
        ...

    def render(self, output: Output, x1: float, y1: float, x2: float, y2: float) -> None:
        ...

class Vbox(Box):

    def __init__(self, height: float, depth: float):
        ...

class Hbox(Box):

    def __init__(self, width: float):
        ...

class Char(Node):

    def __init__(self, c: str, state: ParserState):
        self.c = c
        self.fontset = state.fontset
        self.font = state.font
        self.font_class = state.font_class
        self.fontsize = state.fontsize
        self.dpi = state.dpi

    def __repr__(self) -> str:
        ...

    def _update_metrics(self) -> None:
        metrics = self._metrics = self.fontset.get_metrics(self.font, self.font_class, self.c, self.fontsize, self.dpi)
        self.height = metrics.iceberg
        self.depth = -(metrics.iceberg - metrics.height)
        self.width = metrics.advance
        self.width = metrics.width

    def is_slanted(self) -> bool:
        ...

    def get_kerning(self, next: Node | None) -> float:
        ...

    def render(self, output: Output, x: float, y: float) -> None:
        ...

    def shrink(self) -> None:
        ...

class Accent(Char):

    def _update_metrics(self) -> None:
        metrics = self._metrics = self.fontset.get_metrics(self.font, self.font_class, self.c, self.fontsize, self.dpi)
        self.width = metrics.xmax - metrics.xmin
        self.height = metrics.ymax - metrics.ymin
        self.depth = 0

    def shrink(self) -> None:
        ...

    def render(self, output: Output, x: float, y: float) -> None:
        ...

class List(Box):

    def __init__(self, elements: T.Sequence[Node]):
        self.shift_amount = 0.0
        self.children = [*elements]
        self.glue_set = 0.0
        self.glue_sign = 0
        self.glue_order = 0

    def __repr__(self) -> str:
        ...

    def _set_glue(self, x: float, sign: int, totals: list[float], error_type: str) -> None:
        self.glue_order = o = next((i for i in range(len(totals))[::-1] if totals[i] != 0), 0)
        self.glue_sign = sign
        self.glue_set = x / totals[o]
        self.glue_sign = 0
        self.glue_ratio = 0.0

    def shrink(self) -> None:
        ...

class Hlist(List):

    def __init__(self, elements: T.Sequence[Node], w: float=0.0, m: T.Literal['additional', 'exactly']='additional', do_kern: bool=True):
        self.is_phantom = False

    def is_char_node(self) -> bool:
        ...

    def kern(self) -> None:
        self.children = new_children

    def hpack(self, w: float=0.0, m: T.Literal['additional', 'exactly']='additional') -> None:
        self.height = h
        self.depth = d
        self.width = w
        self.glue_sign = 0
        self.glue_order = 0
        self.glue_ratio = 0.0

class Vlist(List):

    def __init__(self, elements: T.Sequence[Node], h: float=0.0, m: T.Literal['additional', 'exactly']='additional'):
        ...

    def is_char_node(self) -> bool:
        ...

    def vpack(self, h: float=0.0, m: T.Literal['additional', 'exactly']='additional', l: float=np.inf) -> None:
        self.width = w
        self.height = h
        self.depth = l
        self.depth = d
        self.glue_sign = 0
        self.glue_order = 0
        self.glue_ratio = 0.0

class Rule(Box):

    def __init__(self, width: float, height: float, depth: float, state: ParserState):
        self.fontset = state.fontset

    def render(self, output: Output, x: float, y: float, w: float, h: float) -> None:
        ...

class Hrule(Rule):

    def __init__(self, state: ParserState, thickness: float | None=None):
        ...

class Vrule(Rule):

    def __init__(self, state: ParserState):
        ...

class _GlueSpec(NamedTuple):
    width: float
    stretch: float
    stretch_order: int
    shrink: float
    shrink_order: int
_GlueSpec._named = {'fil': _GlueSpec(0.0, 1.0, 1, 0.0, 0), 'fill': _GlueSpec(0.0, 1.0, 2, 0.0, 0), 'filll': _GlueSpec(0.0, 1.0, 3, 0.0, 0), 'neg_fil': _GlueSpec(0.0, 0.0, 0, 1.0, 1), 'neg_fill': _GlueSpec(0.0, 0.0, 0, 1.0, 2), 'neg_filll': _GlueSpec(0.0, 0.0, 0, 1.0, 3), 'empty': _GlueSpec(0.0, 0.0, 0, 0.0, 0), 'ss': _GlueSpec(0.0, 1.0, 1, -1.0, 1)}

class Glue(Node):

    def __init__(self, glue_type: _GlueSpec | T.Literal['fil', 'fill', 'filll', 'neg_fil', 'neg_fill', 'neg_filll', 'empty', 'ss']):
        self.glue_spec = glue_spec

    def shrink(self) -> None:
        self.glue_spec = g._replace(width=g.width * SHRINK_FACTOR)

class HCentered(Hlist):

    def __init__(self, elements: list[Node]):
        ...

class VCentered(Vlist):

    def __init__(self, elements: list[Node]):
        ...

class Kern(Node):
    height = 0
    depth = 0

    def __init__(self, width: float):
        self.width = width

    def __repr__(self) -> str:
        ...

    def shrink(self) -> None:
        ...

class AutoHeightChar(Hlist):

    def __init__(self, c: str, height: float, depth: float, state: ParserState, factor: float | None=None):
        self.shift_amount = shift

class AutoWidthChar(Hlist):

    def __init__(self, c: str, width: float, state: ParserState, char_class: type[Char]=Char):
        self.width = char.width

def ship(box: Box, xy: tuple[float, float]=(0, 0)) -> Output:
    ...

def Error(msg: str) -> ParserElement:
    ...

class _BracedText(Token):
    _escapes = {'t': '\t', 'n': '\n', 'f': '\x0c', 'r': '\r'}

    def __init__(self) -> None:
        self.mayReturnEmpty = True
        self.mayIndexError = False

    def parseImpl(self, instring: str, loc: int, do_actions: bool=True) -> tuple[int, str]:
        ...

class ParserState:

    def __init__(self, fontset: Fonts, font: str, font_class: str, fontsize: float, dpi: float):
        self.fontset = fontset
        self._font = font
        self.font_class = font_class
        self.fontsize = fontsize
        self.dpi = dpi

    def copy(self) -> ParserState:
        ...

    @property
    def font(self) -> str:
        ...

    @font.setter
    def font(self, name: str) -> None:
        self._font = name
        self.font_class = name

    def get_current_underline_thickness(self) -> float:
        ...

def cmd(expr: str, args: ParserElement) -> ParserElement:
    ...

class Parser:

    class _MathStyle(enum.Enum):
        DISPLAYSTYLE = 0
        TEXTSTYLE = 1
        SCRIPTSTYLE = 2
        SCRIPTSCRIPTSTYLE = 3
    _binary_operators = set('+ * - −\n      \\pm             \\sqcap                   \\rhd\n      \\mp             \\sqcup                   \\unlhd\n      \\times          \\vee                     \\unrhd\n      \\div            \\wedge                   \\oplus\n      \\ast            \\setminus                \\ominus\n      \\star           \\wr                      \\otimes\n      \\circ           \\diamond                 \\oslash\n      \\bullet         \\bigtriangleup           \\odot\n      \\cdot           \\bigtriangledown         \\bigcirc\n      \\cap            \\triangleleft            \\dagger\n      \\cup            \\triangleright           \\ddagger\n      \\uplus          \\lhd                     \\amalg\n      \\dotplus        \\dotminus                \\Cap\n      \\Cup            \\barwedge                \\boxdot\n      \\boxminus       \\boxplus                 \\boxtimes\n      \\curlyvee       \\curlywedge              \\divideontimes\n      \\doublebarwedge \\leftthreetimes          \\rightthreetimes\n      \\slash          \\veebar                  \\barvee\n      \\cupdot         \\intercal                \\amalg\n      \\circledcirc    \\circleddash             \\circledast\n      \\boxbar         \\obar                    \\merge\n      \\minuscolon     \\dotsminusdots\n      '.split())
    _relation_symbols = set('\n      = < > :\n      \\leq          \\geq          \\equiv       \\models\n      \\prec         \\succ         \\sim         \\perp\n      \\preceq       \\succeq       \\simeq       \\mid\n      \\ll           \\gg           \\asymp       \\parallel\n      \\subset       \\supset       \\approx      \\bowtie\n      \\subseteq     \\supseteq     \\cong        \\Join\n      \\sqsubset     \\sqsupset     \\neq         \\smile\n      \\sqsubseteq   \\sqsupseteq   \\doteq       \\frown\n      \\in           \\ni           \\propto      \\vdash\n      \\dashv        \\dots         \\doteqdot    \\leqq\n      \\geqq         \\lneqq        \\gneqq       \\lessgtr\n      \\leqslant     \\geqslant     \\eqgtr       \\eqless\n      \\eqslantless  \\eqslantgtr   \\lesseqgtr   \\backsim\n      \\backsimeq    \\lesssim      \\gtrsim      \\precsim\n      \\precnsim     \\gnsim        \\lnsim       \\succsim\n      \\succnsim     \\nsim         \\lesseqqgtr  \\gtreqqless\n      \\gtreqless    \\subseteqq    \\supseteqq   \\subsetneqq\n      \\supsetneqq   \\lessapprox   \\approxeq    \\gtrapprox\n      \\precapprox   \\succapprox   \\precnapprox \\succnapprox\n      \\npreccurlyeq \\nsucccurlyeq \\nsqsubseteq \\nsqsupseteq\n      \\sqsubsetneq  \\sqsupsetneq  \\nlesssim    \\ngtrsim\n      \\nlessgtr     \\ngtrless     \\lnapprox    \\gnapprox\n      \\napprox      \\approxeq     \\approxident \\lll\n      \\ggg          \\nparallel    \\Vdash       \\Vvdash\n      \\nVdash       \\nvdash       \\vDash       \\nvDash\n      \\nVDash       \\oequal       \\simneqq     \\triangle\n      \\triangleq         \\triangleeq         \\triangleleft\n      \\triangleright     \\ntriangleleft      \\ntriangleright\n      \\trianglelefteq    \\ntrianglelefteq    \\trianglerighteq\n      \\ntrianglerighteq  \\blacktriangleleft  \\blacktriangleright\n      \\equalparallel     \\measuredrightangle \\varlrtriangle\n      \\Doteq        \\Bumpeq       \\Subset      \\Supset\n      \\backepsilon  \\because      \\therefore   \\bot\n      \\top          \\bumpeq       \\circeq      \\coloneq\n      \\curlyeqprec  \\curlyeqsucc  \\eqcirc      \\eqcolon\n      \\eqsim        \\fallingdotseq \\gtrdot     \\gtrless\n      \\ltimes       \\rtimes       \\lessdot     \\ne\n      \\ncong        \\nequiv       \\ngeq        \\ngtr\n      \\nleq         \\nless        \\nmid        \\notin\n      \\nprec        \\nsubset      \\nsubseteq   \\nsucc\n      \\nsupset      \\nsupseteq    \\pitchfork   \\preccurlyeq\n      \\risingdotseq \\subsetneq    \\succcurlyeq \\supsetneq\n      \\varpropto    \\vartriangleleft \\scurel\n      \\vartriangleright \\rightangle \\equal     \\backcong\n      \\eqdef        \\wedgeq       \\questeq     \\between\n      \\veeeq        \\disin        \\varisins    \\isins\n      \\isindot      \\varisinobar  \\isinobar    \\isinvb\n      \\isinE        \\nisd         \\varnis      \\nis\n      \\varniobar    \\niobar       \\bagmember   \\ratio\n      \\Equiv        \\stareq       \\measeq      \\arceq\n      \\rightassert  \\rightModels  \\smallin     \\smallowns\n      \\notsmallowns \\nsimeq'.split())
    _arrow_symbols = set('\n     \\leftarrow \\longleftarrow \\uparrow \\Leftarrow \\Longleftarrow\n     \\Uparrow \\rightarrow \\longrightarrow \\downarrow \\Rightarrow\n     \\Longrightarrow \\Downarrow \\leftrightarrow \\updownarrow\n     \\longleftrightarrow \\updownarrow \\Leftrightarrow\n     \\Longleftrightarrow \\Updownarrow \\mapsto \\longmapsto \\nearrow\n     \\hookleftarrow \\hookrightarrow \\searrow \\leftharpoonup\n     \\rightharpoonup \\swarrow \\leftharpoondown \\rightharpoondown\n     \\nwarrow \\rightleftharpoons \\leadsto \\dashrightarrow\n     \\dashleftarrow \\leftleftarrows \\leftrightarrows \\Lleftarrow\n     \\Rrightarrow \\twoheadleftarrow \\leftarrowtail \\looparrowleft\n     \\leftrightharpoons \\curvearrowleft \\circlearrowleft \\Lsh\n     \\upuparrows \\upharpoonleft \\downharpoonleft \\multimap\n     \\leftrightsquigarrow \\rightrightarrows \\rightleftarrows\n     \\rightrightarrows \\rightleftarrows \\twoheadrightarrow\n     \\rightarrowtail \\looparrowright \\rightleftharpoons\n     \\curvearrowright \\circlearrowright \\Rsh \\downdownarrows\n     \\upharpoonright \\downharpoonright \\rightsquigarrow \\nleftarrow\n     \\nrightarrow \\nLeftarrow \\nRightarrow \\nleftrightarrow\n     \\nLeftrightarrow \\to \\Swarrow \\Searrow \\Nwarrow \\Nearrow\n     \\leftsquigarrow \\overleftarrow \\overleftrightarrow \\cwopencirclearrow\n     \\downzigzagarrow \\cupleftarrow \\rightzigzagarrow \\twoheaddownarrow\n     \\updownarrowbar \\twoheaduparrow \\rightarrowbar \\updownarrows\n     \\barleftarrow \\mapsfrom \\mapsdown \\mapsup \\Ldsh \\Rdsh\n     '.split())
    _spaced_symbols = _binary_operators | _relation_symbols | _arrow_symbols
    _punctuation_symbols = set(', ; . ! \\ldotp \\cdotp'.split())
    _overunder_symbols = set('\n       \\sum \\prod \\coprod \\bigcap \\bigcup \\bigsqcup \\bigvee\n       \\bigwedge \\bigodot \\bigotimes \\bigoplus \\biguplus\n       '.split())
    _overunder_functions = set('lim liminf limsup sup max min'.split())
    _dropsub_symbols = set('\\int \\oint \\iint \\oiint \\iiint \\oiiint \\iiiint'.split())
    _fontnames = set('rm cal it tt sf bf bfit default bb frak scr regular normal'.split())
    _function_names = set('\n      arccos csc ker min arcsin deg lg Pr arctan det lim sec arg dim\n      liminf sin cos exp limsup sinh cosh gcd ln sup cot hom log tan\n      coth inf max tanh'.split())
    _ambi_delims = set('\n      | \\| / \\backslash \\uparrow \\downarrow \\updownarrow \\Uparrow\n      \\Downarrow \\Updownarrow . \\vert \\Vert'.split())
    _left_delims = set('\n      ( [ \\{ < \\lfloor \\langle \\lceil \\lbrace \\leftbrace \\lbrack \\leftparen \\lgroup\n      '.split())
    _right_delims = set('\n      ) ] \\} > \\rfloor \\rangle \\rceil \\rbrace \\rightbrace \\rbrack \\rightparen \\rgroup\n      '.split())
    _delims = _left_delims | _right_delims | _ambi_delims
    _small_greek = set([unicodedata.name(chr(i)).split()[-1].lower() for i in range(ord('α'), ord('ω') + 1)])
    _latin_alphabets = set(string.ascii_letters)

    def __init__(self) -> None:
        self._expression = p.main
        self._math_expression = p.math
        self._needs_space_after_subsuper = False

    def parse(self, s: str, fonts_object: Fonts, fontsize: float, dpi: float) -> Hlist:
        self._state_stack = [ParserState(fonts_object, 'default', 'rm', fontsize, dpi)]
        self._em_width_cache: dict[tuple[str, float, float], float] = {}
        self._state_stack = []
        self._needs_space_after_subsuper = False
        self._em_width_cache = {}

    def get_state(self) -> ParserState:
        ...

    def pop_state(self) -> None:
        ...

    def push_state(self) -> None:
        ...

    def main(self, toks: ParseResults) -> list[Hlist]:
        ...

    def math_string(self, toks: ParseResults) -> ParseResults:
        ...

    def math(self, toks: ParseResults) -> T.Any:
        ...

    def non_math(self, toks: ParseResults) -> T.Any:
        self.get_state().font = mpl.rcParams['mathtext.default']
    float_literal = staticmethod(pyparsing_common.convert_to_float)

    def text(self, toks: ParseResults) -> T.Any:
        ...

    def _make_space(self, percentage: float) -> Kern:
        self._em_width_cache[key] = width
    _space_widths = {'\\,': 0.16667, '\\thinspace': 0.16667, '\\/': 0.16667, '\\>': 0.22222, '\\:': 0.22222, '\\;': 0.27778, '\\ ': 0.33333, '~': 0.33333, '\\enspace': 0.5, '\\quad': 1, '\\qquad': 2, '\\!': -0.16667}

    def space(self, toks: ParseResults) -> T.Any:
        ...

    def customspace(self, toks: ParseResults) -> T.Any:
        ...

    def symbol(self, s: str, loc: int, toks: ParseResults | dict[str, str]) -> T.Any:
        ...

    def unknown_symbol(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        ...

    def phantom(self, toks: ParseResults) -> T.Any:
        ...

    def llap(self, toks: ParseResults) -> T.Any:
        ...

    def rlap(self, toks: ParseResults) -> T.Any:
        ...
    _accent_map = {'hat': '\\circumflexaccent', 'breve': '\\combiningbreve', 'bar': '\\combiningoverline', 'grave': '\\combininggraveaccent', 'acute': '\\combiningacuteaccent', 'tilde': '\\combiningtilde', 'dot': '\\combiningdotabove', 'ddot': '\\combiningdiaeresis', 'dddot': '\\combiningthreedotsabove', 'ddddot': '\\combiningfourdotsabove', 'vec': '\\combiningrightarrowabove', '"': '\\combiningdiaeresis', '`': '\\combininggraveaccent', "'": '\\combiningacuteaccent', '~': '\\combiningtilde', '.': '\\combiningdotabove', '^': '\\circumflexaccent', 'overrightarrow': '\\rightarrow', 'overleftarrow': '\\leftarrow', 'mathring': '\\circ'}
    _wide_accents = set('widehat widetilde widebar'.split())

    def accent(self, toks: ParseResults) -> T.Any:
        ...

    def function(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        ...

    def operatorname(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        self._needs_space_after_subsuper = next_char in {'^', '_'}

    def start_group(self, toks: ParseResults) -> T.Any:
        self.get_state().font = toks.get('font')

    def group(self, toks: ParseResults) -> T.Any:
        ...

    def required_group(self, toks: ParseResults) -> T.Any:
        ...
    optional_group = required_group

    def end_group(self) -> T.Any:
        ...

    def unclosed_group(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        ...

    def font(self, toks: ParseResults) -> T.Any:
        self.get_state().font = toks['font']

    def is_overunder(self, nucleus: Node) -> bool:
        ...

    def is_dropsub(self, nucleus: Node) -> bool:
        ...

    def is_slanted(self, nucleus: Node) -> bool:
        ...

    def subsuper(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        self._needs_space_after_subsuper = False
        self._needs_space_after_subsuper = False

    def _genfrac(self, ldelim: str, rdelim: str, rule: float | None, style: _MathStyle, num: Hlist, den: Hlist) -> T.Any:
        ...

    def style_literal(self, toks: ParseResults) -> T.Any:
        ...

    def genfrac(self, toks: ParseResults) -> T.Any:
        ...

    def frac(self, toks: ParseResults) -> T.Any:
        ...

    def dfrac(self, toks: ParseResults) -> T.Any:
        ...

    def binom(self, toks: ParseResults) -> T.Any:
        ...

    def _genset(self, s: str, loc: int, toks: ParseResults) -> T.Any:
        ...
    overset = underset = _genset

    def sqrt(self, toks: ParseResults) -> T.Any:
        ...

    def overline(self, toks: ParseResults) -> T.Any:
        ...

    def underline(self, toks: ParseResults) -> T.Any:
        ...

    def _auto_sized_delimiter(self, front: str, middle: list[Box | Char | str], back: str) -> T.Any:
        ...

    def auto_delim(self, toks: ParseResults) -> T.Any:
        ...

    def boldsymbol(self, toks: ParseResults) -> T.Any:
        ...

    def substack(self, toks: ParseResults) -> T.Any:
        ...
__all__ = ['abc', 'copy', 'enum', 'functools', 'itertools', 'logging', 'math', 'os', 're', 'types', 'unicodedata', 'string', 'textwrap', 'T', 'NamedTuple', 'np', 'NDArray', 'Empty', 'Forward', 'Literal', 'Group', 'NotAny', 'OneOrMore', 'Optional', 'ParseBaseException', 'ParseException', 'ParseExpression', 'ParseFatalException', 'ParserElement', 'ParseResults', 'QuotedString', 'Regex', 'StringEnd', 'Token', 'ZeroOrMore', 'pyparsing_common', 'nested_expr', 'one_of', 'mpl', 'cbook', 'latex_to_bakoma', 'stix_glyph_fixes', 'stix_virtual_fonts', 'tex2uni', 'FontProperties', 'findfont', 'get_font', 'FT2Font', 'Kerning', 'LoadFlags', 'Iterable', 'CharacterCodeType', 'Glyph', 'GlyphIndexType', 'get_unicode_index', 'VectorParse', 'RasterParse', 'Output', 'FontMetrics', 'FontInfo', 'Fonts', 'TruetypeFonts', 'BakomaFonts', 'UnicodeFonts', 'DejaVuFonts', 'DejaVuSerifFonts', 'DejaVuSansFonts', 'StixFonts', 'StixSansFonts', 'SHRINK_FACTOR', 'NUM_SIZE_LEVELS', 'FontConstantsBase', 'ComputerModernFontConstants', 'STIXFontConstants', 'STIXSansFontConstants', 'DejaVuSerifFontConstants', 'DejaVuSansFontConstants', 'Node', 'Box', 'Vbox', 'Hbox', 'Char', 'Accent', 'List', 'Hlist', 'Vlist', 'Rule', 'Hrule', 'Vrule', 'Glue', 'HCentered', 'VCentered', 'Kern', 'AutoHeightChar', 'AutoWidthChar', 'ship', 'Error', 'ParserState', 'cmd', 'Parser']
