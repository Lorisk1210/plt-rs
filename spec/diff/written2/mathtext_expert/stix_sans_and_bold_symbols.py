# Greek, blackboard and calligraphic alphabets in each fontset: glyph counts and extents, plus missing-symbol substitutions.
import warnings
import matplotlib
from matplotlib.mathtext import MathTextParser
from matplotlib.font_manager import FontProperties

alphabets = {
    "greek": r"$" + r"\,".join(r"\%s" % g for g in "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu".split()) + r"$",
    "Greek": r"$\Gamma\Delta\Theta\Lambda\Xi\Pi\Sigma\Upsilon\Phi\Psi\Omega$",
    "bb": r"$\mathbb{ABCRZ}\ \mathbb{abc}\ \mathbb{123}$",
    "cal": r"$\mathcal{ABCDEFG}$",
    "frak": r"$\mathfrak{ABCabc}$",
    "bf": r"$\mathbf{\alpha\beta}\ \boldsymbol{\alpha\beta}$",
    "sf": r"$\mathsf{ABC123}$",
}
parser = MathTextParser("path")
for fontset in ("dejavusans", "dejavuserif", "cm", "stix", "stixsans"):
    with matplotlib.rc_context({"mathtext.fontset": fontset}), warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        out = []
        for name, e in alphabets.items():
            w, h, d, glyphs, rects = parser.parse(e, 72, FontProperties(size=12))
            fonts = sorted({g[0].family_name for g in glyphs})
            out.append((name, round(w, 1), len(glyphs), fonts))
        print(fontset, out)
        print(fontset, sorted({str(c.message)[:50] for c in caught}))
with matplotlib.rc_context({"mathtext.fontset": "cm"}):
    w, h, d, glyphs, rects = parser.parse(r"$\mathbb{R}$", 72, FontProperties(size=12))
    print([g[0].family_name for g in glyphs], [chr(g[2]) if isinstance(g[2], int) else g[2] for g in glyphs])
w, h, d, glyphs, rects = parser.parse(r"$\alpha$", 72, FontProperties(size=12))
print(len(glyphs[0]), round(glyphs[0][3], 2), round(w, 3))
w2 = parser.parse(r"$\alpha$", 72, FontProperties(size=24))[0]
print(round(w2 / w, 3))
