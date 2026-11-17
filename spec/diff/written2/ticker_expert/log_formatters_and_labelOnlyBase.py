# LogFormatter variants on log axes with labelOnlyBase, minor_thresholds and base 2/e; label texts at several spans.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker

fig, ax = plt.subplots(figsize=(5, 2), dpi=60)
for base in (10, 2, np.e):
    ax.set_xscale("log", base=base)
    for lo, hi in [(1, 10), (1, 1e3), (0.5, 3), (1e-2, 1e6)]:
        ax.set_xlim(lo, hi)
        fig.canvas.draw()
        major = [t.get_text() for t in ax.get_xticklabels()][:4]
        minor = [t.get_text() for t in ax.get_xticklabels(minor=True) if t.get_text()][:3]
        print(round(base, 3), (lo, hi), major, minor)
for cls in (ticker.LogFormatter, ticker.LogFormatterExponent, ticker.LogFormatterMathtext, ticker.LogFormatterSciNotation):
    f = cls(base=10, labelOnlyBase=False, minor_thresholds=(1, 0.5))
    ax.set_xscale("log")
    ax.set_xlim(1, 100)
    ax.xaxis.set_major_formatter(f)
    ax.xaxis.set_minor_formatter(cls(base=10, labelOnlyBase=False))
    fig.canvas.draw()
    print(cls.__name__, [t.get_text() for t in ax.get_xticklabels()][:3], f(20), f.format_data(1234.5), f.format_data_short(0.05))
fig.savefig("logfmt.png")
ax.set_xlim(1, 1e12)
ax.xaxis.set_major_formatter(ticker.LogFormatterSciNotation())
ax.xaxis.set_major_locator(ticker.LogLocator(numticks=4))
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()], ax.get_xticks().tolist()[:3])
