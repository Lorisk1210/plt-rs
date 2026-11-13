# The type stub of matplotlib._text_helpers: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from __future__ import annotations
from collections.abc import Iterator
from . import _api
from .ft2font import FT2Font, CharacterCodeType, LayoutItem, LoadFlags

def warn_on_missing_glyph(codepoint: CharacterCodeType, fontnames: str):
    ...

def layout(string: str, font: FT2Font, *, features: tuple[str] | None=None, language: str | tuple[tuple[str, int, int], ...] | None=None) -> Iterator[LayoutItem]:
    ...
__all__ = ['Iterator', 'FT2Font', 'CharacterCodeType', 'LayoutItem', 'LoadFlags', 'warn_on_missing_glyph', 'layout']
