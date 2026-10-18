"""OpenType features and languages of text: layout and extents under
``features``/``language``, HarfBuzz's feature syntax, BCP 47 languages to
OpenType language systems, language ranges, and ``Text``'s properties.

Area ``text_shaping`` (golden ``spec/goldens/probes/text_shaping.json``).

Environment every op assumes
============================
* Reference rcParams (``matplotlib.testing.setup()`` plus ``text.hinting =
  'none'``); only matplotlib's bundled fonts are installed (as in
  ``test_text_metrics.py``).
* HarfBuzz's default language comes from the C locale; the reference runs
  in ``en_IE``, whose OpenType tag (``ENG``) no bundled font has, so text
  without a language uses each font's default language system.

Shared argument encodings
=========================
``file``: basename of a font in ``mpl-data/fonts/ttf``, opened as
``FT2Font(path)`` (no fallback fonts) and sized with ``font.set_size(size,
dpi)``; ``size`` defaults to 12, ``dpi`` to 72.

``features``: list of HarfBuzz feature strings (``hb_feature_from_string``)
or null.  ``language``: a BCP 47 string, a list of ``[language, start,
end]`` (character indices; passed as tuples) or null.  Both are passed to
``FT2Font`` unchanged; strings HarfBuzz cannot parse and language ranges
libraqm refuses (start at or past the end of a non-empty text, end past
it) raise ``RuntimeError``.

Ops
===
``ft2font.FT2Font._layout`` -- args ``{file, text, features?, language?,
size?, dpi?}``.  ``font._layout(text, LoadFlags.NO_HINTING,
features=features, language=language)``; returns ``[[glyph_index, x, y],
...]`` per laid-out glyph with ``x``, ``y`` the glyph position in 1/64
pixels (``round(item.x * 64)``).

``ft2font.FT2Font.set_text`` -- same args.  ``font.set_text(text, 0.0,
flags=LoadFlags.NO_HINTING, features=features, language=language)``;
returns ``[width, height, descent]`` in 1/64 pixels
(``font.get_width_height()`` and ``font.get_descent()``).

``text.Text.get_window_extent`` -- args ``{text, ha?, features?, language?,
dpi?}``.  ``t = fig.text(0.5, 0.5, text, ha=ha)`` on a 6.4 x 4.8 inch
figure (``FigureCanvasAgg``) at ``dpi`` (default 100), then
``t.set_fontfeatures(features)`` and ``t.set_language(language)`` for the
keys present; returns the window extent ``[[x0, y0], [x1, y1]]`` (which
never depends on the features or the language: text is measured without
them).

``text.Text.get_language`` -- args ``{rc_language, set?, language?}``.
Under ``rc_context({'text.language': rc_language})``: ``t =
fig.text(0, 0, 'x')``, then ``t.set_language(language)`` if ``set`` is
true; returns ``t.get_language()`` (a string, a list of ``[language,
start, end]`` or null).
"""

from __future__ import annotations

import matplotlib
import pytest
from matplotlib import cbook
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.ft2font import FT2Font, LoadFlags

from spec_support import ProbeSet
from spec_support.fonts import bundled_fonts_only

probes = ProbeSet("text_shaping")

pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

SANS = "DejaVuSans.ttf"
SERIF_ITALIC = "DejaVuSerif-Italic.ttf"
STIX = "STIXGeneral.ttf"


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    with bundled_fonts_only():
        yield


# --------------------------------------------------------------------------
# Adapters
# --------------------------------------------------------------------------

def _font(file, size, dpi):
    font = FT2Font(cbook._get_data_path("fonts", "ttf", file))
    font.set_size(size, dpi)
    return font


def _language(language):
    if isinstance(language, list):
        return [tuple(item) for item in language]
    return language


def _layout(file, text, features=None, language=None, size=12, dpi=72):
    items = _font(file, size, dpi)._layout(
        text, LoadFlags.NO_HINTING, features=features, language=_language(language))
    return [[item.glyph_index, round(item.x * 64), round(item.y * 64)] for item in items]


def _set_text(file, text, features=None, language=None, size=12, dpi=72):
    font = _font(file, size, dpi)
    font.set_text(text, 0.0, flags=LoadFlags.NO_HINTING, features=features,
                  language=_language(language))
    width, height = font.get_width_height()
    return [width, height, font.get_descent()]


def _window_extent(text, ha="left", dpi=100, **props):
    fig = Figure(figsize=(6.4, 4.8), dpi=dpi)
    renderer = FigureCanvasAgg(fig).get_renderer()
    t = fig.text(0.5, 0.5, text, ha=ha)
    if "features" in props:
        t.set_fontfeatures(props["features"])
    if "language" in props:
        t.set_language(_language(props["language"]))
    return t.get_window_extent(renderer).get_points()


