"""matplotlib's own tests of ``matplotlib.ft2font`` (``tests/test_ft2font.py``):
face attributes, face indices, character maps and glyph names, the name
and sfnt tables, kerning, text set with the autohinter, glyph metrics and
outlines, and which font of a fallback list draws each character.

Area ``mpl_ft2font``. A ``file`` is a font bundled with matplotlib
(``FT2Font(<mpl-data>/fonts/ttf/<file>)``); the tests' ``findfont``
patterns name ``DejaVuSans.ttf``, ``cmtt10.ttf`` and
``STIXSizTwoSymBol.ttf``. Sizes are ``set_size(size, dpi)``.

Ops
---
``ft2font.FT2Font.attrs``  {file} -> ``{"postscript_name", "family_name",
    "style_name", "num_faces", "num_glyphs", "num_charmaps", "face_flags",
    "style_flags", "scalable", "units_per_EM", "underline_position",
    "underline_thickness", "ascender", "descender", "height",
    "max_advance_width", "max_advance_height", "bbox"}``; ``face_flags``
    holds whether the face has each of the flags the tests check
    (``scalable``, ``sfnt``, ``horizontal``, ``kerning``, ``glyph_names``),
    ``style_flags`` ``{"italic", "bold"}``.
``ft2font.FT2Font``  {file, face_index} -> ``{"face_index", "num_faces"}``
    of ``FT2Font(file, face_index=face_index)``, or the error.
``ft2font.FT2Font.get_charmap``  {file, charmap} -> the character map after
    ``set_charmap(charmap)`` (an index) or ``select_charmap(...)`` (``"unic"``
    or ``"armn"``): ``{"size", "checksum", "sample"}``, its size, the sum of
    ``code * 65537 + glyph`` over its entries, and the glyph of each code of
    ``SAMPLE_CODES`` (``null`` when unmapped); or the error.
``ft2font.FT2Font.get_name_index``  {file, name} -> the glyph index.
``ft2font.FT2Font.get_glyph_name``  {file, index} -> the glyph name.
``ft2font.FT2Font.get_sfnt``  {file} -> every name record as ``[platform,
    encoding, language, name_id, hex bytes]``, sorted.
``ft2font.FT2Font.get_sfnt_table``  {file, table} -> ``get_sfnt_table(table)``
    with tuples as lists and bytes as lists of byte values, or ``null``.
``ft2font.FT2Font.get_kerning``  {file, size, dpi, left, right, mode} -> the
    kerning of the glyphs of characters ``left`` and ``right`` in mode
    ``Kerning.<mode>``.
``ft2font.FT2Font.set_text``  {file, text, size, dpi} -> ``{"xys",
    "width_height", "num_glyphs", "descent", "bitmap_offset"}`` after
    ``set_text(text)`` with its default ``FORCE_AUTOHINT``.
``ft2font.FT2Font.load_char``  {file, size, dpi, char} -> the metrics of
    ``load_char(ord(char))`` (``FORCE_AUTOHINT``): ``{"width", "height",
    "horiBearingX", "horiBearingY", "horiAdvance", "linearHoriAdvance",
    "bbox"}``, or the error for a character the font lacks.
``ft2font.FT2Font.load_glyph``  {file, size, dpi, char, flags} -> ``{"advance",
    "path"}`` (the ``text_metrics`` area's).
``ft2font.FT2Font._layout.ft_object``  {files, text} -> ``[[char, file],
    ...]``: the font file drawing each item of ``_layout(line, DEFAULT)``
    per line of ``text``, for ``get_font`` of ``files`` (with matplotlib's
    Last Resort fallback).

Cases
-----
Named after the upstream tests. ``test__layout`` names its fonts by file
rather than through ``_find_fonts_by_props``.

Already covered
---------------
``test_ft2font_features`` and ``test_ft2font_language`` by the
``text_shaping`` area; ``test_fallback_smoke`` by the ``mpl_backends``
figure group (``multi_font`` in every format); ``test_fallback_last_resort``
by the ``mpl_rest`` figure group.

Left out
--------
``test_ft2image_draw_rect_filled`` (the deprecated ``FT2Image``),
``test_ft2font_valid_args``, ``test_ft2font_invalid_args`` and
``test_ft2font_language_invalid`` (Python argument types and deprecation),
``test_ft2font_oversized_read`` (Python file objects), the WenQuanYi half
of ``test_ft2font_face_index`` (not bundled), ``test_ft2font_clear``
(``FT2Font`` object state), the ``num_named_instances`` and
``num_fixed_sizes`` attributes (variable fonts and bitmap strikes, which
plt-font does not read), the vertical metrics of ``test_ft2font_loading``
(FreeType's synthesized vertical layout, unused by matplotlib), the
deprecated kerning constants of ``test_ft2font_get_kerning``, and
``test_fallback_missing`` (warnings). ``test_ft2font_drawing`` is the
``mpl_tests_ft2font`` figure group's.
"""

