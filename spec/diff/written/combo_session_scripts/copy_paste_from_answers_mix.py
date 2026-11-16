# A script assembled from several snippets: rotated dates, a legend outside, value labels, twin axis and tight bbox save.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

rng = np.random.default_rng(245)
idx = pd.date_range("2024-01-01", periods=14, freq="W")
vals = rng.integers(20, 80, 14)
fig, ax = plt.subplots(figsize=(9, 4))
bars = ax.bar(idx, vals, width=5, label="orders")
ax.bar_label(bars, padding=2, fontsize=7)
ax.xaxis.set_major_formatter(mdates.DateFormatter("%d %b"))
plt.setp(ax.get_xticklabels(), rotation=45, ha="right")
ax2 = ax.twinx()
ax2.plot(idx, np.cumsum(vals), "k--", label="cumulative")
h1, l1 = ax.get_legend_handles_labels(); h2, l2 = ax2.get_legend_handles_labels()
ax.legend(h1 + h2, l1 + l2, loc="center left", bbox_to_anchor=(1.08, 0.5))
fig.savefig("snippets.png", dpi=50, bbox_inches="tight")
print([t.get_text() for t in ax.get_xticklabels()][:3], int(np.cumsum(vals)[-1]))
