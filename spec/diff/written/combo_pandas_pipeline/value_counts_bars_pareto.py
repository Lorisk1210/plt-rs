# Pareto chart from value_counts: bars sorted descending, cumulative percent line on a twin axis, 80% threshold.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

rng = np.random.default_rng(115)
causes = pd.Series(rng.choice(["scratch", "dent", "crack", "stain", "warp", "chip", "other"], 400,
                              p=[0.35, 0.25, 0.15, 0.1, 0.07, 0.05, 0.03]))
counts = causes.value_counts()
cum = counts.cumsum() / counts.sum()
fig, ax = plt.subplots(figsize=(7, 4))
counts.plot.bar(ax=ax, color="C0", rot=30)
ax2 = ax.twinx()
ax2.plot(range(len(cum)), cum.values, "ro-")
ax2.yaxis.set_major_formatter(PercentFormatter(1))
ax2.axhline(0.8, color="gray", ls="--")
ax2.set_ylim(0, 1.05)
print(counts.to_dict(), cum.round(3).tolist())
fig.tight_layout()
fig.savefig("pareto.png", dpi=50)
