"""PostScript Type 1 fonts: matplotlib's ``_type1font`` (after its own
``tests/test_type1font.py``) and FreeType's Type 1 driver as
``FT2Font`` loads ``.pfb`` files for ``text.usetex``.

Area ``type1font``. A ``file`` is a fixture of
``crates/plt-font/tests/fixtures/type1`` (copies of matplotlib's test data
``cmr10.pfb`` and ``Courier10PitchBT-Bold.pfb``). Byte strings are recorded
as ``[length, sha1 hex digest]`` (``digest``), texts as Latin-1 decoded
strings.

Ops
---
``_type1font.Type1Font``  {file} -> ``{"parts", "decrypted", "head",
    "tail", "prop", "pos", "abbr"}``: the digests of the three ``parts``
    and of ``decrypted``, the first and last 40 decrypted bytes as text,
    the ``prop`` entries (strings, numbers and booleans as they are,
    ``FontBBox`` as a list, ``Encoding`` as sorted ``[code, name]`` pairs,
    ``CharStrings`` as ``{name: digest}``, ``Subrs`` as a list of digests,
    ``OtherSubrs`` as a digest, ``<...>`` strings as ``{"hex": ...}``),
    ``_pos`` with sorted keys and ``_abbr``.
``_type1font.Type1Font.transform``  {file, effects} -> ``{"cleartext",
    "parts", "decrypted", "prop", "pos"}`` of ``font.transform(effects)``:
    the new cleartext part as text, the parts' and decrypted part's
    digests, its ``FontName``, ``ItalicAngle`` and ``FontMatrix`` and the
    keys of its ``_pos``.
``_type1font.Type1Font.with_encoding``  {file, encoding} -> ``{"parts",
    "decrypted", "encoding"}`` of ``font.with_encoding(dict(encoding))``
    (``encoding`` as ``[code, name]`` pairs): digests and the new
    ``prop['Encoding']`` as sorted pairs.
``_type1font.Type1Font.subset``  {file, chars, prefix} -> ``{"cleartext",
    "parts", "decrypted", "font_name", "encoding", "glyphs", "subrs"}`` of
    ``font.subset([ord(c) for c in chars], prefix)``: the cleartext, the
    digests of the parts (the bytes the PDF backend embeds) and of the
    decrypted part, the new font name, encoding, glyph names and subroutine
    digests.
``_type1font.Type1Font.pfa``  {private} -> the ``_type1font.Type1Font``
    summary (without ``head`` and ``tail``) of a PFA font written as
    matplotlib's tests write one: ``/FontName /X def /FontBBox [0 0 1 1]
    def`` followed by ``private`` (a Latin-1 string), encrypted; or the
    error.
``_type1font._tokenize``  {data, skip_ws} -> ``[[kind, value], ...]`` of
    ``_tokenize(data.encode('latin-1'), skip_ws)``, ``<...>`` values as
    ``{"hex": ...}``; or the error.
``_type1font.Type1Font._encrypt``  {data, key} -> ``{"encrypted",
    "decrypted"}``: ``_encrypt(data, key).hex()`` and whether
    ``_decrypt`` gives the data back.
``ft2font.FT2Font.attrs``  {file} -> the face attributes ``FT2Font``
    reports for the font (as in the ``mpl_ft2font`` area, with
    ``fixed_width``).
``ft2font.FT2Font.load_glyph``  {file, effects, glyph} -> ``{"name",
    "advance", "bbox", "path"}``: glyph ``glyph`` loaded with
    ``LoadFlags.NO_SCALE``: its name, ``horiAdvance``, ``bbox`` and
    ``get_path()`` as ``[[code, 64 x, 64 y], ...]`` (whole font units).
    With ``effects``, the font is first transformed by ``_type1font`` and
    written as a PFB file, as ``dviread`` effects would reach FreeType.
``ft2font.FT2Font.load_glyph.sized``  {file, effects, size, dpi, flags,
    glyph} -> ``{"advance", "path"}``: glyph ``glyph`` after
    ``set_size(size, dpi)``, loaded with ``LoadFlags.<flags>``
    (``TARGET_LIGHT``, as ``textpath`` loads usetex glyphs, ``DEFAULT`` or
    ``NO_HINTING``): ``horiAdvance`` and the path in 26.6 pixels.
``ft2font.FT2Font.get_name_index``  {file, name} -> the glyph index (0 for
    unknown names).
``ft2font.FT2Font._get_type1_encoding_vector``  {file} -> the 256 glyph
    indices.

Left out
--------
``test_Subrs_no_preallocation`` measures memory with ``tracemalloc``;
the ``pfa`` probe records its error and plt-rs' unit tests check that no
count-sized array is allocated.
"""

