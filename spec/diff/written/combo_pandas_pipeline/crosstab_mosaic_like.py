# Mosaic-like plot from a crosstab: column widths by group size, stacked heights by share, labels with counts.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(117)
df = pd.DataFrame({"plan": rng.choice(["free", "pro", "team"], 500, p=[0.6, 0.3, 0.1]),
                   "churned": rng.choice(["yes", "no"], 500, p=[0.3, 0.7])})
ct = pd.crosstab(df["plan"], df["churned"])
widths = ct.sum(axis=1) / ct.values.sum()
fig, ax = plt.subplots(figsize=(7, 4))
left = 0
for plan, w in widths.items():
    bottom = 0
    for col, color in [("no", "C2"), ("yes", "C3")]:
        h = ct.loc[plan, col] / ct.loc[plan].sum()
        ax.bar(left, h, width=w, bottom=bottom, align="edge", color=color, edgecolor="w")
        ax.text(left + w / 2, bottom + h / 2, str(ct.loc[plan, col]), ha="center", va="center", color="w")
        bottom += h
    ax.text(left + w / 2, -0.06, plan, ha="center")
    left += w
ax.set_xlim(0, 1); ax.set_ylim(-0.1, 1); ax.set_xticks([])
print(ct.to_dict(), widths.round(3).tolist())
fig.savefig("mosaic.png", dpi=50)