from pathlib import Path

from matplotlib import cbook, ft2font
import matplotlib.font_manager as fm

from probes.test_text_metrics import _load_glyph
from spec_support import ProbeSet

probes = ProbeSet("mpl_ft2font")

FONTS = ["DejaVuSans.ttf", "cmtt10.ttf", "STIXSizTwoSymBol.ttf"]

#: The codes whose glyphs ``get_charmap`` records: ASCII and the characters
#: whose Unicode and Macintosh Roman codes the test compares, plus two
#: outside the Basic Multilingual Plane.
SAMPLE_CODES = (list(range(1, 128))
                + [0x2020, 0xB0, 0xA3, 0xA7, 0xB6, 0x221E, 0xA0, 0xA1, 0xA4, 0xA6,
                   0x1D305, 0x1F0A1])


def _path(file):
    return cbook._get_data_path("fonts", "ttf", file)


def _font(file, **kw):
    return ft2font.FT2Font(_path(file), **kw)


def _attrs(file):
    font = _font(file)
    flags = ft2font.FaceFlags
    return {
        "postscript_name": font.postscript_name,
        "family_name": font.family_name,
        "style_name": font.style_name,
        "num_faces": font.num_faces,
        "num_glyphs": font.num_glyphs,
        "num_charmaps": font.num_charmaps,
        "face_flags": {name.lower(): getattr(flags, name) in font.face_flags
                       for name in ["SCALABLE", "SFNT", "HORIZONTAL", "KERNING",
                                    "GLYPH_NAMES"]},
        "style_flags": {"italic": ft2font.StyleFlags.ITALIC in font.style_flags,
                        "bold": ft2font.StyleFlags.BOLD in font.style_flags},
        "scalable": font.scalable,
        "units_per_EM": font.units_per_EM,
        "underline_position": font.underline_position,
        "underline_thickness": font.underline_thickness,
        "ascender": font.ascender,
        "descender": font.descender,
        "height": font.height,
        "max_advance_width": font.max_advance_width,
        "max_advance_height": font.max_advance_height,
        "bbox": font.bbox,
    }


def test_ft2font_attrs():
    """``test_ft2font_dejavu_attrs``, ``test_ft2font_cm_attrs`` and
    ``test_ft2font_stix_bold_attrs``."""
    for file in FONTS:
        probes.probe("ft2font.FT2Font.attrs", _attrs, file=file)


def _face(file, face_index):
    font = _font(file, face_index=face_index)
    return {"face_index": font.face_index, "num_faces": font.num_faces}


def test_ft2font_face_index():
    for face_index in [0, 0xff]:
        probes.probe("ft2font.FT2Font", _face, file="DejaVuSans.ttf", face_index=face_index)


def _encoding(name):
    code = 0
    for c in name:
        code = (code << 8) + ord(c)
    return code


def _charmap(file, charmap):
    font = _font(file)
    if isinstance(charmap, str):
        font.select_charmap(_encoding(charmap))
    else:
        font.set_charmap(charmap)
    cmap = font.get_charmap()
    return {"size": len(cmap),
            "checksum": sum(code * 65537 + glyph for code, glyph in cmap.items()),
            "sample": [cmap.get(code) for code in SAMPLE_CODES]}


GLYPH_NAMES = {
    'non-existent-glyph-name': 0, 'plusminus': 115, 'Racute': 278, 'perthousand': 2834,
    'seveneighths': 3057, 'triagup': 3721, 'uni01D3': 405, 'uni0417': 939, 'uni2A02': 4464,
    'u1D305': 5410, 'u1F0A1': 5784,
}


def test_ft2font_charmaps():
    file = "DejaVuSans.ttf"
    for charmap in ["unic", 0, 1, 3, 4, "armn", 2, -1, 5]:
        probes.probe("ft2font.FT2Font.get_charmap", _charmap, file=file, charmap=charmap)
    for name, index in GLYPH_NAMES.items():
        r = probes.probe("ft2font.FT2Font.get_name_index",
                         lambda file, name: _font(file).get_name_index(name), file=file,
                         name=name)
        assert r == index
        probes.probe("ft2font.FT2Font.get_glyph_name",
                     lambda file, index: _font(file).get_glyph_name(index), file=file,
                     index=index)