import hashlib
import struct
import tempfile
from pathlib import Path

import matplotlib._type1font as t1f
from matplotlib import ft2font
from matplotlib.ft2font import FT2Font, LoadFlags

from spec_support import ProbeSet

probes = ProbeSet("type1font")

FIXTURES = Path(__file__).resolve().parents[3] / "crates/plt-font/tests/fixtures/type1"
FILES = ["cmr10.pfb", "Courier10PitchBT-Bold.pfb"]


def _path(file):
    return str(FIXTURES / file)


def _digest(data):
    return [len(data), hashlib.sha1(data).hexdigest()]


def _text(data):
    return data.decode("latin-1")


def _value(value):
    if isinstance(value, bytes):
        return {"hex": value.hex()}
    return value


def _prop(prop):
    out = {}
    for key, value in prop.items():
        if key == "Encoding":
            out[key] = sorted([code, name] for code, name in value.items())
        elif key == "CharStrings":
            out[key] = {name: _digest(code) for name, code in value.items()}
        elif key == "Subrs":
            out[key] = [_digest(code) for code in value]
        elif key == "OtherSubrs":
            out[key] = _digest(value)
        else:
            out[key] = _value(value)
    return out


def _summary(font):
    return {
        "parts": [_digest(part) for part in font.parts],
        "decrypted": _digest(font.decrypted),
        "prop": _prop(font.prop),
        "pos": {key: [list(span) for span in spans]
                for key, spans in sorted(font._pos.items())},
        "abbr": dict(font._abbr),
    }


def _font(file):
    font = t1f.Type1Font(_path(file))
    return {**_summary(font), "head": _text(font.decrypted[:40]),
            "tail": _text(font.decrypted[-40:])}


def test_Type1Font():
    probes.probe("_type1font.Type1Font", _font, file="cmr10.pfb")


def test_Type1Font_2():
    probes.probe("_type1font.Type1Font", _font, file="Courier10PitchBT-Bold.pfb")


def _transform(file, effects):
    font = t1f.Type1Font(_path(file)).transform(effects)
    return {
        "cleartext": _text(font.parts[0]),
        "parts": [_digest(part) for part in font.parts],
        "decrypted": _digest(font.decrypted),
        "prop": {key: font.prop[key] for key in ("FontName", "ItalicAngle", "FontMatrix")},
        "pos": sorted(font._pos),
    }


EFFECTS = [
    {"slant": 1}, {"extend": 0.5}, {"slant": 0.167}, {"slant": -0.25, "extend": 1.2},
    {}, {"slant": 0.0001}, {"extend": 2}, {"slant": 0.333333, "extend": 0.85},
]


def test_transform():
    """``test_Type1Font`` and ``test_overprecision``."""
    for file in FILES:
        for effects in EFFECTS:
            probes.probe("_type1font.Type1Font.transform", _transform, file=file,
                         effects=effects)


def _with_encoding(file, encoding):
    font = t1f.Type1Font(_path(file)).with_encoding(dict(encoding))
    return {
        "parts": [_digest(part) for part in font.parts],
        "decrypted": _digest(font.decrypted),
        "encoding": sorted([code, name] for code, name in font.prop["Encoding"].items()),
    }


ENCODINGS = [
    [[65, "Gamma"], [66, "Delta"], [0, ".notdef"]],
    [[i, name] for i, name in enumerate(["A", "B", "C", "a", "b", "c", "space"])],
]


