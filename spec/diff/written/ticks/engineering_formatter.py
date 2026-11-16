# Frequency response with SI prefixes (kHz, MHz) using EngFormatter on a log axis.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import EngFormatter

f = np.logspace(1, 7, 300)
gain = 20 * np.log10(1 / np.sqrt(1 + (f / 3e4) ** 2))
fig, ax = plt.subplots(figsize=(6.5, 3.5))
ax.semilogx(f, gain)
ax.xaxis.set_major_formatter(EngFormatter(unit="Hz"))
ax.yaxis.set_major_formatter(EngFormatter(unit="dB", places=0, sep="\N{THIN SPACE}"))
ax.set_title("Low-pass filter")
ax.grid(True, which="both", alpha=0.3)
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("filter.png")
