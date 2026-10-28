"""Adobe Font Metrics, ported from matplotlib's own ``tests/test_afm.py``
with the tests' own font data.

Area ``mpl_afm``.

Ops
---
``_afm.AFM.parse``  {data, encoding?} -> what ``AFM(io.BytesIO(data.encode(
    encoding)))`` (``encoding`` ``"latin-1"`` by default) reads:
    ``{"fontname", "fullname", "familyname", "weight", "angle",
    "underline_thickness"}`` (``null`` where the getter raises
    ``KeyError``), and the character metrics ``[width, name, bbox]`` the
    parser found for every code of ``codes`` (``"chars"``) and every name
    of ``names`` (``"by_name"``); or the error of a malformed file.
``_afm.AFM.get_kern_dist_from_name``  {file, name1, name2} -> the kerning
    of a pair in a core font's metrics (the ``afm`` area's op).

Cases
-----
Named after the upstream tests. They call the private parsers
(``_parse_header``, ``_parse_char_metrics``, ``_to_str``); the probes parse
whole files through ``AFM`` and read what those parsers found through its
getters: ``test_parse_header``, ``test_parse_char_metrics`` and
``test_bad_afm`` as they are (a header the parser rejects is rejected by
``AFM`` too), ``test_get_familyname_guessed`` with the ``FamilyName`` line
removed from the data instead of from the parsed header,
``test_malformed_header`` with the character metrics closed so that
``AFM`` reads on (its log record has no counterpart: plt-rs skips the
entry silently, as matplotlib does after logging), ``test_nonascii_str``
with a full name in UTF-8. ``test_afm_kerning`` reads the core Helvetica,
which ``findfont("Helvetica", fontext="afm")`` finds.
``test_font_manager_weight_normalization`` is a unit test of plt-font: an
AFM font whose weight keyword is unknown weighs ``normal``
(``AfmFont::weight_value``).
"""

import io

from matplotlib import _afm, font_manager

from probes.test_afm import _kern
from spec_support import ProbeSet

probes = ProbeSet("mpl_afm")

AFM_TEST_DATA = """StartFontMetrics 2.0
Comment Comments are ignored.
Comment Creation Date:Mon Nov 13 12:34:11 GMT 2017
FontName MyFont-Bold
EncodingScheme FontSpecific
FullName My Font Bold
FamilyName Test Fonts
Weight Bold
ItalicAngle 0.0
IsFixedPitch false
UnderlinePosition -100
UnderlineThickness 56,789
Version 001.000
Notice Copyright \xa9 2017 No one.
FontBBox 0 -321 1234 369
StartCharMetrics 3
C 0 ; WX 250 ; N space ; B 0 0 0 0 ;
C 42 ; WX 1141 ; N foo ; B 40 60 800 360 ;
C 99 ; WX 583 ; N bar ; B 40 -10 543 210 ;
EndCharMetrics
EndFontMetrics
"""

CODES = [0, 42, 99]
NAMES = ["space", "foo", "bar"]


def _or_none(getter):
    try:
        return getter()
    except KeyError:
        return None


def _metrics(char):
    return [char.width, char.name, list(char.bbox)]


def _parse(data, encoding="latin-1", codes=(), names=()):
    afm = _afm.AFM(io.BytesIO(data.encode(encoding)))
    return {
        "fontname": _or_none(afm.get_fontname),
        "fullname": _or_none(afm.get_fullname),
        "familyname": _or_none(afm.get_familyname),
        "weight": _or_none(afm.get_weight),
        "angle": _or_none(afm.get_angle),
        "underline_thickness": _or_none(afm.get_underline_thickness),
        "chars": [_metrics(afm._metrics[code]) for code in codes],
        "by_name": [_metrics(afm._metrics_by_name[name]) for name in names],
    }


def parse(data, **kw):
    return probes.probe("_afm.AFM.parse", _parse, data=data, **kw)


def test_nonascii_str():
    parsed = parse(AFM_TEST_DATA.replace("FullName My Font Bold", "FullName привет"),
                   encoding="utf-8")
    assert parsed["fullname"] == "привет"


def test_parse_header():
    parse(AFM_TEST_DATA)


def test_parse_char_metrics():
    parse(AFM_TEST_DATA, codes=CODES, names=NAMES)


def test_get_familyname_guessed():
    parsed = parse(AFM_TEST_DATA.replace("FamilyName Test Fonts\n", ""))
    assert parsed["familyname"] == "My Font"


def test_bad_afm():
    for data in ["nope\nreally nope",
                 "StartFontMetrics 2.0\nComment Comments are ignored.\n"
                 "Comment Creation Date:Mon Nov 13 12:34:11 GMT 2017\nFontName MyFont-Bold\n"
                 "EncodingScheme FontSpecific"]:
        assert isinstance(parse(data), RuntimeError)


def test_malformed_header():
    head = ("StartFontMetrics 2.0\nComment Comments are ignored.\n"
            "Comment Creation Date:Mon Nov 13 12:34:11 GMT 2017\n")
    tail = "FontName MyFont-Bold\nEncodingScheme FontSpecific\nStartCharMetrics 3\nEndCharMetrics\n"
    for line in ["Aardvark bob\n", "ItalicAngle zero degrees\n"]:
        parse(head + line + tail)


def test_afm_kerning():
    assert font_manager.findfont("Helvetica", fontext="afm").endswith("pdfcorefonts/Helvetica.afm")
    for name1, name2 in [("A", "V"), ("V", "A")]:
        probes.probe("_afm.AFM.get_kern_dist_from_name", _kern, file="Helvetica.afm",
                     name1=name1, name2=name2)