def test_with_encoding():
    for file in FILES:
        for encoding in ENCODINGS:
            probes.probe("_type1font.Type1Font.with_encoding", _with_encoding, file=file,
                         encoding=encoding)


def _subset(file, chars, prefix):
    font = t1f.Type1Font(_path(file)).subset([ord(c) for c in chars], prefix)
    return {
        "cleartext": _text(font.parts[0]),
        "parts": [_digest(part) for part in font.parts],
        "decrypted": _digest(font.decrypted),
        "font_name": font.prop["FontName"],
        "encoding": sorted([code, name] for code, name in font.prop["Encoding"].items()),
        "glyphs": sorted(font.prop["CharStrings"]),
        "subrs": [_digest(code) for code in font.prop["Subrs"]],
    }


SUBSETS = [
    ("Hello World", "ABCDEF+"),
    ("", "XYZXYZ+"),
    ("\x00\x05\x0b\x7f", ""),
    ("The quick brown fox jumps over the lazy dog 0123456789", "AAAAAA+"),
    ("éèĀ", "BBBBBB+"),
]


def test_subset():
    for file in FILES:
        for chars, prefix in SUBSETS:
            probes.probe("_type1font.Type1Font.subset", _subset, file=file, chars=chars,
                         prefix=prefix)


def _pfa(private):
    plaintext = b"/FontName /X def\n/FontBBox [0 0 1 1] def\n" + private.encode("latin-1")
    enc = t1f.Type1Font._encrypt(plaintext, "eexec").hex().encode()
    data = (b"%!PS-AdobeFont-1.0: X 001.000\neexec\n" + enc + b"\n" + b"0" * 512
            + b"\ncleartomark\n")
    with tempfile.NamedTemporaryFile(suffix=".pfa") as fh:
        fh.write(data)
        fh.flush()
        return _summary(t1f.Type1Font(fh.name))


NOTDEF = "/.notdef 5 RD \x00\x01\x02\x03\x04 ND\n"
CHARSTRINGS = "/CharStrings 1 begin\n" + NOTDEF + "end\n"


def _subrs(indices):
    return ("/Subrs 2 array\n"
            + "".join(f"dup {i} 5 RD \x00\x01\x02\x03\x04 NP\n" for i in indices)
            + "ND\n" + CHARSTRINGS)


PRIVATE = [
    # test_Subrs_no_preallocation
    "/Subrs 5000000 array\n",
    # test_Subrs_indices and test_Subrs_bad_indices
    _subrs((0, 1)), _subrs((0, 5)), _subrs((0, 0)), _subrs((-1, 1)),
    # test_incomplete_sections
    "/CharStrings 1 begin\n" + NOTDEF,
    "/CharStrings 1\n",
    "/CharStrings 1 begin\n",
    "/Encoding 1 array\ndup 0 /.notdef put\n",
    "/OtherSubrs [ {} {} \n",
    # More shapes of fonts.
    CHARSTRINGS,
    "/lenIV -1 def\n" + CHARSTRINGS,
    "/lenIV 2 def\n" + CHARSTRINGS,
    "/Weight (Bold) def /FullName (X Bold Italic) def /ItalicAngle -12.5 def\n" + CHARSTRINGS,
    "/FamilyName (Fam) def /isFixedPitch true def /Notice (a\\050b\\)c\\\\d\\101) def\n"
    + CHARSTRINGS,
    "/UniqueID 99 def /Encoding StandardEncoding def\n" + CHARSTRINGS,
    "/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\n"
    "dup 65 /A put dup x /y put dup 66 7 put dup 67.0 /C put\nreadonly def\n" + CHARSTRINGS,
    "/X <0a1b2c> def /Y [1 [2] {3}] def /Z 1.5e3 def /W true def\n" + CHARSTRINGS,
    "/-| {string currentfile exch readstring pop} executeonly def\n"
    "/|- {noaccess def} def /| {noaccess put} def\n"
    "/CharStrings 1 begin\n/.notdef 5 -| \x00\x01\x02\x03\x04 |-\nend\n",
    "/OtherSubrs [{} {} {} {systemdict /internaldict known not {pop 3} {1183615869 "
    "systemdict /internaldict get exec dup /startlock known {/startlock get exec} {dup "
    "/strtlck known {/strtlck get exec} {pop 3} ifelse} ifelse} ifelse} executeonly] ND\n"
    + CHARSTRINGS,
    # Malformed fonts.
    "",
    "/FontBBox {0 0 1} def\n" + CHARSTRINGS,
    "/FontBBox /x def\n" + CHARSTRINGS,
    "/FontBBox 3 def\n" + CHARSTRINGS,
    "/CharStrings 1 begin\n/.notdef x RD \x00 ND\nend\n",
    "/CharStrings 1 begin\n/.notdef 5 XX \x00\x01\x02\x03\x04 ND\nend\n",
    "/CharStrings 1 begin\n/.notdef -6 RD x end\n",
    "/CharStrings 1e5 begin\n" + NOTDEF + "end\n",
    "/Subrs x array\n" + CHARSTRINGS,
    "/Subrs 1 array\ndup x 5 RD \x00\x01\x02\x03\x04 NP\nND\n" + CHARSTRINGS,
    "/lenIV 1.5 def\n" + CHARSTRINGS,
    "/X (unterminated def\n" + CHARSTRINGS,
    "/X <01g2> def\n" + CHARSTRINGS,
    "] " + CHARSTRINGS,
    "[ 1 2 { 3 ] " + CHARSTRINGS,
    "/X [ 1 2 ",
]


