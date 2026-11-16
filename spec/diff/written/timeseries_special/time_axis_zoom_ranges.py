# The same series zoomed to a year, a month and a day with automatic date ticks in each panel.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2024-01-01", periods=24 * 366, freq="h")
s = pd.Series(np.sin(np.arange(len(idx)) * 2 * np.pi / 24) + np.arange(len(idx)) / 2000, index=idx)
fig, axs = plt.subplots(3, 1, figsize=(8, 7))
for ax, (start, end) in zip(axs, [("2024-01-01", "2024-12-31"), ("2024-06-01", "2024-06-30"), ("2024-06-15", "2024-06-16")]):
    part = s[start:end]
    ax.plot(part.index, part.values, lw=0.6)
    ax.set_title(f"{start} .. {end}", fontsize=9)
fig.tight_layout()
fig.canvas.draw()
print([[t.get_text() for t in ax.get_xticklabels()][:2] for ax in axs])
fig.savefig("zooms.png")
