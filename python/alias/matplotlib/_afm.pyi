# The type stub of matplotlib._afm: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import inspect
import logging
import re
from typing import BinaryIO, NamedTuple, TypedDict, cast
from ._mathtext_data import uni2type1
from .ft2font import CharacterCodeType, GlyphIndexType
_log = logging.getLogger(__name__)

def _to_int(x: bytes | str) -> int:
    ...

def _to_float(x: bytes | str) -> float:
    ...

def _to_str(x: bytes) -> str:
    ...

def _to_list_of_ints(s: bytes) -> list[int]:
    ...

def _to_list_of_floats(s: bytes | str) -> list[float]:
    ...

def _to_bool(s: bytes) -> bool:
    ...

class FontMetricsHeader(TypedDict, total=False):
    StartFontMetrics: float
    FontName: str
    FullName: str
    FamilyName: str
    Weight: str
    ItalicAngle: float
    IsFixedPitch: bool
    FontBBox: list[int]
    UnderlinePosition: float
    UnderlineThickness: float
    Version: str
    Notice: bytes
    EncodingScheme: str
    CapHeight: float
    Capheight: float
    XHeight: float
    Ascender: float
    Descender: float
    StdHW: float
    StdVW: float
    StartCharMetrics: int
    CharacterSet: str
    Characters: int

def _parse_header(fh: BinaryIO) -> FontMetricsHeader:
    ...

class CharMetrics(NamedTuple):
    width: float
    name: str
    bbox: tuple[int, int, int, int]
CharMetrics.width.__doc__ = 'The character width (WX).'
CharMetrics.name.__doc__ = 'The character name (N).'
CharMetrics.bbox.__doc__ = '\n    The bbox of the character (B) as a tuple (*llx*, *lly*, *urx*, *ury*).'

def _parse_char_metrics(fh: BinaryIO) -> tuple[dict[CharacterCodeType, CharMetrics], dict[str, CharMetrics]]:
    ...

def _parse_kern_pairs(fh: BinaryIO) -> dict[tuple[str, str], float]:
    ...

class CompositePart(NamedTuple):
    name: bytes
    dx: float
    dy: float
CompositePart.name.__doc__ = "Name of the part, e.g. 'acute'."
CompositePart.dx.__doc__ = 'x-displacement of the part from the origin.'
CompositePart.dy.__doc__ = 'y-displacement of the part from the origin.'

def _parse_composites(fh: BinaryIO) -> dict[bytes, list[CompositePart]]:
    ...

def _parse_optional(fh: BinaryIO) -> tuple[dict[tuple[str, str], float], dict[bytes, list[CompositePart]]]:
    ...

class AFM:

    def __init__(self, fh: BinaryIO):
        self._header = _parse_header(fh)
        self._metrics, self._metrics_by_name = _parse_char_metrics(fh)
        self._kern, self._composite = _parse_optional(fh)

    def get_str_bbox_and_descent(self, s: str) -> tuple[int, int, float, int, int]:
        ...

    def get_glyph_name(self, glyph_ind: GlyphIndexType) -> str:
        ...

    def get_char_index(self, c: CharacterCodeType) -> GlyphIndexType:
        ...

    def get_width_char(self, c: CharacterCodeType) -> float:
        ...

    def get_width_from_char_name(self, name: str) -> float:
        ...

    def get_kern_dist_from_name(self, name1: str, name2: str) -> float:
        ...

    def get_fontname(self) -> str:
        ...

    @property
    def postscript_name(self) -> str:
        ...

    def get_fullname(self) -> str:
        ...

    def get_familyname(self) -> str:
        ...

    @property
    def family_name(self) -> str:
        ...

    def get_weight(self) -> str:
        ...

    def get_angle(self) -> float:
        ...

    def get_ascender(self) -> float:
        ...

    def get_capheight(self) -> float:
        ...

    def get_descender(self) -> float:
        ...

    def get_xheight(self) -> float:
        ...

    def get_underline_thickness(self) -> float:
        ...
__all__ = ['inspect', 'logging', 're', 'BinaryIO', 'NamedTuple', 'TypedDict', 'cast', 'uni2type1', 'CharacterCodeType', 'GlyphIndexType', 'FontMetricsHeader', 'CharMetrics', 'CompositePart', 'AFM']