def _sfnt(file):
    return sorted([*key, value.hex()] for key, value in _font(file).get_sfnt().items())


def test_ft2font_get_sfnt():
    for file in FONTS:
        probes.probe("ft2font.FT2Font.get_sfnt", _sfnt, file=file)


def _plain(value):
    if isinstance(value, bytes):
        return list(value)
    if isinstance(value, tuple):
        return list(value)
    return value


def _sfnt_table(file, table):
    values = _font(file).get_sfnt_table(table)
    return None if values is None else {key: _plain(v) for key, v in values.items()}


def test_ft2font_get_sfnt_table():
    for file in ["DejaVuSans.ttf", "STIXSizTwoSymBol.ttf", "cmtt10.ttf"]:
        for table in ["invalid", "head", "maxp", "OS/2", "hhea", "vhea", "post", "pclt"]:
            probes.probe("ft2font.FT2Font.get_sfnt_table", _sfnt_table, file=file, table=table)


def _kerning(file, size, dpi, left, right, mode):
    font = _font(file)
    font.set_size(size, dpi)
    return font.get_kerning(font.get_char_index(ord(left)), font.get_char_index(ord(right)),
                            getattr(ft2font.Kerning, mode))


def test_ft2font_get_kerning():
    for left, right in [('A', 'A'), ('A', 'À'), ('A', 'Á'), ('A', 'Â'), ('A', 'Ã'), ('A', 'Ä'),
                        ('D', 'A'), ('T', '.'), ('X', 'C'), ('-', 'J')]:
        for mode in ["UNSCALED", "UNFITTED", "DEFAULT"]:
            probes.probe("ft2font.FT2Font.get_kerning", _kerning, file="DejaVuSans.ttf",
                         size=100, dpi=100, left=left, right=right, mode=mode)


def _set_text(file, text, size, dpi):
    font = _font(file)
    font.set_size(size, dpi)
    xys = font.set_text(text)
    return {"xys": xys.reshape(-1, 2), "width_height": font.get_width_height(),
            "num_glyphs": font.get_num_glyphs(), "descent": font.get_descent(),
            "bitmap_offset": font.get_bitmap_offset()}


def test_ft2font_set_text():
    for text in ["", "AADAT.XC-J"]:
        probes.probe("ft2font.FT2Font.set_text", _set_text, file="DejaVuSans.ttf", text=text,
                     size=12, dpi=72)


def test_ft2font_set_size():
    for size, dpi in [(12, 72), (24, 72), (12, 144)]:
        probes.probe("ft2font.FT2Font.set_text", _set_text, file="DejaVuSans.ttf",
                     text="ABabCDcd", size=size, dpi=dpi)


def _load_char(file, size, dpi, char):
    font = _font(file)
    font.set_size(size, dpi)
    glyph = font.load_char(ord(char))
    return {"width": glyph.width, "height": glyph.height,
            "horiBearingX": glyph.horiBearingX, "horiBearingY": glyph.horiBearingY,
            "horiAdvance": glyph.horiAdvance, "linearHoriAdvance": glyph.linearHoriAdvance,
            "bbox": glyph.bbox}


def test_ft2font_loading():
    for char in ["M", "ᥨ"]:
        probes.probe("ft2font.FT2Font.load_char", _load_char, file="DejaVuSans.ttf", size=12,
                     dpi=72, char=char)


def test_ft2font_get_path():
    probes.probe("ft2font.FT2Font.load_glyph", _load_glyph, file="DejaVuSans.ttf", size=12,
                 dpi=72, char="M", flags="FORCE_AUTOHINT")


def _layout_fonts(files, text):
    font = fm.get_font([_path(file) for file in files])
    return [[item.char, Path(item.ft_object.fname).name]
            for line in text.split("\n")
            for item in font._layout(line, ft2font.LoadFlags.DEFAULT)]


def test__layout():
    from matplotlib.testing import _gen_multi_font_text
    fonts, text = _gen_multi_font_text()
    assert fonts == ["cmr10", "DejaVu Sans"]
    probes.probe("ft2font.FT2Font._layout.ft_object", _layout_fonts,
                 files=["cmr10.ttf", "DejaVuSans.ttf"], text=text + "\n几个汉字")
