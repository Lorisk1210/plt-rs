# Ticks at multiples of pi with FuncFormatter fractions, MultipleLocator view_limits, and locator.nonsingular/raise_if_exceeds.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from fractions import Fraction
from matplotlib import ticker


def pi_fmt(x, pos):
    f = Fraction(x / np.pi).limit_denominator(8)
    if f == 0:
        return "0"
    num = "" if abs(f.numerator) == 1 else str(abs(f.numerator))
    sign = "-" if f < 0 else ""
    return f"{sign}{num}π" if f.denominator == 1 else f"{sign}{num}π/{f.denominator}"


fig, ax = plt.subplots(figsize=(6, 2), dpi=60)
x = np.linspace(-2 * np.pi, 2 * np.pi, 200)
ax.plot(x, np.sin(x))
ax.xaxis.set_major_locator(ticker.MultipleLocator(np.pi / 2))
ax.xaxis.set_minor_locator(ticker.MultipleLocator(np.pi / 8))
ax.xaxis.set_major_formatter(ticker.FuncFormatter(pi_fmt))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
print(len(ax.xaxis.get_minorticklocs()))
loc = ticker.MultipleLocator(np.pi / 2)
print([round(v, 4) for v in loc.view_limits(0.3, 5.0)], [round(v, 4) for v in loc.view_limits(-1, -1)])
mx = ticker.MaxNLocator()
mx.set_axis(ax.xaxis)
lg = ticker.LogLocator()
lg.set_axis(ax.yaxis)
print([round(v, 4) for v in mx.nonsingular(1, 1)], [round(v, 4) for v in lg.nonsingular(0, 10)])
ml = ticker.MultipleLocator(1e-6)
ml.set_axis(ax.xaxis)
try:
    ml.raise_if_exceeds(np.arange(5000))
    print("no exceed")
except RuntimeError as e:
    print("RuntimeError", str(e)[:60])
with plt.rc_context({"axes.autolimit_mode": "round_numbers"}):
    f2, a2 = plt.subplots()
    a2.plot([0.3, 9.6], [0.12, 0.97])
    print([round(v, 3) for v in a2.get_xlim()], [round(v, 3) for v in a2.get_ylim()])
ax.set_xlim(-7, 7)
ax.xaxis.set_major_locator(ticker.MultipleLocator(np.pi, offset=np.pi / 2))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("pi_ticks.png")
