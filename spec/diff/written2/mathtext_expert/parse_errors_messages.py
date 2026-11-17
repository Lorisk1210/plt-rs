# Malformed mathtext: which expressions raise ValueError, the first line of each message, and that text with them fails to draw.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.mathtext import MathTextParser
from matplotlib.font_manager import FontProperties

bad = [r"$\frac{1}$", r"$x^$", r"$\undefinedcommand$", r"${x$", r"$x}$", r"$\sqrt[}{x}$", r"$x^2^3$", r"$x_1_2$",
       r"$\left( x$", r"$\right) x$", r"$a \over b \over c$", r"$\mathrm$", r"$\hat$", r"$\fracab$", r"$$"]
parser = MathTextParser("path")
for e in bad:
    try:
        w, h, d, glyphs, rects = parser.parse(e, 72, FontProperties())
        print("ok", repr(e), round(w, 2), len(glyphs))
    except ValueError as err:
        lines = str(err).splitlines()
        print("ValueError", repr(e), lines[0][:60] if lines else "", len(lines))
fig, ax = plt.subplots()
t = ax.text(0.5, 0.5, r"$\frac{1}$")
try:
    fig.canvas.draw()
except ValueError as err:
    print("draw:", str(err).splitlines()[-1][:60])
t.set_text(r"\$5 and \$10 (escaped dollars)")
fig.canvas.draw()
print(round(t.get_window_extent().width, 2), t._preprocess_math(t.get_text()))
t.set_text("odd $ count")
fig.canvas.draw()
print(t._preprocess_math(t.get_text()))
t.set_text("price: $5$ and $x$")
fig.canvas.draw()
print(t._preprocess_math(t.get_text()))
t.set_parse_math(False)
print(t._preprocess_math(r"$\frac{1}$"))
fig.savefig("errors.png")
