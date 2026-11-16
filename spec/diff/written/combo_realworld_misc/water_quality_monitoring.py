# Water quality: parameter time series in small multiples with limit lines, exceedance counts bar, and a site x parameter heat map.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(238)
idx = pd.date_range("2024-01-01", periods=52, freq="W")
params = {"pH": (7.2, 0.3, 8.5), "nitrate": (20, 8, 40), "turbidity": (3, 2, 8), "oxygen": (8, 1, 12)}
fig = plt.figure(figsize=(12, 8))
exceed = {}
for k, (p, (m, s, lim)) in enumerate(params.items()):
    ax = fig.add_subplot(3, 2, k + 1)
    v = np.abs(rng.normal(m, s, 52))
    ax.plot(idx, v, lw=0.8); ax.axhline(lim, color="r", ls="--"); ax.set_title(p, fontsize=9)
    exceed[p] = int((v > lim).sum())
ax = fig.add_subplot(3, 2, 5)
ax.bar(list(exceed), list(exceed.values()), color="C3")
ax = fig.add_subplot(3, 2, 6)
sites = rng.random((4, 4))
ax.imshow(sites, cmap="RdYlGn_r", vmin=0, vmax=1)
ax.set_xticks(range(4), list(params), rotation=30); ax.set_yticks(range(4), [f"site {i}" for i in range(4)])
fig.autofmt_xdate()
fig.tight_layout()
print(exceed)
fig.savefig("water.png", dpi=50)
