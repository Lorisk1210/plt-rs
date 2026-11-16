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


#: Where matplotlib keeps the AFM fonts of each kind of lookup of one
#: (`report_font_lookup`), in its data.
_AFM_DIRECTORIES = {"afm": "fonts/afm", "core fonts": "fonts/pdfcorefonts"}


def report_font_lookup(kind, family, style, variant, weight, stretch, size,
                       times, math_fontfamily=None):
    """
    Look up, with the program's font manager, the fonts of a text pltrs
    (Rust) laid out that matplotlib's font manager logs about (a family no
    font has, a face of another weight than asked for; any text's while
    the deprecated ``text.kerning_factor`` is set), *times* times: as
    matplotlib's renderers do for every text they lay out
    (``_find_fonts_by_props`` for *kind* ``"fonts"``, ``findfont`` for
    ``"font"``, among the AFM fonts for ``"afm"`` and ``"core fonts"``), so
    that the program's font manager logs as matplotlib's does. A
    formula's lookups name its *math_fontfamily* (the fontset of its font
    properties, by which the font manager caches lookups too).
    """
    from . import rcParams
    from .font_manager import FontProperties, fontManager, get_font
    prop = FontProperties(family=family, style=style, variant=variant,
                          weight=weight, stretch=stretch, size=size,
                          math_fontfamily=math_fontfamily)
    if kind in _AFM_DIRECTORIES:
        # The font a PostScript or PDF renderer measures and draws the
        # text in (``_get_font_afm``); the PDF renderer looks the fonts up
        # once more to name the one it writes (``fontName``).
        from . import cbook
        directory = cbook._get_data_path(_AFM_DIRECTORIES[kind])
        fontManager.findfont(prop, fontext="afm", directory=directory)
        if kind == "core fonts":
            fontManager._find_fonts_by_props(prop, fontext="afm", directory=directory)
        return
    find = (fontManager._find_fonts_by_props if kind == "fonts"
            else fontManager.findfont)
    # matplotlib opens the fonts it found (``get_font``), which warns for
    # each one it has not cached while the deprecated kerning factor is
    # set (pltrs (Rust) hands every lookup over then).
    opened = kind == "fonts" and rcParams["text.kerning_factor"] is not None
    for _ in range(times):
        found = find(prop)
        if opened:
            get_font(found)


_register("report_font_lookup", report_font_lookup)


def report_font_open(name, path, parsing):
    """
    Open, with the program's ``get_font``, a font pltrs (Rust)'s mathtext
    opened while the deprecated ``text.kerning_factor`` is set: one pltrs
    bundles, by file *name*, or the one at *path*. matplotlib's mathtext
    opens the default font of a formula as it makes its fontset, then,
    *parsing* it, each font it takes a glyph from or checks for one, which
    warns of the deprecation for a font not opened before.
    """
    import sys
    import warnings
    from . import cbook
    from .font_manager import get_font
    font = path if path is not None else cbook._get_data_path("fonts/ttf", name)
    if not parsing:
        get_font(font)
        return
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        get_font(font)
    # matplotlib's mathtext parses inside pyparsing, whose frame, outside
    # matplotlib, the warnings point at: here, this one.
    line = sys._getframe().f_lineno
    for warning in caught:
        warnings.warn_explicit(warning.message, warning.category, __file__, line)


_register("report_font_open", report_font_open)


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
