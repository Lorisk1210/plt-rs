# Hospital occupancy: stacked area of ward types vs capacity line, a heat map of occupancy % by ward and week, and alerts.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(154)
days = pd.date_range("2024-01-01", periods=84, freq="D")
wards = {"ICU": 20, "general": 120, "surgery": 40}
occ = {w: np.clip(cap * (0.7 + 0.2 * np.sin(np.arange(84) / 10 + i) + rng.normal(0, 0.05, 84)), 0, cap * 1.1) for i, (w, cap) in enumerate(wards.items())}
fig, (a, b) = plt.subplots(2, 1, figsize=(10, 7))
a.stackplot(days, *occ.values(), labels=list(wards), alpha=0.8)
a.axhline(sum(wards.values()), color="k", ls="--", label="capacity")
a.legend(loc="upper left", ncols=4)
weekly = np.array([[occ[w][k * 7:(k + 1) * 7].mean() / cap * 100 for k in range(12)] for w, cap in wards.items()])
im = b.imshow(weekly, aspect="auto", cmap="RdYlGn_r", vmin=50, vmax=110)
b.set_yticks(range(3), list(wards))
b.set_xlabel("week")
for (i, j), v in np.ndenumerate(weekly):
    if v > 95:
        b.text(j, i, "!", ha="center", va="center", fontweight="bold")
fig.colorbar(im, ax=b, label="occupancy %")
print(np.round(weekly.max(axis=1), 2).tolist(), len(b.texts))
fig.savefig("capacity.png", dpi=50)
