# The type stub of matplotlib.backends._backend_pdf_ps: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from __future__ import annotations
from contextlib import contextmanager
from io import BytesIO
import functools
import logging
import typing
from fontTools import subset
import matplotlib as mpl
from .. import font_manager, ft2font
from .._afm import AFM
from ..backend_bases import RendererBase
if typing.TYPE_CHECKING:
    from collections.abc import Generator
    from contextlib import AbstractContextManager
    from .font_manager import FontPath
    from .ft2font import CharacterCodeType, FT2Font, GlyphIndexType
    from fontTools.ttLib import TTFont
_FONT_MAX_GLYPH = {3: 256, 42: 65536}

@functools.lru_cache(50)
def _cached_get_afm_from_fname(fname):
    ...

class SubsetResults(typing.NamedTuple):
    font: TTFont
    glyph_index_map: dict[GlyphIndexType, GlyphIndexType]

    @contextmanager
    def _as_cm(self) -> Generator[SubsetResults, None, None]:
        ...

def get_glyphs_subset(fontfile: FontPath, glyphs: set[GlyphIndexType]) -> AbstractContextManager[SubsetResults]:
    ...

def font_as_file(font):
    ...

class GlyphMap:

    def __init__(self) -> None:
        self._forward: dict[tuple[CharacterCodeType, GlyphIndexType], tuple[int, CharacterCodeType]] = {}
        self._inverse: dict[tuple[int, CharacterCodeType], tuple[CharacterCodeType, GlyphIndexType]] = {}

    def get(self, charcodes: str, glyph_index: GlyphIndexType) -> tuple[int, CharacterCodeType] | None:
        ...

    def iget(self, subset: int, subset_charcode: CharacterCodeType) -> tuple[str, GlyphIndexType]:
        ...

    def add(self, charcode: str, glyph_index: GlyphIndexType, subset: int, subset_charcode: CharacterCodeType) -> None:
        self._forward[charcode, glyph_index] = (subset, subset_charcode)
        self._inverse[subset, subset_charcode] = (charcode, glyph_index)

class CharacterTracker:

    def __init__(self, subset_size: int=0):
        self.used: dict[str, list[dict[CharacterCodeType, GlyphIndexType]]] = {}
        self.glyph_maps: dict[str, GlyphMap] = {}
        self.subset_size = subset_size

    def track(self, font: FT2Font, s: str, features: tuple[str, ...] | None=None, language: str | tuple[tuple[str, int, int], ...] | None=None) -> list[tuple[int, CharacterCodeType]]:
        ...

    def track_glyph(self, font: FT2Font, chars: str | CharacterCodeType, glyph: GlyphIndexType) -> tuple[int, CharacterCodeType]:
        ...

    def subset_to_unicode(self, fontname: str, subset: int, subset_charcode: CharacterCodeType) -> str:
        ...

class RendererPDFPSBase(RendererBase):

    def __init__(self, width, height):
        self.width = width
        self.height = height

    def flipy(self):
        ...

    def option_scale_image(self):
        ...

    def option_image_nocomposite(self):
        ...

    def get_canvas_width_height(self):
        ...

    def _get_font_height_metrics(self, prop):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def _get_font_afm(self, prop):
        ...

    def _get_font_ttf(self, prop):
        ...
__all__ = ['contextmanager', 'BytesIO', 'functools', 'logging', 'typing', 'subset', 'mpl', 'font_manager', 'ft2font', 'AFM', 'RendererBase', 'Generator', 'AbstractContextManager', 'FontPath', 'CharacterCodeType', 'FT2Font', 'GlyphIndexType', 'TTFont', 'SubsetResults', 'get_glyphs_subset', 'font_as_file', 'GlyphMap', 'CharacterTracker', 'RendererPDFPSBase']
