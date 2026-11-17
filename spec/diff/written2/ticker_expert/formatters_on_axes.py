# Formatters installed on a real axis: ScalarFormatter offsets/sci/useMathText, EngFormatter, PercentFormatter, LogFormatterSciNotation.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker

fig, axs = plt.subplots(3, 3, figsize=(9, 7), dpi=50)
specs = [
    ("scalar offset", (1e6, 1e6 + 5), ticker.ScalarFormatter(useOffset=True), "linear"),
    ("scalar sci", (0, 3e-7), ticker.ScalarFormatter(useMathText=True), "linear"),
    ("no offset", (1000, 1001), ticker.ScalarFormatter(useOffset=False), "linear"),
    ("eng", (0, 4.5e4), ticker.EngFormatter(unit="Hz", places=1), "linear"),
    ("eng sep", (1e-9, 1e-6), ticker.EngFormatter(sep="", usetex=False), "linear"),
    ("percent", (0, 0.42), ticker.PercentFormatter(xmax=1, decimals=1), "linear"),
    ("logsci", (1e-3, 1e5), ticker.LogFormatterSciNotation(), "log"),
    ("logexp", (1, 1e8), ticker.LogFormatterExponent(), "log"),
    ("strmethod", (-2, 2), ticker.StrMethodFormatter("{x:+.1f}°"), "linear"),
]
for ax, (name, lim, fmt, scale) in zip(axs.flat, specs):
    ax.set_yscale(scale)
    ax.set_ylim(*lim)
    ax.yaxis.set_major_formatter(fmt)
    fig.canvas.draw()
    print(name, [t.get_text() for t in ax.get_yticklabels()][:4], repr(ax.yaxis.get_offset_text().get_text()))
sf = ticker.ScalarFormatter()
sf.set_powerlimits((-2, 2))
axs[0, 0].yaxis.set_major_formatter(sf)
axs[0, 0].set_ylim(0, 5000)
fig.canvas.draw()
print([t.get_text() for t in axs[0, 0].get_yticklabels()][:3], axs[0, 0].yaxis.get_offset_text().get_text(), sf.orderOfMagnitude)
print(sf.format_data(12345.678), sf.format_data_short(12345.678), sf.get_useOffset(), sf.get_useLocale())
fig.savefig("formatters.png")