def test_pfa_fonts():
    """``test_Subrs_no_preallocation``, ``test_Subrs_indices``,
    ``test_Subrs_bad_indices`` and ``test_incomplete_sections``, and more
    shapes of fonts."""
    for private in PRIVATE:
        probes.probe("_type1font.Type1Font.pfa", _pfa, private=private)


def _tokens(data, skip_ws):
    return [[token.kind, _value(token.value())]
            for token in t1f._tokenize(data.encode("latin-1"), skip_ws)]


TOKENIZE = [
    "1234/abc false -9.81  Foo <<[0 1 2]<0 1ef a\t>>>\n(string with(nested\t\\) par)ens\\\\)",
    "1234 (this (string) is unterminated\\)",
    "/Foo<01234",
    "/Foo<01234abcg>/Bar",
    "//x ///y / ) > % comment\n{ } 1e5x 0x10 .5 5. +3 -.25e-2 1_000 |- \x0b7",
    "1e5",
    "inf",
    "(\\n\\r\\t\\b\\f\\101\\7\\x\\\\\\(\\)) (a(b)c) () <> < 0 > (éÿ)",
    "(trailing \\",
]


def test_tokenize():
    """``test_tokenize`` and ``test_tokenize_errors``."""
    for data in TOKENIZE:
        for skip_ws in (False, True):
            probes.probe("_type1font._tokenize", _tokens, data=data, skip_ws=skip_ws)


def _encrypt(data, key):
    raw = data.encode("latin-1")
    encrypted = t1f.Type1Font._encrypt(raw, key)
    return {"encrypted": encrypted.hex(),
            "decrypted": t1f.Type1Font._decrypt(encrypted, key) == raw}


def test_encrypt_decrypt_roundtrip():
    for data in ["this is my plaintext \0\1\2\3", "", "\xff" * 9]:
        for key in ("eexec", "charstring"):
            probes.probe("_type1font.Type1Font._encrypt", _encrypt, data=data, key=key)


def _ft2font_path(file, effects):
    """The font file FreeType reads: the fixture, or the fixture transformed
    by ``_type1font`` and written as a PFB file."""
    if effects is None:
        return _path(file), None
    font = t1f.Type1Font(_path(file)).transform(effects)
    segments = b"".join(struct.pack("<BBi", 0x80, kind, len(data)) + data
                        for kind, data in zip((1, 2, 1), font.parts))
    tmp = tempfile.NamedTemporaryFile(suffix=".pfb")
    tmp.write(segments + b"\x80\x03")
    tmp.flush()
    return tmp.name, tmp


