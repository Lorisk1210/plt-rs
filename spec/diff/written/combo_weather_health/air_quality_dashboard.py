# Air quality: pollutant lines with WHO limit spans, an AQI category bar per day coloured by level, and a diurnal box plot.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap, BoundaryNorm

rng = np.random.default_rng(151)
idx = pd.date_range("2024-01-01", periods=24 * 30, freq="h")
pm25 = np.clip(15 + 10 * np.sin(idx.hour.to_numpy() / 24 * 2 * np.pi) + rng.gamma(2, 4, len(idx)), 0, None)
no2 = np.clip(25 + 15 * np.cos(idx.hour.to_numpy() / 24 * 2 * np.pi) + rng.normal(0, 5, len(idx)), 0, None)
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
a.plot(idx, pm25, lw=0.6, label="PM2.5")
a.plot(idx, no2, lw=0.6, label="NO2")
a.axhspan(0, 15, color="green", alpha=0.08)
a.axhline(25, color="r", ls="--", label="limit")
a.legend(ncols=3)
daily = pd.Series(pm25, index=idx).resample("D").mean()
cmap = ListedColormap(["#00e400", "#ffff00", "#ff7e00", "#ff0000"])
norm = BoundaryNorm([0, 12, 25, 35, 100], cmap.N)
b = fig.add_subplot(2, 2, 3)
b.bar(daily.index, daily.values, color=cmap(norm(daily.values)), width=0.9)
b.tick_params(axis="x", rotation=45)
c = fig.add_subplot(2, 2, 4)
c.boxplot([pm25[idx.hour == h] for h in range(0, 24, 3)], tick_labels=[f"{h}h" for h in range(0, 24, 3)])
fig.tight_layout()
print(daily.round(2).head(3).tolist(), np.bincount(norm(daily.values), minlength=4).tolist())
fig.savefig("air.png", dpi=50)
