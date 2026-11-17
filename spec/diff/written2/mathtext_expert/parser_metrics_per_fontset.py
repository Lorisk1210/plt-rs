# MathTextParser("path") metrics of a set of expressions under every mathtext.fontset, at two sizes.
import matplotlib
from matplotlib.mathtext import MathTextParser
from matplotlib.font_manager import FontProperties

exprs = [r"$x$", r"$x^2_i$", r"$\frac{a}{b+c}$", r"$\sqrt[3]{x+1}$", r"$\sum_{k=0}^{n} k^2$", r"$\int_0^\infty e^{-x}\,dx$",
         r"$\alpha\beta\gamma\Gamma$", r"$\mathrm{d}\mathbf{x}/\mathit{dt}$"]
parser = MathTextParser("path")
for fontset in ("dejavusans", "dejavuserif", "cm", "stix", "stixsans"):
    with matplotlib.rc_context({"mathtext.fontset": fontset}):
        for size in (10, 18):
            fp = FontProperties(size=size)
            row = []
            for e in exprs:
                width, height, depth, glyphs, rects = parser.parse(e, 72, fp)
                row.append((round(width, 2), round(height, 2), round(depth, 2), len(glyphs), len(rects)))
            print(fontset, size, row)
agg = MathTextParser("agg")
out = agg.parse(r"$\frac{1}{2}$", 100, FontProperties(size=12))
print(type(out).__name__, out.width, out.height, out.depth, out.image.shape if hasattr(out, "image") else None)
for dpi in (72, 100, 300):
    w, h, d, glyphs, rects = parser.parse(r"$\frac{a}{b}$", dpi, FontProperties(size=12))
    print(dpi, round(w, 3), round(h, 3), round(d, 3), [round(g[3], 2) for g in glyphs][:2])
first = parser.parse(r"$x^2$", 72, FontProperties(size=12))
again = parser.parse(r"$x^2$", 72, FontProperties(size=12))
print(first == again, type(first).__name__, first._fields if hasattr(first, "_fields") else len(first))
with matplotlib.rc_context({"mathtext.default": "regular"}):
    print(round(parser.parse(r"$xyz$", 72, FontProperties(size=12))[0], 3))
with matplotlib.rc_context({"mathtext.default": "it"}):
    print(round(parser.parse(r"$xyz$", 72, FontProperties(size=12))[0], 3))
print(round(parser.parse(r"$\mathdefault{xyz}$", 72, FontProperties(size=12))[0], 3))
