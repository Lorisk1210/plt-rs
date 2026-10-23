"""Font matching, ported from matplotlib's own ``tests/test_font_manager.py``.

Area ``mpl_font_manager``. Every op runs under the reference rcParams
(``matplotlib.testing.setup()`` plus ``text.hinting = 'none'``) with only
matplotlib's bundled fonts installed (``spec_support.fonts.
bundled_fonts_only``), so lookups do not depend on the host's fonts.

Shared encodings
----------------
``prop``: a font dict (``FontProperties()`` followed by the setters
``family``, ``style``, ``variant``, ``weight``, ``stretch``, ``size`` for
the keys present), ``null`` for ``FontProperties()``, or a string, parsed
as a fontconfig pattern by ``FontProperties(pattern)``. ``rc``: rcParams
applied with ``rc_context`` around the op (lists as lists).

Ops
---
``FontManager.score_weight``  {weight1, weight2} -> ``fontManager.
    score_weight(weight1, weight2)`` (0 for the same weight name, else
    ``0.95 * |w1 - w2| / 1000 + 0.05`` of the normalised weights).
``font_manager._normalize_weight``  {weight} -> the numeric weight of a
    weight name (numbers pass through); ``KeyError`` for unknown names.
``font_manager.findfont``  {prop, rc?} -> the basename of
    ``fontManager.findfont(prop)``.
``font_manager.FontProperties``  {kw} -> ``FontProperties(**kw)`` read
    back: ``{family, style, variant, weight, stretch, size,
    math_fontfamily, pattern}`` (``size`` in points, ``pattern`` the
    fontconfig pattern).
``font_manager.FontManager.get_font_names``  {} -> ``sorted(fontManager.
    get_font_names())``: the family names of the TrueType font list
    (bundled fonts only).
``font_manager.get_font``  {path} -> the basename of ``get_font(path).
    fname``; ``FileNotFoundError`` for a missing file.
``ft2font.FT2Font.get_charmap``  {file, codes} -> ``{"size": len(cmap),
    "glyphs": [cmap[code] for code in codes]}`` with ``cmap =
    FT2Font(<mpl-data>/fonts/ttf/<file>).get_charmap()``, the character
    codes of the selected character map that map to a glyph other than 0.
``font_manager._get_font_alt_names``  {sfnt, primary} -> the ``[name,
    weight]`` pairs of ``_get_font_alt_names(font, primary)`` for a font
    whose ``get_sfnt()`` returns ``sfnt`` (a list of ``[platform_id,
    encoding_id, language_id, name_id, text]`` records, the text encoded
    as Latin-1 for platform 1 and UTF-16BE otherwise) or raises
    ``ValueError`` (``sfnt`` null, a font without an sfnt table).

Cases
-----
Each probe is named after the upstream test it mirrors in a comment.
Adaptations: ``test_find_invalid`` probes one missing path (the ``str``,
``bytes`` and ``Path`` spellings are Python types);
``test_json_serialization`` probes the three lookups it compares (plt-rs
keeps no JSON font cache); ``test_missing_family``,
``test_font_match_warning``, ``test_mutable_fontproperty_cache_invalidation``
and ``test_fontproperty_default_cache_invalidation`` probe the fonts found
(plt-rs logs nothing); ``test_get_font_names`` probes the names of the
bundled fonts; ``test_addfont_alternative_names`` and
``test_alt_name_weight_from_subfamily`` probe ``_get_font_alt_names``
(the ``addfont`` half is a unit test of plt-font: alternative entries keep
the primary entry's file, style and stretch); of
``test_fontproperties_init_deprecation`` only the calls that are still
accepted are probed.

Left out
--------
``test_font_path`` (the Python object protocol of ``FontPath``),
``test_otf`` (a deprecated function), ``test_get_fontconfig_fonts`` and
``test_user_fonts_linux`` (fontconfig's ``fc-list``),
``test_user_fonts_win32`` (Windows), ``test_utf16m_sfnt``,
``test_find_ttc`` and ``test_find_noto`` (fonts that are not bundled),
``test_find_valid`` (Python path types), ``test_addfont_as_path``
(a test font file outside the bundled set), ``test_fork``,
``test_fontcache_thread_safe`` and ``test_lockfilefailure`` (processes,
threads and cache files of the Python package), ``test_fontentry_dataclass*``
(Jupyter representations), ``test_donot_cache_tracebacks`` (Python garbage
collection).
"""

import os

import matplotlib
import pytest
from matplotlib import cbook, font_manager, ft2font
from matplotlib.font_manager import FontProperties, fontManager
from unittest.mock import MagicMock

from spec_support import ProbeSet
from spec_support.fonts import bundled_fonts_only

probes = ProbeSet("mpl_font_manager")

FONT_SETTERS = ("family", "style", "variant", "weight", "stretch", "size")


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    with bundled_fonts_only():
        yield


def _prop(spec):
    if isinstance(spec, str):
        return FontProperties(spec)
    fp = FontProperties()
    for key in FONT_SETTERS:
        if key in (spec or {}):
            getattr(fp, "set_" + key)(spec[key])
    return fp


def _findfont(prop, rc=None):
    with matplotlib.rc_context(rc or {}):
        return os.path.basename(fontManager.findfont(_prop(prop)))


def findfont(prop, **args):
    return probes.probe("font_manager.findfont", _findfont, prop=prop, **args)


