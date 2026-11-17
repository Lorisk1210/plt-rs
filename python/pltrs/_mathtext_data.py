"""
font data tables for truetype and afm computer modern fonts

matplotlib's tables that its mathtext font classes read
(`._mathtext`), in its shapes, from those pltrs (Rust) typesets
with: a program's subclass of those classes and pltrs' own typesetting
cannot read different tables.
"""

from ._native import mathtext as _native_mathtext

#: Each symbol's BaKoMa font (``'cmr10'``, ...) and character code.
latex_to_bakoma = {symbol: (font, code)
                   for symbol, font, code in _native_mathtext.latex_to_bakoma()}

#: TeX symbol names, without the backslash, and their code points.
tex2uni = dict(_native_mathtext.tex2uni())

#: The code points whose STIX glyphs are swapped, and the code each takes.
stix_glyph_fixes = dict(_native_mathtext.stix_glyph_fixes())

#: The alphabets the STIX fonts keep at other code points, by TeX font
#: name: a list of ``(start, end, font, target)`` ranges, or a dict of such
#: lists by font class.
stix_virtual_fonts = {
    name: dict(by_class) if by_class is not None else single
    for name, by_class, single in _native_mathtext.stix_virtual_fonts()
}
