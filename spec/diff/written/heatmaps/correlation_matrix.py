# Correlation matrix of a DataFrame as a masked lower-triangle heat map with diverging colours.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(1)
base = rng.normal(size=(200, 1))
df = pd.DataFrame(np.hstack([base + rng.normal(0, s, (200, 1)) for s in (0.3, 0.6, 1.0, 2.0, 4.0)]),
                  columns=["a", "b", "c", "d", "e"])
corr = df.corr()
mask = np.triu(np.ones_like(corr, dtype=bool))
fig, ax = plt.subplots(figsize=(5, 4.5))
im = ax.imshow(np.ma.masked_where(mask, corr), cmap="RdBu_r", vmin=-1, vmax=1)
ax.set_xticks(range(5), corr.columns)
ax.set_yticks(range(5), corr.columns)
for i in range(5):
    for j in range(i):
        ax.text(j, i, f"{corr.iloc[i, j]:.2f}", ha="center", va="center", fontsize=9)
fig.colorbar(im, ax=ax, shrink=0.8)
ax.spines[:].set_visible(False)
print(corr.round(3).iloc[1, 0])
fig.savefig("corr.png")