def _font_properties(kw):
    fp = FontProperties(**kw)
    return {
        "family": fp.get_family(),
        "style": fp.get_style(),
        "variant": fp.get_variant(),
        "weight": fp.get_weight(),
        "stretch": fp.get_stretch(),
        "size": fp.get_size_in_points(),
        "math_fontfamily": fp.get_math_fontfamily(),
        "pattern": fp.get_fontconfig_pattern(),
    }


def test_score_weight():
    for w1, w2 in [("regular", "regular"), ("bold", "bold"), (400, 400), ("normal", "bold"),
                   ("normal", "regular"), (300, "bold"), ("light", 200)]:
        probes.probe("FontManager.score_weight", fontManager.score_weight,
                     weight1=w1, weight2=w2)


def test_normalize_weights():
    for weight in [300, "ultralight", "light", "normal", "regular", "book", "medium", "roman",
                   "semibold", "demibold", "demi", "bold", "heavy", "extra bold", "black",
                   "invalid"]:
        probes.probe("font_manager._normalize_weight", font_manager._normalize_weight,
                     weight=weight)


def test_font_priority():
    findfont({"family": ["sans-serif"]},
             rc={"font.sans-serif": ["cmmi10", "Bitstream Vera Sans"]})


def _get_charmap(file, codes):
    cmap = ft2font.FT2Font(cbook._get_data_path("fonts", "ttf", file)).get_charmap()
    return {"size": len(cmap), "glyphs": [cmap[code] for code in codes]}


def test_get_charmap():
    # test_font_priority's smoke test of the font it finds.
    probes.probe("ft2font.FT2Font.get_charmap", _get_charmap, file="cmmi10.ttf", codes=[8729])
    probes.probe("ft2font.FT2Font.get_charmap", _get_charmap, file="DejaVuSans.ttf",
                 codes=[65, 8722])


def test_json_serialization():
    for prop in ({"family": "STIXGeneral"}, {"family": "Bitstream Vera Sans", "weight": 700},
                 {"family": "no such font family"}):
        findfont(prop)


def test_find_invalid():
    probes.probe("font_manager.get_font",
                 lambda path: os.path.basename(font_manager.get_font(path).fname),
                 path="/nonexistent-plt-rs-dir/non-existent-font-name.ttf")


def test_missing_family():
    findfont("sans", rc={"font.sans-serif": ["this-font-does-not-exist"]})


def test_font_match_warning():
    findfont({"family": ["DejaVu Sans"], "weight": 750})


def test_mutable_fontproperty_cache_invalidation():
    findfont(None)
    findfont({"weight": "bold"})


def test_fontproperty_default_cache_invalidation():
    for weight in ("normal", "bold"):
        findfont("DejaVu Sans", rc={"font.weight": weight})


def test_get_font_names():
    probes.probe("font_manager.FontManager.get_font_names",
                 lambda: sorted(fontManager.get_font_names()))


def _alt_names(sfnt, primary):
    font = MagicMock()
    if sfnt is None:
        font.get_sfnt.side_effect = ValueError
    else:
        font.get_sfnt.return_value = {
            (platform, encoding, language, name_id):
                text.encode("latin-1" if platform == 1 else "utf-16-be")
            for platform, encoding, language, name_id, text in sfnt
        }
    return [list(pair) for pair in font_manager._get_font_alt_names(font, primary)]


MAC = [1, 0, 0]
MS = [3, 1, 0x0409]


def alt_names(sfnt, primary):
    probes.probe("font_manager._get_font_alt_names", _alt_names, sfnt=sfnt, primary=primary)


def test_addfont_alternative_names():
    ubuntu_style = [[*MAC, 1, "Test Family"], [*MS, 1, "Test Family Light"],
                    [*MAC, 2, "Light"], [*MS, 2, "Regular"]]
    alt_names(ubuntu_style, "Test Family")
    alt_names(ubuntu_style, "Test Family Light")
    alt_names([[*MAC, 1, "Test Family"], [*MS, 1, "Test Family"],
               [*MS, 16, "Test Family Light"]], "Test Family")
    alt_names([[*MAC, 1, "Test Family"], [*MS, 1, "Test Family"]], "Test Family")
    alt_names(None, "Test Family")
    # WWS names, blank and padded names, and the Mac record winning a tie.
    alt_names([[*MAC, 21, "  Wws Family  "], [*MAC, 22, "Bold Italic"],
               [*MS, 16, " "], [*MS, 1, "Mac And Ms"], [*MAC, 1, "Mac And Ms"],
               [*MAC, 2, "Black"], [*MS, 2, "Thin"]], "Primary")


def test_alt_name_weight_from_subfamily():
    for subfamily in ["Thin", "ExtraLight", "UltraLight", "DemiLight", "SemiLight", "Light",
                      "Book", "Regular", "Normal", "Medium", "DemiBold", "Demi", "SemiBold",
                      "ExtraBold", "SuperBold", "UltraBold", "Bold", "UltraBlack", "SuperBlack",
                      "ExtraBlack", "Ultra", "Black", "Heavy", "", "Semi Bold Italic",
                      "Hyperbold"]:
        alt_names([[*MS, 1, "Family Alt"], [*MS, 2, subfamily]], "Family")


def test_fontproperties_init_deprecation():
    # The calls that are still accepted.
    for kw in ({"family": "Times", "style": "italic"}, {"family": "Times"},
               {"family": "serif-24:style=oblique:weight=bold"}):
        probes.probe("font_manager.FontProperties", _font_properties, kw=kw)
