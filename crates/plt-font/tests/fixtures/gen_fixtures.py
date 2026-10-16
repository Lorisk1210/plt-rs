"""Generate plt-font test fixtures from matplotlib 3.11 (FreeType + HarfBuzz).

Run from the repository root with the project venv:

    MPL_IGNORE_SYSTEM_FONTS=1 MPLCONFIGDIR=$(mktemp -d) PYTHONHASHSEED=0 \
        .venv/bin/python -I crates/plt-font/tests/fixtures/gen_fixtures.py

`MPL_IGNORE_SYSTEM_FONTS` restricts matplotlib's font manager to its bundled
fonts so that `findfont` results are comparable with plt-font's bundled-only
database. The JSON files are written next to this script; tests read them
without needing Python.
"""

import json
import os
import pathlib
import warnings

import matplotlib
from matplotlib import font_manager as fm
from matplotlib import ft2font
from matplotlib.ft2font import FT2Font, Kerning, LoadFlags

HERE = pathlib.Path(__file__).resolve().parent
FONT_DIR = pathlib.Path(matplotlib.get_data_path()) / "fonts" / "ttf"
NO_HINTING = LoadFlags.NO_HINTING

SHAPING_FONTS = [
    "DejaVuSans.ttf",
    "DejaVuSans-Bold.ttf",
    "DejaVuSans-Oblique.ttf",
    "DejaVuSerif.ttf",
    "DejaVuSerif-Italic.ttf",
    "DejaVuSansMono.ttf",
    "STIXGeneral.ttf",
    "STIXGeneralItalic.ttf",
    "cmr10.ttf",
    "cmmi10.ttf",
]

SIZES = [(12.0, 72.0), (10.0, 100.0), (10.5, 72.0), (7.3, 96.0), (100.0, 72.0)]
POSITION_SIZES = [(12.0, 72.0), (10.0, 100.0)]

CORPUS = [
    "",
    " ",
    "Hello world",
    "The quick brown fox jumps over the lazy dog.",
    "AVATAR",
    "To",
    "Wa",
    "LT",
    "Ty Yo Vo P. F, r.",
    "AVAWATAY WAVE",
    "0123456789",
    "3.14159",
    "−1.5",
    "-1.5e-10",
    "x² + y³",
    "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
    "ÀÉÎÕÜ àéîõü ÇçÑñ",
    "Straße",
    "Ωμέγα αβγδε",
    "Το AV ΑΒΓ",
    "Tω",
    "(α) [x]",
    "∑∫∂∇∞≤≥≠±×÷",
    "office fi fl ffi ffl ﬁ",
    "áê x́",
    "i̇́ j̈",
    "a­b x‍y",
    "  leading",
    "trailing  ",
    "Fallback: あ一",
    "1\u2009000 a\u2002b\u205fc\u202fd\u2007e\u2008f\u2011g\u00a0h\u3000i",
]

FEATURE_CASES = [
    ("AVATAR office", ("-kern",)),
    ("AVATAR office", ("-liga",)),
    ("AVATAR office", ("-kern", "-liga")),
]


def layout_items(font, text, features=None):
    items = []
    for item in font._layout(text, NO_HINTING, features=features):
        items.append([
            os.path.basename(item.ft_object.fname),
            int(item.glyph_index),
            round(item.x * 64),
            round(item.y * 64),
        ])
    return items


def measure(font, text, features=None):
    font.set_text(text, 0.0, flags=NO_HINTING, features=features)
    width, height = font.get_width_height()
    return [int(width), int(height), int(font.get_descent())]


def shaping_fixtures():
    cases = []
    for name in SHAPING_FONTS:
        path = str(FONT_DIR / name)
        font = fm.get_font(path)
        for size, dpi in SIZES:
            font.set_size(size, dpi)
            for text in CORPUS:
                case = {"font": name, "size": size, "dpi": dpi, "text": text, "features": []}
                case["extents"] = measure(font, text)
                if (size, dpi) in POSITION_SIZES:
                    case["glyphs"] = layout_items(font, text)
                cases.append(case)
        font.set_size(12.0, 72.0)
        for text, features in FEATURE_CASES:
            case = {"font": name, "size": 12.0, "dpi": 72.0, "text": text, "features": list(features)}
            case["extents"] = measure(font, text, features)
            case["glyphs"] = layout_items(font, text, features)
            cases.append(case)
    return cases


OUTLINE_CHARS = "AVTWaegofij0258%&@\u00c0\u00e9\u00c7\u00df\u03a9\u03b1\u03b2\u2211\u221e\u2212\ufb01\u0301\u00b2"