def _attrs(file):
    font = FT2Font(_path(file))
    return {
        "postscript_name": font.postscript_name,
        "family_name": font.family_name,
        "style_name": font.style_name,
        "num_glyphs": font.num_glyphs,
        "fixed_width": ft2font.FaceFlags.FIXED_WIDTH in font.face_flags,
        "style_flags": {"italic": ft2font.StyleFlags.ITALIC in font.style_flags,
                        "bold": ft2font.StyleFlags.BOLD in font.style_flags},
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
    for file in FILES:
        probes.probe("ft2font.FT2Font.attrs", _attrs, file=file)


def _load_glyph(file, effects, glyph):
    path, tmp = _ft2font_path(file, effects)
    try:
        font = FT2Font(path)
        loaded = font.load_glyph(glyph, LoadFlags.NO_SCALE)
        vertices, codes = font.get_path()
        return {
            "name": font.get_glyph_name(glyph),
            "advance": loaded.horiAdvance,
            "bbox": loaded.bbox,
            "path": [[int(code), round(x * 64), round(y * 64)]
                     for (x, y), code in zip(vertices.tolist(), codes.tolist())],
        }
    finally:
        if tmp is not None:
            tmp.close()


def test_glyph_outlines():
    """Every glyph of both fixtures as FreeType loads it, and every fifth
    glyph of transformed fonts."""
    for file in FILES:
        count = FT2Font(_path(file)).num_glyphs
        for glyph in range(count):
            probes.probe("ft2font.FT2Font.load_glyph", _load_glyph, file=file, effects=None,
                         glyph=glyph)
        for effects in ({"slant": 0.167}, {"extend": 0.8}, {"slant": -0.3, "extend": 1.25}):
            for glyph in range(0, count, 5):
                probes.probe("ft2font.FT2Font.load_glyph", _load_glyph, file=file,
                             effects=effects, glyph=glyph)


def _load_sized(file, effects, size, dpi, flags, glyph):
    path, tmp = _ft2font_path(file, effects)
    try:
        font = FT2Font(path)
        font.set_size(size, dpi)
        loaded = font.load_glyph(glyph, getattr(LoadFlags, flags))
        vertices, codes = font.get_path()
        return {
            "advance": loaded.horiAdvance,
            "path": [[int(code), round(x * 64), round(y * 64)]
                     for (x, y), code in zip(vertices.tolist(), codes.tolist())],
        }
    finally:
        if tmp is not None:
            tmp.close()


SIZES = [(12, 72, "DEFAULT"), (7, 72, "TARGET_LIGHT"), (10, 100, "NO_HINTING"),
         (2500, 72, "TARGET_LIGHT")]


def test_sized_glyphs():
    """Glyphs at sizes, hinted by FreeType's Adobe engine: every third glyph
    at 100 points and 72 dpi with ``TARGET_LIGHT`` (``textpath``'s usetex
    size), and every eleventh at other sizes, flags and effects."""
    for file in FILES:
        count = FT2Font(_path(file)).num_glyphs
        for glyph in range(0, count, 3):
            probes.probe("ft2font.FT2Font.load_glyph.sized", _load_sized, file=file,
                         effects=None, size=100, dpi=72, flags="TARGET_LIGHT", glyph=glyph)
        for glyph in range(1, count, 11):
            for size, dpi, flags in SIZES:
                probes.probe("ft2font.FT2Font.load_glyph.sized", _load_sized, file=file,
                             effects=None, size=size, dpi=dpi, flags=flags, glyph=glyph)
            probes.probe("ft2font.FT2Font.load_glyph.sized", _load_sized, file=file,
                         effects={"slant": 0.167}, size=100, dpi=72, flags="TARGET_LIGHT",
                         glyph=glyph)


def test_glyph_lookup():
    for file in FILES:
        for name in ["Gamma", "A", "a", ".notdef", "suppress", "space", "nonexistent", ""]:
            probes.probe("ft2font.FT2Font.get_name_index",
                         lambda file, name: FT2Font(_path(file)).get_name_index(name),
                         file=file, name=name)
        probes.probe("ft2font.FT2Font._get_type1_encoding_vector",
                     lambda file: list(FT2Font(_path(file))._get_type1_encoding_vector()),
                     file=file)
