"""Text shaped with OpenType features and languages: ``Text.set_fontfeatures``
(HarfBuzz's feature syntax, ranges included), ``Text.set_language`` (a BCP 47
tag or character ranges) and the ``text.language`` rcParam, which every text
matplotlib creates takes. matplotlib hands both to libraqm/HarfBuzz when it
draws a single-line text; multi-line texts and mathtext ignore them, and
text is measured without them, so aligned text is placed by its unfeatured
width.

The bundled fonts provide: DejaVu Sans (ligatures, ``dlig``, ``salt``,
``case``, kerning, a Catalan language system without ligatures and a
Serbian one with its own б), DejaVu Serif Italic (Serbian and Macedonian
forms of б г д п т, the Turkic language systems without the ``fi``
ligature, the Sami ones with another Eng) and STIX General (ligatures,
kerning).
"""

from matplotlib import patheffects as pe

from spec_support import case

SAMPLE = "AVAVA office fi first"
SERBIAN = "бгдпт ѓ"
LATIN = "¡fi ffl Ŋ!"
ITALIC = {"family": "DejaVu Serif", "style": "italic"}


def _rows(fig, rows, size=14, top=0.92, step=0.11, x=0.03):
    """One text per row: ``(text, font kwargs, features, language)``, top
    to bottom; ``features``/``language`` are set when not ``None``."""
    for i, (text, font, features, language) in enumerate(rows):
        t = fig.text(x, top - step * i, text, fontsize=size, **font)
        if features is not None:
            t.set_fontfeatures(features)
        if language is not None:
            t.set_language(language)


@case("shaping/features", figsize=(6, 4))
def _(fig):
    sans, stix = {"family": "DejaVu Sans"}, {"family": "STIXGeneral"}
    _rows(fig, [
        (SAMPLE, sans, None, None),
        (SAMPLE, sans, ["-kern"], None),
        (SAMPLE, sans, ["-liga", "dlig"], None),
        (SAMPLE, sans, ["-kern[0:3]", "liga=0", "liga[13:]"], None),
        ("Ialy ¡Ŋ! ¿Ola?", sans, ["salt", "case"], None),
        ("fi fl ffi ffl AV To", stix, None, None),
        ("fi fl ffi ffl AV To", stix, ["-liga", "-kern"], None),
        ("gag ¡fi! office", ITALIC, ["aalt", "case", "aalt[2]=0"], None),
    ])
    # Aligned by the width measured without the features.
    for i, ha in enumerate(["right", "center"]):
        t = fig.text(0.97 - 0.35 * i, 0.04, "WAVE To", ha=ha, fontsize=14)
        t.set_fontfeatures(["-kern"])


@case("shaping/languages", figsize=(6, 4))
def _(fig):
    _rows(fig, [
        (SERBIAN, ITALIC, None, None),
        (SERBIAN, ITALIC, None, "sr"),
        (SERBIAN, ITALIC, None, "mk"),
        (SERBIAN, ITALIC, None, "SR_rs"),
        (SERBIAN, ITALIC, None, "x-hbotsrb"),
        (SERBIAN, ITALIC, None, "ru"),
    ], size=16, step=0.12)
    _rows(fig, [
        (LATIN, ITALIC, ["case"], None),
        (LATIN, ITALIC, ["case"], "tr"),
        (LATIN, ITALIC, ["case"], "se"),
        (LATIN, ITALIC, ["case"], "ca"),
        (LATIN, ITALIC, ["case"], "azb"),
        (LATIN, ITALIC, ["case"], "trk"),
    ], size=16, step=0.12, x=0.55)
    fig.text(0.03, 0.12, "office бгд", fontsize=16).set_language("sr")
    fig.text(0.55, 0.12, "office бгд", fontsize=16).set_language("ca")


@case("shaping/language_ranges", figsize=(6, 3.5))
def _(fig):
    words = "бгдпт бгдпт"
    _rows(fig, [
        (words, ITALIC, None, [("sr", 0, 5)]),
        (words, ITALIC, None, [("sr", 0, 11), ("ru", 3, 8)]),
        (words, ITALIC, None, [("mk", 6, 11), ("sr", 4, 2)]),
        (words, ITALIC, None, []),
        ("fi бгд fi", ITALIC, None, [("tr", 0, 3), ("sr", 3, 7)]),
    ], size=15, top=0.88, step=0.15)
    _rows(fig, [
        ("office office", {}, None, [("ca", 0, 6)]),
        ("office AV", {}, None, [("en", 0, 2)]),
        ("office AV", {}, None, [("ca", 2, 4)]),
    ], size=15, top=0.88, step=0.15, x=0.6)


@case("shaping/rc_language", figsize=(5, 4),
      rc={"text.language": "sr", "font.family": "DejaVu Serif", "font.style": "italic"})
def _(fig):
    ax = fig.subplots()
    ax.bar(["бг", "дп", "тб"], [3, 1, 2],
           label="гдп")
    ax.set_title("Наслов бгдпт")
    ax.set_xlabel("пут")
    ax.set_ylabel("дуг")
    ax.legend(loc="upper right")
    ax.text(1.0, 2.6, "бгдпт ru", ha="center").set_language("ru")
    ax.text(1.0, 2.0, "бгдпт\nбгд", ha="center")
    ax.annotate("гдп", xy=(2, 2), xytext=(1.6, 1.4),
                arrowprops={"arrowstyle": "->"})


@case("shaping/single_lines_only", figsize=(5, 3.5))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.set_axis_off()
    multi = ax.text(0.3, 9, "office AV\nбгд fi", fontsize=13, va="top")
    multi.set_fontfeatures(["-liga", "-kern"])
    multi.set_language("sr")
    math = ax.text(5.5, 8.6, r"office $\alpha$ fi", fontsize=13)
    math.set_fontfeatures(["-liga"])
    stroked = ax.text(0.3, 5, "office бгд", fontsize=18, **ITALIC,
                      path_effects=[pe.withStroke(linewidth=3, foreground="0.7")])
    stroked.set_fontfeatures(["-liga"])
    stroked.set_language([("sr", 7, 10)])
    rotated = ax.text(8, 1, "office бгд", fontsize=14, rotation=30, **ITALIC)
    rotated.set_fontfeatures(["-liga"])
    rotated.set_language("mk")
    note = ax.annotate("office fi", xy=(5, 2), xytext=(1, 1.2), fontsize=13,
                       arrowprops={"arrowstyle": "->"})
    note.set_fontfeatures(["-liga"])


@case("shaping/hinted", figsize=(5, 2.5), rc={"text.hinting": "default"})
def _(fig):
    _rows(fig, [
        (SAMPLE, {}, ["-kern", "-liga"], None),
        (SERBIAN, ITALIC, None, "sr"),
        (LATIN, ITALIC, ["case"], "tr"),
    ], top=0.75, step=0.3)