def glyph_sample(font):
    """Glyphs of a few characters plus a spread over the glyph range."""
    gids = {font.get_char_index(ord(char)) for char in OUTLINE_CHARS} - {0}
    count = font.num_glyphs
    step = max(1, count // (8 if count > 3000 else 20))
    gids.update(range(0, count, step))
    return sorted(gids)


def path_of(font, scale):
    """Path as a flat list `code, x, y, code, x, y, ...` with coordinates
    multiplied by `scale` (exact integers for the scales used)."""
    vertices, codes = font.get_path()
    flat = []
    for (x, y), code in zip(vertices, codes):
        flat += [int(code), round(x * scale), round(y * scale)]
    return flat


def outline_fixtures():
    out = {}
    for name in SHAPING_FONTS + ["LastResortHE-Regular.ttf", "DejaVuSansDisplay.ttf"]:
        font = FT2Font(str(FONT_DIR / name))
        upem = font.units_per_EM
        glyphs = []
        for number, gid in enumerate(glyph_sample(font)):
            # ppem == units per em: FreeType's 26.6 coordinates are exactly
            # font units times 64; implied midpoints are half units.
            font.set_size(upem, 72)
            font.load_glyph(gid, flags=NO_HINTING)
            entry = {"gid": gid, "units_x2": path_of(font, 2)}
            font.set_size(12.0, 72.0)
            glyph = font.load_glyph(gid, flags=NO_HINTING)
            entry["bbox_12"] = [int(v) for v in glyph.bbox]
            entry["linear_advance_12"] = int(glyph.linearHoriAdvance)
            if number % 4 == 0:
                entry["pixels_12_x64"] = path_of(font, 64)
            font.set_size(10.0, 100.0)
            glyph = font.load_glyph(gid, flags=NO_HINTING)
            entry["bbox_10_100"] = [int(v) for v in glyph.bbox]
            glyphs.append(entry)
        out[name] = glyphs
    return out


def os2_dict(font):
    os2 = font.get_sfnt_table("OS/2")
    if not os2:
        return None
    keys = ["version", "usWeightClass", "usWidthClass", "fsSelection", "sTypoAscender",
            "sTypoDescender", "sTypoLineGap", "usWinAscent", "usWinDescent", "sxHeight", "sCapHeight"]
    return {key: int(os2[key]) for key in keys if key in os2}


def metric_fixtures():
    out = {}
    for path in sorted(FONT_DIR.glob("*.ttf")):
        font = FT2Font(str(path))
        post = font.get_sfnt_table("post")
        sample = list(range(0, font.num_glyphs, max(1, font.num_glyphs // 40)))
        names = {}
        if font.face_flags & ft2font.FaceFlags.GLYPH_NAMES:
            names = {str(gid): font.get_glyph_name(gid) for gid in sample}
        chars = {}
        for code in [0x20, 0x41, 0x61, 0xE9, 0x3B1, 0x2212, 0x221E, 0x4E00, 0x1F600]:
            chars[str(code)] = int(font.get_char_index(code))
        out[path.name] = {
            "family_name": font.family_name,
            "style_name": font.style_name,
            "postscript_name": font.postscript_name,
            "units_per_em": font.units_per_EM,
            "ascender": font.ascender,
            "descender": font.descender,
            "height": font.height,
            "bbox": list(font.bbox),
            "num_glyphs": font.num_glyphs,
            "underline_position": font.underline_position,
            "underline_thickness": font.underline_thickness,
            "italic": bool(font.style_flags & ft2font.StyleFlags.ITALIC),
            "bold": bool(font.style_flags & ft2font.StyleFlags.BOLD),
            "italic_angle": post["italicAngle"][0] + post["italicAngle"][1] / 65536 if post else 0.0,
            "has_glyph_names": bool(font.face_flags & ft2font.FaceFlags.GLYPH_NAMES),
            "glyph_names": names,
            "chars": chars,
            "num_charmaps": font.num_charmaps,
            "os2": os2_dict(font),
        }
    return out


KERN_PAIRS = ["AV", "AT", "TA", "To", "Wa", "LT", "Yo", "AA", "Te", "P.", "r.", "F,", "VA", "ff", "xy"]


def kerning_fixtures():
    out = {}
    for name in SHAPING_FONTS:
        font = FT2Font(str(FONT_DIR / name))
        pairs = []
        for pair in KERN_PAIRS:
            left, right = (font.get_char_index(ord(c)) for c in pair)
            row = {"pair": pair, "unscaled": font.get_kerning(left, right, Kerning.UNSCALED)}
            for size, dpi in [(12.0, 72.0), (10.0, 100.0), (48.0, 72.0)]:
                font.set_size(size, dpi)
                row[f"default_{size}_{dpi}"] = font.get_kerning(left, right, Kerning.DEFAULT)
                row[f"unfitted_{size}_{dpi}"] = font.get_kerning(left, right, Kerning.UNFITTED)
            pairs.append(row)
        out[name] = pairs
    return out


FAMILIES = [
    ["sans-serif"], ["serif"], ["monospace"], ["cursive"], ["fantasy"], ["sans"],
    ["DejaVu Sans"], ["dejavu serif"], ["DejaVu Sans Mono"], ["DejaVu Sans Display"],
    ["STIXGeneral"], ["STIXSizeOneSym"], ["STIXNonUnicode"], ["cmr10"], ["cmtt10"], ["cmb10"],
    ["Computer Modern Roman"], ["Nonexistent Font"], ["Nonexistent Font", "serif"],
    ["cmr10", "DejaVu Sans"], ["Arial", "Helvetica", "sans-serif"],
]
STYLES = ["normal", "italic", "oblique"]
WEIGHTS = ["normal", "bold", "light", "semibold", 100, 900, 500, 650]
STRETCHES = ["normal", "condensed", "expanded"]


def findfont_fixtures():
    manager = fm.fontManager
    entries = [
        {"file": os.path.basename(e.fname), "index": e.index, "name": e.name, "style": e.style,
         "variant": e.variant, "weight": int(fm._normalize_weight(e.weight)), "stretch": e.stretch}
        for e in manager.ttflist
    ]
    queries = []
    for family in FAMILIES:
        for style in STYLES:
            for weight in WEIGHTS:
                stretches = STRETCHES if style == "normal" and weight == "normal" else ["normal"]
                variants = ["normal", "small-caps"] if style == "normal" and weight == "bold" else ["normal"]
                for stretch in stretches:
                    for variant in variants:
                        prop = fm.FontProperties(family=family, style=style, weight=weight,
                                                 stretch=stretch, variant=variant)
                        with warnings.catch_warnings():
                            warnings.simplefilter("ignore")
                            found = os.path.basename(manager.findfont(prop, fallback_to_default=True))
                        query_families = family
                        scores = []
                        for e in manager.ttflist:
                            score = (manager.score_family(prop.get_family(), e.name) * 10
                                     + manager.score_style(prop.get_style(), e.style)
                                     + manager.score_variant(prop.get_variant(), e.variant)
                                     + manager.score_weight(prop.get_weight(), e.weight)
                                     + manager.score_stretch(prop.get_stretch(), e.stretch)
                                     + manager.score_size(prop.get_size(), e.size))
                            scores.append((score, os.path.basename(e.fname)))
                        best = min(score for score, _ in scores)
                        ties = sorted({f for score, f in scores if score == best}) if best < 10 else []
                        queries.append({"families": query_families, "style": style, "weight": str(weight),
                                        "stretch": stretch, "variant": variant, "file": found, "ties": ties})
    return {"entries": entries, "queries": queries}


BIGRAM_FONTS = ["DejaVuSans.ttf", "DejaVuSans-Bold.ttf", "DejaVuSerif.ttf", "STIXGeneral.ttf"]
ASCII = "".join(chr(code) for code in range(32, 127))


def bigram_fixtures():
    """Widths of every printable-ASCII pair at matplotlib's default 10pt /
    100 dpi, in row-major order of `ASCII`."""
    out = {"alphabet": ASCII}
    for name in BIGRAM_FONTS:
        font = fm.get_font(str(FONT_DIR / name))
        font.set_size(10.0, 100.0)
        widths = []
        for first in ASCII:
            for second in ASCII:
                font.set_text(first + second, 0.0, flags=NO_HINTING)
                widths.append(int(font.get_width_height()[0]))
        out[name] = widths
    return out


def write(name, data):
    path = HERE / name
    path.write_text(json.dumps(data, ensure_ascii=True, separators=(",", ":")) + "\n")
    print(f"wrote {path} ({path.stat().st_size} bytes)")


def main():
    if not os.environ.get("MPL_IGNORE_SYSTEM_FONTS"):
        raise SystemExit("set MPL_IGNORE_SYSTEM_FONTS=1 (see module docstring)")
    warnings.simplefilter("ignore")
    write("metrics.json", metric_fixtures())
    write("kerning.json", kerning_fixtures())
    write("findfont.json", findfont_fixtures())
    write("shaping.json", shaping_fixtures())
    write("outlines.json", outline_fixtures())
    write("bigrams.json", bigram_fixtures())


if __name__ == "__main__":
    main()
