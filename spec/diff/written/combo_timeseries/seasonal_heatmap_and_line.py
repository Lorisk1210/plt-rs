# Seasonality: a day-of-week x hour heat map from timestamps, the weekly profile line, and a monthly bar summary.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(52)
idx = pd.date_range("2024-01-01", "2024-06-30 23:00", freq="h")
load = pd.Series(50 + 20 * np.sin((idx.hour.to_numpy() - 7) / 24 * 2 * np.pi) - 8 * (idx.weekday.to_numpy() >= 5)
                 + 3 * np.sin(idx.dayofyear.to_numpy() / 30) + rng.normal(0, 3, len(idx)), index=idx)
pivot = load.groupby([load.index.weekday, load.index.hour]).mean().unstack()
fig = plt.figure(figsize=(11, 7))
ax1 = fig.add_subplot(2, 2, (1, 2))
im = ax1.imshow(pivot.values, aspect="auto", cmap="viridis")
ax1.set_yticks(range(7), ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
ax1.set_xlabel("hour")
fig.colorbar(im, ax=ax1)
ax2 = fig.add_subplot(2, 2, 3)
for d in range(7):
    ax2.plot(pivot.columns, pivot.loc[d], lw=2 if d >= 5 else 0.8, label=str(d))
ax3 = fig.add_subplot(2, 2, 4)
monthly = load.resample("MS").mean()
ax3.bar([m.strftime("%b") for m in monthly.index], monthly.values, color="C2")
ax3.set_ylim(monthly.min() * 0.95, monthly.max() * 1.02)
fig.tight_layout()
print(pivot.shape, monthly.round(3).tolist())
fig.savefig("seasonal.png", dpi=50)