def _get_language(rc_language, set=False, language=None):
    with matplotlib.rc_context({"text.language": rc_language}):
        t = Figure().text(0, 0, "x")
        if set:
            t.set_language(_language(language))
        return t.get_language()


def layout(file, text, **args):
    assert set(args) <= {"features", "language", "size", "dpi"}
    return probes.probe("ft2font.FT2Font._layout", _layout, file=file, text=text, **args)


def extents(file, text, **args):
    assert set(args) <= {"features", "language", "size", "dpi"}
    return probes.probe("ft2font.FT2Font.set_text", _set_text, file=file, text=text, **args)


# --------------------------------------------------------------------------
# Features: HarfBuzz's syntax and what the features do
# --------------------------------------------------------------------------

KERNED = "AVAVA office fi"

FEATURE_STRINGS = [
    "kern", "+kern", "-kern", "kern=0", "kern=1", "kern=off", "kern=OFF", "kern=On", "kern 0",
    "-kern on", "kern xyz", "-kern xyz", "kern=xyz", "kern=", "kern=1.5", "kern=-1", "kern=+0",
    "kern= 0", "kern =0", " -kern ", "- kern", "'kern'=0", '"kern"=0', "'ker'", "'kern",
    "-kern[]", "-kern[:]", "-kern[2:]", "-kern[:2]", "-kern[1:3]", "-kern[2]", "-kern[;3]",
    "-kern[2:4", "-kern[-1]", "-kern[2:1]", "-liga[6:9]", "-liga[7]", "-liga[13:]", "liga=0",
    "-liga", "kern=4294967296", "kern=99999999999999999999", "", "  ", "toolong", "kern\t",
    "ke rn", "kernx", "kern]", "=1", "kern==1", "kern 1 2", "dlig", "hlig", "salt", "aalt",
    "aalt=2", "case", "ss01", "zzzz",
]


def test_feature_strings():
    plain = layout(SANS, KERNED)
    unkerned = layout(SANS, KERNED, features=["-kern"])
    assert plain != unkerned
    for feature in FEATURE_STRINGS:
        layout(SANS, KERNED, features=[feature])
    assert isinstance(layout(SANS, KERNED, features=["kern=x"]), Exception)
    assert layout(SANS, KERNED, features=["kern xyz"]) == plain


def test_feature_lists():
    for features in [
        [], ["-kern", "kern"], ["kern", "-kern"], ["-kern", "kern[1:3]"], ["kern[1:3]=0"],
        ["kern[1:3]=0", "kern"], ["-liga", "liga[6:9]"], ["-kern", "-liga"],
        ["-kern", "bad=feature"],
    ]:
        layout(SANS, KERNED, features=features)


def test_features_change_glyphs_in_bundled_fonts():
    for file, text, features in [
        (SANS, "first stop", ["dlig"]),
        (SANS, "¡HOLA! (¿QUÉ?)", ["case"]),
        (SANS, "g æ Đ đ", ["salt"]),
        (SERIF_ITALIC, "g æ Đ đ ǥ", ["salt"]),
        (SERIF_ITALIC, "gag g", ["aalt=2"]),
        (SERIF_ITALIC, "first fist", ["dlig"]),
        (SERIF_ITALIC, "¡fi! ¿ffl?", ["case", "-liga"]),
        (STIX, "fi fl ffi ffl AV To", []),
        (STIX, "fi fl ffi ffl AV To", ["-liga", "-kern"]),
    ]:
        layout(file, text)
        layout(file, text, features=features)
        extents(file, text)
        extents(file, text, features=features)


# --------------------------------------------------------------------------
# Languages: BCP 47 to OpenType language systems
# --------------------------------------------------------------------------

#: Latin and Cyrillic letters DejaVu Serif Italic's language systems treat
#: differently: ``case`` (requested) for the inverted marks, the ``fi``
#: ligature Turkic languages drop, ``ffl``, the Sami form of Eng, and the
#: Serbian/Macedonian forms of the Cyrillic letters.
LANGUAGE_SAMPLE = "¡fi ffl Ŋ бгдпт"

