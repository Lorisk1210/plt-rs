# Formatters mixed: percent y on a bar chart, engineering x on a log line plot, and scientific colour bar labels.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter, EngFormatter, LogFormatterSciNotation

fig, axs = plt.subplots(1, 3, figsize=(13, 3.8))
axs[0].bar(["a", "b", "c"], [0.12, 0.45, 0.31])
axs[0].yaxis.set_major_formatter(PercentFormatter(1))
f = np.logspace(2, 8, 100)
axs[1].semilogx(f, 1 / np.sqrt(1 + (f / 1e5) ** 2))
axs[1].xaxis.set_major_formatter(EngFormatter("Hz"))
im = axs[2].imshow(np.logspace(0, 6, 36).reshape(6, 6), norm="log")
cb = fig.colorbar(im, ax=axs[2])
cb.ax.yaxis.set_major_formatter(LogFormatterSciNotation())
fig.tight_layout()
fig.canvas.draw()
print([t.get_text() for t in axs[0].get_yticklabels()][:3], [t.get_text() for t in axs[1].get_xticklabels()][1:4])
print([t.get_text() for t in cb.ax.get_yticklabels()][:3])
fig.savefig("formatters_mix.png", dpi=50)
