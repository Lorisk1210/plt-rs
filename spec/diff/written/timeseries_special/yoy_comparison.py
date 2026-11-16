# Year-over-year comparison: each year's daily series plotted against day of year.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
idx = pd.date_range("2021-01-01", "2024-12-31", freq="D")
s = pd.Series(20 + 8 * np.sin((idx.dayofyear - 100) / 365 * 2 * np.pi) + (idx.year - 2021) * 0.5 + rng.normal(0, 1.5, len(idx)), index=idx)
fig, ax = plt.subplots(figsize=(8, 4))
for year, g in s.groupby(s.index.year):
    smooth = g.rolling(14, center=True).mean()
    ax.plot(g.index.dayofyear, smooth, label=str(year), lw=2 if year == 2024 else 1)
ax.legend(title="year")
ax.set_xlabel("day of year")
print(s.groupby(s.index.year).mean().round(3).to_dict())
fig.savefig("yoy.png")