LANGUAGES = [
    # two-letter codes
    "tr", "az", "kk", "tt", "ca", "es", "gl", "se", "mo", "ro", "sr", "mk", "sh", "en", "de",
    "nl", "qq", "TR", "Tr_tR", "tr-TR", "sr-Latn", "sr-Cyrl-RS", "tr@euro", "TR_TR.UTF-8",
    # three-letter codes: tables, several tags, blocked, upper-cased
    "tur", "aze", "azj", "azb", "crh", "gag", "kaz", "kaa", "tat", "cat", "spa", "glg", "smn",
    "sjd", "smj", "sma", "sms", "sme", "mol", "ron", "srp", "mkd", "cnr", "nld",
    "trk", "crt", "gal", "krk", "ism", "ksm", "nsm", "sks", "ssm", "srb",
    # extended language subtags
    "zh-yue", "ab-cat", "ab-trk", "ab-tur", "abc-cat",
    # multi-subtag languages
    "ro-MD", "ro-MD-x-y", "zh-Hant-HK", "art-lojban", "el-polyton", "und-fonipa", "sr-u-ca-x-a",
    # private-use overrides
    "x-hbottrk", "x-hbotTRK", "x-hbot-54524b20", "en-x-hbotcat", "x-hbot-5452", "x-hbotdflt",
    "x-foo", "tr-x-foo", "tr-x-hbsccyrl", "sr-x-hbsclatn", "x-hbscdflt", "sr-x-hbscdflt",
    "tr-u-ca-x-hbsclatn", "en-u-ab-x-hbotcat", "x-hbsc-6379726c-hbotsrb", "ca-x-hbsc",
    # no language system
    "", "@", " tr", "x", "toolong",
]


def test_languages():
    default = layout(SERIF_ITALIC, LANGUAGE_SAMPLE, features=["case"])
    seen = set()
    for language in LANGUAGES:
        result = layout(SERIF_ITALIC, LANGUAGE_SAMPLE, features=["case"], language=language)
        seen.add(str(result))
    assert str(default) in seen and len(seen) >= 6
    for language in ["ca", "es", "gl", "se", "smn", "mo", "sr", "x-hbotcat", ""]:
        layout(SANS, LANGUAGE_SAMPLE, language=language)


def test_language_extents():
    for file, text, language in [
        (SERIF_ITALIC, "бгдпт", "sr"),
        (SERIF_ITALIC, "бгдпт", "mk"),
        (SERIF_ITALIC, "fi Ŋ", "tr"),
        (SERIF_ITALIC, "fi Ŋ", "se"),
        (SANS, "office", "ca"),
    ]:
        extents(file, text)
        extents(file, text, language=language)


# --------------------------------------------------------------------------
# Language ranges
# --------------------------------------------------------------------------

SERBIAN = "бгдпт"


def test_language_ranges():
    whole = layout(SERIF_ITALIC, SERBIAN, language="sr")
    assert whole == layout(SERIF_ITALIC, SERBIAN, language=[["sr", 0, 5]])
    for ranges in [
        [["sr", 1, 3]], [["sr", 0, 4], ["ru", 2, 5]], [["ru", 0, 5], ["sr", 4, 5]],
        [["sr", 3, 1]], [["sr", 0, 0]], [], [["", 0, 5]], [["SR", 0, 2], ["sr", 2, 5]],
        [["sr", 0, 6]], [["sr", 5, 5]], [["sr", 7, 9]], [["sr", 0, 2], ["sr", 2, 9]],
        [["sr", -1, 2]],
    ]:
        layout(SERIF_ITALIC, SERBIAN, language=ranges)
        extents(SERIF_ITALIC, SERBIAN, language=ranges)
    layout(SERIF_ITALIC, "", language=[["sr", 0, 9]])
    layout(SERIF_ITALIC, "", language="sr")
    # Languages split runs, which stops ligatures and kerning across them.
    for ranges in [[["en", 0, 2]], [["@", 0, 2]], [["", 0, 2]], [["ca", 2, 4]],
                   [["en", 0, 2], ["en", 2, 6]]]:
        layout(SANS, "office AV", language=ranges)
    # Mixed-script text with per-script languages.
    layout(SERIF_ITALIC, "fi бгд fi", language=[["tr", 0, 3], ["sr", 3, 7]])


# --------------------------------------------------------------------------
# Text properties
# --------------------------------------------------------------------------

def test_text_extents_ignore_features_and_language():
    for text, ha in [("AVAVA office fi", "right"), ("бгдпт fi", "center")]:
        base = probes.probe("text.Text.get_window_extent", _window_extent, text=text, ha=ha)
        for props in [{"features": ["-kern", "-liga"]}, {"language": "sr"},
                      {"language": [["sr", 0, 3]]}, {"features": ["dlig"], "language": "ca"}]:
            ext = probes.probe("text.Text.get_window_extent", _window_extent, text=text, ha=ha,
                               **props)
            assert ext.tolist() == base.tolist()


def test_text_language_property():
    for args in [
        {"rc_language": None},
        {"rc_language": "tr"},
        {"rc_language": "tr", "set": True, "language": "sr"},
        {"rc_language": "tr", "set": True, "language": None},
        {"rc_language": None, "set": True, "language": None},
        {"rc_language": None, "set": True, "language": [["sr", 0, 2], ["ru", 1, 3]]},
        {"rc_language": "sr", "set": True, "language": []},
    ]:
        probes.probe("text.Text.get_language", _get_language, **args)
