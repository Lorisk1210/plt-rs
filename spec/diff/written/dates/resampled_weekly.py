# Resample a minute-level pandas series to hourly means and daily maxima and plot both.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2024-06-01", periods=3 * 24 * 60, freq="min")
rng = np.random.default_rng(21)
load = pd.Series(50 + 20 * np.sin(np.arange(idx.size) * 2 * np.pi / 1440) + rng.normal(0, 4, idx.size), index=idx)
hourly = load.resample("h").mean()
daily_max = load.resample("D").max()
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(hourly.index, hourly, label="hourly mean")
ax.step(daily_max.index, daily_max, where="post", label="daily max")
ax.legend()
ax.set_ylabel("load [%]")
print(len(hourly), daily_max.round(2).tolist())
fig.autofmt_xdate(rotation=30)
fig.savefig("load.png")
