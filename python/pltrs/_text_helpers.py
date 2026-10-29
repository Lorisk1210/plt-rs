"""
Low-level text helper utilities (matplotlib's ``_text_helpers``, from
matplotlib 3.11.2, under the Matplotlib License).
"""

from __future__ import annotations

from collections.abc import Iterator

from . import _api
from ._native import register as _register
from .ft2font import FT2Font, CharacterCodeType, LayoutItem, LoadFlags


def warn_on_missing_glyph(codepoint: CharacterCodeType, fontnames: str):
    """Warn that no font of *fontnames* has a glyph for *codepoint*."""
    _api.warn_external(
        f"Glyph {codepoint} "
        f"({chr(codepoint).encode('ascii', 'namereplace').decode('ascii')}) "
        f"missing from font(s) {fontnames}.")


# pltrs (Rust) issues the warning for the texts it lays out while drawing.
_register("warn_on_missing_glyph", warn_on_missing_glyph)


def layout(string: str, font: FT2Font, *,
           features: tuple[str] | None = None,
           language: str | tuple[tuple[str, int, int], ...] | None = None
           ) -> Iterator[LayoutItem]:
    """
    Render *string* with *font*.

    For each character in *string*, yield a LayoutItem instance. When such an instance
    is yielded, the font's glyph is set to the corresponding character.

    Parameters
    ----------
    string : str
        The string to be rendered.
    font : FT2Font
        The font.
    features : tuple of str, optional
        The font features to apply to the text.
    language : str, optional
        The language of the text in a format accepted by libraqm, namely `a BCP47
        language code <https://www.w3.org/International/articles/language-tags/>`_.

    Yields
    ------
    LayoutItem
    """
    for raqm_item in font._layout(string, LoadFlags.NO_HINTING,
                                  features=features, language=language):
        raqm_item.ft_object.load_glyph(raqm_item.glyph_index,
                                       flags=LoadFlags.NO_HINTING)
        yield raqm_item
